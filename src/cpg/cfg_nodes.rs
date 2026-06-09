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

pub(super) fn if_bodies(node: Node) -> (Option<Node>, Option<Node>) {
    if let Some(c) = node.child_by_field_name("consequence").or_else(|| node.child_by_field_name("body")) {
        return (Some(c), None);
    }
    let cond_id = node.child_by_field_name("condition").map(|c| c.id());
    let mut then_b: Option<Node> = None;
    let mut else_b: Option<Node> = None;
    let mut seen_else = false;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "else" {
            seen_else = true;
        } else if child.is_named() && Some(child.id()) != cond_id {
            if seen_else {
                else_b = else_b.or(Some(child));
            } else {
                then_b = then_b.or(Some(child));
            }
        }
    }
    (then_b, else_b)
}
