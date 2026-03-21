use tree_sitter::{Node, Tree};

use super::{
    collect_field_accesses_for, compute_assert_fingerprint, compute_skeleton_hash,
    compute_structural_fingerprint, count_code_lines, count_consecutive_asserts,
    find_child_by_kind, measure_nesting_depth, node_text, FileMetrics, FunctionMetrics,
    ModuleMetrics, WalkState,
};

const COMMENT_PREFIXES: &[&str] = &["//", "/*", "*"];
const _SELF_NAMES: &[&str] = &[];
const PRIMITIVE_TYPES: &[&str] = &[
    "int", "int8", "int16", "int32", "int64", "uint", "uint8", "uint16", "uint32", "uint64",
    "uintptr", "float32", "float64", "bool", "string", "byte", "rune", "complex64", "complex128",
    "error",
];
const NESTING_BRANCH_KINDS: &[&str] = &[
    "if_statement",
    "for_statement",
    "expression_switch_statement",
    "type_switch_statement",
    "select_statement",
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
            "function_declaration" => {
                if let Some(metrics) = analyze_function(child, source) {
                    functions.push(metrics);
                }
            }
            "method_declaration" => {
                if let Some(metrics) = analyze_method(child, source) {
                    functions.push(metrics);
                }
            }
            _ => {}
        }
    }
}

struct MethodContext {
    name: String,
    arg_count: u32,
    primitive_type_count: u32,
    typed_param_count: u32,
    field_accesses: Vec<String>,
    class_name: Option<String>,
}

fn analyze_function(node: Node, source: &str) -> Option<FunctionMetrics> {
    let name = find_child_by_kind(node, "identifier")
        .map_or_else(|| "<anonymous>".into(), |n| node_text(n, source).to_string());
    let (arg_count, primitive_type_count, typed_param_count) =
        count_parameters_from_node(node, source);
    let info = MethodContext { name, arg_count, primitive_type_count, typed_param_count, field_accesses: Vec::new(), class_name: None };
    build_metrics(node, source, info)
}

fn analyze_method(node: Node, source: &str) -> Option<FunctionMetrics> {
    let param_lists = collect_param_lists(node);

    let receiver_type = param_lists
        .first()
        .and_then(|r| extract_receiver_type(*r, source));

    let method_name = find_child_by_kind(node, "field_identifier")
        .map_or_else(|| "<anonymous>".into(), |n| node_text(n, source).to_string());

    let name = match receiver_type {
        Some(ref t) => format!("{t}.{method_name}"),
        None => method_name,
    };

    let (arg_count, prim, typed) = if param_lists.len() >= 2 {
        count_param_children(param_lists[1], source)
    } else {
        (0, 0, 0)
    };

    let self_names: Vec<&str> = param_lists
        .first()
        .and_then(|r| extract_receiver_name(*r, source))
        .map(|n| vec![n])
        .unwrap_or_default();
    let mut field_accesses = Vec::new();
    if !self_names.is_empty() {
        collect_field_accesses_for(node, source, &self_names, &mut field_accesses);
    }

    let info = MethodContext { name, arg_count, primitive_type_count: prim, typed_param_count: typed, field_accesses, class_name: receiver_type };
    build_metrics(node, source, info)
}

fn collect_param_lists(node: Node) -> Vec<Node> {
    let mut lists = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "parameter_list" {
            lists.push(child);
        }
    }
    lists
}

fn build_metrics(node: Node, source: &str, info: MethodContext) -> Option<FunctionMetrics> {
    let start_line = node.start_position().row as u32 + 1;
    let end_line = node.end_position().row as u32 + 1;
    let loc = end_line.saturating_sub(start_line) + 1;

    let body = find_child_by_kind(node, "block")?;
    let mut s = WalkState::new();
    walk_body(body, source, 0, &mut s);

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
        is_constructor: false,
        max_embedded_block_loc: s.max_embedded_block_loc,
        structural_hash: compute_structural_fingerprint(body),
        skeleton_hash: compute_skeleton_hash(body),
        consecutive_asserts: count_consecutive_asserts(body, "expression_statement"),
        assert_hash: compute_assert_fingerprint(body, "expression_statement"),
        primitive_type_count: info.primitive_type_count,
        typed_param_count: info.typed_param_count,
        empty_catch_count: 0,
        field_accesses: info.field_accesses,
        class_name: info.class_name,
    })
}

fn extract_receiver_type(receiver_list: Node, source: &str) -> Option<String> {
    let param = find_child_by_kind(receiver_list, "parameter_declaration")?;
    // Type can be type_identifier directly or pointer_type > type_identifier
    if let Some(ti) = find_child_by_kind(param, "type_identifier") {
        return Some(node_text(ti, source).to_string());
    }
    if let Some(ptr) = find_child_by_kind(param, "pointer_type") {
        if let Some(ti) = find_child_by_kind(ptr, "type_identifier") {
            return Some(node_text(ti, source).to_string());
        }
    }
    None
}

