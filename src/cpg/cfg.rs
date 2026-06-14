use tree_sitter::Node;

use crate::cpg::cfg_nodes::{end_line, if_bodies, line, stmt_seq_node, unwrap_stmt, when_entry_body};
use crate::cpg::defuse::{self, DefUseRecord};
use crate::walk::{find_child_by_kinds, DepthGuard};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeLabel {
    Epsilon,
    True,
    False,
    Back,
    ToHandler,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CfgEdge {
    pub from: u32,
    pub to: u32,
    pub label: EdgeLabel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    Entry,
    Exit,
    Stmt,
    Def,
    Join,
    Predicate,
    LoopHead,
    Handler,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CfgNode {
    pub id: u32,
    pub line: u32,
    pub kind: NodeKind,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Cfg {
    pub nodes: Vec<CfgNode>,
    pub edges: Vec<CfgEdge>,
    pub entry: u32,
    pub exit: u32,
}

pub struct NestedKinds {
    pub fns: &'static [&'static str],
    pub items: &'static [&'static str],
}

#[allow(clippy::struct_field_names)]
pub struct CfgLang {
    pub if_kinds: &'static [&'static str],
    pub loop_kinds: &'static [&'static str],
    pub return_kinds: &'static [&'static str],
    pub break_kinds: &'static [&'static str],
    pub continue_kinds: &'static [&'static str],
    pub try_kinds: &'static [&'static str],
    pub handler_kinds: &'static [&'static str],
    pub def_kinds: &'static [&'static str],
    pub aug_kinds: &'static [&'static str],
    pub block_kinds: &'static [&'static str],
    pub nested: NestedKinds,
    pub switch_kinds: &'static [&'static str],
    pub case_kinds: &'static [&'static str],
    pub hoist_kinds: &'static [&'static str],
}

pub(super) type Incoming = Option<(u32, EdgeLabel)>;

#[derive(Clone, Copy)]
enum JumpTo {
    Exit,
    Break,
    Continue,
}

struct LoopCtx {
    head: u32,
    after: u32,
}

pub(super) struct Builder<'a> {
    pub(super) lang: &'a CfgLang,
    pub(super) source: &'a str,
    pub(super) nodes: Vec<CfgNode>,
    pub(super) edges: Vec<CfgEdge>,
    loops: Vec<LoopCtx>,
    pub(super) def_use: Vec<DefUseRecord>,
    entry: u32,
    exit: u32,
    pub(super) labels: Vec<(String, u32)>,
    pub(super) pending_gotos: Vec<(u32, String)>,
}

pub fn build_cfg(body: Node, source: &str, lang: &CfgLang) -> (Cfg, Vec<DefUseRecord>) {
    let mut b = Builder {
        lang,
        source,
        nodes: Vec::new(),
        edges: Vec::new(),
        loops: Vec::new(),
        def_use: Vec::new(),
        entry: 0,
        exit: 0,
        labels: Vec::new(),
        pending_gotos: Vec::new(),
    };
    let entry = b.add(NodeKind::Entry, line(body));
    let exit = b.add(NodeKind::Exit, end_line(body));
    b.entry = entry;
    b.exit = exit;
    if let Some(e) = b.seq(body, Some((entry, EdgeLabel::Epsilon))) {
        b.edge(e, exit, EdgeLabel::Epsilon);
    }
    b.resolve_gotos();
    (Cfg { nodes: b.nodes, edges: b.edges, entry, exit }, b.def_use)
}

