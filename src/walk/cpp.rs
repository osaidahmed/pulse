use tree_sitter::{Node, Tree};

use super::counters::{count_short_variables, count_string_match_arms, max_same_primitive};
use super::shared::{
    self, count_boolean_ops, count_cogc_sequences, BlockWalkCtx, BranchHandlers, BranchKinds, ElseBranchCfg,
    ElseHandlers, GlobalMetricsConfig,
};
use super::{
    collect_field_accesses_for, collect_foreign_field_accesses_for, compute_assert_fingerprint, compute_skeleton_hash,
    compute_structural_fingerprint, count_code_lines, count_consecutive_asserts, count_distinct_node_kinds,
    find_child_by_kind, is_catch_body_empty, node_text, track_embedded_block, FileMetrics, FunctionMetrics,
    ModuleMetrics, WalkState,
};

const COMMENT_PREFIXES: &[&str] = &["//", "/*", "*"];
const SELF_NAMES: &[&str] = &["this"];
const PRIMITIVE_TYPES: &[&str] = &[
    "int", "char", "float", "double", "void", "long", "short", "unsigned", "signed", "bool", "size_t", "ssize_t",
    "int8_t", "int16_t", "int32_t", "int64_t", "uint8_t", "uint16_t", "uint32_t", "uint64_t", "auto", "string",
];
const NESTING_BRANCH_KINDS: &[&str] =
    &["if_statement", "for_statement", "for_range_loop", "while_statement", "do_statement", "switch_statement"];
const BOOL_OPS: &[&str] = &["&&", "||"];
const BOOL_STOPS: &[&str] = &["compound_statement", "function_definition", "lambda_expression"];
const GLOBAL_CFG: GlobalMetricsConfig = GlobalMetricsConfig {
    cond: &["if_statement"],
    loops: &["for_statement", "while_statement", "do_statement"],
    branches: NESTING_BRANCH_KINDS,
    recurse: &[],
};
const COND_KINDS: &[&str] = &["parenthesized_expression", "condition_clause"];

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
        match child.kind() {
            "function_definition" => extend_with_metrics(child, source, functions),
            "class_specifier" | "struct_specifier" => collect_class_methods(child, source, functions),
            "namespace_definition" => recurse_namespace(child, source, functions),
            _ => {}
        }
    }
}

fn extend_with_metrics(node: Node, source: &str, functions: &mut Vec<FunctionMetrics>) {
    if let Some(m) = analyze_function(node, source) {
        functions.push(m);
    }
}

fn recurse_namespace(node: Node, source: &str, functions: &mut Vec<FunctionMetrics>) {
    let Some(body) = find_child_by_kind(node, "declaration_list") else {
        return;
    };
    collect_functions(body, source, functions);
}

fn collect_class_methods(class_node: Node, source: &str, functions: &mut Vec<FunctionMetrics>) {
    let class_name = find_child_by_kind(class_node, "type_identifier")
        .or_else(|| find_child_by_kind(class_node, "name"))
        .map(|n| node_text(n, source).to_string())
        .unwrap_or_default();

    let Some(body) = find_child_by_kind(class_node, "field_declaration_list") else {
        return;
    };

    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        if child.kind() != "function_definition" {
            continue;
        }
        let Some(mut metrics) = analyze_function(child, source) else {
            continue;
        };
        let method_name = metrics.name.clone();
        metrics.name = format!("{class_name}::{method_name}");
        metrics.class_name = Some(class_name.clone());
        metrics.is_constructor = method_name == class_name;
        collect_field_accesses_for(child, source, SELF_NAMES, &mut metrics.field_accesses);

        collect_foreign_field_accesses_for(child, source, SELF_NAMES, &mut metrics.foreign_field_accesses);

        functions.push(metrics);
    }
}

fn analyze_function(node: Node, source: &str) -> Option<FunctionMetrics> {
    let name = extract_function_name(node, source);

    let start_line = node.start_position().row as u32 + 1;
    let end_line = node.end_position().row as u32 + 1;
    let loc = end_line.saturating_sub(start_line) + 1;

    let (arg_count, primitive_type_count, typed_param_count, max_same_primitive_count) = count_parameters(node, source);

    let body = find_child_by_kind(node, "compound_statement")?;
    let cpg = super::cpg_for(body, node, source, &crate::cpg::CPP);
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
        empty_catch_count: s.empty_catch_count,
        field_accesses: Vec::new(),
        foreign_field_accesses: Vec::new(),
        class_name: None,
        parent_class: None,
        short_var_count: count_short_variables(body, source, &["declaration"]),
        string_match_arms: count_string_match_arms(
            body,
            "switch_statement",
            "case_statement",
            &["string_literal", "raw_string_literal", "concatenated_string"],
            &[],
        ),
        cpg,
    })
}

