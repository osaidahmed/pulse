use tree_sitter::{Node, Tree};

use super::counters::{count_short_variables, count_string_match_arms, max_same_primitive};
use super::shared::{self, count_boolean_ops, count_cogc_sequences, GlobalMetricsConfig};
use super::{
    collect_field_accesses_for, collect_foreign_field_accesses_for, compute_assert_fingerprint, compute_skeleton_hash,
    compute_structural_fingerprint, count_code_lines, count_consecutive_asserts,
    find_child_by_kind, is_catch_body_empty, node_text, track_embedded_block, FileMetrics,
    FunctionMetrics, ModuleMetrics, WalkState,
};

const COMMENT_PREFIXES: &[&str] = &["//", "/*", "*"];
const SELF_NAMES: &[&str] = &["self"];
const PRIMITIVE_TYPES: &[&str] = &[
    "int", "char", "float", "double", "void", "long", "short", "unsigned", "signed", "bool",
    "size_t", "ssize_t", "int8_t", "int16_t", "int32_t", "int64_t", "uint8_t", "uint16_t",
    "uint32_t", "uint64_t", "BOOL", "NSInteger", "NSUInteger", "CGFloat", "NSTimeInterval",
];
const NESTING_BRANCH_KINDS: &[&str] = &[
    "if_statement",
    "for_statement",
    "while_statement",
    "do_statement",
    "switch_statement",
];
const BOOL_OPS: &[&str] = &["&&", "||"];
const BOOL_STOPS: &[&str] = &["compound_statement", "function_definition", "method_definition"];
const GLOBAL_CFG: GlobalMetricsConfig = GlobalMetricsConfig {
    cond: &["if_statement"],
    loops: &["for_statement", "while_statement", "do_statement"],
    branches: NESTING_BRANCH_KINDS,
    recurse: &[],
};
const COND_KINDS: &[&str] = &["parenthesized_expression"];

pub fn walk(tree: &Tree, source: &str) -> FileMetrics {
    let root = tree.root_node();
    let total_loc = count_code_lines(source, COMMENT_PREFIXES);
    let mut functions = Vec::new();
    let mut gcc: u32 = 0;
    let mut gmn: u32 = 0;

    collect_functions(root, source, &mut functions);
    shared::collect_global_metrics(root, &mut gcc, &mut gmn, &GLOBAL_CFG);

    let module = ModuleMetrics {
        total_loc,
        total_functions: functions.len() as u32,
        sum_cc: functions.iter().map(|f| f.cc).sum(),
        global_conditional_count: gcc,
        global_max_nesting: gmn,
        declaration_count: count_declarations(root),
        struct_fields: Vec::new(),
    };
    FileMetrics { functions, module }
}

// ─── Function collection ────────────────────────────────────────────────

fn collect_functions(node: Node, source: &str, out: &mut Vec<FunctionMetrics>) {
    let mut cursor = node.walk();
    node.children(&mut cursor).for_each(|child| {
        if child.kind() == "class_implementation" {
            collect_class_methods(child, source, out);
        } else if child.kind() == "function_definition" {
            out.extend(analyze_c_function(child, source));
        }
    });
}

fn collect_class_methods(class_node: Node, source: &str, out: &mut Vec<FunctionMetrics>) {
    let class_name = find_child_by_kind(class_node, "identifier")
        .map(|n| node_text(n, source).to_string())
        .unwrap_or_default();

    let mut cursor = class_node.walk();
    for child in class_node.children(&mut cursor) {
        if child.kind() != "implementation_definition" {
            continue;
        }
        let Some(method) = find_child_by_kind(child, "method_definition") else {
            continue;
        };
        let name = find_child_by_kind(method, "identifier")
            .map_or_else(|| "<anonymous>".into(), |n| node_text(n, source).to_string());
        let params = count_method_parameters(method, source);
        let Some(mut m) = build_metrics(method, source, name.clone(), params) else {
            continue;
        };
        m.name = format!("{class_name}.{name}");
        m.class_name = Some(class_name.clone());
        m.is_constructor = name == "init" || name.starts_with("initW");
        collect_field_accesses_for(method, source, SELF_NAMES, &mut m.field_accesses);

        collect_foreign_field_accesses_for(method, source, SELF_NAMES, &mut m.foreign_field_accesses);

        out.push(m);
    }
}

