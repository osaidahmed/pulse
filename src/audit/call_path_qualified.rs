use tree_sitter::Node;

use super::calls::{line_of, node_text, RawCall};

const CALL_KINDS: &[&str] = &[
    "call_expression",
    "call",
    "scoped_call_expression",
    "qualified_call_expression",
];

const QUALIFIED_KINDS: &[&str] = &[
    "scoped_identifier",
    "qualified_identifier",
    "qualified_name",
    "qualified_type",
    "namespace_qualifier",
];

pub fn match_node(node: Node, source: &str) -> Option<RawCall> {
    if !CALL_KINDS.contains(&node.kind()) {
        return None;
    }
    let function = node.child_by_field_name("function").or_else(|| first_qualified(node))?;
    let qualified = if QUALIFIED_KINDS.contains(&function.kind()) {
        function
    } else {
        return None;
    };
    let (path_segment, name) = split_qualified(qualified, source)?;
    Some(RawCall {
        callee_name: name,
        receiver_hint: Some(path_segment),
        line: line_of(node),
    })
}

#[allow(clippy::manual_find)]
fn first_qualified(node: Node) -> Option<Node> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if QUALIFIED_KINDS.contains(&child.kind()) {
            return Some(child);
        }
    }
    None
}

fn split_qualified(node: Node, source: &str) -> Option<(String, String)> {
    let path = node.child_by_field_name("path");
    let name = node.child_by_field_name("name");
    if let (Some(p), Some(n)) = (path, name) {
        return Some((last_segment(node_text(p, source)), node_text(n, source).to_string()));
    }
    let text = node_text(node, source);
    split_text(text)
}

fn last_segment(text: &str) -> String {
    text.rsplit("::").next().unwrap_or(text).rsplit('.').next().unwrap_or(text).to_string()
}

fn split_text(text: &str) -> Option<(String, String)> {
    if let Some((path, name)) = text.rsplit_once("::") {
        return Some((last_segment(path), name.to_string()));
    }
    None
}
