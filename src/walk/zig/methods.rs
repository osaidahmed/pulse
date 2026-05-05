use tree_sitter::Node;

use super::super::{
    collect_field_accesses_for, collect_foreign_field_accesses_for, compute_assert_fingerprint,
    compute_skeleton_hash, compute_structural_fingerprint, count_consecutive_asserts,
    find_child_by_kind, node_text, FunctionMetrics, WalkState,
};
use super::super::counters::{count_short_variables, count_string_match_arms};

const PRIMITIVE_TYPES: &[&str] = &[
    "i8", "i16", "i32", "i64", "i128", "u8", "u16", "u32", "u64", "u128", "usize", "isize",
    "f16", "f32", "f64", "f80", "f128", "bool", "void", "noreturn", "anyerror", "anytype",
    "comptime_int", "comptime_float", "c_int", "c_uint", "c_long", "c_ulong", "c_longlong",
    "c_ulonglong", "c_char", "c_short", "c_ushort",
];

pub fn try_collect_struct_methods(var_decl: Node, source: &str, functions: &mut Vec<FunctionMetrics>) {
    let Some(struct_decl) = find_child_by_kind(var_decl, "struct_declaration") else { return; };
    let type_name = find_child_by_kind(var_decl, "identifier").map(|n| node_text(n, source));
    let mut cursor = struct_decl.walk();
    for child in struct_decl.children(&mut cursor) {
        if child.kind() == "function_declaration" {
            try_add_method(child, source, type_name, functions);
        }
    }
}

fn try_add_method(node: Node, source: &str, type_name: Option<&str>, functions: &mut Vec<FunctionMetrics>) {
    let Some(mut m) = analyze_function(node, source) else { return; };
    let method_name = m.name.clone();
    let self_present = has_self_param(node, source);
    if let Some(tn) = type_name { m.name = format!("{tn}.{method_name}"); }
    m.class_name = type_name.map(String::from);
    m.is_constructor = method_name == "init" || method_name == "deinit";
    if self_present {
        m.arg_count = m.arg_count.saturating_sub(1);
        if !m.is_constructor {
            collect_field_accesses_for(node, source, &["self"], &mut m.field_accesses);
            collect_foreign_field_accesses_for(node, source, &["self"], &mut m.foreign_field_accesses);
            m.field_accesses.sort();
            m.field_accesses.dedup();
        }
    }
    functions.push(m);
}

fn has_self_param(func: Node, source: &str) -> bool {
    let Some(params) = find_child_by_kind(func, "parameters") else { return false; };
    let mut cursor = params.walk();
    for child in params.children(&mut cursor) {
        if child.kind() != "parameter" { continue; }
        let name = find_child_by_kind(child, "identifier").map(|n| node_text(n, source));
        return matches!(name, Some("self"));
    }
    false
}

pub fn analyze_function(node: Node, source: &str) -> Option<FunctionMetrics> {
    let name = find_child_by_kind(node, "identifier")
        .map_or_else(|| "<anonymous>".into(), |n| node_text(n, source).to_string());
    let params = count_parameters(node, source);
    build_metrics(node, source, name, params)
}

pub fn analyze_test(node: Node, source: &str) -> Option<FunctionMetrics> {
    let name = find_child_by_kind(node, "string")
        .and_then(|s| find_child_by_kind(s, "string_content"))
        .map_or_else(
            || "test_unnamed".into(),
            |n| format!("test_{}", node_text(n, source).replace(' ', "_")),
        );
    let params = ParamCounts { total: 0, primitive: 0, typed: 0 };
    build_metrics(node, source, name, params)
}

struct ParamCounts {
    total: u32,
    primitive: u32,
    typed: u32,
}

fn build_metrics(
    node: Node, source: &str, name: String, params: ParamCounts,
) -> Option<FunctionMetrics> {
    let start_line = node.start_position().row as u32 + 1;
    let end_line = node.end_position().row as u32 + 1;
    let loc = end_line.saturating_sub(start_line) + 1;
    let body = find_child_by_kind(node, "block")?;
    let mut s = WalkState::new();
    super::walk_body_pub(body, source, 0, &mut s);
    Some(FunctionMetrics {
        name, start_line, end_line, loc,
        cc: s.cc, cognitive_complexity: s.cogc,
        max_nesting: s.max_nesting, bump_count: s.bump_count,
        arg_count: params.total,
        compound_condition_count: s.compound_condition_count,
        is_constructor: false,
        max_embedded_block_loc: s.max_embedded_block_loc,
        structural_hash: compute_structural_fingerprint(body),
        skeleton_hash: compute_skeleton_hash(body),
        consecutive_asserts: count_consecutive_asserts(body, "expression_statement"),
        assert_hash: compute_assert_fingerprint(body, "expression_statement"),
        primitive_type_count: params.primitive,
        typed_param_count: params.typed,
        empty_catch_count: 0,
        field_accesses: Vec::new(),
        foreign_field_accesses: Vec::new(),
        class_name: None,
        parent_class: None,
        short_var_count: count_short_variables(body, source, &["VarDecl"]),
        string_match_arms: count_string_match_arms(body, "SwitchExpr", "SwitchProng", &["STRINGLITERALSINGLE"]),
    })
}

fn count_parameters(func: Node, source: &str) -> ParamCounts {
    let Some(params) = find_child_by_kind(func, "parameters") else {
        return ParamCounts { total: 0, primitive: 0, typed: 0 };
    };
    let mut cursor = params.walk();
    params
        .children(&mut cursor)
        .filter(|c| c.kind() == "parameter")
        .fold(ParamCounts { total: 0, primitive: 0, typed: 0 }, |mut counts, child| {
            counts.total += 1;
            counts.typed += 1;
            if is_primitive_param(child, source) { counts.primitive += 1; }
            counts
        })
}

fn is_primitive_param(param: Node, source: &str) -> bool {
    find_child_by_kind(param, "builtin_type")
        .map(|n| node_text(n, source))
        .is_some_and(|text| PRIMITIVE_TYPES.contains(&text))
}
