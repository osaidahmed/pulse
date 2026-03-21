use tree_sitter::{Node, Tree};

use super::{
    collect_field_accesses_for, compute_assert_fingerprint, compute_skeleton_hash,
    compute_structural_fingerprint, count_code_lines, count_consecutive_asserts,
    find_child_by_kind, is_catch_body_empty, node_text, FileMetrics,
    FunctionMetrics, ModuleMetrics, WalkState, track_embedded_block,
};
const COND_KINDS: &[&str] = &["parenthesized_expression", "binary_expression"];

use super::shared::{self, count_boolean_ops, count_cogc_sequences, GlobalMetricsConfig};

const COMMENT_PREFIXES: &[&str] = &["//", "/*", "*"];
const SELF_NAMES: &[&str] = &["this"];
const PRIMITIVE_TYPES: &[&str] = &[
    "string",
    "number",
    "boolean",
    "bigint",
    "symbol",
    "undefined",
    "null",
    "void",
    "any",
    "unknown",
    "never",
];
const NESTING_BRANCH_KINDS: &[&str] = &[
    "if_statement",
    "for_statement",
    "for_in_statement",
    "while_statement",
    "switch_statement",
];
const BOOL_OPS: &[&str] = &["&&", "||", "??"];
const BOOL_STOPS: &[&str] = &["statement_block", "function_declaration", "class_declaration", "arrow_function"];
const GLOBAL_CFG: GlobalMetricsConfig = GlobalMetricsConfig {
    cond: &["if_statement"],
    loops: &["for_statement", "for_in_statement", "while_statement", "do_statement"],
    branches: NESTING_BRANCH_KINDS,
    recurse: &["export_statement"],
};

/// Walk a TypeScript (or JavaScript) AST. When `has_types` is true, type annotations
/// are analyzed for primitive obsession. JavaScript callers pass false.
pub fn walk(tree: &Tree, source: &str, has_types: bool) -> FileMetrics {
    let root = tree.root_node();
    let total_loc = count_code_lines(source, COMMENT_PREFIXES);

    let mut functions = Vec::new();
    let mut global_conditional_count: u32 = 0;
    let mut global_max_nesting: u32 = 0;

    collect_functions(root, source, &mut functions, has_types);
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

fn collect_functions(
    node: Node,
    source: &str,
    functions: &mut Vec<FunctionMetrics>,
    has_types: bool,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "function_declaration" | "generator_function_declaration" => {
                if let Some(metrics) = analyze_function(child, source, has_types) {
                    functions.push(metrics);
                }
            }
            "export_statement" => {
                collect_functions(child, source, functions, has_types);
            }
            "lexical_declaration" | "variable_declaration" => {
                collect_arrow_functions(child, source, functions, has_types);
            }
            "class_declaration" => {
                collect_class_methods(child, source, functions, has_types);
            }
            _ => {}
        }
    }
}

fn collect_arrow_functions(
    decl_node: Node,
    source: &str,
    functions: &mut Vec<FunctionMetrics>,
    has_types: bool,
) {
    let mut cursor = decl_node.walk();
    for child in decl_node.children(&mut cursor) {
        if child.kind() != "variable_declarator" {
            continue;
        }
        let Some(name_node) = find_child_by_kind(child, "identifier") else {
            continue;
        };
        let Some(fn_node) = find_child_by_kind(child, "arrow_function")
            .or_else(|| find_child_by_kind(child, "function"))
        else {
            continue;
        };
        let Some(mut metrics) = analyze_function(fn_node, source, has_types) else {
            continue;
        };
        metrics.name = node_text(name_node, source).to_string();
        functions.push(metrics);
    }
}

fn collect_class_methods(
    class_node: Node,
    source: &str,
    functions: &mut Vec<FunctionMetrics>,
    has_types: bool,
) {
    let Some(body) = find_child_by_kind(class_node, "class_body") else {
        return;
    };

    let class_name = find_child_by_kind(class_node, "identifier")
        .or_else(|| find_child_by_kind(class_node, "type_identifier"))
        .map(|n| node_text(n, source).to_string())
        .unwrap_or_default();

    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        if child.kind() != "method_definition" {
            continue;
        }
        let Some(mut metrics) = analyze_function(child, source, has_types) else {
            continue;
        };
        let method_name = metrics.name.clone();
        metrics.name = format!("{class_name}.{method_name}");
        metrics.is_constructor = method_name == "constructor";
        metrics.class_name = Some(class_name.clone());
        collect_field_accesses_for(child, source, SELF_NAMES, &mut metrics.field_accesses);
        functions.push(metrics);
    }
}

fn analyze_function(node: Node, source: &str, has_types: bool) -> Option<FunctionMetrics> {
    let name = find_child_by_kind(node, "identifier")
        .or_else(|| find_child_by_kind(node, "property_identifier"))
        .map_or_else(|| "<anonymous>".into(), |n| node_text(n, source).to_string());

    let start_line = node.start_position().row as u32 + 1;
    let end_line = node.end_position().row as u32 + 1;
    let loc = end_line.saturating_sub(start_line) + 1;

    let (arg_count, primitive_type_count, typed_param_count) = if has_types {
        count_parameters(node, source)
    } else {
        (count_parameters_untyped(node), 0, 0)
    };

    let body = find_child_by_kind(node, "statement_block")?;
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
        empty_catch_count: s.empty_catch_count,
        field_accesses: Vec::new(),
        class_name: None,
    })
}

fn walk_body(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    s.reset_bump();

    for child in node.children(&mut cursor) {
        walk_node(child, source, depth, s);
    }
}

