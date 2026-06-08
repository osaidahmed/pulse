use tree_sitter::Node;

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
    pub nested_fn_kinds: &'static [&'static str],
}

pub const PYTHON: CfgLang = CfgLang {
    if_kinds: &["if_statement"],
    loop_kinds: &["for_statement", "while_statement"],
    return_kinds: &["return_statement"],
    break_kinds: &["break_statement"],
    continue_kinds: &["continue_statement"],
    try_kinds: &["try_statement"],
    handler_kinds: &["except_clause"],
    def_kinds: &["assignment", "augmented_assignment"],
    aug_kinds: &["augmented_assignment"],
    block_kinds: &["block"],
    nested_fn_kinds: &["lambda", "function_definition"],
};

pub const RUST: CfgLang = CfgLang {
    if_kinds: &["if_expression"],
    loop_kinds: &["for_expression", "while_expression", "loop_expression"],
    return_kinds: &["return_expression"],
    break_kinds: &["break_expression"],
    continue_kinds: &["continue_expression"],
    try_kinds: &[],
    handler_kinds: &[],
    def_kinds: &["let_declaration", "assignment_expression", "compound_assignment_expr"],
    aug_kinds: &["compound_assignment_expr"],
    block_kinds: &["block"],
    nested_fn_kinds: &["closure_expression", "function_item"],
};

pub const TYPESCRIPT: CfgLang = CfgLang {
    if_kinds: &["if_statement"],
    loop_kinds: &["while_statement", "for_statement", "for_in_statement", "do_statement"],
    return_kinds: &["return_statement"],
    break_kinds: &["break_statement"],
    continue_kinds: &["continue_statement"],
    try_kinds: &["try_statement"],
    handler_kinds: &["catch_clause"],
    def_kinds: &[],
    aug_kinds: &[],
    block_kinds: &["statement_block"],
    nested_fn_kinds: &[
        "arrow_function",
        "function_declaration",
        "function_expression",
        "method_definition",
        "generator_function",
        "generator_function_declaration",
    ],
};

pub const JAVASCRIPT: CfgLang = TYPESCRIPT;

pub const JAVA: CfgLang = CfgLang {
    if_kinds: &["if_statement"],
    loop_kinds: &["while_statement", "for_statement", "enhanced_for_statement", "do_statement"],
    return_kinds: &["return_statement"],
    break_kinds: &["break_statement"],
    continue_kinds: &["continue_statement"],
    try_kinds: &["try_statement"],
    handler_kinds: &["catch_clause"],
    def_kinds: &[],
    aug_kinds: &[],
    block_kinds: &["block", "constructor_body"],
    nested_fn_kinds: &["lambda_expression"],
};

pub const CSHARP: CfgLang = CfgLang {
    if_kinds: &["if_statement"],
    loop_kinds: &["while_statement", "for_statement", "foreach_statement", "do_statement"],
    return_kinds: &["return_statement"],
    break_kinds: &["break_statement"],
    continue_kinds: &["continue_statement"],
    try_kinds: &["try_statement"],
    handler_kinds: &["catch_clause"],
    def_kinds: &[],
    aug_kinds: &[],
    block_kinds: &["block"],
    nested_fn_kinds: &["lambda_expression", "anonymous_method_expression", "local_function_statement"],
};

pub const GO: CfgLang = CfgLang {
    if_kinds: &["if_statement"],
    loop_kinds: &["for_statement"],
    return_kinds: &["return_statement"],
    break_kinds: &["break_statement"],
    continue_kinds: &["continue_statement"],
    try_kinds: &[],
    handler_kinds: &[],
    def_kinds: &[],
    aug_kinds: &[],
    block_kinds: &["block"],
    nested_fn_kinds: &["func_literal"],
};

pub const C: CfgLang = CfgLang {
    if_kinds: &["if_statement"],
    loop_kinds: &["while_statement", "for_statement", "do_statement"],
    return_kinds: &["return_statement"],
    break_kinds: &["break_statement"],
    continue_kinds: &["continue_statement"],
    try_kinds: &[],
    handler_kinds: &[],
    def_kinds: &[],
    aug_kinds: &[],
    block_kinds: &["compound_statement"],
    nested_fn_kinds: &[],
};

pub const CPP: CfgLang = CfgLang {
    if_kinds: &["if_statement"],
    loop_kinds: &["while_statement", "for_statement", "for_range_loop", "do_statement"],
    return_kinds: &["return_statement"],
    break_kinds: &["break_statement"],
    continue_kinds: &["continue_statement"],
    try_kinds: &["try_statement"],
    handler_kinds: &["catch_clause"],
    def_kinds: &[],
    aug_kinds: &[],
    block_kinds: &["compound_statement"],
    nested_fn_kinds: &["lambda_expression"],
};

type Incoming = Option<(u32, EdgeLabel)>;

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

struct Builder<'a> {
    lang: &'a CfgLang,
    source: &'a str,
    nodes: Vec<CfgNode>,
    edges: Vec<CfgEdge>,
    loops: Vec<LoopCtx>,
    def_use: Vec<DefUseRecord>,
    exit: u32,
}

