use tree_sitter::Node;

use crate::cpg::defuse::{push_idents, DefUse, DefUseRecord, Mark};
use crate::walk::{node_text, DepthGuard};

pub(crate) fn seed_string_interpolation(node: Node, source: &str, exit: u32, out: &mut Vec<DefUseRecord>) {
    let Some(_g) = DepthGuard::enter() else { return };
    if node.kind() == "string_literal" {
        scan_dollar_idents(node, source, exit, out);
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.is_named() {
            seed_string_interpolation(child, source, exit, out);
        }
    }
}

fn scan_dollar_idents(node: Node, source: &str, exit: u32, out: &mut Vec<DefUseRecord>) {
    let text = node_text(node, source).as_bytes();
    let line = node.start_position().row as u32 + 1;
    let mut i = 0;
    while i < text.len() {
        if text[i] != b'$' || i + 1 >= text.len() || !(text[i + 1].is_ascii_alphabetic() || text[i + 1] == b'_') {
            i += 1;
            continue;
        }
        let start = i + 1;
        let mut j = start;
        while j < text.len() && (text[j].is_ascii_alphanumeric() || text[j] == b'_') {
            j += 1;
        }
        let name = String::from_utf8_lossy(&text[start..j]).into_owned();
        out.push(DefUseRecord { name, block: exit, kind: DefUse::Use, line, decl: false });
        i = j;
    }
}

pub(crate) fn seed(node: Node, source: &str, exit: u32, out: &mut Vec<DefUseRecord>) {
    let Some(_g) = DepthGuard::enter() else { return };
    match node.kind() {
        "assignment" | "operator_assignment" => {
            if let Some(l) = node.child_by_field_name("left") {
                push_idents(l, source, exit, Mark::Use, out);
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
