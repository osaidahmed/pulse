use tree_sitter::Node;

use super::super::node_text;

const SCOPE_BOUNDARY: &[&str] = &["lambda", "lambda_case"];

pub fn count_bool_ops(node: Node, source: &str, cc: &mut u32) {
    walk_bool_tree(node, source, &mut |_| *cc += 1);
}

pub fn count_cogc_sequences(node: Node, source: &str, cogc: &mut u32) {
    let mut last_op: Option<String> = None;
    walk_bool_tree(node, source, &mut |op| {
        if last_op.as_deref() != Some(op) {
            *cogc += 1;
            last_op = Some(op.to_string());
        }
    });
}

fn walk_bool_tree(node: Node, source: &str, on_op: &mut dyn FnMut(&str)) {
    check_infix_op(node, source, on_op);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if SCOPE_BOUNDARY.contains(&child.kind()) {
            continue;
        }
        walk_bool_tree(child, source, on_op);
    }
}

fn check_infix_op(child: Node, source: &str, on_op: &mut dyn FnMut(&str)) {
    if child.kind() != "infix" {
        return;
    }
    let Some(op) = operator_text(child, source) else {
        return;
    };
    if op == "&&" || op == "||" {
        on_op(&op);
    }
}

pub fn check_compound(node: Node, source: &str, count: &mut u32) {
    let text = node_text(node, source);
    let ops = text.matches("&&").count() + text.matches("||").count();
    if ops >= 2 {
        *count += 1;
    }
}

fn operator_text(infix: Node, source: &str) -> Option<String> {
    let mut cursor = infix.walk();
    let result = infix
        .children(&mut cursor)
        .find(|c| c.kind() == "operator")
        .map(|n| node_text(n, source).trim().to_string());
    result
}

pub fn is_otherwise_guard(boolean_node: Node, source: &str) -> bool {
    node_text(boolean_node, source).trim() == "otherwise"
}
