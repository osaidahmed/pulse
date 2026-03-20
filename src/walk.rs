use tree_sitter::{Node, Tree};

#[derive(Debug)]
pub struct FunctionMetrics {
    pub name: String,
    pub start_line: u32,
    pub end_line: u32,
    pub loc: u32,
    pub cc: u32,
    pub max_nesting: u32,
    pub bump_count: u32,
    pub arg_count: u32,
    pub compound_condition_count: u32,
    pub is_constructor: bool,
    pub max_embedded_block_loc: u32,
}

#[derive(Debug)]
pub struct ModuleMetrics {
    pub total_loc: u32,
    pub total_functions: u32,
    pub sum_cc: u32,
    pub global_conditional_count: u32,
    pub global_max_nesting: u32,
}

pub type FileMetrics = (Vec<FunctionMetrics>, ModuleMetrics);

pub fn walk_python(tree: &Tree, source: &str) -> FileMetrics {
    let root = tree.root_node();
    let total_loc = count_code_lines(source);

    let mut functions = Vec::new();
    let mut global_conditional_count: u32 = 0;
    let mut global_max_nesting: u32 = 0;

    collect_functions_python(root, source, &mut functions);
    collect_global_metrics_python(root, &mut global_conditional_count, &mut global_max_nesting);

    let total_functions = functions.len() as u32;
    let sum_cc: u32 = functions.iter().map(|f| f.cc).sum();

    let module = ModuleMetrics {
        total_loc,
        total_functions,
        sum_cc,
        global_conditional_count,
        global_max_nesting,
    };

    (functions, module)
}

fn collect_functions_python(node: Node, source: &str, functions: &mut Vec<FunctionMetrics>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "function_definition" | "decorated_definition" => {
                let func_node = if child.kind() == "decorated_definition" {
                    find_child_by_kind(child, "function_definition")
                } else {
                    Some(child)
                };
                if let Some(fn_node) = func_node {
                    if let Some(metrics) = analyze_function_python(fn_node, source) {
                        functions.push(metrics);
                    }
                }
            }
            "class_definition" => {
                collect_class_methods_python(child, source, functions);
            }
            _ => {}
        }
    }
}

fn collect_class_methods_python(
    class_node: Node,
    source: &str,
    functions: &mut Vec<FunctionMetrics>,
) {
    let body = match find_child_by_kind(class_node, "block") {
        Some(b) => b,
        None => return,
    };

    let class_name = find_child_by_kind(class_node, "identifier")
        .map(|n| node_text(n, source))
        .unwrap_or_default();

    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        match child.kind() {
            "function_definition" | "decorated_definition" => {
                let func_node = if child.kind() == "decorated_definition" {
                    find_child_by_kind(child, "function_definition")
                } else {
                    Some(child)
                };
                if let Some(fn_node) = func_node {
                    if let Some(mut metrics) = analyze_function_python(fn_node, source) {
                        let method_name = metrics.name.clone();
                        metrics.name = format!("{}.{}", class_name, method_name);
                        metrics.is_constructor = method_name == "__init__";
                        // Subtract 'self'/'cls' from arg count for methods
                        if metrics.arg_count > 0 {
                            metrics.arg_count -= 1;
                        }
                        functions.push(metrics);
                    }
                }
            }
            _ => {}
        }
    }
}

fn analyze_function_python(node: Node, source: &str) -> Option<FunctionMetrics> {
    let name = find_child_by_kind(node, "identifier")
        .map(|n| node_text(n, source))
        .unwrap_or_else(|| "<anonymous>".into());

    let start_line = node.start_position().row as u32 + 1;
    let end_line = node.end_position().row as u32 + 1;
    let loc = end_line.saturating_sub(start_line) + 1;

    let arg_count = count_parameters_python(node);

    let body = find_child_by_kind(node, "block")?;
    let mut cc: u32 = 1;
    let mut max_nesting: u32 = 0;
    let mut bump_count: u32 = 0;
    let mut compound_condition_count: u32 = 0;
    let mut max_embedded_block_loc: u32 = 0;

    walk_body_python(
        body,
        source,
        0,
        &mut cc,
        &mut max_nesting,
        &mut bump_count,
        &mut compound_condition_count,
        &mut max_embedded_block_loc,
    );

    Some(FunctionMetrics {
        name,
        start_line,
        end_line,
        loc,
        cc,
        max_nesting,
        bump_count,
        arg_count,
        compound_condition_count,
        is_constructor: false,
        max_embedded_block_loc,
    })
}

