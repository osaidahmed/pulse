use tree_sitter::{Node, Tree};

use super::counters::{count_short_variables, count_string_match_arms, max_same_primitive};
use super::shared::{self, count_boolean_ops, count_cogc_sequences, GlobalMetricsConfig};
use super::{
    collect_field_accesses_for, collect_foreign_field_accesses_for, compute_assert_fingerprint, compute_skeleton_hash,
    compute_structural_fingerprint, count_code_lines, count_consecutive_asserts, count_distinct_node_kinds,
    find_child_by_kind, is_catch_body_empty, node_text, track_embedded_block, FileMetrics, FunctionMetrics,
    ModuleMetrics, WalkState,
};

const COMMENT_PREFIXES: &[&str] = &["//", "/*", "*"];
const SELF_NAMES: &[&str] = &["this"];
const PRIMITIVE_TYPES: &[&str] =
    &["int", "long", "short", "byte", "float", "double", "boolean", "char", "void", "String"];
const NESTING_BRANCH_KINDS: &[&str] = &[
    "if_statement",
    "for_statement",
    "enhanced_for_statement",
    "while_statement",
    "do_statement",
    "switch_expression",
];
const BOOL_OPS: &[&str] = &["&&", "||"];
const BOOL_STOPS: &[&str] = &["block", "method_declaration", "class_declaration", "lambda_expression"];
const GLOBAL_CFG: GlobalMetricsConfig = GlobalMetricsConfig {
    cond: &["if_statement"],
    loops: &["for_statement", "enhanced_for_statement", "while_statement", "do_statement"],
    branches: NESTING_BRANCH_KINDS,
    recurse: &[],
};
const COND_KINDS: &[&str] = &["parenthesized_expression", "condition"];

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
        struct_fields: Vec::new(),
    };

    FileMetrics { functions, module }
}

fn collect_functions(node: Node, source: &str, functions: &mut Vec<FunctionMetrics>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "class_declaration" => collect_class_methods(child, source, functions),
            "method_declaration" => {
                if let Some(metrics) = analyze_callable(child, source, &METHOD_CONFIG) {
                    functions.push(metrics);
                }
            }
            _ => collect_functions(child, source, functions),
        }
    }
}

fn collect_class_methods(class_node: Node, source: &str, functions: &mut Vec<FunctionMetrics>) {
    let class_name =
        find_child_by_kind(class_node, "identifier").map(|n| node_text(n, source).to_string()).unwrap_or_default();
    let parent_class = extract_parent_class_java(class_node, source);

    let Some(body) = find_child_by_kind(class_node, "class_body") else {
        return;
    };

    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        match child.kind() {
            "method_declaration" => {
                let Some(mut metrics) = analyze_callable(child, source, &METHOD_CONFIG) else {
                    continue;
                };
                let method_name = metrics.name.clone();
                metrics.name = format!("{class_name}.{method_name}");
                metrics.class_name = Some(class_name.clone());
                metrics.parent_class = parent_class.clone();
                collect_field_accesses_for(child, source, SELF_NAMES, &mut metrics.field_accesses);

                collect_foreign_field_accesses_for(child, source, SELF_NAMES, &mut metrics.foreign_field_accesses);

                functions.push(metrics);
            }
            "constructor_declaration" => {
                let Some(mut metrics) = analyze_callable(child, source, &CTOR_CONFIG) else {
                    continue;
                };
                metrics.name = format!("{class_name}.{class_name}");
                metrics.is_constructor = true;
                metrics.class_name = Some(class_name.clone());
                metrics.parent_class = parent_class.clone();
                functions.push(metrics);
            }
            "class_declaration" => collect_class_methods(child, source, functions),
            _ => {}
        }
    }
}

fn extract_parent_class_java(class_node: Node, source: &str) -> Option<String> {
    let superclass = find_child_by_kind(class_node, "superclass")?;
    let id = find_child_by_kind(superclass, "type_identifier")?;
    Some(node_text(id, source).to_string())
}

