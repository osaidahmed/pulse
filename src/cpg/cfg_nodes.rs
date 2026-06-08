use tree_sitter::Node;

pub(super) fn line(node: Node) -> u32 {
    node.start_position().row as u32 + 1
}

pub(super) fn end_line(node: Node) -> u32 {
    node.end_position().row as u32 + 1
}

pub(super) fn stmt_seq_node(block: Node) -> Node {
    let mut cursor = block.walk();
    for child in block.children(&mut cursor) {
        if child.kind() == "statement_list" {
            return child;
        }
    }
    block
}

pub(super) fn unwrap_stmt(node: Node) -> Node {
    if node.kind() == "expression_statement" {
        let mut cursor = node.walk();
        let inner = node.children(&mut cursor).find(tree_sitter::Node::is_named);
        if let Some(n) = inner {
            return n;
        }
    }
    node
}