fn analyze_c_function(node: Node, source: &str) -> Option<FunctionMetrics> {
    let name = find_c_declarator(node)
        .and_then(|d| find_child_by_kind(d, "identifier"))
        .map_or_else(|| "<anonymous>".into(), |n| node_text(n, source).to_string());
    build_metrics(node, source, name, count_c_parameters(node, source))
}

struct ParamCounts {
    total: u32,
    primitive: u32,
    typed: u32,
    max_same: u32,
}

fn build_metrics(node: Node, source: &str, name: String, p: ParamCounts) -> Option<FunctionMetrics> {
    let start_line = node.start_position().row as u32 + 1;
    let end_line = node.end_position().row as u32 + 1;
    let body = find_child_by_kind(node, "compound_statement")?;
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
        arg_count: p.total,
        compound_condition_count: s.compound_condition_count,
        is_constructor: false,
        max_embedded_block_loc: s.max_embedded_block_loc,
        structural_hash: compute_structural_fingerprint(body),
        skeleton_hash: compute_skeleton_hash(body),
        consecutive_asserts: count_consecutive_asserts(body, "expression_statement"),
        assert_hash: compute_assert_fingerprint(body, "expression_statement"),
        primitive_type_count: p.primitive,
        typed_param_count: p.typed,
        max_same_primitive_count: p.max_same,
        empty_catch_count: s.empty_catch_count,
        field_accesses: Vec::new(),
        foreign_field_accesses: Vec::new(),
        class_name: None,
        parent_class: None,
        short_var_count: count_short_variables(body, source, &["declaration"]),
        string_match_arms: count_string_match_arms(body, "switch_statement", "case_statement", &["string_literal", "concatenated_string"], &[]),
    })
}

// ─── Parameter extraction ───────────────────────────────────────────────

fn find_c_declarator(node: Node) -> Option<Node> {
    find_child_by_kind(node, "function_declarator").or_else(|| {
        find_child_by_kind(node, "pointer_declarator")
            .and_then(|p| find_child_by_kind(p, "function_declarator"))
    })
}

fn count_method_parameters(node: Node, source: &str) -> ParamCounts {
    let mut cursor = node.walk();
    let mut total = 0;
    let mut typed = 0;
    let mut prims: Vec<&str> = Vec::new();
    for child in node.children(&mut cursor).filter(|c| c.kind() == "method_parameter") {
        total += 1;
        typed += 1;
        if let Some(ty) =
            find_child_by_kind(child, "method_type").and_then(|mt| objc_primitive_type(mt, source))
        {
            prims.push(ty);
        }
    }
    ParamCounts { total, primitive: prims.len() as u32, typed, max_same: max_same_primitive(&prims) }
}

fn count_c_parameters(func_node: Node, source: &str) -> ParamCounts {
    let Some(params) = find_c_declarator(func_node)
        .and_then(|d| find_child_by_kind(d, "parameter_list"))
    else {
        return ParamCounts { total: 0, primitive: 0, typed: 0, max_same: 0 };
    };
    let mut cursor = params.walk();
    let mut count = 0;
    let mut typed = 0;
    let mut prims: Vec<&str> = Vec::new();
    for child in params.children(&mut cursor) {
        match child.kind() {
            "parameter_declaration" => {
                count += 1;
                typed += 1;
                if let Some(ty) = objc_primitive_type(child, source) {
                    prims.push(ty);
                }
            }
            "variadic_parameter" => count += 1,
            _ => {}
        }
    }
    let collapsed: String = node_text(params, source).split_whitespace().collect();
    if count == 1 && collapsed.contains("void") && !collapsed.contains('*') {
        return ParamCounts { total: 0, primitive: 0, typed: 0, max_same: 0 };
    }
    ParamCounts { total: count, primitive: prims.len() as u32, typed, max_same: max_same_primitive(&prims) }
}

