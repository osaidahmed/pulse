use tree_sitter::{Node, Tree};

use super::shared::{self, count_boolean_ops, count_cogc_sequences, GlobalMetricsConfig};
use super::{
    collect_field_accesses_for, compute_assert_fingerprint, compute_skeleton_hash,
    compute_structural_fingerprint, count_code_lines, count_consecutive_asserts,
    find_child_by_kind, node_text, FileMetrics, FunctionMetrics,
    ModuleMetrics, WalkState, track_embedded_block,
};

const COMMENT_PREFIXES: &[&str] = &["//", "/*", "*"];
const SELF_NAMES: &[&str] = &["self"];
const PRIMITIVE_TYPES: &[&str] = &[
    "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize", "f32",
    "f64", "bool", "char", "str", "String",
];
const NESTING_BRANCH_KINDS: &[&str] = &[
    "if_expression",
    "for_expression",
    "while_expression",
    "loop_expression",
    "match_expression",
];
const BOOL_OPS: &[&str] = &["&&", "||"];
const BOOL_STOPS: &[&str] = &["block", "function_item", "closure_expression"];
const GLOBAL_CFG: GlobalMetricsConfig = GlobalMetricsConfig {
    cond: &["if_expression"],
    loops: &["for_expression", "while_expression", "loop_expression"],
    branches: NESTING_BRANCH_KINDS,
    recurse: &[],
};
const COND_KINDS: &[&str] = &["binary_expression", "parenthesized_expression"];

pub fn walk(tree: &Tree, source: &str) -> FileMetrics {
    let root = tree.root_node();
    let total_loc = count_code_lines(source, COMMENT_PREFIXES);

    let mut functions = Vec::new();
    let mut global_conditional_count: u32 = 0;
    let mut global_max_nesting: u32 = 0;

    collect_functions(root, source, &mut functions);
    shared::collect_global_metrics(root, &mut global_conditional_count, &mut global_max_nesting, &GLOBAL_CFG);

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
        match child.kind() {
            "function_item" => {
                if let Some(metrics) = analyze_function(child, source) {
                    functions.push(metrics);
                }
            }
            "impl_item" => {
                collect_impl_methods(child, source, functions);
            }
            _ => {}
        }
    }
}

fn collect_impl_methods(impl_node: Node, source: &str, functions: &mut Vec<FunctionMetrics>) {
    let type_name = find_child_by_kind(impl_node, "type_identifier")
        .map(|n| node_text(n, source).to_string())
        .unwrap_or_default();

    let Some(body) = find_child_by_kind(impl_node, "declaration_list") else {
        return;
    };

    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        if child.kind() != "function_item" {
            continue;
        }
        let Some(mut metrics) = analyze_function(child, source) else {
            continue;
        };
        let method_name = metrics.name.clone();
        metrics.name = format!("{type_name}.{method_name}");
        metrics.is_constructor = method_name == "new";
        metrics.class_name = Some(type_name.clone());
        collect_field_accesses_for(child, source, SELF_NAMES, &mut metrics.field_accesses);
        if has_self_param(child) && metrics.arg_count > 0 {
            metrics.arg_count -= 1;
        }
        functions.push(metrics);
    }
}

fn analyze_function(node: Node, source: &str) -> Option<FunctionMetrics> {
    let name = find_child_by_kind(node, "identifier")
        .map_or_else(|| "<anonymous>".into(), |n| node_text(n, source).to_string());

    let start_line = node.start_position().row as u32 + 1;
    let end_line = node.end_position().row as u32 + 1;
    let loc = end_line.saturating_sub(start_line) + 1;

    let (arg_count, primitive_type_count, typed_param_count) = count_parameters(node, source);

    let body = find_child_by_kind(node, "block")?;
    let mut s = WalkState::new();
    walk_body(body, source, 0, &mut s);

    let structural_hash = compute_structural_fingerprint(body);
    let skeleton_hash = compute_skeleton_hash(body);
    let consecutive_asserts = count_consecutive_asserts(body, "expression_statement");
    let assert_hash = compute_assert_fingerprint(body, "expression_statement");

    Some(FunctionMetrics {
        name,
        start_line,
        end_line,
        loc,
        cc: s.cc,
        cognitive_complexity: s.cogc,
        max_nesting: s.max_nesting,
        bump_count: s.bump_count,
        arg_count,
        compound_condition_count: s.compound_condition_count,
        is_constructor: false,
        max_embedded_block_loc: s.max_embedded_block_loc,
        structural_hash,
        skeleton_hash,
        consecutive_asserts,
        assert_hash,
        primitive_type_count,
        typed_param_count,
        empty_catch_count: 0,
        field_accesses: Vec::new(),
        class_name: None,
    })
}

