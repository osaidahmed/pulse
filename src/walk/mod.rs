pub mod python;
pub mod typescript;
pub mod javascript;
pub mod rust;
pub mod c;
pub mod cpp;
pub mod java;

use std::hash::{Hash, Hasher};

pub struct WalkState {
    pub cc: u32,
    pub max_nesting: u32,
    pub bump_count: u32,
    pub compound_condition_count: u32,
    pub max_embedded_block_loc: u32,
    pub saw_bump: bool,
}

impl WalkState {
    pub fn new() -> Self {
        Self {
            cc: 1,
            max_nesting: 0,
            bump_count: 0,
            compound_condition_count: 0,
            max_embedded_block_loc: 0,
            saw_bump: false,
        }
    }

    pub fn track_if(&mut self, depth: u32) {
        self.cc += 1;
        let d = depth + 1;
        if d > self.max_nesting { self.max_nesting = d; }
        if depth >= 2 && !self.saw_bump {
            self.bump_count += 1;
            self.saw_bump = true;
        }
    }

    pub fn track_loop(&mut self, depth: u32) {
        self.cc += 1;
        let d = depth + 1;
        if d > self.max_nesting { self.max_nesting = d; }
    }

    pub fn track_nesting(&mut self, depth: u32) {
        let d = depth + 1;
        if d > self.max_nesting { self.max_nesting = d; }
    }

    pub fn track_embedded(&mut self, node: Node) {
        let lines = node.end_position().row.saturating_sub(node.start_position().row) as u32 + 1;
        if lines > self.max_embedded_block_loc { self.max_embedded_block_loc = lines; }
    }

    pub fn reset_bump(&mut self) {
        self.saw_bump = false;
    }
}
use tree_sitter::Node;

#[derive(Debug)]
pub struct FunctionMetrics {
    pub name: String,
    pub start_line: u32,
    pub end_line: u32,
    pub loc: u32,
    pub cc: u32,
    pub max_nesting: u32,
    pub bump_count: u32,
    pub arg_count: u32,
    pub compound_condition_count: u32,
    pub is_constructor: bool,
    pub max_embedded_block_loc: u32,
    pub structural_hash: u64,
    pub consecutive_asserts: u32,
    pub assert_hash: u64,
    pub primitive_type_count: u32,
    pub typed_param_count: u32,
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
    source
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !comment_prefixes.iter().any(|p| trimmed.starts_with(p))
        })
        .count() as u32
}

pub fn find_child_by_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    let result = node.children(&mut cursor).find(|c| c.kind() == kind);
    result
}

pub fn node_text(node: Node, source: &str) -> String {
    source[node.byte_range()].to_string()
}

pub fn compute_structural_fingerprint(node: Node) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    fingerprint_walk(node, &mut hasher);
    hasher.finish()
}

pub fn fingerprint_walk(node: Node, hasher: &mut impl Hasher) {
    let kind = node.kind();

    match kind {
        "identifier" | "string" | "integer" | "float" | "true" | "false"
        | "none" | "concatenated_string" | "template_string" | "number"
        | "string_fragment" | "property_identifier" | "shorthand_property_identifier"
        | "shorthand_property_identifier_pattern" | "null" | "undefined" => {
            0xFF_u8.hash(hasher);
        }
        _ => {
            kind.hash(hasher);
        }
    }

    if node.child_count() > 0 {
        0xFE_u8.hash(hasher);
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            fingerprint_walk(child, hasher);
        }
        0xFD_u8.hash(hasher);
    }
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
    "attribute", "member_expression", "field_expression", "field_access",
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
    if !self_names.contains(&obj.as_str()) {
        return None;
    }
    let attr_name = children.last()?;
    if !FIELD_NAME_KINDS.contains(&attr_name.kind()) {
        return None;
    }
    Some(node_text(*attr_name, source))
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
