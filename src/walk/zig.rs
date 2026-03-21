use tree_sitter::{Node, Tree};

use super::shared::{self, count_boolean_ops, count_cogc_sequences, GlobalMetricsConfig};
use super::{
    collect_field_accesses_for, compute_assert_fingerprint, compute_skeleton_hash,
    compute_structural_fingerprint, count_code_lines, count_consecutive_asserts,
    find_child_by_kind, node_text, track_embedded_block, FileMetrics, FunctionMetrics,
    ModuleMetrics, WalkState,
};

const COMMENT_PREFIXES: &[&str] = &["//", "/*", "*"];
const PRIMITIVE_TYPES: &[&str] = &[
    "i8", "i16", "i32", "i64", "i128", "u8", "u16", "u32", "u64", "u128", "usize", "isize",
    "f16", "f32", "f64", "f80", "f128", "bool", "void", "noreturn", "anyerror", "anytype",
    "comptime_int", "comptime_float", "c_int", "c_uint", "c_long", "c_ulong", "c_longlong",
    "c_ulonglong", "c_char", "c_short", "c_ushort",
];
const NESTING_BRANCH_KINDS: &[&str] = &[
    "if_statement",
    "for_statement",
    "while_statement",
    "switch_expression",
];
const BOOL_OPS: &[&str] = &["and", "or"];
const BOOL_STOPS: &[&str] = &[
    "block",
    "function_declaration",
    "test_declaration",
    "struct_declaration",
];
const GLOBAL_CFG: GlobalMetricsConfig = GlobalMetricsConfig {
    cond: &["if_statement"],
    loops: &["while_statement", "for_statement"],
    branches: NESTING_BRANCH_KINDS,
    recurse: &[],
};

pub fn walk(tree: &Tree, source: &str) -> FileMetrics {
    let root = tree.root_node();
    let total_loc = count_code_lines(source, COMMENT_PREFIXES);

    let mut functions = Vec::new();
    let mut global_conditional_count: u32 = 0;
    let mut global_max_nesting: u32 = 0;

    collect_functions(root, source, &mut functions);
    shared::collect_global_metrics(
        root,
        &mut global_conditional_count,
        &mut global_max_nesting,
        &GLOBAL_CFG,
    );

    let total_functions = functions.len() as u32;
    let sum_cc: u32 = functions.iter().map(|f| f.cc).sum();
    let declaration_count = count_declarations(root);

    let module = ModuleMetrics {
        total_loc,
        total_functions,
        sum_cc,
        global_conditional_count,
        global_max_nesting,
        declaration_count,
    };

    (functions, module)
}

fn collect_functions(node: Node, source: &str, functions: &mut Vec<FunctionMetrics>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        dispatch_top_level(child, source, functions);
    }
}

fn dispatch_top_level(child: Node, source: &str, functions: &mut Vec<FunctionMetrics>) {
    match child.kind() {
        "function_declaration" => try_add_function(child, source, functions),
        "test_declaration" => try_add_test(child, source, functions),
        "variable_declaration" => try_collect_struct_methods(child, source, functions),
        _ => {}
    }
}

fn try_add_function(node: Node, source: &str, functions: &mut Vec<FunctionMetrics>) {
    if let Some(m) = analyze_function(node, source) {
        functions.push(m);
    }
}

fn try_add_test(node: Node, source: &str, functions: &mut Vec<FunctionMetrics>) {
    if let Some(m) = analyze_test(node, source) {
        functions.push(m);
    }
}

fn try_collect_struct_methods(
    var_decl: Node,
    source: &str,
    functions: &mut Vec<FunctionMetrics>,
) {
    let Some(struct_decl) = find_child_by_kind(var_decl, "struct_declaration") else {
        return;
    };
    let type_name = find_child_by_kind(var_decl, "identifier")
        .map(|n| node_text(n, source));
    let mut cursor = struct_decl.walk();
    for child in struct_decl.children(&mut cursor) {
        if child.kind() == "function_declaration" {
            try_add_method(child, source, type_name, functions);
        }
    }
}

