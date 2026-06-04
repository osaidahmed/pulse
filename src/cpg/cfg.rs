use tree_sitter::Node;

use crate::walk::{find_child_by_kind, DepthGuard};

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
}

pub const PYTHON: CfgLang = CfgLang {
    if_kinds: &["if_statement"],
    loop_kinds: &["for_statement", "while_statement"],
    return_kinds: &["return_statement"],
    break_kinds: &["break_statement"],
    continue_kinds: &["continue_statement"],
    try_kinds: &["try_statement"],
    handler_kinds: &["except_clause"],
};

pub const RUST: CfgLang = CfgLang {
    if_kinds: &["if_expression"],
    loop_kinds: &["for_expression", "while_expression", "loop_expression"],
    return_kinds: &["return_expression"],
    break_kinds: &["break_expression"],
    continue_kinds: &["continue_expression"],
    try_kinds: &[],
    handler_kinds: &[],
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
    nodes: Vec<CfgNode>,
    edges: Vec<CfgEdge>,
    loops: Vec<LoopCtx>,
    exit: u32,
}

pub fn build_cfg(body: Node, lang: &CfgLang) -> Cfg {
    let mut b = Builder { lang, nodes: Vec::new(), edges: Vec::new(), loops: Vec::new(), exit: 0 };
    let entry = b.add(NodeKind::Entry, line(body));
    let exit = b.add(NodeKind::Exit, end_line(body));
    b.exit = exit;
    if let Some(e) = b.seq(body, Some((entry, EdgeLabel::Epsilon))) {
        b.edge(e, exit, EdgeLabel::Epsilon);
    }
    Cfg { nodes: b.nodes, edges: b.edges, entry, exit }
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
        let mut cur = incoming;
        let mut cursor = block.walk();
        for child in block.children(&mut cursor) {
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

    fn do_if(&mut self, node: Node, incoming: Incoming) -> Option<u32> {
        let p = self.add(NodeKind::Predicate, line(node));
        self.link(incoming, p);
        let then_end = self.branch(node.child_by_field_name("consequence"), p, EdgeLabel::True);
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

    fn branch(&mut self, block: Option<Node>, p: u32, label: EdgeLabel) -> Option<u32> {
        match block {
            Some(b) => self.seq(b, Some((p, label))),
            None => Some(p),
        }
    }

    fn do_else(&mut self, alt: Node, p: u32) -> Option<u32> {
        if self.lang.if_kinds.contains(&alt.kind()) {
            return self.do_if(alt, Some((p, EdgeLabel::False)));
        }
        let mut cursor = alt.walk();
        for child in alt.children(&mut cursor) {
            if self.lang.if_kinds.contains(&child.kind()) {
                return self.do_if(child, Some((p, EdgeLabel::False)));
            }
            if child.kind() == "block" {
                return self.seq(child, Some((p, EdgeLabel::False)));
            }
        }
        Some(p)
    }

    #[allow(clippy::unnecessary_wraps)]
    fn do_loop(&mut self, node: Node, incoming: Incoming) -> Option<u32> {
        let head = self.add(NodeKind::LoopHead, line(node));
        self.link(incoming, head);
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

    #[allow(clippy::unnecessary_wraps)]
    fn do_try(&mut self, node: Node, incoming: Incoming) -> Option<u32> {
        let entry = self.add(NodeKind::Stmt, line(node));
        self.link(incoming, entry);
        let after = self.add(NodeKind::Stmt, end_line(node));
        if let Some(b) = node.child_by_field_name("body") {
            if let Some(e) = self.seq(b, Some((entry, EdgeLabel::Epsilon))) {
                self.edge(e, after, EdgeLabel::Epsilon);
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if self.lang.handler_kinds.contains(&child.kind()) {
                self.handler(child, entry, after);
            }
        }
        Some(after)
    }

    fn handler(&mut self, child: Node, entry: u32, after: u32) {
        let h = self.add(NodeKind::Handler, line(child));
        self.edge(entry, h, EdgeLabel::ToHandler);
        if let Some(hb) = find_child_by_kind(child, "block") {
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
