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

pub(super) fn kotlin_for_iterable(node: Node) -> Option<Node> {
    if node.kind() != "for_statement" {
        return None;
    }
    let mut cursor = node.walk();
    let kids: Vec<Node> = node.children(&mut cursor).filter(tree_sitter::Node::is_named).collect();
    if !kids.iter().any(|c| matches!(c.kind(), "variable_declaration" | "multi_variable_declaration")) {
        return None;
    }
    kids.into_iter().find(|c| {
        !matches!(
            c.kind(),
            "variable_declaration" | "multi_variable_declaration" | "block" | "statement" | "annotation"
        )
    })
}

pub(super) fn when_entry_body(node: Node) -> Option<Node> {
    let mut cond_cursor = node.walk();
    let cond_ids: Vec<usize> = node.children_by_field_name("condition", &mut cond_cursor).map(|c| c.id()).collect();
    let mut cursor = node.walk();
    let mut body = None;
    for child in node.children(&mut cursor) {
        if child.is_named() && !cond_ids.contains(&child.id()) {
            body = Some(child);
        }
    }
    body
}

pub(super) fn if_bodies(node: Node) -> (Option<Node>, Option<Node>) {
    if let Some(c) = node.child_by_field_name("consequence").or_else(|| node.child_by_field_name("body")) {
        return (Some(c), None);
    }
    let mut cond_cursor = node.walk();
    let cond_ids: Vec<usize> = node.children_by_field_name("condition", &mut cond_cursor).map(|c| c.id()).collect();
    let mut then_b: Option<Node> = None;
    let mut else_b: Option<Node> = None;
    let mut seen_else = false;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "else" {
            seen_else = true;
        } else if child.is_named() && !cond_ids.contains(&child.id()) {
            if seen_else {
                else_b = else_b.or(Some(child));
            } else {
                then_b = then_b.or(Some(child));
            }
        }
    }
    (then_b, else_b)
}
