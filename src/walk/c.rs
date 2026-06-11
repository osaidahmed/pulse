use tree_sitter::{Node, Tree};

use super::counters::{count_short_variables, count_string_match_arms, max_same_primitive};
use super::shared::{
    self, count_boolean_ops, count_cogc_sequences, BlockWalkCtx, BranchHandlers, BranchKinds, ElseBranchCfg,
    ElseHandlers, GlobalMetricsConfig,
};
use super::{
    compute_assert_fingerprint, compute_skeleton_hash, compute_structural_fingerprint, count_code_lines,
    count_consecutive_asserts, count_distinct_node_kinds, find_child_by_kind, node_text, track_embedded_block,
    FileMetrics, FunctionMetrics, ModuleMetrics, WalkState,
};

const COMMENT_PREFIXES: &[&str] = &["//", "/*", "*"];
const PRIMITIVE_TYPES: &[&str] = &[
    "int", "char", "float", "double", "void", "long", "short", "unsigned", "signed", "size_t", "ssize_t", "int8_t",
    "int16_t", "int32_t", "int64_t", "uint8_t", "uint16_t", "uint32_t", "uint64_t", "bool", "_Bool",
];
const NESTING_BRANCH_KINDS: &[&str] =
    &["if_statement", "for_statement", "while_statement", "do_statement", "switch_statement"];
const BOOL_OPS: &[&str] = &["&&", "||"];
const BOOL_STOPS: &[&str] = &["compound_statement", "function_definition"];
const GLOBAL_CFG: GlobalMetricsConfig = GlobalMetricsConfig {
    cond: &["if_statement"],
    loops: &["for_statement", "while_statement", "do_statement"],
    branches: NESTING_BRANCH_KINDS,
    recurse: &[],
};
const COND_KINDS: &[&str] = &["parenthesized_expression", "binary_expression"];

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
        find_child_by_kind(node, "pointer_declarator").and_then(|p| find_child_by_kind(p, "function_declarator"))
    })?;

    let name = find_child_by_kind(declarator, "identifier")
        .map_or_else(|| "<anonymous>".into(), |n| node_text(n, source).to_string());

    let start_line = node.start_position().row as u32 + 1;
    let end_line = node.end_position().row as u32 + 1;
    let loc = crate::walk::span_code_lines(node, source, COMMENT_PREFIXES);

    let (arg_count, primitive_type_count, typed_param_count, max_same_primitive_count) =
        count_parameters(declarator, source);

    let body = find_child_by_kind(node, "compound_statement")?;
    let cpg = super::cpg_for(body, node, source, &crate::cpg::C);
    let mut s = WalkState::new();
    walk_body(body, source, 0, &mut s);

    let structural_hash = compute_structural_fingerprint(body);
    let distinct_node_kinds = count_distinct_node_kinds(body);
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
        distinct_node_kinds,
        skeleton_hash,
        consecutive_asserts,
        assert_hash,
        primitive_type_count,
        typed_param_count,
        max_same_primitive_count,
        empty_catch_count: 0,
        field_accesses: Vec::new(),
        foreign_field_accesses: Vec::new(),
        class_name: None,
        parent_class: None,
        short_var_count: count_short_variables(body, source, &["declaration"]),
        string_match_arms: count_string_match_arms(
            body,
            "switch_statement",
            "case_statement",
            &["string_literal", "concatenated_string"],
            &[],
        ),
        cpg,
    })
}

fn walk_body(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        walk_node(child, source, depth, s);
    }
}

type NodeHandler = fn(Node, &str, u32, &mut WalkState);

const NODE_HANDLERS: &[(&[&str], NodeHandler)] = &[
    (&["if_statement"], handle_if),
    (&["for_statement", "while_statement", "do_statement"], handle_loop),
    (&["switch_statement"], handle_switch),
    (&["case_statement"], handle_case),
    (&["conditional_expression"], handle_ternary),
];

const STRING_KINDS: &[&str] = &["string_literal", "concatenated_string"];

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

