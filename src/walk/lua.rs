use tree_sitter::{Node, Tree};

use super::counters::count_short_variables;
use super::shared::{self, count_boolean_ops, count_cogc_sequences, GlobalMetricsConfig};
use super::{
    collect_field_accesses_for, compute_assert_fingerprint, compute_skeleton_hash,
    compute_structural_fingerprint, count_code_lines, count_consecutive_asserts,
    find_child_by_kind, node_text, track_embedded_block, FileMetrics, FunctionMetrics,
    ModuleMetrics, WalkState,
};

const COMMENT_PREFIXES: &[&str] = &["--"];
const NESTING_BRANCH_KINDS: &[&str] = &[
    "if_statement",
    "for_statement",
    "while_statement",
    "repeat_statement",
];
const BOOL_OPS: &[&str] = &["and", "or"];
const BOOL_STOPS: &[&str] = &["block", "function_declaration", "function_definition"];
const GLOBAL_CFG: GlobalMetricsConfig = GlobalMetricsConfig {
    cond: &["if_statement"],
    loops: &["while_statement", "for_statement", "repeat_statement"],
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
        struct_fields: Vec::new(),
    };

    FileMetrics { functions, module }
}

fn collect_functions(root: Node, source: &str, functions: &mut Vec<FunctionMetrics>) {
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        match child.kind() {
            "function_declaration" => {
                try_add_function(child, source, functions);
            }
            "variable_declaration" => {
                try_add_anon_function(child, source, functions);
            }
            _ => {}
        }
    }
}

fn try_add_function(node: Node, source: &str, functions: &mut Vec<FunctionMetrics>) {
    if let Some(m) = analyze_named_function(node, source) {
        functions.push(m);
    }
}

fn try_add_anon_function(node: Node, source: &str, functions: &mut Vec<FunctionMetrics>) {
    let Some(func_def) = find_nested_function_def(node) else { return };
    let name = extract_var_name(node, source);
    let arg_count = count_parameters(func_def);
    let info = FnInfo { name, arg_count, is_constructor: false };
    if let Some(m) = build_metrics(func_def, source, info) {
        functions.push(m);
    }
}

fn find_nested_function_def(node: Node) -> Option<Node> {
    let mut stack = vec![node];
    while let Some(current) = stack.pop() {
        let mut cursor = current.walk();
        for child in current.children(&mut cursor) {
            if child.kind() == "function_definition" {
                return Some(child);
            }
            stack.push(child);
        }
    }
    None
}

fn extract_var_name(node: Node, source: &str) -> String {
    find_child_by_kind(node, "assignment_statement")
        .and_then(|a| find_child_by_kind(a, "variable_list"))
        .and_then(|vl| find_child_by_kind(vl, "identifier"))
        .or_else(|| find_child_by_kind(node, "identifier"))
        .map_or_else(|| "<anonymous>".into(), |n| node_text(n, source).to_string())
}

fn analyze_named_function(node: Node, source: &str) -> Option<FunctionMetrics> {
    let mut cursor = node.walk();
    let name_node = node.children(&mut cursor)
        .find(|c| matches!(c.kind(), "identifier" | "method_index_expression" | "dot_index_expression"))?;
    let arg_count = count_parameters(node);
    let (info, class) = extract_fn_identity(name_node, source, arg_count);
    let mut m = build_metrics(node, source, info)?;
    if let Some(table) = class {
        m.class_name = Some(table.to_string());
        if name_node.kind() == "method_index_expression" {
            collect_field_accesses_for(node, source, &["self"], &mut m.field_accesses);
            m.field_accesses.sort();
            m.field_accesses.dedup();
        }
    }
    Some(m)
}

fn extract_fn_identity<'a>(
    name_node: Node<'a>,
    source: &'a str,
    arg_count: u32,
) -> (FnInfo, Option<&'a str>) {
    match name_node.kind() {
        "method_index_expression" => {
            let table = name_node.child(0).map_or("", |n| node_text(n, source));
            let method = name_node.child(2).map_or("unknown", |n| node_text(n, source));
            let is_ctor = method == "new" || method == "init";
            let info = FnInfo { name: format!("{table}.{method}"), arg_count, is_constructor: is_ctor };
            (info, Some(table))
        }
        "dot_index_expression" => {
            let table = name_node.child(0).map_or("", |n| node_text(n, source));
            let field = name_node.child(2).map_or("unknown", |n| node_text(n, source));
            let info = FnInfo { name: format!("{table}.{field}"), arg_count, is_constructor: false };
            (info, Some(table))
        }
        _ => {
            let name = node_text(name_node, source).to_string();
            let info = FnInfo { name, arg_count, is_constructor: false };
            (info, None)
        }
    }
}

struct FnInfo {
    name: String,
    arg_count: u32,
    is_constructor: bool,
}

