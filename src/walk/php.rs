use tree_sitter::{Node, Tree};

use super::counters::{count_short_variables, count_string_match_arms, max_same_primitive};
use super::shared::{self, count_boolean_ops, count_cogc_sequences, GlobalMetricsConfig};
use super::{
    collect_field_accesses_for, collect_foreign_field_accesses_for, compute_assert_fingerprint, compute_skeleton_hash,
    compute_structural_fingerprint, count_code_lines, count_consecutive_asserts, count_distinct_node_kinds,
    find_child_by_kind, is_catch_body_empty, node_text, track_embedded_block, FileMetrics, FunctionMetrics,
    ModuleMetrics, WalkState,
};

const COMMENT_PREFIXES: &[&str] = &["//", "#", "/*", "*"];
const SELF_NAMES: &[&str] = &["$this"];
const PRIMITIVE_TYPES: &[&str] = &[
    "int", "float", "string", "bool", "array", "callable", "iterable", "mixed", "void", "null", "false", "true",
    "never",
];
const NESTING_BRANCH_KINDS: &[&str] = &[
    "if_statement",
    "for_statement",
    "foreach_statement",
    "while_statement",
    "do_statement",
    "switch_statement",
    "match_expression",
];
const BOOL_OPS: &[&str] = &["&&", "||", "and", "or"];
const BOOL_STOPS: &[&str] = &[
    "compound_statement",
    "function_definition",
    "method_declaration",
    "class_declaration",
    "arrow_function",
    "anonymous_function",
];
const GLOBAL_CFG: GlobalMetricsConfig = GlobalMetricsConfig {
    cond: &["if_statement"],
    loops: &["for_statement", "foreach_statement", "while_statement", "do_statement"],
    branches: NESTING_BRANCH_KINDS,
    recurse: &["namespace_definition"],
};
const COND_KINDS: &[&str] = &["parenthesized_expression"];
const SCOPE_BOUNDARY: &[&str] = &["arrow_function", "anonymous_function"];
const EMBEDDED_KINDS: &[&str] = &["encapsed_string", "heredoc", "nowdoc", "string"];
const LOOP_KINDS: &[&str] = &["for_statement", "foreach_statement", "while_statement", "do_statement"];

fn is_body_block(kind: &str) -> bool {
    matches!(kind, "compound_statement" | "colon_block")
}

pub fn walk(tree: &Tree, source: &str) -> FileMetrics {
    let root = tree.root_node();
    let total_loc = count_code_lines(source, COMMENT_PREFIXES);
    let mut functions = Vec::new();
    let mut gcc: u32 = 0;
    let mut gmn: u32 = 0;

    collect_functions(root, source, &mut functions);
    shared::collect_global_metrics(root, &mut gcc, &mut gmn, &GLOBAL_CFG);

    let total_functions = functions.len() as u32;
    let sum_cc: u32 = functions.iter().map(|f| f.cc).sum();
    let declaration_count = count_declarations(root);

    let module = ModuleMetrics {
        total_loc,
        total_functions,
        sum_cc,
        global_conditional_count: gcc,
        global_max_nesting: gmn,
        declaration_count,
        struct_fields: Vec::new(),
    };
    FileMetrics { functions, module }
}

// ─── Function collection ────────────────────────────────────────────────

fn collect_functions(node: Node, source: &str, fns: &mut Vec<FunctionMetrics>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let kind = child.kind();
        if kind == "function_definition" {
            fns.extend(analyze_function(child, source));
        } else if kind == "namespace_definition" {
            if let Some(body) = find_child_by_kind(child, "compound_statement") {
                collect_functions(body, source, fns);
            }
        } else {
            collect_type_body(child, source, fns);
        }
    }
}

fn collect_type_body(type_node: Node, source: &str, fns: &mut Vec<FunctionMetrics>) {
    let bk = match type_node.kind() {
        "class_declaration" | "interface_declaration" | "trait_declaration" => "declaration_list",
        "enum_declaration" => "enum_declaration_list",
        _ => return,
    };
    let type_name = find_child_by_kind(type_node, "name").map(|n| node_text(n, source).to_string()).unwrap_or_default();
    let Some(body) = find_child_by_kind(type_node, bk) else { return };

    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        if child.kind() == "method_declaration" {
            add_method(child, source, &type_name, fns);
            continue;
        }
        collect_type_body(child, source, fns);
    }
}

fn add_method(node: Node, source: &str, type_name: &str, fns: &mut Vec<FunctionMetrics>) {
    let Some(mut m) = analyze_function(node, source) else { return };
    let method_name = m.name.clone();
    m.name = format!("{type_name}.{method_name}");
    m.class_name = Some(type_name.to_string());
    m.is_constructor = method_name == "__construct";
    if !m.is_constructor {
        collect_field_accesses_for(node, source, SELF_NAMES, &mut m.field_accesses);

        collect_foreign_field_accesses_for(node, source, SELF_NAMES, &mut m.foreign_field_accesses);
    }
    fns.push(m);
}

