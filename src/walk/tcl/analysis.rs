use tree_sitter::Node;

use super::super::{find_child_by_kind, node_text};

const BOOL_STOPS: &[&str] = &["procedure", "braced_word"];

pub fn count_boolean_ops(node: Node, cc: &mut u32) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let k = child.kind();
        if k == "binop_expr" {
            if !bool_op_category(child).is_empty() {
                *cc += 1;
            }
            count_boolean_ops(child, cc);
        } else if !BOOL_STOPS.contains(&k) {
            count_boolean_ops(child, cc);
        }
    }
}

pub fn walk_cogc(node: Node, cogc: &mut u32, last: &mut Option<String>) {
    let mut c = node.child(0);
    while let Some(child) = c {
        c = child.next_sibling();
        let k = child.kind();
        if BOOL_STOPS.contains(&k) {
            continue;
        }
        if k != "binop_expr" {
            walk_cogc(child, cogc, last);
            continue;
        }
        let op = bool_op_category(child);
        if !op.is_empty() && last.as_deref() != Some(op) {
            *cogc += 1;
            *last = Some(op.to_string());
        }
        walk_cogc(child, cogc, last);
    }
}

fn bool_op_category(node: Node) -> &'static str {
    let n = node.child_count();
    for i in 0..n {
        let Some(c) = node.child(i) else { continue };
        if c.is_named() {
            continue;
        }
        if c.kind() == "&&" {
            return "and";
        }
        if c.kind() == "||" {
            return "or";
        }
    }
    ""
}

pub fn collect_field_accesses(node: Node, source: &str, fields: &mut Vec<String>) {
    let mut stack = vec![node];
    while let Some(current) = stack.pop() {
        let mut cursor = current.walk();
        for child in current.children(&mut cursor) {
            extract_variable_decl(child, source, fields);
            stack.push(child);
        }
    }
}

fn extract_variable_decl(child: Node, source: &str, fields: &mut Vec<String>) {
    if child.kind() != "command" || cmd_name(child, source) != "variable" {
        return;
    }
    let v = find_child_by_kind(child, "word_list").and_then(|w| find_child_by_kind(w, "simple_word"));
    if let Some(v) = v {
        fields.push(node_text(v, source).to_string());
    }
}

pub fn cmd_name<'a>(node: Node<'a>, source: &'a str) -> &'a str {
    node.child_by_field_name("name").map_or("", |n| node_text(n, source))
}

pub fn count_switch_arms(body: Node, source: &str) -> u32 {
    let mut total = 0;
    count_arms_recursive(body, source, &mut total);
    total
}

fn count_arms_recursive(node: Node, source: &str, count: &mut u32) {
    let n = node.child_count();
    for i in 0..n {
        let child = node.child(i).unwrap();
        if child.kind() == "command" && cmd_name(child, source) == "switch" {
            tally_switch(child, source, count);
        }
        count_arms_recursive(child, source, count);
    }
}

fn tally_switch(switch_cmd: Node, source: &str, count: &mut u32) {
    for bw in &braced_in_wordlist(switch_cmd, source) {
        let n = bw.child_count();
        for i in 0..n {
            let case = bw.child(i).unwrap();
            if is_string_case(case, source) {
                *count += 1;
            }
        }
    }
}

fn is_string_case(node: Node, source: &str) -> bool {
    if node.kind() != "command" {
        return false;
    }
    let name = cmd_name(node, source);
    !name.is_empty() && name != "default" && !name.starts_with('$')
}

pub fn count_named_consecutive_asserts(body: Node) -> u32 {
    let mut max: u32 = 0;
    let mut current: u32 = 0;
    let n = body.named_child_count();
    for i in 0..n {
        let child = body.named_child(i).unwrap();
        if child.kind() == "command" {
            current += 1;
        } else {
            current = 0;
        }
        if current > max {
            max = current;
        }
    }
    max
}

pub fn braced_in_wordlist<'a>(node: Node<'a>, _source: &str) -> Vec<Node<'a>> {
    let Some(wl) = find_child_by_kind(node, "word_list") else { return Vec::new() };
    let mut cursor = wl.walk();
    wl.children(&mut cursor).filter(|c| c.kind() == "braced_word").collect()
}