fn walk_body(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    s.reset_bump();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "if_expression" => {
                s.track_if(depth);
                s.track_cogc_branch();
                count_boolean_ops(child, &mut s.cc, BOOL_OPS, BOOL_STOPS);
                count_cogc_sequences(child, &mut s.cogc, BOOL_OPS, BOOL_STOPS);
                shared::check_condition_complexity_text(child, source, &mut s.compound_condition_count, COND_KINDS);
                walk_children(child, source, depth + 1, s);
            }
            "for_expression" | "while_expression" | "loop_expression" => {
                s.track_loop(depth);
                s.track_cogc_branch();
                walk_children(child, source, depth + 1, s);
            }
            "match_expression" => {
                s.track_nesting(depth);
                s.track_cogc_branch();
                let saved = s.cogc_nesting;
                s.cogc_nesting += 1;
                walk_match_arms(child, source, depth + 1, s);
                s.cogc_nesting = saved;
            }
            "closure_expression" => {}
            "string_literal" | "raw_string_literal" => track_embedded_block(&mut s.max_embedded_block_loc, child),
            _ => walk_body(child, source, depth, s),
        }
    }
}

fn walk_match_arms(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let Some(body) = find_child_by_kind(node, "match_block") else {
        return;
    };
    let mut arm_cursor = body.walk();
    for arm in body.children(&mut arm_cursor) {
        if arm.kind() != "match_arm" {
            continue;
        }
        let is_wildcard = find_child_by_kind(arm, "match_pattern").is_some_and(|p| {
            let mut pc = p.walk();
            let result = p.children(&mut pc).any(|c| c.kind() == "_");
            result
        });
        if !is_wildcard {
            s.cc += 1;
        }
        walk_children(arm, source, depth, s);
    }
}

fn walk_children(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "block" => {
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
            "block" => {
                s.track_cogc_flat();
                let saved = s.cogc_nesting;
                s.cogc_nesting += 1;
                walk_body(child, source, depth, s);
                s.cogc_nesting = saved;
            }
            "if_expression" => {
                s.cc += 1;
                s.track_cogc_branch();
                count_boolean_ops(child, &mut s.cc, BOOL_OPS, BOOL_STOPS);
                count_cogc_sequences(child, &mut s.cogc, BOOL_OPS, BOOL_STOPS);
                shared::check_condition_complexity_text(child, source, &mut s.compound_condition_count, COND_KINDS);
                walk_children(child, source, depth, s);
            }
            _ => {}
        }
    }
}

fn count_parameters(func_node: Node, source: &str) -> (u32, u32, u32) {
    let Some(params) = find_child_by_kind(func_node, "parameters") else {
        return (0, 0, 0);
    };
    let mut count: u32 = 0;
    let mut primitive_count: u32 = 0;
    let mut typed_count: u32 = 0;
    let mut cursor = params.walk();
    for child in params.children(&mut cursor) {
        match child.kind() {
            "parameter" => {
                count += 1;
                typed_count += 1;
                if has_primitive_type(child, source) {
                    primitive_count += 1;
                }
            }
            "self_parameter" => {
                count += 1;
            }
            _ => {}
        }
    }
    (count, primitive_count, typed_count)
}

fn has_primitive_type(param_node: Node, source: &str) -> bool {
    let type_node = find_type_leaf(param_node)
        .or_else(|| find_child_by_kind(param_node, "reference_type").and_then(find_type_leaf));
    type_node.is_some_and(|n| PRIMITIVE_TYPES.contains(&&source[n.byte_range()]))
}

fn find_type_leaf(node: Node) -> Option<Node> {
    let mut cursor = node.walk();
    let result = node
        .children(&mut cursor)
        .find(|c| c.kind() == "type_identifier" || c.kind() == "primitive_type");
    result
}

fn has_self_param(func_node: Node) -> bool {
    let Some(params) = find_child_by_kind(func_node, "parameters") else {
        return false;
    };
    let mut cursor = params.walk();
    let result = params
        .children(&mut cursor)
        .any(|c| c.kind() == "self_parameter");
    result
}

fn count_declarations(root: Node) -> u32 {
    let mut count: u32 = 0;
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        match child.kind() {
            "struct_item" | "enum_item" | "trait_item" | "type_item" => {
                count += 1;
            }
            _ => {}
        }
    }
    count
}