const NAME_KINDS: &[&str] = &["identifier", "field_identifier", "qualified_identifier", "destructor_name"];

fn extract_function_name(node: Node, source: &str) -> String {
    let decl = find_child_by_kind(node, "function_declarator").or_else(|| {
        find_child_by_kind(node, "pointer_declarator").and_then(|p| find_child_by_kind(p, "function_declarator"))
    });
    decl.and_then(|d| find_name_in(d, source)).unwrap_or_else(|| "<anonymous>".into())
}

fn find_name_in(decl: Node, source: &str) -> Option<String> {
    let mut cursor = decl.walk();
    for child in decl.children(&mut cursor) {
        if NAME_KINDS.contains(&child.kind()) {
            return Some(node_text(child, source).to_string());
        }
    }
    None
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
    (&["for_statement", "for_range_loop", "while_statement", "do_statement"], handle_loop),
    (&["switch_statement"], handle_switch),
    (&["case_statement"], handle_case),
    (&["catch_clause"], handle_catch),
    (&["try_statement"], walk_children),
];

const STRING_KINDS: &[&str] = &["string_literal", "raw_string_literal", "concatenated_string"];

fn walk_node(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    let kind = child.kind();
    if STRING_KINDS.contains(&kind) {
        track_embedded_block(&mut s.max_embedded_block_loc, child);
        return;
    }
    if kind == "lambda_expression" {
        return;
    }
    if kind == "conditional_expression" {
        s.cc += 1;
        s.track_cogc_branch();
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
    if !node_text(child, source).trim_start().starts_with("default") {
        s.cc += 1;
    }
    walk_body(child, source, depth, s);
}

fn handle_catch(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.cc += 1;
    s.track_cogc_branch();
    if is_catch_body_empty(child, "compound_statement", None) {
        s.empty_catch_count += 1;
    }
    walk_children(child, source, depth, s);
}

const BRANCH_KINDS: BranchKinds = BranchKinds {
    blocks: &["compound_statement"],
    else_clause: "else_clause",
    catch_clause: Some("catch_clause"),
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

fn count_parameters(func_node: Node, source: &str) -> (u32, u32, u32, u32) {
    let Some(declarator) = find_child_by_kind(func_node, "function_declarator").or_else(|| {
        find_child_by_kind(func_node, "pointer_declarator").and_then(|p| find_child_by_kind(p, "function_declarator"))
    }) else {
        return (0, 0, 0, 0);
    };
    let Some(params) = find_child_by_kind(declarator, "parameter_list") else {
        return (0, 0, 0, 0);
    };
    let mut cursor = params.walk();
    let mut count = 0;
    let mut typed = 0;
    let mut prims: Vec<&str> = Vec::new();
    for child in params.children(&mut cursor) {
        match child.kind() {
            "parameter_declaration" | "optional_parameter_declaration" => {
                count += 1;
                typed += 1;
                if let Some(ty) = primitive_type_of(child, source) {
                    prims.push(ty);
                }
            }
            "variadic_parameter_declaration" | "variadic_parameter" => count += 1,
            _ => {}
        }
    }
    let text = node_text(params, source);
    let is_void = count == 1 && text.contains("void") && !text.contains("void *") && !text.contains("void*");
    if is_void {
        (0, 0, 0, 0)
    } else {
        (count, prims.len() as u32, typed, max_same_primitive(&prims))
    }
}

fn primitive_type_of<'a>(param: Node, source: &'a str) -> Option<&'a str> {
    let mut cursor = param.walk();
    for child in param.children(&mut cursor) {
        match child.kind() {
            "primitive_type" | "sized_type_specifier" => return Some(&source[child.byte_range()]),
            "type_identifier" => {
                let name = &source[child.byte_range()];
                return PRIMITIVE_TYPES.contains(&name).then_some(name);
            }
            _ => {}
        }
    }
    None
}

fn count_declarations(root: Node) -> u32 {
    let mut count: u32 = 0;
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        match child.kind() {
            "class_specifier" | "struct_specifier" | "enum_specifier" | "type_definition" => {
                count += 1;
            }
            _ => {}
        }
    }
    count
}
