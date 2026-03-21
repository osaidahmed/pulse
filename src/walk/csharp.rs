use tree_sitter::{Node, Tree};

use super::{
    collect_field_accesses_for, compute_assert_fingerprint, compute_skeleton_hash,
    compute_structural_fingerprint, count_code_lines, count_consecutive_asserts,
    find_child_by_kind, is_catch_body_empty, node_text, track_global_nesting, FileMetrics,
    FunctionMetrics, ModuleMetrics, WalkState,
};

const COMMENT_PREFIXES: &[&str] = &["//", "/*", "*"];
const SELF_NAMES: &[&str] = &["this"];
const PRIMITIVE_TYPES: &[&str] = &[
    "int", "long", "float", "double", "bool", "string", "char",
    "byte", "sbyte", "short", "ushort", "uint", "ulong", "decimal",
    "object", "void",
];
const NESTING_BRANCH_KINDS: &[&str] = &[
    "if_statement",
    "for_statement",
    "foreach_statement",
    "while_statement",
    "switch_statement",
];

pub fn walk(tree: &Tree, source: &str) -> FileMetrics {
    let root = tree.root_node();
    let total_loc = count_code_lines(source, COMMENT_PREFIXES);

    let mut functions = Vec::new();
    let mut global_conditional_count: u32 = 0;
    let mut global_max_nesting: u32 = 0;

    collect_functions(root, source, &mut functions);
    collect_global_metrics(root, &mut global_conditional_count, &mut global_max_nesting);

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
            "class_declaration" | "struct_declaration" | "interface_declaration"
            | "record_declaration" => collect_class_methods(child, source, functions),
            "namespace_declaration" => collect_namespace_functions(child, source, functions),
            "method_declaration" | "local_function_statement" => {
                try_add_method(child, source, functions);
            }
            _ => collect_functions(child, source, functions),
        }
    }
}

fn collect_namespace_functions(node: Node, source: &str, functions: &mut Vec<FunctionMetrics>) {
    if let Some(body) = find_child_by_kind(node, "declaration_list") {
        collect_functions(body, source, functions);
    }
}

fn try_add_method(node: Node, source: &str, functions: &mut Vec<FunctionMetrics>) {
    if let Some(metrics) = analyze_method(node, source) {
        functions.push(metrics);
    }
}

fn collect_class_methods(class_node: Node, source: &str, functions: &mut Vec<FunctionMetrics>) {
    let class_name = find_child_by_kind(class_node, "identifier")
        .map(|n| node_text(n, source).to_string())
        .unwrap_or_default();

    let Some(body) = find_child_by_kind(class_node, "declaration_list") else {
        return;
    };

    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        match child.kind() {
            "method_declaration" => {
                let Some(mut metrics) = analyze_method(child, source) else {
                    continue;
                };
                let method_name = metrics.name.clone();
                metrics.name = format!("{class_name}.{method_name}");
                metrics.class_name = Some(class_name.clone());
                collect_field_accesses_for(child, source, SELF_NAMES, &mut metrics.field_accesses);
                functions.push(metrics);
            }
            "constructor_declaration" => {
                let Some(mut metrics) = analyze_constructor(child, source) else {
                    continue;
                };
                metrics.name = format!("{class_name}.{class_name}");
                metrics.is_constructor = true;
                metrics.class_name = Some(class_name.clone());
                functions.push(metrics);
            }
            "class_declaration" | "struct_declaration" | "interface_declaration" => {
                collect_class_methods(child, source, functions);
            }
            _ => {}
        }
    }
}

fn analyze_method(node: Node, source: &str) -> Option<FunctionMetrics> {
    analyze_callable(node, source, "block", "<anonymous>")
}

fn analyze_constructor(node: Node, source: &str) -> Option<FunctionMetrics> {
    let mut m = analyze_callable(node, source, "constructor_body", "<constructor>")?;
    m.is_constructor = true;
    Some(m)
}

fn analyze_callable(
    node: Node,
    source: &str,
    body_kind: &str,
    fallback_name: &str,
) -> Option<FunctionMetrics> {
    let name = find_child_by_kind(node, "identifier")
        .map_or_else(|| fallback_name.into(), |n| node_text(n, source).to_string());

    let start_line = node.start_position().row as u32 + 1;
    let end_line = node.end_position().row as u32 + 1;
    let loc = end_line.saturating_sub(start_line) + 1;

    let (arg_count, primitive_type_count, typed_param_count) = count_parameters(node, source);

    let body = find_child_by_kind(node, body_kind).or_else(|| find_child_by_kind(node, "block"))?;

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
        "for_statement" | "foreach_statement" | "while_statement" | "do_statement" => {
            handle_loop(child, source, depth, s);
        }
        "switch_statement" | "switch_section" => handle_switch_or_section(child, source, depth, s),
        "catch_clause" | "try_statement" => handle_exception(child, source, depth, s),
        "conditional_expression" => handle_ternary(s),
        "string_literal" | "raw_string_literal" | "verbatim_string_literal"
        | "interpolated_string_expression" => s.track_embedded(child),
        "lambda_expression" => {}
        _ => walk_body(child, source, depth, s),
    }
}