fn extract_receiver_name<'a>(receiver_list: Node<'a>, source: &'a str) -> Option<&'a str> {
    let param = find_child_by_kind(receiver_list, "parameter_declaration")?;
    let id = find_child_by_kind(param, "identifier")?;
    Some(node_text(id, source))
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
                walk_if_children(child, source, depth + 1, s);
            }
            "for_statement" => {
                s.track_loop(depth);
                s.track_cogc_branch();
                walk_for_body(child, source, depth + 1, s);
            }
            "expression_switch_statement"
            | "type_switch_statement"
            | "select_statement" => {
                s.track_nesting(depth);
                s.track_cogc_branch();
                let saved = s.cogc_nesting;
                s.cogc_nesting += 1;
                walk_switch_cases(child, source, depth + 1, s);
                s.cogc_nesting = saved;
            }
            "go_statement" | "defer_statement" => {
                walk_body(child, source, depth, s);
            }
            "func_literal" => {}
            "interpreted_string_literal" | "raw_string_literal" => s.track_embedded(child),
            _ => walk_body(child, source, depth, s),
        }
    }
}

fn walk_for_body(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "block" {
            let saved = s.cogc_nesting;
            s.cogc_nesting += 1;
            walk_body(child, source, depth, s);
            s.cogc_nesting = saved;
        }
    }
}

fn walk_if_children(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    // Go's if_statement children in order:
    //   "if" keyword, optional init, condition, block (consequence),
    //   optional: "else" keyword, then either block (else) or if_statement (else-if)
    //
    // We iterate children. The first "block" is the consequence body.
    // After that, if we see another "block" it's the else body.
    // If we see an "if_statement" it's else-if.
    let mut saw_consequence = false;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "block" => {
                if saw_consequence {
                    // Else body
                    s.track_cogc_flat();
                    let saved = s.cogc_nesting;
                    s.cogc_nesting += 1;
                    walk_body(child, source, depth, s);
                    s.cogc_nesting = saved;
                } else {
                    // Consequence (then) body
                    saw_consequence = true;
                    let saved = s.cogc_nesting;
                    s.cogc_nesting += 1;
                    walk_body(child, source, depth, s);
                    s.cogc_nesting = saved;
                }
            }
            "if_statement" => {
                // else-if: no nesting increase, but cc and cogc
                s.cc += 1;
                s.track_cogc_branch();
                count_boolean_operators(child, &mut s.cc);
                count_cogc_boolean_sequences(child, &mut s.cogc);
                check_condition_complexity(child, source, &mut s.compound_condition_count);
                walk_if_children(child, source, depth, s);
            }
            _ => {}
        }
    }
}

fn walk_switch_cases(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "expression_case" | "type_case" | "communication_case" => {
                s.cc += 1;
                walk_case_body(child, source, depth, s);
            }
            "default_case" => {
                walk_case_body(child, source, depth, s);
            }
            _ => {}
        }
    }
}

fn walk_case_body(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        // Walk all statement children inside the case
        match child.kind() {
            "expression_case" | "type_case" | "communication_case" | "default_case" | ":"
            | "case" | "default" => {}
            _ => walk_body(child, source, depth, s),
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
            "block" | "function_declaration" | "method_declaration" | "func_literal" => {}
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
            "block" | "function_declaration" | "method_declaration" | "func_literal" => {}
            _ => collect_boolean_ops(child, cogc, last_op),
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

fn count_parameters_from_node(func_node: Node, source: &str) -> (u32, u32, u32) {
    let Some(params) = find_child_by_kind(func_node, "parameter_list") else {
        return (0, 0, 0);
    };
    count_param_children(params, source)
}

fn count_param_children(params: Node, source: &str) -> (u32, u32, u32) {
    let mut count: u32 = 0;
    let mut primitive_count: u32 = 0;
    let mut typed_count: u32 = 0;
    let mut cursor = params.walk();
    for child in params.children(&mut cursor) {
        let (n, is_prim) = match child.kind() {
            "parameter_declaration" => {
                let names = count_param_names(child);
                (if names == 0 { 1 } else { names }, has_primitive_type(child, source))
            }
            "variadic_parameter_declaration" => (1, has_primitive_type(child, source)),
            _ => continue,
        };
        count += n;
        typed_count += n;
        if is_prim {
            primitive_count += n;
        }
    }
    (count, primitive_count, typed_count)
}

fn count_param_names(param: Node) -> u32 {
    let mut count: u32 = 0;
    let mut cursor = param.walk();
    for child in param.children(&mut cursor) {
        if child.kind() == "identifier" {
            count += 1;
        }
    }
    count
}

fn has_primitive_type(param: Node, source: &str) -> bool {
    // Check for type_identifier directly or inside pointer_type / slice_type
    if let Some(ti) = find_child_by_kind(param, "type_identifier") {
        let name = node_text(ti, source);
        return PRIMITIVE_TYPES.contains(&name);
    }
    if let Some(ptr) = find_child_by_kind(param, "pointer_type") {
        if let Some(ti) = find_child_by_kind(ptr, "type_identifier") {
            let name = node_text(ti, source);
            return PRIMITIVE_TYPES.contains(&name);
        }
    }
    if let Some(slice) = find_child_by_kind(param, "slice_type") {
        if let Some(ti) = find_child_by_kind(slice, "type_identifier") {
            let name = node_text(ti, source);
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
            "for_statement" => {
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
    let mut count: u32 = 0;
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if child.kind() != "type_declaration" {
            continue;
        }
        let mut inner = child.walk();
        for spec in child.children(&mut inner) {
            if spec.kind() != "type_spec" {
                continue;
            }
            if find_child_by_kind(spec, "struct_type").is_some()
                || find_child_by_kind(spec, "interface_type").is_some()
            {
                count += 1;
            }
        }
    }
    count
}