fn try_add_method(
    node: Node,
    source: &str,
    type_name: Option<&str>,
    functions: &mut Vec<FunctionMetrics>,
) {
    let Some(mut m) = analyze_function(node, source) else {
        return;
    };
    let method_name = m.name.clone();
    let self_present = has_self_param(node, source);

    if let Some(tn) = type_name {
        m.name = format!("{tn}.{method_name}");
    }
    m.class_name = type_name.map(String::from);
    m.is_constructor = method_name == "init" || method_name == "deinit";

    if self_present {
        m.arg_count = m.arg_count.saturating_sub(1);
        if !m.is_constructor {
            collect_field_accesses_for(node, source, &["self"], &mut m.field_accesses);
            m.field_accesses.sort();
            m.field_accesses.dedup();
        }
    }

    functions.push(m);
}

fn has_self_param(func: Node, source: &str) -> bool {
    let Some(params) = find_child_by_kind(func, "parameters") else {
        return false;
    };
    let mut cursor = params.walk();
    for child in params.children(&mut cursor) {
        if child.kind() != "parameter" {
            continue;
        }
        let name = find_child_by_kind(child, "identifier").map(|n| node_text(n, source));
        return matches!(name, Some("self"));
    }
    false
}

fn analyze_function(node: Node, source: &str) -> Option<FunctionMetrics> {
    let name = find_child_by_kind(node, "identifier")
        .map_or_else(|| "<anonymous>".into(), |n| node_text(n, source).to_string());
    let params = count_parameters(node, source);
    build_metrics(node, source, name, params)
}

fn analyze_test(node: Node, source: &str) -> Option<FunctionMetrics> {
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
    node: Node,
    source: &str,
    name: String,
    params: ParamCounts,
) -> Option<FunctionMetrics> {
    let start_line = node.start_position().row as u32 + 1;
    let end_line = node.end_position().row as u32 + 1;
    let loc = end_line.saturating_sub(start_line) + 1;

    let body = find_child_by_kind(node, "block")?;
    let mut s = WalkState::new();
    walk_body(body, source, 0, &mut s);

    Some(FunctionMetrics {
        name,
        start_line,
        end_line,
        loc,
        cc: s.cc,
        cognitive_complexity: s.cogc,
        max_nesting: s.max_nesting,
        bump_count: s.bump_count,
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
        class_name: None,
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

// ─── Body walking ──────────────────────────────────────────────────────

fn walk_body(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    s.reset_bump();
    for child in node.children(&mut cursor) {
        walk_node(child, source, depth, s);
    }
}

fn walk_node(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    if dispatch_control_flow(child, source, depth, s) {
        return;
    }
    match child.kind() {
        "comptime_statement" => {}
        "string" | "multiline_string" => {
            track_embedded_block(&mut s.max_embedded_block_loc, child);
        }
        _ => walk_body(child, source, depth, s),
    }
}

fn dispatch_control_flow(node: Node, source: &str, depth: u32, s: &mut WalkState) -> bool {
    match node.kind() {
        "if_statement" => handle_if(node, source, depth, s),
        "labeled_statement" => walk_labeled_statement(node, source, depth, s),
        "for_statement" | "while_statement" => handle_loop(node, source, depth, s),
        "switch_expression" => handle_switch(node, source, depth, s),
        "catch_expression" => handle_catch(node, source, depth, s),
        "binary_expression" => handle_binary(node, source, depth, s),
        "defer_statement" | "errdefer_statement" | "labeled_type_expression" => {
            walk_body(node, source, depth, s);
        }
        _ => return false,
    }
    true
}

fn walk_labeled_statement(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let inner = find_child_by_kind(node, "for_statement")
        .or_else(|| find_child_by_kind(node, "while_statement"));
    if let Some(loop_node) = inner {
        handle_loop(loop_node, source, depth, s);
    } else {
        walk_body(node, source, depth, s);
    }
}

fn handle_binary(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();
    let has_orelse = children.iter().any(|c| c.kind() == "orelse");
    if has_orelse {
        s.cc += 1;
        s.track_cogc_branch();
    }
    for child in children {
        walk_node(child, source, depth, s);
    }
}

fn handle_catch(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.cc += 1;
    s.track_cogc_branch();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "block" {
            let saved = s.cogc_nesting;
            s.cogc_nesting += 1;
            walk_body(child, source, depth + 1, s);
            s.cogc_nesting = saved;
        }
    }
}

fn handle_if(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.track_if(depth);
    s.track_cogc_branch();
    count_boolean_ops(node, &mut s.cc, BOOL_OPS, BOOL_STOPS);
    count_cogc_sequences(node, &mut s.cogc, BOOL_OPS, BOOL_STOPS);
    check_condition_complexity(node, source, &mut s.compound_condition_count);
    walk_if_children(node, source, depth + 1, s);
}

fn walk_if_children(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "block_expression" => {
                let saved = s.cogc_nesting;
                s.cogc_nesting += 1;
                walk_body(child, source, depth, s);
                s.cogc_nesting = saved;
            }
            "else_clause" => walk_else_clause(child, source, depth, s),
            _ => {}
        }
    }
}

