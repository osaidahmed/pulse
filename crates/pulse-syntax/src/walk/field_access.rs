use tree_sitter::Node;

use super::node_text;

const FIELD_ACCESS_KINDS: &[&str] = &[
    "attribute",
    "member_expression",
    "field_expression",
    "field_access",
    "navigation_expression",
    "dot_index_expression",
    "member_access_expression",
    "method_invocation",
    "invocation_expression",
    "selector_expression",
    "member_call_expression",
    "method_index_expression",
    "message_expression",
];
const SELF_OBJ_KINDS: &[&str] = &["identifier", "this", "self", "this_expression", "variable_name"];
const FIELD_NAME_KINDS: &[&str] = &["identifier", "property_identifier", "field_identifier", "name"];

pub fn collect_field_accesses_for(func_node: Node, source: &str, self_names: &[&str], fields: &mut Vec<String>) {
    visit_field_accesses(func_node, source, self_names, fields, try_extract_field);
    fields.sort();
    fields.dedup();
}

pub fn collect_foreign_field_accesses_for(
    func_node: Node,
    source: &str,
    self_names: &[&str],
    foreign: &mut Vec<(String, String)>,
) {
    let mut raw: Vec<(String, String)> = Vec::new();
    visit_field_accesses(func_node, source, self_names, &mut raw, try_extract_foreign);
    let scope = super::scope::build_scope_table(func_node, source);
    let unique: std::collections::BTreeSet<(String, String)> =
        raw.into_iter().map(|(recv, field)| (scope.get(&recv).cloned().unwrap_or(recv), field)).collect();
    foreign.extend(unique);
}

fn visit_field_accesses<T>(
    node: Node,
    source: &str,
    self_names: &[&str],
    out: &mut Vec<T>,
    extract: fn(Node, &str, &[&str]) -> Option<T>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if FIELD_ACCESS_KINDS.contains(&child.kind()) {
            if let Some(item) = extract(child, source, self_names) {
                out.push(item);
            }
        }
        visit_field_accesses(child, source, self_names, out, extract);
    }
}

fn try_extract_field(child: Node, source: &str, self_names: &[&str]) -> Option<String> {
    let mut attr_cursor = child.walk();
    let children: Vec<_> = child.children(&mut attr_cursor).collect();
    if children.len() < 2 {
        return None;
    }
    if !SELF_OBJ_KINDS.contains(&children[0].kind()) {
        return None;
    }
    let obj = node_text(children[0], source);
    if !self_names.contains(&obj) {
        return None;
    }
    children.iter().rev().find(|n| FIELD_NAME_KINDS.contains(&n.kind())).map(|n| node_text(*n, source).to_string())
}

fn try_extract_foreign(child: Node, source: &str, self_names: &[&str]) -> Option<(String, String)> {
    let mut attr_cursor = child.walk();
    let children: Vec<_> = child.children(&mut attr_cursor).collect();
    if children.len() < 2 {
        return None;
    }
    let receiver_text = if SELF_OBJ_KINDS.contains(&children[0].kind()) {
        let obj = node_text(children[0], source);
        if self_names.contains(&obj) {
            return None;
        }
        obj.to_string()
    } else {
        "?".to_string()
    };
    let field = children
        .iter()
        .rev()
        .find(|n| FIELD_NAME_KINDS.contains(&n.kind()))
        .map(|n| node_text(*n, source).to_string())?;
    Some((receiver_text, field))
}
