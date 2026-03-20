use tree_sitter::{Node, Tree};

use super::{
    collect_field_accesses_for, compute_assert_fingerprint, compute_structural_fingerprint,
    count_code_lines, count_consecutive_asserts, find_child_by_kind, measure_nesting_depth,
    node_text, FileMetrics, FunctionMetrics, ModuleMetrics, WalkState,
};

const COMMENT_PREFIXES: &[&str] = &["//", "/*", "*"];
const SELF_NAMES: &[&str] = &["self"];
const PRIMITIVE_TYPES: &[&str] = &[
    "i8", "i16", "i32", "i64", "i128", "isize",
    "u8", "u16", "u32", "u64", "u128", "usize",
    "f32", "f64", "bool", "char", "str", "String",
];
const NESTING_BRANCH_KINDS: &[&str] = &[
    "if_expression",
    "for_expression",
    "while_expression",
    "loop_expression",
    "match_expression",
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
        .map(|n| node_text(n, source))
        .unwrap_or_default();

    let body = match find_child_by_kind(impl_node, "declaration_list") {
        Some(b) => b,
        None => return,
    };

    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        if child.kind() != "function_item" {
            continue;
        }
        let mut metrics = match analyze_function(child, source) {
            Some(m) => m,
            None => continue,
        };
        let method_name = metrics.name.clone();
        metrics.name = format!("{}.{}", type_name, method_name);
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
        .map(|n| node_text(n, source))
        .unwrap_or_else(|| "<anonymous>".into());

    let start_line = node.start_position().row as u32 + 1;
    let end_line = node.end_position().row as u32 + 1;
    let loc = end_line.saturating_sub(start_line) + 1;

    let (arg_count, primitive_type_count, typed_param_count) = count_parameters(node, source);

    let body = find_child_by_kind(node, "block")?;
    let mut s = WalkState::new();
    walk_body(body, source, 0, &mut s);

    let structural_hash = compute_structural_fingerprint(body);
    let consecutive_asserts = count_consecutive_asserts(body, "expression_statement");
    let assert_hash = compute_assert_fingerprint(body, "expression_statement");

    Some(FunctionMetrics {
        name,
        start_line,
        end_line,
        loc,
        cc: s.cc,
        max_nesting: s.max_nesting,
        bump_count: s.bump_count,
        arg_count,
        compound_condition_count: s.compound_condition_count,
        is_constructor: false,
        max_embedded_block_loc: s.max_embedded_block_loc,
        structural_hash,
        consecutive_asserts,
        assert_hash,
        primitive_type_count,
        typed_param_count,
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
                count_boolean_operators(child, &mut s.cc);
                check_condition_complexity(child, source, &mut s.compound_condition_count);
                walk_children(child, source, depth + 1, s);
            }
            "for_expression" | "while_expression" | "loop_expression" => {
                s.track_loop(depth);
                walk_children(child, source, depth + 1, s);
            }
            "match_expression" => {
                s.track_nesting(depth);
                walk_match_arms(child, source, depth + 1, s);
            }
            "closure_expression" => {}
            "string_literal" | "raw_string_literal" => s.track_embedded(child),
            _ => walk_body(child, source, depth, s),
        }
    }
}

fn walk_match_arms(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let body = match find_child_by_kind(node, "match_block") {
        Some(b) => b,
        None => return,
    };
    let mut arm_cursor = body.walk();
    for arm in body.children(&mut arm_cursor) {
        if arm.kind() != "match_arm" {
            continue;
        }
        let is_wildcard = find_child_by_kind(arm, "match_pattern")
            .map(|p| {
                let mut pc = p.walk();
                let result = p.children(&mut pc).any(|c| c.kind() == "_");
                result
            })
            .unwrap_or(false);
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
            "block" => walk_body(child, source, depth, s),
            "else_clause" => walk_else_clause(child, source, depth, s),
            _ => {}
        }
    }
}

fn walk_else_clause(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "block" => walk_body(child, source, depth, s),
            "if_expression" => {
                s.cc += 1;
                count_boolean_operators(child, &mut s.cc);
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
            "&&" | "||" => { *cc += 1; }
            "block" | "function_item" | "closure_expression" => {}
            _ => count_boolean_operators(child, cc),
        }
    }
}

fn check_condition_complexity(node: Node, source: &str, compound_conditions: &mut u32) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "binary_expression" || child.kind() == "parenthesized_expression" {
            let text = node_text(child, source);
            let logical_ops = text.matches("&&").count() + text.matches("||").count();
            if logical_ops >= 2 {
                *compound_conditions += 1;
                return;
            }
        }
    }
}

fn count_parameters(func_node: Node, source: &str) -> (u32, u32, u32) {
    let params = match find_child_by_kind(func_node, "parameters") {
        Some(p) => p,
        None => return (0, 0, 0),
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
        .or_else(|| {
            find_child_by_kind(param_node, "reference_type")
                .and_then(find_type_leaf)
        });
    type_node.map_or(false, |n| PRIMITIVE_TYPES.contains(&&source[n.byte_range()]))
}

fn find_type_leaf(node: Node) -> Option<Node> {
    let mut cursor = node.walk();
    let result = node.children(&mut cursor)
        .find(|c| c.kind() == "type_identifier" || c.kind() == "primitive_type");
    result
}

fn has_self_param(func_node: Node) -> bool {
    let params = match find_child_by_kind(func_node, "parameters") {
        Some(p) => p,
        None => return false,
    };
    let mut cursor = params.walk();
    let result = params.children(&mut cursor).any(|c| c.kind() == "self_parameter");
    result
}

fn collect_global_metrics(root: Node, conditional_count: &mut u32, max_nesting: &mut u32) {
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        match child.kind() {
            "if_expression" => {
                *conditional_count += 1;
                let depth = measure_nesting_depth(child, 1, NESTING_BRANCH_KINDS);
                if depth > *max_nesting {
                    *max_nesting = depth;
                }
            }
            "for_expression" | "while_expression" | "loop_expression" => {
                let depth = measure_nesting_depth(child, 1, NESTING_BRANCH_KINDS);
                if depth > *max_nesting {
                    *max_nesting = depth;
                }
            }
            "function_item" | "impl_item" | "struct_item" | "enum_item" | "mod_item" => {}
            _ => {}
        }
    }
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