fn objc_primitive_type<'a>(node: Node, source: &'a str) -> Option<&'a str> {
    if let Some(direct) = find_child_by_kind(node, "primitive_type")
        .or_else(|| find_child_by_kind(node, "sized_type_specifier"))
    {
        return Some(node_text(direct, source));
    }
    let named = find_child_by_kind(node, "type_identifier")
        .or_else(|| find_child_by_kind(node, "typedefed_specifier"))
        .map(|n| node_text(n, source))
        .filter(|name| PRIMITIVE_TYPES.contains(name));
    if let Some(name) = named {
        return Some(name);
    }
    find_child_by_kind(node, "type_name").and_then(|tn| objc_primitive_type(tn, source))
}

// ─── Body walking ───────────────────────────────────────────────────────

fn walk_body(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        dispatch(child, source, depth, s);
    }
}

type NodeHandler = fn(Node, &str, u32, &mut WalkState);

const NODE_HANDLERS: &[(&[&str], NodeHandler)] = &[
    (&["if_statement"], handle_if),
    (&["for_statement", "while_statement", "do_statement"], handle_loop),
    (&["switch_statement"], handle_switch),
    (&["case_statement"], handle_case),
    (&["try_statement"], descend),
    (&["catch_clause"], handle_catch),
    (&["conditional_expression"], handle_ternary),
];

const STRING_KINDS: &[&str] = &["string_literal", "concatenated_string"];

fn dispatch(child: Node, source: &str, depth: u32, s: &mut WalkState) {
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

fn handle_loop(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.track_loop(depth);
    s.track_cogc_branch();
    descend(node, source, depth + 1, s);
}

fn handle_switch(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.track_nesting(depth);
    s.track_cogc_branch();
    descend(node, source, depth + 1, s);
}

fn handle_ternary(_node: Node, _source: &str, _depth: u32, s: &mut WalkState) {
    s.cc += 1;
    s.track_cogc_branch();
}

fn handle_if(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.track_if(depth);
    s.track_cogc_branch();
    count_boolean_ops(node, &mut s.cc, BOOL_OPS, BOOL_STOPS);
    count_cogc_sequences(node, &mut s.cogc, BOOL_OPS, BOOL_STOPS);
    shared::check_condition_complexity(node, &mut s.compound_condition_count, COND_KINDS, BOOL_OPS, BOOL_STOPS);
    descend(node, source, depth + 1, s);
}

fn handle_case(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    if !node_text(node, source).trim_start().starts_with("default") {
        s.cc += 1;
    }
    walk_body(node, source, depth, s);
}

fn handle_catch(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.cc += 1;
    s.track_cogc_branch();
    if is_catch_body_empty(node, "compound_statement", None) {
        s.empty_catch_count += 1;
    }
    descend(node, source, depth, s);
}

fn descend(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let kind = child.kind();
        if kind == "compound_statement" {
            let saved = s.cogc_nesting;
            s.cogc_nesting += 1;
            walk_body(child, source, depth, s);
            s.cogc_nesting = saved;
        } else if kind == "else_clause" {
            descend_else(child, source, depth, s);
        } else if kind == "catch_clause" {
            handle_catch(child, source, depth, s);
        }
    }
}

fn descend_else(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    if let Some(inner_if) = find_child_by_kind(node, "if_statement") {
        s.cc += 1;
        s.track_cogc_branch();
        count_boolean_ops(inner_if, &mut s.cc, BOOL_OPS, BOOL_STOPS);
        count_cogc_sequences(inner_if, &mut s.cogc, BOOL_OPS, BOOL_STOPS);
        shared::check_condition_complexity(inner_if, &mut s.compound_condition_count, COND_KINDS, BOOL_OPS, BOOL_STOPS);
        descend(inner_if, source, depth, s);
    } else if let Some(body) = find_child_by_kind(node, "compound_statement") {
        s.track_cogc_flat();
        let saved = s.cogc_nesting;
        s.cogc_nesting += 1;
        walk_body(body, source, depth, s);
        s.cogc_nesting = saved;
    }
}

fn count_declarations(root: Node) -> u32 {
    let mut cursor = root.walk();
    root.children(&mut cursor)
        .filter(|c| matches!(c.kind(), "class_implementation" | "protocol_declaration"))
        .count() as u32
}