#[allow(clippy::unnecessary_wraps)]
fn build_metrics(node: Node, source: &str, info: FnInfo) -> Option<FunctionMetrics> {
    let start_line = node.start_position().row as u32 + 1;
    let end_line = node.end_position().row as u32 + 1;
    let loc = end_line.saturating_sub(start_line) + 1;
    let body = find_child_by_kind(node, "block");

    let mut s = WalkState::new();
    if let Some(b) = body {
        walk_body(b, source, 0, &mut s);
    }

    Some(FunctionMetrics {
        name: info.name,
        start_line,
        end_line,
        loc,
        cc: s.cc,
        cognitive_complexity: s.cogc,
        max_nesting: s.max_nesting,
        bump_count: s.bump_count,
        arg_count: info.arg_count,
        compound_condition_count: s.compound_condition_count,
        is_constructor: info.is_constructor,
        max_embedded_block_loc: s.max_embedded_block_loc,
        structural_hash: body.map_or(0, compute_structural_fingerprint),
        skeleton_hash: body.map_or(0, compute_skeleton_hash),
        consecutive_asserts: body.map_or(0, |b| count_consecutive_asserts(b, "function_call")),
        assert_hash: body.map_or(0, |b| compute_assert_fingerprint(b, "function_call")),
        primitive_type_count: 0,
        typed_param_count: 0,
        empty_catch_count: 0,
        field_accesses: Vec::new(),
        class_name: None,
        short_var_count: body.map_or(0, |b| count_short_variables(b, source, &["assignment_statement", "variable_declaration"])),
        string_match_arms: 0,
    })
}

fn count_parameters(func: Node) -> u32 {
    let Some(params) = find_child_by_kind(func, "parameters") else {
        return 0;
    };
    let mut cursor = params.walk();
    params
        .children(&mut cursor)
        .filter(|c| c.kind() == "identifier" || c.kind() == "vararg_expression")
        .count() as u32
}

fn walk_body(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    s.reset_bump();
    for child in node.children(&mut cursor) {
        walk_node(child, source, depth, s);
    }
}

fn walk_node(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    if dispatch_control(child, source, depth, s) {
        return;
    }
    if child.kind() == "string" {
        track_embedded_block(&mut s.max_embedded_block_loc, child);
        return;
    }
    walk_body(child, source, depth, s);
}

fn dispatch_control(node: Node, source: &str, depth: u32, s: &mut WalkState) -> bool {
    match node.kind() {
        "if_statement" => handle_if(node, source, depth, s),
        "for_statement" | "while_statement" => handle_loop(node, source, depth, s),
        "repeat_statement" => handle_repeat(node, source, depth, s),
        "binary_expression" => handle_binary(node, source, depth, s),
        _ => return false,
    }
    true
}

fn handle_if(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.track_if(depth);
    s.track_cogc_branch();
    count_boolean_ops(node, &mut s.cc, BOOL_OPS, BOOL_STOPS);
    count_cogc_sequences(node, &mut s.cogc, BOOL_OPS, BOOL_STOPS);
    check_condition_complexity(node, source, &mut s.compound_condition_count);

    let saved = s.cogc_nesting;
    s.cogc_nesting += 1;

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "block" => walk_body(child, source, depth + 1, s),
            "elseif_statement" => handle_elseif(child, source, depth, s),
            "else_statement" => {
                s.track_cogc_flat();
                if let Some(blk) = find_child_by_kind(child, "block") {
                    walk_body(blk, source, depth + 1, s);
                }
            }
            _ => {}
        }
    }
    s.cogc_nesting = saved;
}

fn handle_elseif(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.cc += 1;
    s.track_cogc_branch();
    count_boolean_ops(node, &mut s.cc, BOOL_OPS, BOOL_STOPS);
    count_cogc_sequences(node, &mut s.cogc, BOOL_OPS, BOOL_STOPS);
    check_condition_complexity(node, source, &mut s.compound_condition_count);

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "block" {
            walk_body(child, source, depth + 1, s);
        }
    }
}

fn handle_loop(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.track_loop(depth);
    s.track_cogc_branch();
    let saved = s.cogc_nesting;
    s.cogc_nesting += 1;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "block" {
            walk_body(child, source, depth + 1, s);
        }
    }
    s.cogc_nesting = saved;
}

fn handle_repeat(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.track_loop(depth);
    s.track_cogc_branch();
    count_boolean_ops(node, &mut s.cc, BOOL_OPS, BOOL_STOPS);
    count_cogc_sequences(node, &mut s.cogc, BOOL_OPS, BOOL_STOPS);
    let saved = s.cogc_nesting;
    s.cogc_nesting += 1;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "block" {
            walk_body(child, source, depth + 1, s);
        }
    }
    s.cogc_nesting = saved;
}

fn handle_binary(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    walk_body(node, source, depth, s);
}

fn check_condition_complexity(node: Node, source: &str, compound_conditions: &mut u32) {
    let text = node_text(node, source);
    let cond_text = text.lines().next().unwrap_or("");
    let ops = cond_text.matches(" and ").count() + cond_text.matches(" or ").count();
    if ops >= 2 {
        *compound_conditions += 1;
    }
}

fn count_declarations(root: Node) -> u32 {
    let mut count: u32 = 0;
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if child.kind() == "variable_declaration" || child.kind() == "local_variable_declaration" {
            count += 1;
        }
    }
    count
}
