use tree_sitter::Node;

use crate::cpg::cfg::{Builder, EdgeLabel, Incoming, NodeKind};
use crate::cpg::cfg_nodes::line;
use crate::cpg::defuse;
use crate::walk::node_text;

impl Builder<'_> {
    #[allow(clippy::option_option)]
    pub(super) fn goto_or_label(&mut self, node: Node, k: &str, incoming: Incoming) -> Option<Option<u32>> {
        match k {
            "goto_statement" => {
                self.do_goto(node, incoming);
                Some(None)
            }
            "labeled_statement" => Some(Some(self.do_label(node, incoming))),
            _ => None,
        }
    }

    fn do_goto(&mut self, node: Node, incoming: Incoming) {
        let n = self.add(NodeKind::Stmt, line(node));
        self.link(incoming, n);
        if let Some(name) = label_name(node, self.source) {
            self.pending_gotos.push((n, name));
        }
    }

    fn do_label(&mut self, node: Node, incoming: Incoming) -> u32 {
        let n = self.add(NodeKind::Stmt, line(node));
        self.link(incoming, n);
        if let Some(name) = label_name(node, self.source) {
            self.labels.push((name, n));
        }
        defuse::collect(node, self.source, n, self.lang, &mut self.def_use);
        n
    }

    pub(super) fn is_dispatched_stmt(&self, k: &str) -> bool {
        matches!(k, "goto_statement" | "labeled_statement")
            || self.lang.if_kinds.contains(&k)
            || self.lang.loop_kinds.contains(&k)
            || self.lang.return_kinds.contains(&k)
            || self.lang.break_kinds.contains(&k)
            || self.lang.continue_kinds.contains(&k)
            || self.lang.try_kinds.contains(&k)
            || self.lang.switch_kinds.contains(&k)
    }

    pub(super) fn resolve_gotos(&mut self) {
        let gotos = std::mem::take(&mut self.pending_gotos);
        for (from, name) in gotos {
            if let Some((_, to)) = self.labels.iter().find(|(n, _)| n == &name) {
                self.edge(from, *to, EdgeLabel::Epsilon);
            }
        }
    }
}

fn label_name(node: Node, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    let found = node.children(&mut cursor).find(|c| matches!(c.kind(), "statement_identifier" | "label_name"));
    found.map(|c| node_text(c, source).to_string())
}