fn walk_node(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    match child.kind() {
        "if_statement" => handle_if(child, source, depth, s),
        "for_statement" | "for_in_statement" | "while_statement" | "do_statement" => {
            handle_loop(child, source, depth, s);
        }
        "switch_case" => handle_switch_case(child, source, depth, s),
        "switch_statement" => handle_switch(child, source, depth, s),
        "catch_clause" | "try_statement" => handle_exception(child, source, depth, s),
        "ternary_expression" => handle_ternary(s),
        "template_string" | "string" => track_embedded_block(&mut s.max_embedded_block_loc, child),
        _ => walk_body(child, source, depth, s),
    }
}

fn handle_exception(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    if child.kind() == "catch_clause" {
        handle_catch(child, source, depth, s);
    } else {
        walk_children(child, source, depth, s);
    }
}

fn handle_if(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.track_if(depth);
    s.track_cogc_branch();
    shared::check_condition_complexity_text(child, source, &mut s.compound_condition_count, COND_KINDS);
    count_boolean_ops(child, &mut s.cc, BOOL_OPS, BOOL_STOPS);
    count_cogc_sequences(child, &mut s.cogc, BOOL_OPS, BOOL_STOPS);
    walk_children(child, source, depth + 1, s);
}

fn handle_loop(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.track_loop(depth);
    s.track_cogc_branch();
    walk_children(child, source, depth + 1, s);
}

fn handle_switch_case(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    let is_default = find_child_by_kind(child, "default").is_some()
        || child.child_count() > 0 && child.child(0).is_some_and(|c| c.kind() == "default");
    if !is_default {
        s.cc += 1;
    }
    walk_children(child, source, depth, s);
}

fn handle_switch(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.track_nesting(depth);
    s.track_cogc_branch();
    walk_children(child, source, depth + 1, s);
}

fn handle_catch(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.cc += 1;
    s.track_cogc_branch();
    if is_catch_body_empty(child, "statement_block", None) {
        s.empty_catch_count += 1;
    }
    walk_children(child, source, depth, s);
}

fn handle_ternary(s: &mut WalkState) {
    s.cc += 1;
    s.track_cogc_branch();
}

fn walk_children(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "statement_block" | "switch_body" => {
                let saved = s.cogc_nesting;
                s.cogc_nesting += 1;
                walk_body(child, source, depth, s);
                s.cogc_nesting = saved;
            }
            "else_clause" => walk_else_clause(child, source, depth, s),
            "finally_clause" => shared::walk_block_children(child, &mut shared::BlockWalkCtx { source, depth, state: s }, "statement_block", walk_body),
            "catch_clause" => {
                s.cc += 1;
                s.track_cogc_branch();
                if is_catch_body_empty(child, "statement_block", None) {
                    s.empty_catch_count += 1;
                }
                shared::walk_block_children(child, &mut shared::BlockWalkCtx { source, depth, state: s }, "statement_block", walk_body);
            }
            _ => {}
        }
    }
}

fn walk_else_clause(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "statement_block" => {
                s.track_cogc_flat();
                let saved = s.cogc_nesting;
                s.cogc_nesting += 1;
                walk_body(child, source, depth, s);
                s.cogc_nesting = saved;
            }
            "if_statement" => {
                s.cc += 1;
                s.track_cogc_branch();
                shared::check_condition_complexity_text(child, source, &mut s.compound_condition_count, COND_KINDS);
                count_boolean_ops(child, &mut s.cc, BOOL_OPS, BOOL_STOPS);
                count_cogc_sequences(child, &mut s.cogc, BOOL_OPS, BOOL_STOPS);
                walk_children(child, source, depth, s);
            }
            _ => {}
        }
    }
}

fn count_parameters(func_node: Node, source: &str) -> (u32, u32, u32) {
    let Some(params) = find_child_by_kind(func_node, "formal_parameters") else {
        return (0, 0, 0);
    };
    let mut cursor = params.walk();
    params.children(&mut cursor).fold((0, 0, 0), |(cnt, prim, typed), child| match child.kind() {
        "identifier" | "rest_pattern" | "object_pattern" | "array_pattern"
        | "assignment_pattern" => (cnt + 1, prim, typed),
        "required_parameter" | "optional_parameter" => {
            let has_ann = find_child_by_kind(child, "type_annotation");
            match has_ann {
                Some(ann) => {
                    let p = u32::from(is_primitive_type(ann, source));
                    (cnt + 1, prim + p, typed + 1)
                }
                None => (cnt + 1, prim, typed),
            }
        }
        _ => (cnt, prim, typed),
    })
}

fn count_parameters_untyped(func_node: Node) -> u32 {
    let Some(params) = find_child_by_kind(func_node, "formal_parameters") else {
        return 0;
    };
    let mut count: u32 = 0;
    let mut cursor = params.walk();
    for child in params.children(&mut cursor) {
        match child.kind() {
            "identifier" | "rest_pattern" | "object_pattern" | "array_pattern"
            | "assignment_pattern" => {
                count += 1;
            }
            _ => {}
        }
    }
    count
}

fn is_primitive_type(type_ann: Node, source: &str) -> bool {
    let mut cursor = type_ann.walk();
    for child in type_ann.children(&mut cursor) {
        if child.kind() == "predefined_type" || child.kind() == "type_identifier" {
            let name = &source[child.byte_range()];
            return PRIMITIVE_TYPES.contains(&name);
        }
    }
    false
}

fn count_declarations(root: Node) -> u32 {
    let mut count: u32 = 0;
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        match child.kind() {
            "class_declaration" => count += 1,
            "export_statement" => count += count_exported_classes(child),
            _ => {}
        }
    }
    count
}

fn count_exported_classes(node: Node) -> u32 {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .filter(|c| c.kind() == "class_declaration")
        .count() as u32
}
