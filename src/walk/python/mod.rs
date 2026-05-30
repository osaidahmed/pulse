mod booleans;

use tree_sitter::{Node, Tree};

use super::{
    collect_field_accesses_for, collect_foreign_field_accesses_for, compute_assert_fingerprint, compute_skeleton_hash,
    compute_structural_fingerprint, count_code_lines, count_consecutive_asserts,
    find_child_by_kind, is_catch_body_empty, node_text, FileMetrics,
    FunctionMetrics, ModuleMetrics, WalkState, track_embedded_block,
};

use super::counters::{count_short_variables, count_string_match_arms};
use super::shared::{self, GlobalMetricsConfig};
use booleans::{count_boolean_operators, count_cogc_boolean_sequences, has_boolean_child};

const COMMENT_PREFIXES: &[&str] = &["#"];
const SELF_NAMES: &[&str] = &["self", "cls"];
const PRIMITIVE_TYPES: &[&str] = &["str", "int", "float", "bool", "bytes", "complex", "None"];
const NESTING_BRANCH_KINDS: &[&str] = &[
    "if_statement",
    "for_statement",
    "while_statement",
    "with_statement",
];
const GLOBAL_CFG: GlobalMetricsConfig = GlobalMetricsConfig {
    cond: &["if_statement"],
    loops: &["for_statement", "while_statement"],
    branches: NESTING_BRANCH_KINDS,
    recurse: &[],
};

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
        let kind = child.kind();
        if kind == "class_definition" { collect_class_methods(child, source, functions); continue; }
        if kind != "function_definition" && kind != "decorated_definition" { continue; }
        let target = if kind == "decorated_definition" {
            find_child_by_kind(child, "function_definition")
        } else { Some(child) };
        let Some(t) = target else { continue; };
        if let Some(m) = analyze_function(t, source) { functions.push(m); }
    }
}

fn collect_class_methods(class_node: Node, source: &str, functions: &mut Vec<FunctionMetrics>) {
    let Some(body) = find_child_by_kind(class_node, "block") else {
        return;
    };
    let class_name = find_child_by_kind(class_node, "identifier")
        .map(|n| node_text(n, source).to_string())
        .unwrap_or_default();
    let parent_class = extract_parent_class_python(class_node, source);

    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        if child.kind() != "function_definition" && child.kind() != "decorated_definition" {
            continue;
        }
        let fn_node = if child.kind() == "decorated_definition" {
            let Some(f) = find_child_by_kind(child, "function_definition") else { continue; };
            f
        } else {
            child
        };
        let Some(mut metrics) = analyze_function(fn_node, source) else {
            continue;
        };
        let method_name = metrics.name.clone();
        metrics.name = format!("{class_name}.{method_name}");
        metrics.is_constructor = method_name == "__init__";
        metrics.class_name = Some(class_name.clone());
        metrics.parent_class = parent_class.clone();
        if super::extras_enabled(fn_node.start_byte(), fn_node.end_byte()) {
            collect_field_accesses_for(fn_node, source, SELF_NAMES, &mut metrics.field_accesses);
            collect_foreign_field_accesses_for(fn_node, source, SELF_NAMES, &mut metrics.foreign_field_accesses);
        }

        if metrics.arg_count > 0 {
            metrics.arg_count -= 1;
        }
        functions.push(metrics);
    }
}

fn extract_parent_class_python(class_node: Node, source: &str) -> Option<String> {
    let bases = find_child_by_kind(class_node, "argument_list")?;
    let first_id = find_child_by_kind(bases, "identifier")?;
    Some(node_text(first_id, source).to_string())
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

    let mut structural_hash = 0;
    let mut skeleton_hash = 0;
    let mut consecutive_asserts = 0;
    let mut assert_hash = 0;
    let mut short_var_count = 0;
    let mut string_match_arms = 0;
    if super::extras_enabled(node.start_byte(), node.end_byte()) {
        structural_hash = compute_structural_fingerprint(body);
        skeleton_hash = compute_skeleton_hash(body);
        consecutive_asserts = count_consecutive_asserts(body, "assert_statement");
        assert_hash = compute_assert_fingerprint(body, "assert_statement");
        short_var_count = count_short_variables(body, source, &["assignment", "augmented_assignment"]);
        string_match_arms = count_string_match_arms(body, "match_statement", "case_clause", &["string"]);
    }

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
        foreign_field_accesses: Vec::new(),
        class_name: None,
        parent_class: None,
        short_var_count,
        string_match_arms,
    })
}

fn walk_body(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    s.reset_bump();

    for child in node.children(&mut cursor) {
        walk_node(child, source, depth, s);
    }
}

type NodeHandler = fn(Node, &str, u32, &mut WalkState);

const NODE_HANDLERS: &[(&[&str], NodeHandler)] = &[
    (&["if_statement"], handle_if),
    (&["for_statement", "while_statement", "with_statement"], handle_loop),
    (&["except_clause"], handle_except),
    (&["else_clause", "try_statement"], walk_children),
    (&["conditional_expression", "assert_statement"], handle_expression_dispatch),
];