pub fn build_cfg(body: Node, source: &str, lang: &CfgLang) -> (Cfg, Vec<DefUseRecord>) {
    let mut b =
        Builder { lang, source, nodes: Vec::new(), edges: Vec::new(), loops: Vec::new(), def_use: Vec::new(), exit: 0 };
    let entry = b.add(NodeKind::Entry, line(body));
    let exit = b.add(NodeKind::Exit, end_line(body));
    b.exit = exit;
    if let Some(e) = b.seq(body, Some((entry, EdgeLabel::Epsilon))) {
        b.edge(e, exit, EdgeLabel::Epsilon);
    }
    (Cfg { nodes: b.nodes, edges: b.edges, entry, exit }, b.def_use)
}

impl Builder<'_> {
    fn add(&mut self, kind: NodeKind, line: u32) -> u32 {
        let id = self.nodes.len() as u32;
        self.nodes.push(CfgNode { id, line, kind });
        id
    }

    fn edge(&mut self, from: u32, to: u32, label: EdgeLabel) {
        self.edges.push(CfgEdge { from, to, label });
    }

    fn link(&mut self, incoming: Incoming, to: u32) {
        if let Some((from, label)) = incoming {
            self.edge(from, to, label);
        }
    }

    fn seq(&mut self, block: Node, incoming: Incoming) -> Option<u32> {
        let _g = DepthGuard::enter()?;
        let seq_node = stmt_seq_node(block);
        let mut cur = incoming;
        let mut cursor = seq_node.walk();
        for child in seq_node.children(&mut cursor) {
            if !child.is_named() {
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
        if self.lang.if_kinds.contains(&k) {
            return self.do_if(node, incoming);
        }
        if self.lang.loop_kinds.contains(&k) {
            return self.do_loop(node, incoming);
        }
        if self.lang.try_kinds.contains(&k) {
            return self.do_try(node, incoming);
        }
        if let Some(to) = self.jump_kind(k) {
            return self.do_jump(node, incoming, to);
        }
        let n = self.add(NodeKind::Stmt, line(node));
        self.link(incoming, n);
        defuse::collect(node, self.source, n, self.lang, &mut self.def_use);
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
        if let Some(c) = node.child_by_field_name("condition") {
            defuse::collect(c, self.source, block, self.lang, &mut self.def_use);
        }
    }

    fn do_if(&mut self, node: Node, incoming: Incoming) -> Option<u32> {
        let p = self.add(NodeKind::Predicate, line(node));
        self.link(incoming, p);
        self.record_cond(node, p);
        let then_end = self.seq_opt_block(node.child_by_field_name("consequence"), Some((p, EdgeLabel::True)));
        let alt = node.child_by_field_name("alternative");
        let else_end = match alt {
            Some(a) => self.do_else(a, p),
            None => Some(p),
        };
        if then_end.is_none() && else_end.is_none() {
            return None;
        }
        let merge = self.add(NodeKind::Stmt, end_line(node));
        if let Some(e) = then_end {
            self.edge(e, merge, EdgeLabel::Epsilon);
        }
        if let Some(e) = else_end {
            let label = if alt.is_some() { EdgeLabel::Epsilon } else { EdgeLabel::False };
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

    fn do_else(&mut self, alt: Node, p: u32) -> Option<u32> {
        if self.lang.if_kinds.contains(&alt.kind()) {
            return self.do_if(alt, Some((p, EdgeLabel::False)));
        }
        if self.lang.block_kinds.contains(&alt.kind()) {
            return self.seq(alt, Some((p, EdgeLabel::False)));
        }
        let mut cursor = alt.walk();
        for child in alt.children(&mut cursor) {
            if self.lang.if_kinds.contains(&child.kind()) {
                return self.do_if(child, Some((p, EdgeLabel::False)));
            }
            if self.lang.block_kinds.contains(&child.kind()) {
                return self.seq(child, Some((p, EdgeLabel::False)));
            }
        }
        Some(p)
    }

    #[allow(clippy::unnecessary_wraps)]
    fn do_loop(&mut self, node: Node, incoming: Incoming) -> Option<u32> {
        let head = self.add(NodeKind::LoopHead, line(node));
        self.link(incoming, head);
        defuse::loop_header(node, self.source, head, self.lang, &mut self.def_use);
        let after = self.add(NodeKind::Stmt, end_line(node));
        self.edge(head, after, EdgeLabel::False);
        self.loops.push(LoopCtx { head, after });
        if let Some(b) = node.child_by_field_name("body") {
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
        let after = self.add(NodeKind::Stmt, end_line(node));
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

fn line(node: Node) -> u32 {
    node.start_position().row as u32 + 1
}

fn end_line(node: Node) -> u32 {
    node.end_position().row as u32 + 1
}

fn stmt_seq_node(block: Node) -> Node {
    let mut cursor = block.walk();
    for child in block.children(&mut cursor) {
        if child.kind() == "statement_list" {
            return child;
        }
    }
    block
}

fn unwrap_stmt(node: Node) -> Node {
    if node.kind() == "expression_statement" {
        let mut cursor = node.walk();
        let inner = node.children(&mut cursor).find(tree_sitter::Node::is_named);
        if let Some(n) = inner {
            return n;
        }
    }
    node
}
