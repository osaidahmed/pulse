use tree_sitter::Node;

pub fn count_boolean_operators(node: Node, cc: &mut u32) {
    let mut cursor = node.walk();
    node.children(&mut cursor).for_each(|child| match child.kind() {
        "boolean_operator" | "not_operator" => {
            *cc += 1;
            count_boolean_operators(child, cc);
        }
        "block" | "function_definition" | "class_definition" => {}
        _ => count_boolean_operators(child, cc),
    });
}

pub fn count_cogc_boolean_sequences(node: Node, cogc: &mut u32) {
    let mut last_op: Option<String> = None;
    collect_boolean_ops(node, cogc, &mut last_op);
}

fn collect_boolean_ops(node: Node, cogc: &mut u32, last_op: &mut Option<String>) {
    let mut child_opt = node.child(0);
    while let Some(child) = child_opt {
        child_opt = child.next_sibling();
        match child.kind() {
            "boolean_operator" => {
                track_operator(child, cogc, last_op);
                collect_boolean_ops(child, cogc, last_op);
            }
            "block" | "function_definition" | "class_definition" => {}
            _ => collect_boolean_ops(child, cogc, last_op),
        }
    }
}

fn track_operator(boolean_op: Node, cogc: &mut u32, last_op: &mut Option<String>) {
    let mut cursor = boolean_op.walk();
    let op_node = boolean_op
        .children(&mut cursor)
        .find(|c| c.kind() == "and" || c.kind() == "or" || c.kind() == "not");
    let Some(op_node) = op_node else { return; };
    let op = op_node.kind().to_string();
    if last_op.as_deref() == Some(&op) { return; }
    *cogc += 1;
    *last_op = Some(op);
}

pub fn has_boolean_child(node: Node) -> bool {
    let mut child_opt = node.child(0);
    while let Some(c) = child_opt {
        if c.kind() == "boolean_operator" { return true; }
        child_opt = c.next_sibling();
    }
    false
}
