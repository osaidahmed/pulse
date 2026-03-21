use tree_sitter::{Node, Tree};

use super::{
    compute_assert_fingerprint, compute_skeleton_hash, compute_structural_fingerprint,
    count_code_lines, count_consecutive_asserts, find_child_by_kind, measure_nesting_depth,
    node_text, FileMetrics, FunctionMetrics, ModuleMetrics, WalkState,
};

const COMMENT_PREFIXES: &[&str] = &["//", "/*", "*"];
const PRIMITIVE_TYPES: &[&str] = &[
    "int", "char", "float", "double", "void", "long", "short", "unsigned", "signed", "size_t",
    "ssize_t", "int8_t", "int16_t", "int32_t", "int64_t", "uint8_t", "uint16_t", "uint32_t",
    "uint64_t", "bool", "_Bool",
];
const NESTING_BRANCH_KINDS: &[&str] = &[
    "if_statement",
    "for_statement",
    "while_statement",
    "do_statement",
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
        if child.kind() != "function_definition" {
            continue;
        }
        if let Some(metrics) = analyze_function(child, source) {
            functions.push(metrics);
        }
    }
}

fn analyze_function(node: Node, source: &str) -> Option<FunctionMetrics> {
    let declarator = find_child_by_kind(node, "function_declarator").or_else(|| {
        find_child_by_kind(node, "pointer_declarator")
            .and_then(|p| find_child_by_kind(p, "function_declarator"))
    })?;

    let name = find_child_by_kind(declarator, "identifier")
        .map_or_else(|| "<anonymous>".into(), |n| node_text(n, source).to_string());

    let start_line = node.start_position().row as u32 + 1;
    let end_line = node.end_position().row as u32 + 1;
    let loc = end_line.saturating_sub(start_line) + 1;

    let (arg_count, primitive_type_count, typed_param_count) = count_parameters(declarator, source);

    let body = find_child_by_kind(node, "compound_statement")?;
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
            "if_statement" => {
                s.track_if(depth);
                s.track_cogc_branch();
                count_boolean_operators(child, &mut s.cc);
                count_cogc_boolean_sequences(child, &mut s.cogc);
                check_condition_complexity(child, source, &mut s.compound_condition_count);
                walk_children(child, source, depth + 1, s);
            }
            "for_statement" | "while_statement" | "do_statement" => {
                s.track_loop(depth);
                s.track_cogc_branch();
                walk_children(child, source, depth + 1, s);
            }
            "switch_statement" => {
                s.track_nesting(depth);
                s.track_cogc_branch();
                walk_children(child, source, depth + 1, s);
            }
            "case_statement" => {
                let is_default = find_child_by_kind(child, "default").is_some()
                    || node_text(child, source).trim_start().starts_with("default");
                if !is_default {
                    s.cc += 1;
                }
                walk_body(child, source, depth, s);
            }
            "conditional_expression" => {
                s.cc += 1;
                s.track_cogc_branch();
            }
            "string_literal" | "concatenated_string" => s.track_embedded(child),
            _ => walk_body(child, source, depth, s),
        }
    }
}

fn walk_children(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "compound_statement" => {
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
            "compound_statement" => {
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

fn count_boolean_operators(node: Node, cc: &mut u32) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "&&" | "||" => {
                *cc += 1;
            }
            "compound_statement" | "function_definition" => {}
            _ => count_boolean_operators(child, cc),
        }
    }
}

fn count_cogc_boolean_sequences(node: Node, cogc: &mut u32) {
    let mut last_op: Option<&str> = None;
    collect_boolean_ops_c(node, cogc, &mut last_op);
}

fn collect_boolean_ops_c(node: Node, cogc: &mut u32, last_op: &mut Option<&str>) {
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
            "compound_statement" | "function_definition" => {}
            _ => collect_boolean_ops_c(child, cogc, last_op),
        }
    }
}

fn check_condition_complexity(node: Node, source: &str, compound_conditions: &mut u32) {
    let Some(cond) = find_child_by_kind(node, "parenthesized_expression")
        .or_else(|| find_child_by_kind(node, "binary_expression"))
    else {
        return;
    };
    let text = node_text(cond, source);
    let ops = text.matches("&&").count() + text.matches("||").count();
    if ops >= 2 {
        *compound_conditions += 1;
    }
}

fn count_parameters(declarator: Node, source: &str) -> (u32, u32, u32) {
    let Some(params) = find_child_by_kind(declarator, "parameter_list") else {
        return (0, 0, 0);
    };
    let (count, primitive_count, typed_count) = count_param_children(params, source);
    if is_void_param_list(params, count, source) {
        return (0, 0, 0);
    }
    (count, primitive_count, typed_count)
}

fn count_param_children(params: Node, source: &str) -> (u32, u32, u32) {
    let mut count: u32 = 0;
    let mut primitive_count: u32 = 0;
    let mut typed_count: u32 = 0;
    let mut cursor = params.walk();
    for child in params.children(&mut cursor) {
        match child.kind() {
            "parameter_declaration" => {
                count += 1;
                typed_count += 1;
                if has_primitive_type(child, source) {
                    primitive_count += 1;
                }
            }
            "variadic_parameter" => count += 1,
            _ => {}
        }
    }
    (count, primitive_count, typed_count)
}

fn is_void_param_list(params: Node, count: u32, source: &str) -> bool {
    if count != 1 {
        return false;
    }
    let text = node_text(params, source);
    text.contains("void") && !text.contains("void *") && !text.contains("void*")
}

fn has_primitive_type(param: Node, source: &str) -> bool {
    let mut cursor = param.walk();
    for child in param.children(&mut cursor) {
        if child.kind() == "primitive_type" || child.kind() == "sized_type_specifier" {
            return true;
        }
        if child.kind() == "type_identifier" {
            let name = &source[child.byte_range()];
            return PRIMITIVE_TYPES.contains(&name);
        }
    }
    false
}

fn collect_global_metrics(root: Node, conditional_count: &mut u32, max_nesting: &mut u32) {
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        match child.kind() {
            "if_statement" => {
                *conditional_count += 1;
                let depth = measure_nesting_depth(child, 1, NESTING_BRANCH_KINDS);
                if depth > *max_nesting {
                    *max_nesting = depth;
                }
            }
            "for_statement" | "while_statement" | "do_statement" => {
                let depth = measure_nesting_depth(child, 1, NESTING_BRANCH_KINDS);
                if depth > *max_nesting {
                    *max_nesting = depth;
                }
            }
            _ => {}
        }
    }
}

fn count_declarations(root: Node) -> u32 {
    let mut cursor = root.walk();
    root.children(&mut cursor)
        .filter(|c| {
            matches!(
                c.kind(),
                "type_definition" | "struct_specifier" | "enum_specifier"
            )
        })
        .count() as u32
}
