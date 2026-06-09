use tree_sitter::Node;

use crate::walk::{node_text, DepthGuard};

pub(super) fn line(node: Node) -> u32 {
    node.start_position().row as u32 + 1
}

pub(super) fn is_in(name: Option<&str>, set: &[&str]) -> bool {
    name.is_some_and(|n| set.contains(&n))
}

fn first_field<'a>(node: Node<'a>, fields: &[&str]) -> Option<Node<'a>> {
    fields.iter().find_map(|f| node.child_by_field_name(f))
}

pub(super) fn binding_left(node: Node) -> Option<Node> {
    first_field(node, &["left", "pattern", "name", "declarator"])
}

pub(super) fn binding_right(node: Node) -> Option<Node> {
    node.child_by_field_name("right").or_else(|| node.child_by_field_name("value")).or_else(|| initializer_child(node))
}

fn initializer_child(node: Node) -> Option<Node> {
    if node.kind() != "variable_declarator" {
        return None;
    }
    let name_id = node.child_by_field_name("name").map(|n| n.id());
    let mut cursor = node.walk();
    let mut found = None;
    for child in node.children(&mut cursor) {
        if child.is_named() && Some(child.id()) != name_id {
            found = Some(child);
        }
    }
    found
}

pub(super) fn is_field_or_index_target(kind: &str) -> bool {
    matches!(kind, "attribute" | "subscript" | "field_expression" | "index_expression")
}

pub(super) fn is_callee(node: Node, call_kinds: &[&str]) -> bool {
    node.parent().is_some_and(|p| {
        call_kinds.contains(&p.kind())
            && ["function", "name", "method"]
                .iter()
                .any(|f| p.child_by_field_name(f).is_some_and(|c| c.id() == node.id()))
    })
}

pub(super) fn callee_name(call: Node, source: &str) -> Option<String> {
    let f = call
        .child_by_field_name("function")
        .or_else(|| call.child_by_field_name("name"))
        .or_else(|| call.child_by_field_name("method"))?;
    Some(trailing_name(f, source))
}

fn trailing_name(node: Node, source: &str) -> String {
    let field = |name: &str| node.child_by_field_name(name).map(|c| node_text(c, source).to_string());
    match node.kind() {
        "identifier" | "field_identifier" | "property_identifier" => node_text(node, source).to_string(),
        "attribute" => field("attribute").unwrap_or_default(),
        "field_expression" => field("field").unwrap_or_default(),
        "scoped_identifier" => field("name").unwrap_or_default(),
        "member_expression" => field("property").unwrap_or_default(),
        _ => last_identifier(node, source),
    }
}

fn last_identifier(node: Node, source: &str) -> String {
    let mut cursor = node.walk();
    let mut found = String::new();
    for child in node.children(&mut cursor) {
        if matches!(child.kind(), "identifier" | "field_identifier" | "property_identifier") {
            found = node_text(child, source).to_string();
        }
    }
    if found.is_empty() {
        node_text(node, source).to_string()
    } else {
        found
    }
}

pub(super) fn idents_in(node: Node, source: &str, ident_kinds: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    collect_idents(node, source, ident_kinds, &mut out);
    out
}

fn collect_idents(node: Node, source: &str, ident_kinds: &[&str], out: &mut Vec<String>) {
    let Some(_g) = DepthGuard::enter() else { return };
    if ident_kinds.contains(&node.kind()) {
        out.push(node_text(node, source).to_string());
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.is_named() {
            collect_idents(child, source, ident_kinds, out);
        }
    }
}
