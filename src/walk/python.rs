use tree_sitter::{Node, Tree};

use super::{
    collect_field_accesses_for, compute_assert_fingerprint, compute_skeleton_hash, compute_structural_fingerprint,
    count_code_lines, count_consecutive_asserts, find_child_by_kind, measure_nesting_depth,
    node_text, FileMetrics, FunctionMetrics, ModuleMetrics, WalkState,
};

const COMMENT_PREFIXES: &[&str] = &["#"];
const SELF_NAMES: &[&str] = &["self", "cls"];
const PRIMITIVE_TYPES: &[&str] = &["str", "int", "float", "bool", "bytes", "complex", "None"];
const NESTING_BRANCH_KINDS: &[&str] = &[
    "if_statement",
    "for_statement",
    "while_statement",
    "with_statement",
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
            "function_definition" | "decorated_definition" => {
                try_add_function(child, source, functions);
            }
            "class_definition" => collect_class_methods(child, source, functions),
            _ => {}
        }
    }
}

fn try_add_function(node: Node, source: &str, functions: &mut Vec<FunctionMetrics>) {
    let fn_node = match unwrap_decorated(node) {
        Some(n) => n,
        None => return,
    };
    if let Some(m) = analyze_function(fn_node, source) {
        functions.push(m);
    }
}

fn unwrap_decorated(node: Node) -> Option<Node> {
    if node.kind() == "decorated_definition" {
        find_child_by_kind(node, "function_definition")
    } else {
        Some(node)
    }
}

fn collect_class_methods(class_node: Node, source: &str, functions: &mut Vec<FunctionMetrics>) {
    let body = match find_child_by_kind(class_node, "block") {
        Some(b) => b,
        None => return,
    };
    let class_name = find_child_by_kind(class_node, "identifier")
        .map(|n| node_text(n, source))
        .unwrap_or_default();

    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        if child.kind() != "function_definition" && child.kind() != "decorated_definition" {
            continue;
        }
        let fn_node = match unwrap_decorated(child) {
            Some(n) => n,
            None => continue,
        };
        let mut metrics = match analyze_function(fn_node, source) {
            Some(m) => m,
            None => continue,
        };
        let method_name = metrics.name.clone();
        metrics.name = format!("{}.{}", class_name, method_name);
        metrics.is_constructor = method_name == "__init__";
        metrics.class_name = Some(class_name.clone());
        collect_field_accesses_for(fn_node, source, SELF_NAMES, &mut metrics.field_accesses);
        if metrics.arg_count > 0 {
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
    let skeleton_hash = compute_skeleton_hash(body);
    let consecutive_asserts = count_consecutive_asserts(body, "assert_statement");
    let assert_hash = compute_assert_fingerprint(body, "assert_statement");

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
            "for_statement" | "while_statement" | "with_statement" => {
                s.track_loop(depth);
                walk_children(child, source, depth + 1, s);
            }
            "except_clause" => { s.cc += 1; walk_children(child, source, depth, s); }
            "else_clause" | "try_statement" => walk_children(child, source, depth, s),
            "conditional_expression" => s.cc += 1,
            "assert_statement" => { if has_boolean_child(child) { s.cc += 1; } }
            "string" | "concatenated_string" => s.track_embedded(child),
            _ => walk_body(child, source, depth, s),
        }
    }
}

fn walk_children(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "block" => walk_body(child, source, depth, s),
            "elif_clause" => {
                s.cc += 1;
                if depth > s.max_nesting { s.max_nesting = depth; }
                check_condition_complexity(child, source, &mut s.compound_condition_count);
                count_boolean_operators(child, &mut s.cc);
                walk_block_children(child, source, depth, s);
            }
            "else_clause" | "finally_clause" => walk_block_children(child, source, depth, s),
            "except_clause" => {
                s.cc += 1;
                walk_block_children(child, source, depth, s);
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
            "boolean_operator" | "not_operator" => {
                *cc += 1;
                count_boolean_operators(child, cc);
            }
            "block" | "function_definition" | "class_definition" => {}
            _ => count_boolean_operators(child, cc),
        }
    }
}

fn check_condition_complexity(node: Node, source: &str, count: &mut u32) {
    let condition = find_child_by_kind(node, "comparison_operator")
        .or_else(|| find_child_by_kind(node, "boolean_operator"))
        .or_else(|| find_child_by_kind(node, "not_operator"));
    if let Some(cond) = condition {
        let text = node_text(cond, source);
        let ops = text.matches(" and ").count() + text.matches(" or ").count() + text.matches(" not ").count();
        if ops >= 2 { *count += 1; }
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
            "identifier" | "list_splat_pattern" | "dictionary_splat_pattern" | "default_parameter" => {
                count += 1;
            }
            "typed_parameter" | "typed_default_parameter" => {
                count += 1;
                typed_count += 1;
                if has_primitive_type(child, source) { primitive_count += 1; }
            }
            _ => {}
        }
    }
    (count, primitive_count, typed_count)
}

fn has_primitive_type(param_node: Node, source: &str) -> bool {
    let type_node = match find_child_by_kind(param_node, "type") {
        Some(t) => t,
        None => return false,
    };
    let id_node = match find_child_by_kind(type_node, "identifier") {
        Some(n) => n,
        None => return false,
    };
    let name = &source[id_node.byte_range()];
    PRIMITIVE_TYPES.contains(&name)
}

fn collect_global_metrics(root: Node, conditional_count: &mut u32, max_nesting: &mut u32) {
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        match child.kind() {
            "if_statement" => {
                *conditional_count += 1;
                let depth = measure_nesting_depth(child, 1, NESTING_BRANCH_KINDS);
                if depth > *max_nesting { *max_nesting = depth; }
            }
            "for_statement" | "while_statement" => {
                let depth = measure_nesting_depth(child, 1, NESTING_BRANCH_KINDS);
                if depth > *max_nesting { *max_nesting = depth; }
            }
            "function_definition" | "class_definition" | "decorated_definition" => {}
            _ => {}
        }
    }
}

fn count_declarations(root: Node) -> u32 {
    let mut cursor = root.walk();
    root.children(&mut cursor)
        .filter(|c| c.kind() == "class_definition" || c.kind() == "decorated_definition")
        .count() as u32
}

fn has_boolean_child(node: Node) -> bool {
    let mut cursor = node.walk();
    let result = node.children(&mut cursor).any(|c| c.kind() == "boolean_operator");
    result
}