fn count_declarations(root: Node) -> u32 {
    let mut count: u32 = 0;
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        match child.kind() {
            "class_declaration" | "interface_declaration" | "trait_declaration" | "enum_declaration" => count += 1,
            "namespace_definition" => {
                if let Some(body) = find_child_by_kind(child, "compound_statement") {
                    count += count_declarations(body);
                }
            }
            _ => {}
        }
    }
    count
}

// ─── Function analysis ──────────────────────────────────────────────────

fn analyze_function(node: Node, source: &str) -> Option<FunctionMetrics> {
    let name =
        find_child_by_kind(node, "name").map_or_else(|| "<anonymous>".into(), |n| node_text(n, source).to_string());
    let start_line = node.start_position().row as u32 + 1;
    let end_line = node.end_position().row as u32 + 1;
    let (arg_count, primitive_type_count, typed_param_count, max_same_primitive_count) = count_parameters(node, source);
    let body = find_child_by_kind(node, "compound_statement")?;
    let cpg = super::cpg_for(body, node, source, &crate::cpg::PHP).map(|mut c| {
        c.flag_all_dead_stores = true;
        c
    });

    let mut s = WalkState::new();
    walk_body(body, source, 0, &mut s);

    Some(FunctionMetrics {
        name,
        start_line,
        end_line,
        loc: end_line.saturating_sub(start_line) + 1,
        cc: s.cc,
        cognitive_complexity: s.cogc,
        max_nesting: s.max_nesting,
        bump_count: s.bump_count,
        arg_count,
        compound_condition_count: s.compound_condition_count,
        is_constructor: false,
        max_embedded_block_loc: s.max_embedded_block_loc,
        structural_hash: compute_structural_fingerprint(body),
        distinct_node_kinds: count_distinct_node_kinds(body),
        skeleton_hash: compute_skeleton_hash(body),
        consecutive_asserts: count_consecutive_asserts(body, "expression_statement"),
        assert_hash: compute_assert_fingerprint(body, "expression_statement"),
        primitive_type_count,
        typed_param_count,
        max_same_primitive_count,
        empty_catch_count: s.empty_catch_count,
        field_accesses: Vec::new(),
        foreign_field_accesses: Vec::new(),
        class_name: None,
        parent_class: None,
        short_var_count: count_short_variables(
            body,
            source,
            &["assignment_expression", "augmented_assignment_expression"],
        ),
        string_match_arms: count_string_match_arms(
            body,
            "match_expression",
            "match_conditional_expression",
            &["string", "encapsed_string"],
            &["match_default_expression"],
        ),
        cpg,
    })
}

fn count_parameters(node: Node, source: &str) -> (u32, u32, u32, u32) {
    let Some(params) = find_child_by_kind(node, "formal_parameters") else { return (0, 0, 0, 0) };
    let mut cursor = params.walk();
    let mut count = 0;
    let mut typed = 0;
    let mut prims: Vec<&str> = Vec::new();
    for child in params
        .children(&mut cursor)
        .filter(|c| matches!(c.kind(), "simple_parameter" | "variadic_parameter" | "property_promotion_parameter"))
    {
        count += 1;
        let mut pc = child.walk();
        let type_node = child.children(&mut pc).find(|c| {
            matches!(c.kind(), "primitive_type" | "named_type" | "optional_type" | "union_type" | "intersection_type")
        });
        if let Some(tn) = type_node {
            typed += 1;
            if is_primitive_type_node(tn, source) {
                prims.push(node_text(tn, source));
            }
        }
    }
    (count, prims.len() as u32, typed, max_same_primitive(&prims))
}

fn is_primitive_type_node(node: Node, source: &str) -> bool {
    match node.kind() {
        "primitive_type" => true,
        "named_type" => PRIMITIVE_TYPES.contains(&node_text(node, source)),
        "optional_type" => {
            let mut c = node.walk();
            let result = node
                .children(&mut c)
                .find(|ch| matches!(ch.kind(), "primitive_type" | "named_type"))
                .is_some_and(|inner| is_primitive_type_node(inner, source));
            result
        }
        _ => false,
    }
}

// ─── Control flow walking ───────────────────────────────────────────────

fn walk_body(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    node.children(&mut cursor).for_each(|child| dispatch(child, source, depth, s));
}

