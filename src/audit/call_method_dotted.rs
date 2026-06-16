use tree_sitter::Node;

use super::calls::{line_of, node_text, RawCall};

const CALL_KINDS: &[&str] = &[
    "call",
    "call_expression",
    "method_invocation",
    "method_call_expression",
    "function_call",
    "function_call_expression",
    "method_call",
    "message_expression",
    "send",
];

const RECEIVER_FIELDS: &[&str] = &["object", "receiver", "value"];

const NAME_FIELDS: &[&str] = &["name", "attribute", "property", "method", "field", "method_name"];

const DOTTED_KINDS: &[&str] = &[
    "attribute",
    "field_expression",
    "member_expression",
    "selector_expression",
    "member_access_expression",
    "navigation_expression",
];

const BARE_KINDS: &[&str] = &["identifier", "name"];

pub fn match_node(node: Node, source: &str) -> Option<RawCall> {
    if !CALL_KINDS.contains(&node.kind()) {
        return None;
    }
    if let Some(function) = node.child_by_field_name("function") {
        return extract_from_callsite(function, node, source);
    }
    extract_from_invocation(node, source)
}

fn extract_from_callsite<'a>(callsite: Node<'a>, parent: Node<'a>, source: &'a str) -> Option<RawCall> {
    let kind = callsite.kind();
    let dotted_first = DOTTED_KINDS.contains(&kind) || !BARE_KINDS.contains(&kind);
    if dotted_first {
        if let Some(c) = extract_dotted(callsite, parent, source) {
            return Some(c);
        }
    }
    extract_bare(callsite, parent, source).or_else(|| extract_dotted(callsite, parent, source))
}

fn extract_dotted<'a>(callsite: Node<'a>, parent: Node<'a>, source: &'a str) -> Option<RawCall> {
    extract_via_fields(callsite, parent, source)
}

fn extract_bare<'a>(callsite: Node<'a>, parent: Node<'a>, source: &'a str) -> Option<RawCall> {
    if callsite.kind() != "identifier" && callsite.kind() != "name" {
        return None;
    }
    Some(RawCall { callee_name: node_text(callsite, source).to_string(), receiver_hint: None, line: line_of(parent) })
}

fn extract_from_invocation<'a>(node: Node<'a>, source: &'a str) -> Option<RawCall> {
    extract_via_fields(node, node, source)
}

fn extract_via_fields<'a>(field_owner: Node<'a>, line_owner: Node<'a>, source: &'a str) -> Option<RawCall> {
    let name_node = field_first(field_owner, NAME_FIELDS)?;
    let recv = field_first(field_owner, RECEIVER_FIELDS);
    Some(RawCall {
        callee_name: node_text(name_node, source).to_string(),
        receiver_hint: recv.map(|r| receiver_text(r, source)),
        line: line_of(line_owner),
    })
}

fn field_first<'a>(node: Node<'a>, fields: &[&str]) -> Option<Node<'a>> {
    for f in fields {
        if let Some(c) = node.child_by_field_name(f) {
            return Some(c);
        }
    }
    None
}

fn receiver_text(node: Node, source: &str) -> String {
    let text = node_text(node, source);
    if let Some(last) = text.rsplit('.').next() {
        return last.to_string();
    }
    text.to_string()
}