fn handle_switch_or_section(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    if child.kind() == "switch_statement" {
        handle_switch(child, source, depth, s);
    } else {
        handle_switch_section(child, source, depth, s);
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
    count_boolean_operators(child, &mut s.cc);
    count_cogc_boolean_sequences(child, &mut s.cogc);
    check_condition_complexity(child, source, &mut s.compound_condition_count);
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

fn handle_switch_section(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    let label_text = find_child_by_kind(child, "switch_label")
        .map(|l| node_text(l, source))
        .unwrap_or_default();
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

fn handle_ternary(s: &mut WalkState) {
    s.cc += 1;
    s.track_cogc_branch();
}

fn walk_children(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    let mut saw_else = false;
    for child in node.children(&mut cursor) {
        match child.kind() {
            "block" | "switch_body" => {
                walk_nested_block(child, source, depth, saw_else, s);
                saw_else = false;
            }
            "else_clause" => {
                saw_else = true;
                walk_else_clause(child, source, depth, s);
            }
            "if_statement" => {
                handle_elif(child, source, depth, s);
                saw_else = false;
            }
            "catch_clause" => handle_catch_in_children(child, source, depth, s),
            "finally_clause" => walk_block_children(child, source, depth, s),
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
    count_boolean_operators(child, &mut s.cc);
    count_cogc_boolean_sequences(child, &mut s.cogc);
    check_condition_complexity(child, source, &mut s.compound_condition_count);
    walk_children(child, source, depth, s);
}

fn handle_catch_in_children(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.cc += 1;
    s.track_cogc_branch();
    if is_catch_body_empty(child, "block", None) {
        s.empty_catch_count += 1;
    }
    walk_block_children(child, source, depth, s);
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
            "if_statement" => {
                s.cc += 1;
                s.track_cogc_branch();
                count_boolean_operators(child, &mut s.cc);
                count_cogc_boolean_sequences(child, &mut s.cogc);
                check_condition_complexity(child, source, &mut s.compound_condition_count);
                walk_children(child, source, depth, s);
            }
            _ => {}
        }
    }
}

fn walk_block_children(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "block" {
            walk_body(child, source, depth, s);
        }
    }
}

fn count_boolean_operators(node: Node, cc: &mut u32) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "&&" | "||" => {
                *cc += 1;
            }
            "block" | "method_declaration" | "class_declaration" | "lambda_expression" => {}
            _ => count_boolean_operators(child, cc),
        }
    }
}

fn count_cogc_boolean_sequences(node: Node, cogc: &mut u32) {
    let mut last_op: Option<&str> = None;
    collect_boolean_ops(node, cogc, &mut last_op);
}

fn collect_boolean_ops(node: Node, cogc: &mut u32, last_op: &mut Option<&str>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "&&" | "||" => {
                let op = child.kind();
                if *last_op != Some(op) {
                    *cogc += 1;
                    *last_op = Some(op);
                }
            }
            "block" | "method_declaration" | "class_declaration" | "lambda_expression" => {}
            _ => collect_boolean_ops(child, cogc, last_op),
        }
    }
}

fn check_condition_complexity(node: Node, source: &str, compound_conditions: &mut u32) {
    let Some(cond) = find_child_by_kind(node, "parenthesized_expression")
        .or_else(|| find_child_by_kind(node, "condition"))
    else {
        return;
    };
    let text = node_text(cond, source);
    let ops = text.matches("&&").count() + text.matches("||").count();
    if ops >= 2 {
        *compound_conditions += 1;
    }
}

fn count_parameters(node: Node, source: &str) -> (u32, u32, u32) {
    let Some(params) = find_child_by_kind(node, "parameter_list") else {
        return (0, 0, 0);
    };
    let mut count: u32 = 0;
    let mut primitive_count: u32 = 0;
    let mut typed_count: u32 = 0;
    let mut cursor = params.walk();
    for child in params.children(&mut cursor) {
        if child.kind() != "parameter" {
            continue;
        }
        count += 1;
        typed_count += 1;
        if has_primitive_type(child, source) {
            primitive_count += 1;
        }
    }
    (count, primitive_count, typed_count)
}

fn has_primitive_type(param: Node, source: &str) -> bool {
    let mut cursor = param.walk();
    for child in param.children(&mut cursor) {
        match child.kind() {
            "predefined_type" => {
                return true;
            }
            "type_identifier" => {
                let name = &source[child.byte_range()];
                return PRIMITIVE_TYPES.contains(&name);
            }
            _ => {}
        }
    }
    false
}

fn collect_global_metrics(root: Node, conditional_count: &mut u32, max_nesting: &mut u32) {
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if child.kind() == "if_statement" {
            *conditional_count += 1;
            track_global_nesting(child, max_nesting, NESTING_BRANCH_KINDS);
        }
    }
}

fn count_declarations(root: Node) -> u32 {
    let mut count: u32 = 0;
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        match child.kind() {
            "class_declaration" | "struct_declaration" | "interface_declaration"
            | "enum_declaration" | "record_declaration" => {
                count += 1;
            }
            _ => {}
        }
    }
    count
}
