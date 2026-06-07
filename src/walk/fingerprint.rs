use std::hash::{Hash, Hasher};

use tree_sitter::Node;
use xxhash_rust::xxh3::Xxh3;

pub const FINGERPRINT_VERSION: u64 = 1;

fn fingerprint_hasher() -> Xxh3 {
    Xxh3::with_seed(FINGERPRINT_VERSION)
}

pub fn compute_skeleton_hash(node: Node) -> u64 {
    let Some(_guard) = super::DepthGuard::enter() else {
        return 0;
    };
    let mut child_hashes: Vec<u64> = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if !child.is_named() {
            continue;
        }
        let mut h = fingerprint_hasher();
        child.kind().hash(&mut h);
        compute_skeleton_hash(child).hash(&mut h);
        child_hashes.push(h.finish());
    }
    child_hashes.sort_unstable();
    let mut hasher = fingerprint_hasher();
    for ch in &child_hashes {
        ch.hash(&mut hasher);
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

pub fn count_distinct_node_kinds(node: Node) -> u32 {
    let mut kinds: std::collections::HashSet<&str> = std::collections::HashSet::new();
    collect_distinct_kinds(node, &mut kinds);
    kinds.len() as u32
}

pub fn count_distinct_node_kinds_multi(nodes: &[Node]) -> u32 {
    let mut kinds: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for node in nodes {
        collect_distinct_kinds(*node, &mut kinds);
    }
    kinds.len() as u32
}

fn collect_distinct_kinds<'a>(node: Node<'a>, kinds: &mut std::collections::HashSet<&'a str>) {
    let Some(_guard) = super::DepthGuard::enter() else {
        return;
    };
    let kind = node.kind();
    if is_literal_kind(kind) {
        kinds.insert("\u{0}literal");
        return;
    }
    kinds.insert(kind);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.is_named() {
            collect_distinct_kinds(child, kinds);
        }
    }
}

const ASSERT_GAP_TOLERANCE: u32 = 1;

pub fn count_consecutive_asserts(body: Node, assert_kind: &str) -> u32 {
    let mut max_consecutive: u32 = 0;
    let mut current: u32 = 0;
    let mut gap: u32 = 0;
    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        if !child.is_named() {
            continue;
        }
        if child.kind() == assert_kind {
            current += 1;
            gap = 0;
        } else {
            gap += 1;
            if gap > ASSERT_GAP_TOLERANCE {
                current = 0;
            }
        }
        max_consecutive = max_consecutive.max(current);
    }
    max_consecutive
}

pub fn compute_assert_fingerprint(body: Node, assert_kind: &str) -> u64 {
    let mut hasher = fingerprint_hasher();
    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        if !child.is_named() {
            continue;
        }
        if child.kind() == assert_kind {
            fingerprint_walk(child, &mut hasher);
        }
    }
    hasher.finish()
}
