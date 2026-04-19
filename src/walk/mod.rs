pub mod c;
pub mod cobol;
pub mod counters;
pub mod d;
pub mod cpp;
pub mod csharp;
pub mod fingerprint;
pub mod go;
pub mod groovy;
pub mod haskell;
pub mod java;
pub mod javascript;
pub mod kotlin;
pub mod lua;
pub mod objc;
pub mod php;
pub mod python;
pub mod r;
pub mod ruby;
pub mod rust;
pub mod shared;
pub mod swift;
pub mod tcl;
pub mod typescript;
pub mod zig;

// Re-export fingerprint and shared items so existing walker imports work unchanged.
pub use fingerprint::{
    collect_field_accesses_for, compute_assert_fingerprint, compute_skeleton_hash,
    compute_structural_fingerprint, count_consecutive_asserts,
};
pub use shared::is_catch_body_empty;

use tree_sitter::Node;

#[derive(Debug)]
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
        update_max(&mut self.max_nesting, depth + 1);
        if depth >= 2 && !self.saw_bump {
            self.bump_count += 1;
            self.saw_bump = true;
        }
    }

    pub fn track_loop(&mut self, depth: u32) {
        self.cc += 1;
        update_max(&mut self.max_nesting, depth + 1);
    }

    pub fn track_nesting(&mut self, depth: u32) {
        update_max(&mut self.max_nesting, depth + 1);
    }

    pub fn reset_bump(&mut self) {
        self.saw_bump = false;
    }
}

fn update_max(current: &mut u32, new: u32) {
    if new > *current {
        *current = new;
    }
}

pub fn track_embedded_block(max: &mut u32, node: Node) {
    let lines =
        node.end_position().row.saturating_sub(node.start_position().row) as u32 + 1;
    update_max(max, lines);
}

#[derive(Debug, Clone)]
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
    pub short_var_count: u32,
    pub string_match_arms: u32,
}

#[derive(Debug, Clone)]
pub struct ModuleMetrics {
    pub total_loc: u32,
    pub total_functions: u32,
    pub sum_cc: u32,
    pub global_conditional_count: u32,
    pub global_max_nesting: u32,
    pub declaration_count: u32,
    pub struct_fields: Vec<(String, u32)>,
}

pub struct FileMetrics {
    pub functions: Vec<FunctionMetrics>,
    pub module: ModuleMetrics,
}

// ─── Shared utilities ──────────────────────────────────────────────────

pub fn count_code_lines(source: &str, comment_prefixes: &[&str]) -> u32 {
    source
        .lines()
        .filter(|line| is_code_line(line.as_bytes(), comment_prefixes))
        .count() as u32
}

fn is_code_line(line: &[u8], comment_prefixes: &[&str]) -> bool {
    let trimmed = trim_leading_whitespace(line);
    !trimmed.is_empty()
        && !comment_prefixes
            .iter()
            .any(|p| trimmed.starts_with(p.as_bytes()))
}

fn trim_leading_whitespace(line: &[u8]) -> &[u8] {
    let skip = line
        .iter()
        .take_while(|&&b| b == b' ' || b == b'\t')
        .count();
    &line[skip..]
}

pub fn find_child_by_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    // cursor must outlive the iterator — binding extends the borrow
    let result = node.children(&mut cursor).find(|c| c.kind() == kind);
    result
}

pub fn node_text<'a>(node: Node, source: &'a str) -> &'a str {
    &source[node.byte_range()]
}

pub fn track_global_nesting(child: Node, max_nesting: &mut u32, branch_kinds: &[&str]) {
    let depth = measure_nesting_depth(child, 1, branch_kinds);
    update_max(max_nesting, depth);
}

pub fn measure_nesting_depth(node: Node, current: u32, branch_kinds: &[&str]) -> u32 {
    let mut max = current;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let child_depth = if branch_kinds.contains(&child.kind()) {
            measure_nesting_depth(child, current + 1, branch_kinds)
        } else if matches!(child.kind(), "block" | "statement_block" | "block_statement" | "body_statement" | "then" | "do" | "braced_word" | "braced_expression") {
            measure_nesting_depth(child, current, branch_kinds)
        } else {
            current
        };
        update_max(&mut max, child_depth);
    }
    max
}