impl Builder<'_> {
    pub(super) fn add(&mut self, kind: NodeKind, line: u32) -> u32 {
        let id = self.nodes.len() as u32;
        self.nodes.push(CfgNode { id, line, kind });
        id
    }

    pub(super) fn edge(&mut self, from: u32, to: u32, label: EdgeLabel) {
        self.edges.push(CfgEdge { from, to, label });
    }

    pub(super) fn link(&mut self, incoming: Incoming, to: u32) {
        if let Some((from, label)) = incoming {
            self.edge(from, to, label);
        }
    }

    fn seq(&mut self, block: Node, incoming: Incoming) -> Option<u32> {
        if self.is_dispatched_stmt(block.kind()) {
            return self.stmt(block, incoming);
        }
        let seq_node = stmt_seq_node(block);
        let mut cursor = seq_node.walk();
        let nodes: Vec<Node> = seq_node.children(&mut cursor).collect();
        self.thread_stmts(&nodes, incoming)
    }

    fn thread_stmts(&mut self, nodes: &[Node], incoming: Incoming) -> Option<u32> {
        let _g = DepthGuard::enter()?;
        let mut cur = incoming;
        for &child in nodes {
            if !child.is_named() || child.is_extra() {
                continue;
            }
            match cur {
                Some(_) => cur = self.stmt(child, cur).map(|id| (id, EdgeLabel::Epsilon)),
                None => {
                    self.stmt(child, None);
                }
            }
        }
        cur.map(|(id, _)| id)
    }

    fn stmt(&mut self, node: Node, incoming: Incoming) -> Option<u32> {
        let _g = DepthGuard::enter()?;
        let node = unwrap_stmt(node);
        let k = node.kind();
        if let Some(result) = self.goto_or_label(node, k, incoming) {
            return result;
        }
        if self.lang.if_kinds.contains(&k) {
            return self.do_if(node, incoming);
        }
        if self.lang.loop_kinds.contains(&k) {
            return self.do_loop(node, incoming);
        }
        if self.lang.try_kinds.contains(&k) {
            return self.do_try(node, incoming);
        }
        if self.lang.switch_kinds.contains(&k) {
            return self.do_switch(node, incoming);
        }
        if let Some(to) = self.jump_kind(k) {
            return self.do_jump(node, incoming, to);
        }
        let is_def = self.lang.nested.fns.contains(&k) || self.lang.nested.items.contains(&k);
        let kind = [NodeKind::Stmt, NodeKind::Def][usize::from(is_def)];
        let n = self.add(kind, line(node));
        self.link(incoming, n);
        if matches!(k, "global_declaration" | "function_static_declaration") {
            defuse::seed_escaping(node, self.source, self.entry, self.exit, &mut self.def_use);
        } else {
            defuse::collect(node, self.source, n, self.lang, &mut self.def_use);
        }
        Some(n)
    }

    fn jump_kind(&self, k: &str) -> Option<JumpTo> {
        if self.lang.return_kinds.contains(&k) {
            return Some(JumpTo::Exit);
        }
        if self.lang.break_kinds.contains(&k) {
            return Some(JumpTo::Break);
        }
        if self.lang.continue_kinds.contains(&k) {
            return Some(JumpTo::Continue);
        }
        None
    }

    fn do_jump(&mut self, node: Node, incoming: Incoming, to: JumpTo) -> Option<u32> {
        let n = self.add(NodeKind::Stmt, line(node));
        self.link(incoming, n);
        defuse::collect(node, self.source, n, self.lang, &mut self.def_use);
        let target = match to {
            JumpTo::Exit => Some(self.exit),
            JumpTo::Break => self.loops.last().map(|l| l.after),
            JumpTo::Continue => self.loops.last().map(|l| l.head),
        };
        if let Some(t) = target {
            self.edge(n, t, EdgeLabel::Epsilon);
        }
        None
    }

    fn record_cond(&mut self, node: Node, block: u32) {
        let mut cursor = node.walk();
        for c in node.children_by_field_name("condition", &mut cursor) {
            if let Some(pat) = c.child_by_field_name("pattern") {
                defuse::push_idents(pat, self.source, self.entry, defuse::Mark::Assign, &mut self.def_use);
                if let Some(val) = c.child_by_field_name("value") {
                    defuse::collect(val, self.source, block, self.lang, &mut self.def_use);
                }
            } else {
                defuse::collect(c, self.source, block, self.lang, &mut self.def_use);
            }
        }
    }

    #[allow(clippy::unnecessary_wraps)]
    fn do_switch(&mut self, node: Node, incoming: Incoming) -> Option<u32> {
        let p = self.add(NodeKind::Predicate, line(node));
        self.link(incoming, p);
        let mut subj_cursor = node.walk();
        for subject in node.children_by_field_name("subject", &mut subj_cursor) {
            defuse::collect(subject, self.source, p, self.lang, &mut self.def_use);
        }
        if let Some(value) = node
            .child_by_field_name("value")
            .or_else(|| node.child_by_field_name("condition"))
            .or_else(|| node.child_by_field_name("expr"))
            .or_else(|| find_child_by_kinds(node, &["when_subject"]))
        {
            defuse::collect(value, self.source, p, self.lang, &mut self.def_use);
        }
        if let Some(init) = node.child_by_field_name("initializer") {
            defuse::collect(init, self.source, p, self.lang, &mut self.def_use);
        }
        if let Some(alias) = node.child_by_field_name("alias") {
            defuse::push_idents(alias, self.source, self.entry, defuse::Mark::Decl, &mut self.def_use);
        }
        let after = self.add(NodeKind::Join, end_line(node));
        self.edge(p, after, EdgeLabel::False);
        let continue_head = self.loops.last().map_or(after, |l| l.head);
        self.loops.push(LoopCtx { head: continue_head, after });
        let body = node.child_by_field_name("body").unwrap_or(node);
        let mut cursor = body.walk();
        let mut fall: Option<u32> = None;
        for case in body.children(&mut cursor) {
            if self.lang.case_kinds.contains(&case.kind()) {
                fall = self.do_case(case, p, after, fall);
            }
        }
        if let Some(end) = fall {
            self.edge(end, after, EdgeLabel::Epsilon);
        }
        self.loops.pop();
        Some(after)
    }

    fn do_case(&mut self, case: Node, p: u32, after: u32, fall_in: Option<u32>) -> Option<u32> {
        if let Some(guard) = case.child_by_field_name("guard").or_else(|| case.child_by_field_name("pattern")) {
            defuse::collect(guard, self.source, p, self.lang, &mut self.def_use);
        }
        if matches!(case.kind(), "switch_case" | "switch_default") {
            let mut body_cursor = case.walk();
            let body_stmts: Vec<Node> = case.children_by_field_name("body", &mut body_cursor).collect();
            return self.do_switch_case(case, p, &body_stmts, fall_in);
        }
        if matches!(case.kind(), "case_clause" | "match_arm" | "when" | "in_clause" | "when_entry") {
            self.record_cond(case, p);
            if let Some(pat) = case.child_by_field_name("pattern") {
                defuse::push_idents(pat, self.source, self.entry, defuse::Mark::Assign, &mut self.def_use);
            }
            let body = case
                .child_by_field_name("consequence")
                .or_else(|| case.child_by_field_name("value"))
                .or_else(|| case.child_by_field_name("body"))
                .or_else(|| when_entry_body(case))?;
            let end = if self.lang.block_kinds.contains(&body.kind()) {
                self.seq(body, Some((p, EdgeLabel::True)))
            } else {
                self.stmt(body, Some((p, EdgeLabel::True)))
            };
            if let Some(e) = end {
                self.edge(e, after, EdgeLabel::Epsilon);
            }
            return None;
        }
        defuse::seed_case_bindings(case, self.source, self.entry, &mut self.def_use);
        let mut kids_cursor = case.walk();
        let kids: Vec<Node> = case.children(&mut kids_cursor).filter(Node::is_named).collect();
        self.do_switch_case(case, p, &kids, fall_in)
    }

    fn do_switch_case(&mut self, case: Node, p: u32, body_stmts: &[Node], fall_in: Option<u32>) -> Option<u32> {
        if let Some(label) = case.child_by_field_name("value") {
            defuse::collect(label, self.source, p, self.lang, &mut self.def_use);
        }
        let entry = self.add(NodeKind::Stmt, line(case));
        self.edge(p, entry, EdgeLabel::True);
        if let Some(prev) = fall_in {
            self.edge(prev, entry, EdgeLabel::Epsilon);
        }
        self.thread_stmts(body_stmts, Some((entry, EdgeLabel::Epsilon)))
    }

    fn do_if(&mut self, node: Node, incoming: Incoming) -> Option<u32> {
        let p = self.add(NodeKind::Predicate, line(node));
        self.link(incoming, p);
        self.record_cond(node, p);
        let mut alt_cursor = node.walk();
        let mut alts: Vec<Node> = node.children_by_field_name("alternative", &mut alt_cursor).collect();
        let (consequence, pos_else) = if_bodies(node);
        if alts.is_empty() {
            alts.extend(pos_else);
        }
        let then_end = self.seq_opt_block(consequence, Some((p, EdgeLabel::True)));
        let had_alt = !alts.is_empty();
        let else_end = self.do_else_chain(&alts, p);
        if then_end.is_none() && else_end.is_none() {
            return None;
        }
        let merge = self.add(NodeKind::Join, end_line(node));
        if let Some(e) = then_end {
            self.edge(e, merge, EdgeLabel::Epsilon);
        }
        if let Some(e) = else_end {
            let label = if had_alt { EdgeLabel::Epsilon } else { EdgeLabel::False };
            self.edge(e, merge, label);
        }
        Some(merge)
    }

    fn seq_opt_block(&mut self, block: Option<Node>, incoming: Incoming) -> Option<u32> {
        match block {
            Some(b) => self.seq(b, incoming),
            None => incoming.map(|(id, _)| id),
        }
    }

    fn do_else_chain(&mut self, alts: &[Node], p: u32) -> Option<u32> {
        let Some((first, rest)) = alts.split_first() else {
            return Some(p);
        };
        if self.lang.if_kinds.contains(&first.kind()) {
            return self.do_if(*first, Some((p, EdgeLabel::False)));
        }
        if first.child_by_field_name("condition").is_some() {
            return self.do_elseif(*first, rest, p);
        }
        if self.lang.block_kinds.contains(&first.kind()) {
            return self.seq(*first, Some((p, EdgeLabel::False)));
        }
        let mut cursor = first.walk();
        for child in first.children(&mut cursor) {
            if self.lang.if_kinds.contains(&child.kind()) {
                return self.do_if(child, Some((p, EdgeLabel::False)));
            }
            if self.lang.block_kinds.contains(&child.kind()) {
                return self.seq(child, Some((p, EdgeLabel::False)));
            }
        }
        Some(p)
    }

    fn do_elseif(&mut self, clause: Node, rest: &[Node], p: u32) -> Option<u32> {
        let q = self.add(NodeKind::Predicate, line(clause));
        self.edge(p, q, EdgeLabel::False);
        self.record_cond(clause, q);
        let body = clause.child_by_field_name("consequence").or_else(|| clause.child_by_field_name("body"));
        let then_end = self.seq_opt_block(body, Some((q, EdgeLabel::True)));
        let else_end = self.do_else_chain(rest, q);
        if then_end.is_none() && else_end.is_none() {
            return None;
        }
        let merge = self.add(NodeKind::Join, end_line(clause));
        if let Some(e) = then_end {
            self.edge(e, merge, EdgeLabel::Epsilon);
        }
        if let Some(e) = else_end {
            let label = if rest.is_empty() { EdgeLabel::False } else { EdgeLabel::Epsilon };
            self.edge(e, merge, label);
        }
        Some(merge)
    }

    #[allow(clippy::unnecessary_wraps)]
    fn do_loop(&mut self, node: Node, incoming: Incoming) -> Option<u32> {
        let head = self.add(NodeKind::LoopHead, line(node));
        self.link(incoming, head);
        defuse::loop_header(node, self.source, head, self.lang, &mut self.def_use);
        let after = self.add(NodeKind::Join, end_line(node));
        self.edge(head, after, EdgeLabel::False);
        self.loops.push(LoopCtx { head, after });
        let body = node.child_by_field_name("body").or_else(|| find_child_by_kinds(node, self.lang.block_kinds));
        if let Some(b) = body {
            if let Some(e) = self.seq(b, Some((head, EdgeLabel::True))) {
                self.edge(e, head, EdgeLabel::Back);
            }
        }
        self.loops.pop();
        Some(after)
    }

    fn do_try(&mut self, node: Node, incoming: Incoming) -> Option<u32> {
        let entry = self.add(NodeKind::Stmt, line(node));
        self.link(incoming, entry);
        let after = self.add(NodeKind::Join, end_line(node));
        let body_end =
            node.child_by_field_name("body").map_or(Some(entry), |b| self.seq(b, Some((entry, EdgeLabel::Epsilon))));
        let mut normal = body_end;
        let mut finalizer = None;
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let k = child.kind();
            if self.lang.handler_kinds.contains(&k) {
                self.handler(child, entry, body_end, after);
            } else if k == "else_clause" {
                let inc = normal.map(|n| (n, EdgeLabel::Epsilon));
                normal = self.seq_opt_block(find_child_by_kinds(child, self.lang.block_kinds), inc);
            } else if k == "finally_clause" {
                finalizer = Some(child);
            }
        }
        if let Some(e) = normal {
            self.edge(e, after, EdgeLabel::Epsilon);
        }
        match finalizer {
            Some(fc) => {
                self.seq_opt_block(find_child_by_kinds(fc, self.lang.block_kinds), Some((after, EdgeLabel::Epsilon)))
            }
            None => Some(after),
        }
    }

    fn handler(&mut self, child: Node, entry: u32, body_end: Option<u32>, after: u32) {
        let h = self.add(NodeKind::Handler, line(child));
        self.edge(entry, h, EdgeLabel::ToHandler);
        if let Some(b) = body_end {
            self.edge(b, h, EdgeLabel::ToHandler);
        }
        if let Some(hb) = find_child_by_kinds(child, self.lang.block_kinds) {
            if let Some(e) = self.seq(hb, Some((h, EdgeLabel::Epsilon))) {
                self.edge(e, after, EdgeLabel::Epsilon);
            }
        }
    }
}
