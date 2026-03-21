use std::hash::{Hash, Hasher};

use tree_sitter::Node;

use super::node_text;

pub fn compute_skeleton_hash(body: Node) -> u64 {
    let mut kinds: Vec<&str> = Vec::new();
    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        kinds.push(child.kind());
    }
    kinds.sort_unstable();
    kinds.dedup();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for kind in &kinds {
        kind.hash(&mut hasher);
    }
    hasher.finish()
}

pub fn compute_structural_fingerprint(node: Node) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    let mut cursor = node.walk();
    fingerprint_cursor(&mut cursor, &mut hasher);
    hasher.finish()
}

fn is_literal_kind(kind: &str) -> bool {
    matches!(
        kind,
        "identifier"
            | "string"
            | "integer"
            | "float"
            | "true"
            | "false"
            | "none"
            | "concatenated_string"
            | "template_string"
            | "number"
            | "string_fragment"
            | "property_identifier"
            | "shorthand_property_identifier"
            | "shorthand_property_identifier_pattern"
            | "null"
            | "undefined"
    )
}

fn fingerprint_cursor(cursor: &mut tree_sitter::TreeCursor, hasher: &mut impl Hasher) {
    let node = cursor.node();
    let kind = node.kind();

    if is_literal_kind(kind) {
        0xFF_u8.hash(hasher);
        return;
    }

    kind.hash(hasher);

    if is_expression_leaf(kind) {
        return;
    }

    if cursor.goto_first_child() {
        0xFE_u8.hash(hasher);
        loop {
            fingerprint_cursor(cursor, hasher);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
        0xFD_u8.hash(hasher);
    }
}

fn is_expression_leaf(kind: &str) -> bool {
    kind.ends_with("_expression")
        || kind.ends_with("_operator")
        || kind.ends_with("_literal")
        || kind.ends_with("_type")
        || matches!(
            kind,
            "call"
                | "attribute"
                | "subscript"
                | "tuple"
                | "list"
                | "dictionary"
                | "set"
                | "array"
                | "object"
                | "argument_list"
                | "arguments"
                | "type_arguments"
                | "parameters"
                | "formal_parameters"
                | "parameter_list"
                | "string"
                | "comment"
                | "line_comment"
                | "block_comment"
        )
}

pub fn fingerprint_walk(node: Node, hasher: &mut impl Hasher) {
    let mut cursor = node.walk();
    fingerprint_cursor(&mut cursor, hasher);
}

pub fn count_consecutive_asserts(body: Node, assert_kind: &str) -> u32 {
    let mut max_consecutive: u32 = 0;
    let mut current: u32 = 0;
    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        if child.kind() == assert_kind {
            current += 1;
        } else {
            current = 0;
        }
        if current > max_consecutive {
            max_consecutive = current;
        }
    }
    max_consecutive
}

pub fn compute_assert_fingerprint(body: Node, assert_kind: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        if child.kind() == assert_kind {
            fingerprint_walk(child, &mut hasher);
        }
    }
    hasher.finish()
}

// ─── Field access extraction ───────────────────────────────────────────

const FIELD_ACCESS_KINDS: &[&str] = &[
    "attribute",
    "member_expression",
    "field_expression",
    "field_access",
];
const SELF_OBJ_KINDS: &[&str] = &["identifier", "this", "self"];
const FIELD_NAME_KINDS: &[&str] = &["identifier", "property_identifier", "field_identifier"];

pub fn collect_field_accesses_for(
    func_node: Node,
    source: &str,
    self_names: &[&str],
    fields: &mut Vec<String>,
) {
    collect_field_accesses_recursive(func_node, source, self_names, fields);
    fields.sort();
    fields.dedup();
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
    let attr_name = children.last()?;
    if !FIELD_NAME_KINDS.contains(&attr_name.kind()) {
        return None;
    }
    Some(node_text(*attr_name, source).to_string())
}

fn collect_field_accesses_recursive(
    node: Node,
    source: &str,
    self_names: &[&str],
    fields: &mut Vec<String>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if FIELD_ACCESS_KINDS.contains(&child.kind()) {
            if let Some(field) = try_extract_field(child, source, self_names) {
                fields.push(field);
            }
        }
        collect_field_accesses_recursive(child, source, self_names, fields);
    }
}