fn walk_body_python(
    node: Node,
    source: &str,
    depth: u32,
    cc: &mut u32,
    max_nesting: &mut u32,
    bump_count: &mut u32,
    compound_conditions: &mut u32,
    max_embedded_loc: &mut u32,
) {
    let mut cursor = node.walk();
    let mut saw_bump_at_this_level = false;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "if_statement" => {
                *cc += 1;
                let new_depth = depth + 1;
                if new_depth > *max_nesting {
                    *max_nesting = new_depth;
                }
                if depth >= 2 && !saw_bump_at_this_level {
                    *bump_count += 1;
                    saw_bump_at_this_level = true;
                }
                check_condition_complexity(child, source, compound_conditions);
                count_boolean_operators(child, cc);
                walk_children_python(child, source, new_depth, cc, max_nesting, bump_count, compound_conditions, max_embedded_loc);
            }
            "for_statement" | "while_statement" => {
                *cc += 1;
                let new_depth = depth + 1;
                if new_depth > *max_nesting {
                    *max_nesting = new_depth;
                }
                walk_children_python(child, source, new_depth, cc, max_nesting, bump_count, compound_conditions, max_embedded_loc);
            }
            "except_clause" => {
                *cc += 1;
                walk_children_python(child, source, depth, cc, max_nesting, bump_count, compound_conditions, max_embedded_loc);
            }
            "else_clause" => {
                walk_children_python(child, source, depth, cc, max_nesting, bump_count, compound_conditions, max_embedded_loc);
            }
            "try_statement" => {
                walk_children_python(child, source, depth, cc, max_nesting, bump_count, compound_conditions, max_embedded_loc);
            }
            "with_statement" => {
                let new_depth = depth + 1;
                if new_depth > *max_nesting {
                    *max_nesting = new_depth;
                }
                walk_children_python(child, source, new_depth, cc, max_nesting, bump_count, compound_conditions, max_embedded_loc);
            }
            "conditional_expression" => {
                *cc += 1;
            }
            "assert_statement" => {
                if has_boolean_child(child) {
                    *cc += 1;
                }
            }
            "string" | "concatenated_string" => {
                let lines = child.end_position().row.saturating_sub(child.start_position().row) as u32 + 1;
                if lines > *max_embedded_loc {
                    *max_embedded_loc = lines;
                }
            }
            _ => {
                walk_body_python(child, source, depth, cc, max_nesting, bump_count, compound_conditions, max_embedded_loc);
            }
        }
    }
}

fn walk_children_python(
    node: Node,
    source: &str,
    depth: u32,
    cc: &mut u32,
    max_nesting: &mut u32,
    bump_count: &mut u32,
    compound_conditions: &mut u32,
    max_embedded_loc: &mut u32,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "block" => {
                walk_body_python(child, source, depth, cc, max_nesting, bump_count, compound_conditions, max_embedded_loc);
            }
            "elif_clause" => {
                *cc += 1;
                if depth > *max_nesting {
                    *max_nesting = depth;
                }
                check_condition_complexity(child, source, compound_conditions);
                count_boolean_operators(child, cc);
                let mut inner_cursor = child.walk();
                for inner in child.children(&mut inner_cursor) {
                    if inner.kind() == "block" {
                        walk_body_python(inner, source, depth, cc, max_nesting, bump_count, compound_conditions, max_embedded_loc);
                    }
                }
            }
            "else_clause" => {
                let mut inner_cursor = child.walk();
                for inner in child.children(&mut inner_cursor) {
                    if inner.kind() == "block" {
                        walk_body_python(inner, source, depth, cc, max_nesting, bump_count, compound_conditions, max_embedded_loc);
                    }
                }
            }
            _ => {}
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
            _ => {
                count_boolean_operators(child, cc);
            }
        }
    }
}

fn check_condition_complexity(node: Node, source: &str, compound_conditions: &mut u32) {
    if let Some(condition) = find_child_by_kind(node, "comparison_operator")
        .or_else(|| find_child_by_kind(node, "boolean_operator"))
        .or_else(|| find_child_by_kind(node, "not_operator"))
    {
        let text = node_text(condition, source);
        let logical_ops = text.matches(" and ").count() + text.matches(" or ").count() + text.matches(" not ").count();
        if logical_ops >= 2 {
            *compound_conditions += 1;
        }
    }
}

fn count_parameters_python(func_node: Node) -> u32 {
    let params = match find_child_by_kind(func_node, "parameters") {
        Some(p) => p,
        None => return 0,
    };
    let mut count: u32 = 0;
    let mut cursor = params.walk();
    for child in params.children(&mut cursor) {
        match child.kind() {
            "identifier"
            | "typed_parameter"
            | "default_parameter"
            | "typed_default_parameter"
            | "list_splat_pattern"
            | "dictionary_splat_pattern" => {
                count += 1;
            }
            _ => {}
        }
    }
    count
}

fn collect_global_metrics_python(
    root: Node,
    conditional_count: &mut u32,
    max_nesting: &mut u32,
) {
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        match child.kind() {
            "if_statement" => {
                *conditional_count += 1;
                let depth = measure_nesting_depth(child, 1);
                if depth > *max_nesting {
                    *max_nesting = depth;
                }
            }
            "for_statement" | "while_statement" => {
                let depth = measure_nesting_depth(child, 1);
                if depth > *max_nesting {
                    *max_nesting = depth;
                }
            }
            "function_definition" | "class_definition" | "decorated_definition" => {}
            _ => {}
        }
    }
}

fn measure_nesting_depth(node: Node, current: u32) -> u32 {
    let mut max = current;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let child_depth = match child.kind() {
            "if_statement" | "for_statement" | "while_statement" | "with_statement" => {
                measure_nesting_depth(child, current + 1)
            }
            "block" => measure_nesting_depth(child, current),
            _ => current,
        };
        if child_depth > max {
            max = child_depth;
        }
    }
    max
}

fn count_code_lines(source: &str) -> u32 {
    source
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with('#')
        })
        .count() as u32
}

fn find_child_by_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    let result = node.children(&mut cursor).find(|c| c.kind() == kind);
    result
}

fn node_text(node: Node, source: &str) -> String {
    source[node.byte_range()].to_string()
}

fn has_boolean_child(node: Node) -> bool {
    let mut cursor = node.walk();
    let result = node.children(&mut cursor)
        .any(|c| c.kind() == "boolean_operator");
    result
}
