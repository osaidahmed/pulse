use tree_sitter::{Node, Tree};

use super::{
    collect_field_accesses_for, compute_assert_fingerprint, compute_skeleton_hash, compute_structural_fingerprint,
    count_code_lines, count_consecutive_asserts, find_child_by_kind, measure_nesting_depth,
    node_text, FileMetrics, FunctionMetrics, ModuleMetrics, WalkState,
};

const COMMENT_PREFIXES: &[&str] = &["//", "/*", "*"];
const SELF_NAMES: &[&str] = &["this"];
const PRIMITIVE_TYPES: &[&str] = &[
    "string", "number", "boolean", "bigint", "symbol", "undefined", "null", "void", "any",
    "unknown", "never",
];
const NESTING_BRANCH_KINDS: &[&str] = &[
    "if_statement",
    "for_statement",
    "for_in_statement",
    "while_statement",
    "switch_statement",
];

/// Walk a TypeScript (or JavaScript) AST. When `has_types` is true, type annotations
/// are analyzed for primitive obsession. JavaScript callers pass false.
pub fn walk(tree: &Tree, source: &str, has_types: bool) -> FileMetrics {
    let root = tree.root_node();
    let total_loc = count_code_lines(source, COMMENT_PREFIXES);

    let mut functions = Vec::new();
    let mut global_conditional_count: u32 = 0;
    let mut global_max_nesting: u32 = 0;

    collect_functions(root, source, &mut functions, has_types);
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
        let name_node = match find_child_by_kind(child, "identifier") {
            Some(n) => n,
            None => continue,
        };
        let fn_node = match find_child_by_kind(child, "arrow_function")
            .or_else(|| find_child_by_kind(child, "function"))
        {
            Some(n) => n,
            None => continue,
        };
        let mut metrics = match analyze_function(fn_node, source, has_types) {
            Some(m) => m,
            None => continue,
        };
        metrics.name = node_text(name_node, source);
        functions.push(metrics);
    }
}

fn collect_class_methods(
    class_node: Node,
    source: &str,
    functions: &mut Vec<FunctionMetrics>,
    has_types: bool,
) {
    let body = match find_child_by_kind(class_node, "class_body") {
        Some(b) => b,
        None => return,
    };

    let class_name = find_child_by_kind(class_node, "identifier")
        .or_else(|| find_child_by_kind(class_node, "type_identifier"))
        .map(|n| node_text(n, source))
        .unwrap_or_default();

    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        if child.kind() != "method_definition" {
            continue;
        }
        let mut metrics = match analyze_function(child, source, has_types) {
            Some(m) => m,
            None => continue,
        };
        let method_name = metrics.name.clone();
        metrics.name = format!("{}.{}", class_name, method_name);
        metrics.is_constructor = method_name == "constructor";
        metrics.class_name = Some(class_name.clone());
        collect_field_accesses_for(child, source, SELF_NAMES, &mut metrics.field_accesses);
        functions.push(metrics);
    }
}

fn analyze_function(node: Node, source: &str, has_types: bool) -> Option<FunctionMetrics> {
    let name = find_child_by_kind(node, "identifier")
        .or_else(|| find_child_by_kind(node, "property_identifier"))
        .map(|n| node_text(n, source))
        .unwrap_or_else(|| "<anonymous>".into());

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
                check_condition_complexity(child, source, &mut s.compound_condition_count);
                count_boolean_operators(child, &mut s.cc);
                walk_children(child, source, depth + 1, s);
            }
            "for_statement" | "for_in_statement" | "while_statement" | "do_statement" => {
                s.track_loop(depth);
                walk_children(child, source, depth + 1, s);
            }
            "switch_case" => {
                let is_default = find_child_by_kind(child, "default").is_some()
                    || child.child_count() > 0
                        && child.child(0).map_or(false, |c| c.kind() == "default");
                if !is_default {
                    s.cc += 1;
                }
                walk_children(child, source, depth, s);
            }
            "switch_statement" => {
                s.track_nesting(depth);
                walk_children(child, source, depth + 1, s);
            }
            "catch_clause" => { s.cc += 1; walk_children(child, source, depth, s); }
            "try_statement" => walk_children(child, source, depth, s),
            "ternary_expression" => { s.cc += 1; }
            "template_string" | "string" => s.track_embedded(child),
            _ => walk_body(child, source, depth, s),
        }
    }
}

fn walk_children(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "statement_block" | "switch_body" => walk_body(child, source, depth, s),
            "else_clause" => walk_else_clause(child, source, depth, s),
            "finally_clause" => walk_block_children(child, source, depth, s),
            "catch_clause" => {
                s.cc += 1;
                walk_block_children(child, source, depth, s);
            }
            _ => {}
        }
    }
}

fn walk_else_clause(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "statement_block" => walk_body(child, source, depth, s),
            "if_statement" => {
                s.cc += 1;
                check_condition_complexity(child, source, &mut s.compound_condition_count);
                count_boolean_operators(child, &mut s.cc);
                walk_children(child, source, depth, s);
            }
            _ => {}
        }
    }
}

fn walk_block_children(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "statement_block" {
            walk_body(child, source, depth, s);
        }
    }
}

fn count_boolean_operators(node: Node, cc: &mut u32) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "&&" | "||" | "??" => { *cc += 1; }
            "!" => {}
            "statement_block" | "function_declaration" | "class_declaration"
            | "arrow_function" => {}
            _ => count_boolean_operators(child, cc),
        }
    }
}

fn check_condition_complexity(node: Node, source: &str, compound_conditions: &mut u32) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "parenthesized_expression" || child.kind() == "binary_expression" {
            let text = node_text(child, source);
            let logical_ops = text.matches("&&").count()
                + text.matches("||").count()
                + text.matches("??").count();
            if logical_ops >= 2 {
                *compound_conditions += 1;
                return;
            }
        }
    }
}

fn count_parameters(func_node: Node, source: &str) -> (u32, u32, u32) {
    let params = match find_child_by_kind(func_node, "formal_parameters") {
        Some(p) => p,
        None => return (0, 0, 0),
    };
    let mut count: u32 = 0;
    let mut primitive_count: u32 = 0;
    let mut typed_count: u32 = 0;
    let mut cursor = params.walk();
    for child in params.children(&mut cursor) {
        match child.kind() {
            "identifier" | "rest_pattern" | "object_pattern" | "array_pattern"
            | "assignment_pattern" => {
                count += 1;
            }
            "required_parameter" | "optional_parameter" => {
                count += 1;
                let type_ann = match find_child_by_kind(child, "type_annotation") {
                    Some(t) => t,
                    None => continue,
                };
                typed_count += 1;
                if is_primitive_type(type_ann, source) {
                    primitive_count += 1;
                }
            }
            _ => {}
        }
    }
    (count, primitive_count, typed_count)
}

fn count_parameters_untyped(func_node: Node) -> u32 {
    let params = match find_child_by_kind(func_node, "formal_parameters") {
        Some(p) => p,
        None => return 0,
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
            "for_statement" | "for_in_statement" | "while_statement" => {
                let depth = measure_nesting_depth(child, 1, NESTING_BRANCH_KINDS);
                if depth > *max_nesting {
                    *max_nesting = depth;
                }
            }
            "export_statement" => {
                collect_global_metrics(child, conditional_count, max_nesting);
            }
            "function_declaration" | "class_declaration" | "lexical_declaration"
            | "variable_declaration" => {}
            _ => {}
        }
    }
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