const STRING_KINDS: &[&str] = &["string", "concatenated_string"];

fn walk_node(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    let kind = child.kind();
    if STRING_KINDS.contains(&kind) {
        track_embedded_block(&mut s.max_embedded_block_loc, child);
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

fn handle_expression_dispatch(child: Node, _source: &str, _depth: u32, s: &mut WalkState) {
    if child.kind() == "conditional_expression" {
        s.cc += 1;
        s.track_cogc_branch();
    } else if has_boolean_child(child) {
        s.cc += 1;
    }
}

fn handle_if(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.track_if(depth);
    s.track_cogc_branch();
    count_cogc_boolean_sequences(child, &mut s.cogc);
    check_condition_complexity(child, source, &mut s.compound_condition_count);
    count_boolean_operators(child, &mut s.cc);
    walk_children(child, source, depth + 1, s);
}

fn handle_loop(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.track_loop(depth);
    s.track_cogc_branch();
    walk_children(child, source, depth + 1, s);
}

fn handle_except(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.cc += 1;
    s.track_cogc_branch();
    if is_catch_body_empty(child, "block", Some("pass_statement")) {
        s.empty_catch_count += 1;
    }
    walk_children(child, source, depth, s);
}

fn walk_children(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    let kids: Vec<Node> = node.children(&mut cursor).collect();
    for child in kids {
        match child.kind() {
            "block" => walk_nested_block(child, source, depth, s),
            "elif_clause" => handle_elif(child, source, depth, s),
            "else_clause" => handle_else(child, source, depth, s),
            "finally_clause" => shared::walk_block_children(child, &mut shared::BlockWalkCtx { source, depth, state: s }, "block", walk_body),
            "except_clause" => handle_except_in_children(child, source, depth, s),
            _ => {}
        }
    }
}

fn walk_nested_block(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    let saved = s.cogc_nesting;
    s.cogc_nesting += 1;
    walk_body(child, source, depth, s);
    s.cogc_nesting = saved;
}

fn handle_elif(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.cc += 1;
    s.track_cogc_branch();
    if depth > s.max_nesting {
        s.max_nesting = depth;
    }
    check_condition_complexity(child, source, &mut s.compound_condition_count);
    count_boolean_operators(child, &mut s.cc);
    count_cogc_boolean_sequences(child, &mut s.cogc);
    shared::walk_block_children(child, &mut shared::BlockWalkCtx { source, depth, state: s }, "block", walk_body);
}

fn handle_else(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.track_cogc_flat();
    let saved = s.cogc_nesting;
    s.cogc_nesting += 1;
    shared::walk_block_children(child, &mut shared::BlockWalkCtx { source, depth, state: s }, "block", walk_body);
    s.cogc_nesting = saved;
}

fn handle_except_in_children(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.cc += 1;
    s.track_cogc_branch();
    if is_catch_body_empty(child, "block", Some("pass_statement")) {
        s.empty_catch_count += 1;
    }
    let saved = s.cogc_nesting;
    s.cogc_nesting += 1;
    shared::walk_block_children(child, &mut shared::BlockWalkCtx { source, depth, state: s }, "block", walk_body);
    s.cogc_nesting = saved;
}

fn check_condition_complexity(node: Node, source: &str, count: &mut u32) {
    let condition = find_child_by_kind(node, "comparison_operator")
        .or_else(|| find_child_by_kind(node, "boolean_operator"))
        .or_else(|| find_child_by_kind(node, "not_operator"));
    if let Some(cond) = condition {
        let text = node_text(cond, source);
        let ops = text.matches(" and ").count()
            + text.matches(" or ").count()
            + text.matches(" not ").count();
        if ops >= 2 {
            *count += 1;
        }
    }
}

fn count_parameters(func_node: Node, source: &str) -> (u32, u32, u32) {
    let Some(params) = find_child_by_kind(func_node, "parameters") else {
        return (0, 0, 0);
    };
    let mut cursor = params.walk();
    params.children(&mut cursor).fold((0, 0, 0), |(cnt, prim, typed), child| match child.kind() {
        "identifier" | "list_splat_pattern" | "dictionary_splat_pattern" | "default_parameter" => {
            (cnt + 1, prim, typed)
        }
        "typed_parameter" | "typed_default_parameter" => {
            let p = u32::from(has_primitive_type(child, source));
            (cnt + 1, prim + p, typed + 1)
        }
        _ => (cnt, prim, typed),
    })
}

fn has_primitive_type(param_node: Node, source: &str) -> bool {
    let Some(type_node) = find_child_by_kind(param_node, "type") else {
        return false;
    };
    let Some(id_node) = find_child_by_kind(type_node, "identifier") else {
        return false;
    };
    let name = &source[id_node.byte_range()];
    PRIMITIVE_TYPES.contains(&name)
}

fn count_declarations(root: Node) -> u32 {
    let mut cursor = root.walk();
    root.children(&mut cursor)
        .filter(|c| c.kind() == "class_definition" || c.kind() == "decorated_definition")
        .count() as u32
}