struct CallableConfig {
    body_kind: &'static str,
    fallback_name: &'static str,
}

const METHOD_CONFIG: CallableConfig = CallableConfig { body_kind: "block", fallback_name: "<anonymous>" };
const CTOR_CONFIG: CallableConfig = CallableConfig { body_kind: "constructor_body", fallback_name: "<constructor>" };

fn analyze_callable(node: Node, source: &str, cfg: &CallableConfig) -> Option<FunctionMetrics> {
    let name = find_child_by_kind(node, "identifier")
        .map_or_else(|| cfg.fallback_name.into(), |n| node_text(n, source).to_string());

    let start_line = node.start_position().row as u32 + 1;
    let end_line = node.end_position().row as u32 + 1;
    let loc = end_line.saturating_sub(start_line) + 1;

    let (arg_count, primitive_type_count, typed_param_count, max_same_primitive_count) = count_parameters(node, source);

    let body = find_child_by_kind(node, cfg.body_kind).or_else(|| find_child_by_kind(node, "block"))?;
    let cpg = super::cpg_for(body, node, source, &crate::cpg::JAVA);

    let mut s = WalkState::new();
    walk_body(body, source, 0, &mut s);

    let structural_hash = compute_structural_fingerprint(body);
    let distinct_node_kinds = count_distinct_node_kinds(body);
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
        distinct_node_kinds,
        skeleton_hash,
        consecutive_asserts,
        assert_hash,
        primitive_type_count,
        typed_param_count,
        max_same_primitive_count,
        empty_catch_count: s.empty_catch_count,
        field_accesses: Vec::new(),
        foreign_field_accesses: Vec::new(),
        class_name: None,
        parent_class: None,
        short_var_count: count_short_variables(body, source, &["local_variable_declaration"]),
        string_match_arms: count_string_match_arms(
            body,
            "switch_expression",
            "switch_block_statement_group",
            &["string_literal"],
            &[],
        ),
        cpg,
    })
}

fn walk_body(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        walk_node(child, source, depth, s);
    }
}

type NodeHandler = fn(Node, &str, u32, &mut WalkState);

const NODE_HANDLERS: &[(&[&str], NodeHandler)] = &[
    (&["if_statement"], handle_if),
    (&["for_statement", "enhanced_for_statement", "while_statement", "do_statement"], handle_loop),
    (&["switch_expression"], handle_switch),
    (&["switch_block_statement_group"], handle_switch_case),
    (&["catch_clause"], handle_catch),
    (&["try_statement", "try_with_resources_statement"], walk_children),
    (&["ternary_expression", "conditional_expression"], handle_ternary),
];

const STRING_KINDS: &[&str] = &["string_literal", "text_block"];

fn walk_node(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    let kind = child.kind();
    if STRING_KINDS.contains(&kind) {
        track_embedded_block(&mut s.max_embedded_block_loc, child);
        return;
    }
    if kind == "lambda_expression" {
        return;
    }
    for (kinds, handler) in NODE_HANDLERS {
        if kinds.contains(&kind) {
            handler(child, source, depth, s);
            return;
        }
    }
    walk_body(child, source, depth, s);
}

fn handle_ternary(_child: Node, _source: &str, _depth: u32, s: &mut WalkState) {
    s.cc += 1;
    s.track_cogc_branch();
}

fn handle_if(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.track_if(depth);
    s.track_cogc_branch();
    count_boolean_ops(child, &mut s.cc, BOOL_OPS, BOOL_STOPS);
    count_cogc_sequences(child, &mut s.cogc, BOOL_OPS, BOOL_STOPS);
    shared::check_condition_complexity(child, &mut s.compound_condition_count, COND_KINDS, BOOL_OPS, BOOL_STOPS);
    walk_children(child, source, depth + 1, s);
}