fn route_structured(node: Node, source: &str, depth: u32, s: &mut WalkState) -> bool {
    let kind = node.kind();
    if kind == "if_statement" {
        handle_if(node, source, depth, s);
        return true;
    }
    if LOOP_KINDS.contains(&kind) {
        handle_loop(node, source, depth, s);
        return true;
    }
    if kind == "switch_statement" || kind == "match_expression" {
        handle_branching(node, source, depth, s);
        return true;
    }
    if kind == "try_statement" {
        walk_try(node, source, depth, s);
        return true;
    }
    if kind == "catch_clause" {
        handle_catch(node, source, depth, s);
        return true;
    }
    false
}

fn dispatch(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    if route_structured(node, source, depth, s) {
        return;
    }
    let kind = node.kind();
    if kind == "conditional_expression" {
        s.cc += 1;
        s.track_cogc_branch();
        return;
    }
    if EMBEDDED_KINDS.contains(&kind) {
        track_embedded_block(&mut s.max_embedded_block_loc, node);
        return;
    }
    if !SCOPE_BOUNDARY.contains(&kind) {
        walk_body(node, source, depth, s);
    }
}

fn track_condition(node: Node, s: &mut WalkState) {
    count_boolean_ops(node, &mut s.cc, BOOL_OPS, BOOL_STOPS);
    count_cogc_sequences(node, &mut s.cogc, BOOL_OPS, BOOL_STOPS);
    shared::check_condition_complexity(node, &mut s.compound_condition_count, COND_KINDS, BOOL_OPS, BOOL_STOPS);
}

fn handle_if(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.track_if(depth);
    s.track_cogc_branch();
    track_condition(node, s);
    let n = node.child_count();
    for i in 0..n {
        let child = node.child(i).unwrap();
        let kind = child.kind();
        if is_body_block(kind) {
            s.cogc_nesting += 1;
            walk_body(child, source, depth + 1, s);
            s.cogc_nesting -= 1;
        } else if kind == "else_if_clause" {
            handle_elseif(child, source, depth + 1, s);
        } else if kind == "else_clause" {
            handle_else(child, source, depth + 1, s);
        }
    }
}

fn handle_elseif(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.cc += 1;
    s.track_cogc_branch();
    track_condition(node, s);
    let n = node.child_count();
    for i in 0..n {
        let child = node.child(i).unwrap();
        if is_body_block(child.kind()) {
            s.cogc_nesting += 1;
            walk_body(child, source, depth, s);
            s.cogc_nesting -= 1;
        }
    }
}

fn handle_else(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let n = node.child_count();
    for i in 0..n {
        let child = node.child(i).unwrap();
        if is_body_block(child.kind()) {
            s.track_cogc_flat();
            s.cogc_nesting += 1;
            walk_body(child, source, depth, s);
            s.cogc_nesting -= 1;
        } else if child.kind() == "if_statement" {
            handle_if(child, source, depth - 1, s);
        }
    }
}

fn handle_loop(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.track_loop(depth);
    s.track_cogc_branch();
    let n = node.child_count();
    for i in 0..n {
        let child = node.child(i).unwrap();
        if is_body_block(child.kind()) {
            s.cogc_nesting += 1;
            walk_body(child, source, depth + 1, s);
            s.cogc_nesting -= 1;
        }
    }
}

fn handle_branching(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let is_switch = node.kind() == "switch_statement";
    let block_kind = if is_switch { "switch_block" } else { "match_block" };
    let case_kind = if is_switch { "case_statement" } else { "match_conditional_expression" };
    s.track_nesting(depth);
    s.track_cogc_branch();
    let Some(block) = find_child_by_kind(node, block_kind) else { return };
    let saved = s.cogc_nesting;
    s.cogc_nesting += 1;
    let n = block.child_count();
    for i in 0..n {
        let child = block.child(i).unwrap();
        if !child.is_named() {
            continue;
        }
        if child.kind() == case_kind {
            s.cc += 1;
        }
        walk_body(child, source, depth + 1, s);
    }
    s.cogc_nesting = saved;
}

fn walk_try(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    if let Some(body) = find_child_by_kind(node, "compound_statement") {
        walk_body(body, source, depth, s);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "catch_clause" {
            handle_catch(child, source, depth, s);
        } else if child.kind() == "finally_clause" {
            if let Some(body) = find_child_by_kind(child, "compound_statement") {
                walk_body(body, source, depth, s);
            }
        }
    }
}

fn handle_catch(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.cc += 1;
    s.track_cogc_branch();
    if is_catch_body_empty(node, "compound_statement", None) {
        s.empty_catch_count += 1;
    }
    if let Some(body) = find_child_by_kind(node, "compound_statement") {
        s.cogc_nesting += 1;
        walk_body(body, source, depth, s);
        s.cogc_nesting -= 1;
    }
}
