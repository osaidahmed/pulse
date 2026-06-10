use tree_sitter::Node;

use crate::cpg::defuse::{push_idents, DefUse, DefUseRecord};
use crate::walk::DepthGuard;

pub(crate) fn seed(node: Node, source: &str, exit: u32, out: &mut Vec<DefUseRecord>) {
    let Some(_g) = DepthGuard::enter() else { return };
    match node.kind() {
        "assignment" | "operator_assignment" => {
            if let Some(l) = node.child_by_field_name("left") {
                push_idents(l, source, exit, DefUse::Use, out);
            }
        }
        "if" | "unless" | "elsif" | "conditional" => seed_if(node, source, exit, out),
        "case" | "case_match" => seed_cases(node, source, exit, out),
        "binary" => seed_binary(node, source, exit, out),
        "begin"
        | "body_statement"
        | "then"
        | "do"
        | "else"
        | "when"
        | "in_clause"
        | "rescue"
        | "parenthesized_statements" => {
            seed_seq(node, source, exit, out);
        }
        _ => {}
    }
}

fn seed_binary(node: Node, source: &str, exit: u32, out: &mut Vec<DefUseRecord>) {
    for f in ["left", "right"] {
        if let Some(c) = node.child_by_field_name(f) {
            seed(c, source, exit, out);
        }
    }
}

fn seed_if(node: Node, source: &str, exit: u32, out: &mut Vec<DefUseRecord>) {
    if let Some(c) = node.child_by_field_name("consequence") {
        seed(c, source, exit, out);
    }
    let mut cursor = node.walk();
    for alt in node.children_by_field_name("alternative", &mut cursor) {
        seed(alt, source, exit, out);
    }
}

fn seed_cases(node: Node, source: &str, exit: u32, out: &mut Vec<DefUseRecord>) {
    let mut cursor = node.walk();
    for c in node.children(&mut cursor) {
        if matches!(c.kind(), "when" | "in_clause" | "else") {
            seed(c, source, exit, out);
        }
    }
}

fn seed_seq(node: Node, source: &str, exit: u32, out: &mut Vec<DefUseRecord>) {
    if let Some(b) = node.child_by_field_name("body") {
        seed(b, source, exit, out);
        return;
    }
    let mut cursor = node.walk();
    let kids: Vec<Node> = node.children(&mut cursor).filter(tree_sitter::Node::is_named).collect();
    if let Some(last) = kids.iter().rev().find(|c| !matches!(c.kind(), "rescue" | "ensure" | "else")) {
        seed(*last, source, exit, out);
    }
    for c in &kids {
        if matches!(c.kind(), "rescue" | "else") {
            seed(*c, source, exit, out);
        }
    }
}