fn handle_if(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.track_if(depth);
    s.track_cogc_branch();
    count_boolean_ops(child, &mut s.cc, BOOL_OPS, BOOL_STOPS);
    count_cogc_sequences(child, &mut s.cogc, BOOL_OPS, BOOL_STOPS);
    shared::check_condition_complexity(child, &mut s.compound_condition_count, COND_KINDS, BOOL_OPS, BOOL_STOPS);
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

fn handle_case(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    let is_default =
        find_child_by_kind(child, "default").is_some() || node_text(child, source).trim_start().starts_with("default");
    if !is_default {
        s.cc += 1;
    }
    walk_body(child, source, depth, s);
}

fn handle_ternary(_child: Node, _source: &str, _depth: u32, s: &mut WalkState) {
    s.cc += 1;
    s.track_cogc_branch();
}

const BRANCH_KINDS: BranchKinds = BranchKinds {
    blocks: &["compound_statement"],
    else_clause: "else_clause",
    catch_clause: None,
    finally_clause: None,
    catch_body_kind: "compound_statement",
};

const ELSE_CFG: ElseBranchCfg = ElseBranchCfg {
    block_kind: "compound_statement",
    if_kind: "if_statement",
    cond_kinds: COND_KINDS,
    bool_ops: BOOL_OPS,
    bool_stops: BOOL_STOPS,
};

const BRANCH_HANDLERS: BranchHandlers = BranchHandlers { kinds: &BRANCH_KINDS, walk_body, walk_else: walk_else_clause };

const ELSE_HANDLERS: ElseHandlers = ElseHandlers { cfg: &ELSE_CFG, walk_body, walk_children };

fn walk_children(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    shared::walk_branches(node, &mut BlockWalkCtx { source, depth, state: s }, &BRANCH_HANDLERS);
}

fn walk_else_clause(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    shared::walk_else_branch(node, &mut BlockWalkCtx { source, depth, state: s }, &ELSE_HANDLERS);
}

fn count_parameters(declarator: Node, source: &str) -> (u32, u32, u32, u32) {
    let Some(params) = find_child_by_kind(declarator, "parameter_list") else {
        return (0, 0, 0, 0);
    };
    let (count, prims, typed_count) = count_param_children(params, source);
    if is_void_param_list(params, count, source) {
        return (0, 0, 0, 0);
    }
    (count, prims.len() as u32, typed_count, max_same_primitive(&prims))
}

fn count_param_children<'a>(params: Node, source: &'a str) -> (u32, Vec<&'a str>, u32) {
    let mut cursor = params.walk();
    let mut count = 0;
    let mut typed = 0;
    let mut prims: Vec<&str> = Vec::new();
    for child in params.children(&mut cursor) {
        match child.kind() {
            "parameter_declaration" => {
                count += 1;
                typed += 1;
                if let Some(ty) = primitive_type_of(child, source) {
                    prims.push(ty);
                }
            }
            "variadic_parameter" => count += 1,
            _ => {}
        }
    }
    (count, prims, typed)
}

fn is_void_param_list(params: Node, count: u32, source: &str) -> bool {
    if count != 1 {
        return false;
    }
    let text = node_text(params, source);
    text.contains("void") && !text.contains("void *") && !text.contains("void*")
}

fn primitive_type_of<'a>(param: Node, source: &'a str) -> Option<&'a str> {
    let mut cursor = param.walk();
    for child in param.children(&mut cursor) {
        if child.kind() == "primitive_type" || child.kind() == "sized_type_specifier" {
            return Some(&source[child.byte_range()]);
        }
        if child.kind() == "type_identifier" {
            let name = &source[child.byte_range()];
            return PRIMITIVE_TYPES.contains(&name).then_some(name);
        }
    }
    None
}

fn count_declarations(root: Node) -> u32 {
    let mut cursor = root.walk();
    root.children(&mut cursor)
        .filter(|c| matches!(c.kind(), "type_definition" | "struct_specifier" | "enum_specifier"))
        .count() as u32
}
