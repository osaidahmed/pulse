use std::hash::{Hash, Hasher};

use tree_sitter::Node;
use xxhash_rust::xxh3::Xxh3;

use super::node_text;

pub const FINGERPRINT_VERSION: u64 = 1;

fn fingerprint_hasher() -> Xxh3 {
    Xxh3::with_seed(FINGERPRINT_VERSION)
}

pub fn compute_skeleton_hash(body: Node) -> u64 {
    let mut kinds: Vec<&str> = Vec::new();
    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        kinds.push(child.kind());
    }
    kinds.sort_unstable();
    // No dedup — multiset hash preserves kind COUNTS, not just vocabulary.
    // Two functions must use each statement type the same number of times to match.
    let mut hasher = fingerprint_hasher();
    for kind in &kinds {
        kind.hash(&mut hasher);
    }
    hasher.finish()
}

pub fn compute_structural_fingerprint(node: Node) -> u64 {
    let mut hasher = fingerprint_hasher();
    let mut cursor = node.walk();
    fingerprint_cursor(&mut cursor, &mut hasher);
    hasher.finish()
}

#[allow(dead_code)]
pub fn compute_subtree_fingerprint(node: Node) -> u64 {
    let mut hasher = fingerprint_hasher();
    fingerprint_subtree_into(node, &mut hasher);
    hasher.finish()
}

pub fn compute_subtree_fingerprint_seeded(node: Node, seed: u64) -> u64 {
    let mut hasher = fingerprint_hasher();
    seed.hash(&mut hasher);
    fingerprint_subtree_into(node, &mut hasher);
    hasher.finish()
}

fn fingerprint_subtree_into(node: Node, hasher: &mut impl Hasher) {
    let Some(_guard) = super::DepthGuard::enter() else {
        return;
    };
    if is_subtree_skipped_kind(node.kind()) {
        return;
    }
    if let Some(inner) = unwrap_passthrough(node) {
        fingerprint_subtree_into(inner, hasher);
        return;
    }
    node.kind().hash(hasher);
    if node.named_child_count() == 0 {
        return;
    }
    0xFE_u8.hash(hasher);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.is_named() {
            fingerprint_subtree_into(child, hasher);
        }
    }
    0xFD_u8.hash(hasher);
}

fn is_subtree_skipped_kind(kind: &str) -> bool {
    matches!(kind, "comment" | "line_comment" | "block_comment")
}

fn unwrap_passthrough(node: Node) -> Option<Node> {
    if !matches!(node.kind(), "parenthesized_expression") {
        return None;
    }
    let mut cursor = node.walk();
    let result = node.children(&mut cursor).find(tree_sitter::Node::is_named);
    result
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
    let Some(_guard) = super::DepthGuard::enter() else {
        return;
    };
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
        fingerprint_siblings(cursor, hasher);
        cursor.goto_parent();
        0xFD_u8.hash(hasher);
    }
}

fn fingerprint_siblings(cursor: &mut tree_sitter::TreeCursor, hasher: &mut impl Hasher) {
    const MAX_SIBLINGS: usize = 1 << 20;
    let mut sibs: usize = 0;
    loop {
        fingerprint_cursor(cursor, hasher);
        sibs += 1;
        if sibs >= MAX_SIBLINGS || !cursor.goto_next_sibling() {
            break;
        }
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

const ASSERT_GAP_TOLERANCE: u32 = 1;

pub fn count_consecutive_asserts(body: Node, assert_kind: &str) -> u32 {
    let mut max_consecutive: u32 = 0;
    let mut current: u32 = 0;
    let mut gap: u32 = 0;
    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        if !child.is_named() { continue; }
        if child.kind() == assert_kind {
            current += 1;
            gap = 0;
        } else {
            gap += 1;
            if gap > ASSERT_GAP_TOLERANCE { current = 0; }
        }
        max_consecutive = max_consecutive.max(current);
    }
    max_consecutive
}

pub fn compute_assert_fingerprint(body: Node, assert_kind: &str) -> u64 {
    let mut hasher = fingerprint_hasher();
    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        if !child.is_named() { continue; }
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

pub fn collect_field_accesses_for(
    func_node: Node,
    source: &str,
    self_names: &[&str],
    fields: &mut Vec<String>,
) {
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
    let unique: std::collections::BTreeSet<(String, String)> = raw.into_iter().collect();
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
    children
        .iter()
        .rev()
        .find(|n| FIELD_NAME_KINDS.contains(&n.kind()))
        .map(|n| node_text(*n, source).to_string())
}

fn try_extract_foreign(
    child: Node,
    source: &str,
    self_names: &[&str],
) -> Option<(String, String)> {
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
