pub mod c;
pub mod cpp;
pub mod csharp;
pub mod java;
pub mod javascript;
pub mod python;
pub mod rust;
pub mod typescript;

use std::hash::{Hash, Hasher};

pub struct WalkState {
    pub cc: u32,
    pub cogc: u32,
    pub cogc_nesting: u32,
    pub max_nesting: u32,
    pub bump_count: u32,
    pub compound_condition_count: u32,
    pub max_embedded_block_loc: u32,
    pub empty_catch_count: u32,
    pub saw_bump: bool,
}

impl WalkState {
    pub fn new() -> Self {
        Self {
            cc: 1,
            cogc: 0,
            cogc_nesting: 0,
            max_nesting: 0,
            bump_count: 0,
            compound_condition_count: 0,
            max_embedded_block_loc: 0,
            empty_catch_count: 0,
            saw_bump: false,
        }
    }

    pub fn track_cogc_branch(&mut self) {
        self.cogc += 1 + self.cogc_nesting;
    }

    pub fn track_cogc_flat(&mut self) {
        self.cogc += 1;
    }

    pub fn track_if(&mut self, depth: u32) {
        self.cc += 1;
        let d = depth + 1;
        if d > self.max_nesting {
            self.max_nesting = d;
        }
        if depth >= 2 && !self.saw_bump {
            self.bump_count += 1;
            self.saw_bump = true;
        }
    }

    pub fn track_loop(&mut self, depth: u32) {
        self.cc += 1;
        let d = depth + 1;
        if d > self.max_nesting {
            self.max_nesting = d;
        }
    }

    pub fn track_nesting(&mut self, depth: u32) {
        let d = depth + 1;
        if d > self.max_nesting {
            self.max_nesting = d;
        }
    }

    pub fn track_embedded(&mut self, node: Node) {
        let lines = node
            .end_position()
            .row
            .saturating_sub(node.start_position().row) as u32
            + 1;
        if lines > self.max_embedded_block_loc {
            self.max_embedded_block_loc = lines;
        }
    }

    pub fn reset_bump(&mut self) {
        self.saw_bump = false;
    }
}
use tree_sitter::Node;

pub fn is_catch_body_empty(catch_node: Node, body_kind: &str, pass_kind: Option<&str>) -> bool {
    let Some(body) = find_child_by_kind(catch_node, body_kind) else {
        return true;
    };
    let mut cursor = body.walk();
    let meaningful_children = body
        .children(&mut cursor)
        .filter(|c| {
            let kind = c.kind();
            kind != body_kind
                && kind != "comment"
                && kind != "line_comment"
                && kind != "block_comment"
                && kind != "{"
                && kind != "}"
                && kind != ":"
                && (pass_kind != Some(kind))
        })
        .count();
    meaningful_children == 0
}

#[derive(Debug)]
pub struct FunctionMetrics {
    pub name: String,
    pub start_line: u32,
    pub end_line: u32,
    pub loc: u32,
    pub cc: u32,
    pub cognitive_complexity: u32,
    pub max_nesting: u32,
    pub bump_count: u32,
    pub arg_count: u32,
    pub compound_condition_count: u32,
    pub is_constructor: bool,
    pub max_embedded_block_loc: u32,
    pub structural_hash: u64,
    pub skeleton_hash: u64,
    pub consecutive_asserts: u32,
    pub assert_hash: u64,
    pub primitive_type_count: u32,
    pub typed_param_count: u32,
    pub empty_catch_count: u32,
    pub field_accesses: Vec<String>,
    pub class_name: Option<String>,
}

#[derive(Debug)]
pub struct ModuleMetrics {
    pub total_loc: u32,
    pub total_functions: u32,
    pub sum_cc: u32,
    pub global_conditional_count: u32,
    pub global_max_nesting: u32,
    pub declaration_count: u32,
}

pub type FileMetrics = (Vec<FunctionMetrics>, ModuleMetrics);

// ---------------------------------------------------------------------------
// Shared utilities — used by all language walkers
// ---------------------------------------------------------------------------

pub fn count_code_lines(source: &str, comment_prefixes: &[&str]) -> u32 {
    let bytes = source.as_bytes();
    let mut count: u32 = 0;
    let mut start = 0;

    loop {
        // Skip leading whitespace
        while start < bytes.len() && (bytes[start] == b' ' || bytes[start] == b'\t') {
            start += 1;
        }

        if start >= bytes.len() {
            break;
        }

        // Find end of line
        let end = memchr::memchr(b'\n', &bytes[start..])
            .map_or(bytes.len(), |pos| start + pos);

        // Non-empty, non-whitespace-only line — check for comment prefix
        if start < end {
            let line_start = &bytes[start..end];
            let is_comment = comment_prefixes
                .iter()
                .any(|p| line_start.starts_with(p.as_bytes()));
            if !is_comment {
                count += 1;
            }
        }

        if end >= bytes.len() {
            break;
        }
        start = end + 1;
    }

    count
}

pub fn find_child_by_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    let result = node.children(&mut cursor).find(|c| c.kind() == kind);
    result
}

pub fn node_text<'a>(node: Node, source: &'a str) -> &'a str {
    &source[node.byte_range()]
}

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

    // Skip recursing into expression internals — hash their kind but not children.
    // Statement-level structure is sufficient for clone detection.
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
    // Skip recursing into expression/type internals for fingerprinting.
    // Statement-level structure (control flow shape) is what matters for clone detection.
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

// Keep the old signature available for assert fingerprinting
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

pub fn measure_nesting_depth(node: Node, current: u32, branch_kinds: &[&str]) -> u32 {
    let mut max = current;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let child_depth = if branch_kinds.contains(&child.kind()) {
            measure_nesting_depth(child, current + 1, branch_kinds)
        } else if child.kind() == "block" || child.kind() == "statement_block" {
            measure_nesting_depth(child, current, branch_kinds)
        } else {
            current
        };
        if child_depth > max {
            max = child_depth;
        }
    }
    max
}

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

const FIELD_ACCESS_KINDS: &[&str] = &[
    "attribute",
    "member_expression",
    "field_expression",
    "field_access",
];
const SELF_OBJ_KINDS: &[&str] = &["identifier", "this", "self"];
const FIELD_NAME_KINDS: &[&str] = &["identifier", "property_identifier", "field_identifier"];

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