fn handle_loop(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.track_loop(depth);
    s.track_cogc_branch();
    walk_children(child, source, depth + 1, s);
}

fn handle_switch(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.track_nesting(depth);
    s.track_cogc_branch();
    walk_children(child, source, depth + 1, s);
}

fn handle_switch_case(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    let label_text = find_child_by_kind(child, "switch_label").map(|l| node_text(l, source)).unwrap_or_default();
    if !label_text.contains("default") {
        s.cc += 1;
    }
    walk_body(child, source, depth, s);
}

fn handle_catch(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.cc += 1;
    s.track_cogc_branch();
    if is_catch_body_empty(child, "block", None) {
        s.empty_catch_count += 1;
    }
    walk_children(child, source, depth, s);
}

fn walk_children(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    let mut saw_else = false;
    for child in node.children(&mut cursor) {
        match child.kind() {
            "block" | "switch_block" => {
                walk_nested_block(child, source, depth, saw_else, s);
                saw_else = false;
            }
            "else" => {
                saw_else = true;
            }
            "if_statement" => {
                handle_elif(child, source, depth, s);
                saw_else = false;
            }
            "catch_clause" => {
                s.cc += 1;
                s.track_cogc_branch();
                if is_catch_body_empty(child, "block", None) {
                    s.empty_catch_count += 1;
                }
                shared::walk_block_children(
                    child,
                    &mut shared::BlockWalkCtx { source, depth, state: s },
                    "block",
                    walk_body,
                );
            }
            "finally_clause" => shared::walk_block_children(
                child,
                &mut shared::BlockWalkCtx { source, depth, state: s },
                "block",
                walk_body,
            ),
            _ => {}
        }
    }
}

fn walk_nested_block(child: Node, source: &str, depth: u32, saw_else: bool, s: &mut WalkState) {
    if saw_else {
        s.track_cogc_flat();
    }
    let saved = s.cogc_nesting;
    s.cogc_nesting += 1;
    walk_body(child, source, depth, s);
    s.cogc_nesting = saved;
}

fn handle_elif(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.cc += 1;
    s.track_cogc_branch();
    count_boolean_ops(child, &mut s.cc, BOOL_OPS, BOOL_STOPS);
    count_cogc_sequences(child, &mut s.cogc, BOOL_OPS, BOOL_STOPS);
    shared::check_condition_complexity(child, &mut s.compound_condition_count, COND_KINDS, BOOL_OPS, BOOL_STOPS);
    walk_children(child, source, depth, s);
}

fn count_parameters(node: Node, source: &str) -> (u32, u32, u32, u32) {
    let Some(params) = find_child_by_kind(node, "formal_parameters") else {
        return (0, 0, 0, 0);
    };
    let mut cursor = params.walk();
    let mut count = 0;
    let mut prims: Vec<&str> = Vec::new();
    for child in
        params.children(&mut cursor).filter(|c| c.kind() == "formal_parameter" || c.kind() == "spread_parameter")
    {
        count += 1;
        if let Some(ty) = primitive_type_of(child, source) {
            prims.push(ty);
        }
    }
    (count, prims.len() as u32, count, max_same_primitive(&prims))
}

fn primitive_type_of<'a>(param: Node, source: &'a str) -> Option<&'a str> {
    let mut cursor = param.walk();
    for child in param.children(&mut cursor) {
        match child.kind() {
            "integral_type" | "floating_point_type" | "boolean_type" | "void_type" => {
                return Some(&source[child.byte_range()]);
            }
            "type_identifier" => {
                let name = &source[child.byte_range()];
                return PRIMITIVE_TYPES.contains(&name).then_some(name);
            }
            _ => {}
        }
    }
    None
}

fn count_declarations(root: Node) -> u32 {
    let mut count: u32 = 0;
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        match child.kind() {
            "class_declaration" | "interface_declaration" | "enum_declaration" => {
                count += 1;
            }
            _ => {}
        }
    }
    count
}