fn walk_else_clause(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "if_statement" => {
                s.cc += 1;
                s.track_cogc_branch();
                count_boolean_ops(child, &mut s.cc, BOOL_OPS, BOOL_STOPS);
                count_cogc_sequences(child, &mut s.cogc, BOOL_OPS, BOOL_STOPS);
                check_condition_complexity(child, source, &mut s.compound_condition_count);
                walk_if_children(child, source, depth, s);
            }
            "labeled_statement" | "block_expression" => {
                s.track_cogc_flat();
                let saved = s.cogc_nesting;
                s.cogc_nesting += 1;
                walk_body(child, source, depth, s);
                s.cogc_nesting = saved;
            }
            _ => {}
        }
    }
}

fn handle_loop(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.track_loop(depth);
    s.track_cogc_branch();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "block_expression" || child.kind() == "block" {
            let saved = s.cogc_nesting;
            s.cogc_nesting += 1;
            walk_body(child, source, depth + 1, s);
            s.cogc_nesting = saved;
        }
    }
}

fn handle_switch(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.track_nesting(depth);
    s.track_cogc_branch();
    let saved = s.cogc_nesting;
    s.cogc_nesting += 1;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() != "switch_case" {
            continue;
        }
        if !is_default_case(child) {
            s.cc += 1;
        }
        walk_case_body(child, source, depth + 1, s);
    }
    s.cogc_nesting = saved;
}

fn is_default_case(case: Node) -> bool {
    let mut cursor = case.walk();
    let result = case.children(&mut cursor).any(|c| c.kind() == "else");
    result
}

fn walk_case_body(case: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = case.walk();
    for child in case.children(&mut cursor) {
        match child.kind() {
            "block" | "block_expression" => walk_body(child, source, depth, s),
            "if_statement" | "binary_expression" | "call_expression" | "return_expression"
            | "catch_expression" | "switch_expression" => {
                walk_node(child, source, depth, s);
            }
            _ => {}
        }
    }
}

fn check_condition_complexity(node: Node, source: &str, compound_conditions: &mut u32) {
    let text = node_text(node, source);
    let cond_text = text.split('{').next().unwrap_or("");
    let ops = cond_text.matches(" and ").count() + cond_text.matches(" or ").count();
    if ops >= 2 {
        *compound_conditions += 1;
    }
}

fn count_declarations(root: Node) -> u32 {
    let mut count: u32 = 0;
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if child.kind() == "variable_declaration" && is_type_declaration(child) {
            count += 1;
        }
    }
    count
}

fn is_type_declaration(var_decl: Node) -> bool {
    find_child_by_kind(var_decl, "struct_declaration").is_some()
        || find_child_by_kind(var_decl, "enum_declaration").is_some()
        || find_child_by_kind(var_decl, "union_declaration").is_some()
}
