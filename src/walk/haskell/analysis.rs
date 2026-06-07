use tree_sitter::Node;

use super::super::counters::{count_short_variables, count_string_match_arms, max_same_primitive};
use super::super::{
    compute_assert_fingerprint, compute_skeleton_hash, compute_structural_fingerprint, count_consecutive_asserts,
    count_distinct_node_kinds, find_child_by_kind, node_text, FunctionMetrics, WalkState,
};
use super::extract_name;
use super::walk_tree::walk_function_matches;

const PRIMITIVE_TYPES: &[&str] = &[
    "Int", "Integer", "Float", "Double", "Char", "Bool", "String", "Word", "Word8", "Word16", "Word32", "Word64",
    "Int8", "Int16", "Int32", "Int64",
];

pub fn analyze_function_group(group: &[Node], source: &str, name: &str) -> Option<FunctionMetrics> {
    let first = *group.first()?;
    let last = *group.last()?;
    let mut s = WalkState::new();
    s.cc += (group.len() as u32).saturating_sub(1);
    for &node in group {
        walk_function_matches(node, source, 0, &mut s);
    }
    let (argc, prim, typed, max_same) = count_parameters(first, source, name);
    let mut m = build_metrics(first, last, first, source, &s);
    m.name = name.to_string();
    m.arg_count = argc;
    m.primitive_type_count = prim;
    m.typed_param_count = typed;
    m.max_same_primitive_count = max_same;
    Some(m)
}

#[allow(clippy::unnecessary_wraps)]
pub fn analyze_bind(node: Node, source: &str, name: &str) -> Option<FunctionMetrics> {
    let mut s = WalkState::new();
    walk_function_matches(node, source, 0, &mut s);
    let mut m = build_metrics(node, node, node, source, &s);
    m.name = name.to_string();
    Some(m)
}

fn count_parameters(func_node: Node, source: &str, name: &str) -> (u32, u32, u32, u32) {
    let argc = find_child_by_kind(func_node, "patterns").map_or(0, |p| {
        let mut c = p.walk();
        p.children(&mut c).count() as u32
    });
    let (prim, typed, max_same) = typed_from_signature(func_node, source, name);
    (argc, prim, typed, max_same)
}

fn typed_from_signature(func_node: Node, source: &str, name: &str) -> (u32, u32, u32) {
    let sig = find_sibling_signature(func_node, source, name);
    let Some(sig) = sig else { return (0, 0, 0) };
    let sig_text = node_text(sig, source);
    let Some(after) = sig_text.split("::").nth(1) else { return (0, 0, 0) };
    let segments = split_type_arrows(after);
    if segments.len() <= 1 {
        return (0, 0, 0);
    }
    let arg_types = &segments[..segments.len() - 1];
    let prim_types: Vec<&str> = arg_types.iter().map(|t| t.trim()).filter(|t| PRIMITIVE_TYPES.contains(t)).collect();
    (prim_types.len() as u32, arg_types.len() as u32, max_same_primitive(&prim_types))
}

fn find_sibling_signature<'a>(func_node: Node<'a>, source: &str, name: &str) -> Option<Node<'a>> {
    let parent = func_node.parent()?;
    let mut cursor = parent.walk();
    let result = parent.children(&mut cursor).find(|c| {
        c.kind() == "signature" && find_child_by_kind(*c, "variable").is_some_and(|v| node_text(v, source) == name)
    });
    result
}

fn split_type_arrows(type_str: &str) -> Vec<&str> {
    let mut segments = Vec::new();
    let mut depth = 0i32;
    let mut start = 0;
    let bytes = type_str.as_bytes();
    let mut i = 0;
    let cap = bytes.len().saturating_add(1);
    let mut steps = 0usize;
    while i < bytes.len() && steps < cap {
        match bytes[i] {
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth -= 1,
            b'-' if depth == 0 && i + 1 < bytes.len() && bytes[i + 1] == b'>' => {
                segments.push(&type_str[start..i]);
                i += 2;
                start = i;
                steps += 1;
                continue;
            }
            _ => {}
        }
        i += 1;
        steps += 1;
    }
    if start < type_str.len() {
        segments.push(&type_str[start..]);
    }
    segments
}

fn build_metrics(first: Node, last: Node, body: Node, source: &str, s: &WalkState) -> FunctionMetrics {
    let sl = first.start_position().row as u32 + 1;
    let el = last.end_position().row as u32 + 1;
    let name = extract_name(first, source);
    let (prim, typed, max_same) = typed_from_signature(first, source, &name);
    FunctionMetrics {
        name: String::new(),
        start_line: sl,
        end_line: el,
        loc: el.saturating_sub(sl) + 1,
        cc: s.cc,
        cognitive_complexity: s.cogc,
        max_nesting: s.max_nesting,
        bump_count: s.bump_count,
        arg_count: 0,
        compound_condition_count: s.compound_condition_count,
        is_constructor: false,
        max_embedded_block_loc: s.max_embedded_block_loc,
        structural_hash: compute_structural_fingerprint(body),
        distinct_node_kinds: count_distinct_node_kinds(body),
        skeleton_hash: compute_skeleton_hash(body),
        consecutive_asserts: count_consecutive_asserts(body, "apply"),
        assert_hash: compute_assert_fingerprint(body, "apply"),
        primitive_type_count: prim,
        typed_param_count: typed,
        max_same_primitive_count: max_same,
        empty_catch_count: 0,
        field_accesses: Vec::new(),
        foreign_field_accesses: Vec::new(),
        class_name: None,
        parent_class: None,
        short_var_count: count_short_variables(body, source, &["bind"]),
        string_match_arms: count_string_match_arms(body, "case", "alternative", &["string"], &[]),
        cpg: None,
    }
}
