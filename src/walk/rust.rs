use tree_sitter::{Node, Tree};

use super::counters::{count_short_variables, count_string_match_arms, max_same_primitive};
use super::shared::{
    self, count_boolean_ops, count_cogc_sequences, BlockWalkCtx, BranchHandlers, BranchKinds,
    ElseBranchCfg, ElseHandlers, GlobalMetricsConfig,
};
use super::{
    collect_field_accesses_for, collect_foreign_field_accesses_for, compute_assert_fingerprint, compute_skeleton_hash,
    compute_structural_fingerprint, count_code_lines, count_consecutive_asserts,
    count_distinct_node_kinds,
    find_child_by_kind, node_text, FileMetrics, FunctionMetrics,
    ModuleMetrics, WalkState, track_embedded_block,
};

const COMMENT_PREFIXES: &[&str] = &["//", "/*", "*"];
const SELF_NAMES: &[&str] = &["self"];
const PRIMITIVE_TYPES: &[&str] = &[
    "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize", "f32",
    "f64", "bool", "char", "str", "String",
];
const NESTING_BRANCH_KINDS: &[&str] = &[
    "if_expression",
    "for_expression",
    "while_expression",
    "loop_expression",
    "match_expression",
];
const BOOL_OPS: &[&str] = &["&&", "||"];
const BOOL_STOPS: &[&str] = &["block", "function_item", "closure_expression"];
const GLOBAL_CFG: GlobalMetricsConfig = GlobalMetricsConfig {
    cond: &["if_expression"],
    loops: &["for_expression", "while_expression", "loop_expression"],
    branches: NESTING_BRANCH_KINDS,
    recurse: &[],
};
const COND_KINDS: &[&str] = &["binary_expression", "parenthesized_expression"];

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
    let (declaration_count, struct_fields) = count_declarations_and_fields(root, source);

    let module = ModuleMetrics {
        total_loc,
        total_functions,
        sum_cc,
        global_conditional_count,
        global_max_nesting,
        declaration_count,
        struct_fields,
    };

    FileMetrics { functions, module }
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
    let type_name = extract_impl_type_name(impl_node, source);
    let parent_class = extract_trait_for_impl(impl_node, source);

    let Some(body) = find_child_by_kind(impl_node, "declaration_list") else {
        return;
    };

    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        if child.kind() != "function_item" {
            continue;
        }
        let Some(mut metrics) = analyze_function(child, source) else {
            continue;
        };
        let method_name = metrics.name.clone();
        metrics.name = format!("{type_name}.{method_name}");
        metrics.is_constructor = method_name == "new";
        metrics.class_name = Some(type_name.clone());
        metrics.parent_class = parent_class.clone();
        if super::extras_enabled(child.start_byte(), child.end_byte()) {
            collect_field_accesses_for(child, source, SELF_NAMES, &mut metrics.field_accesses);
            collect_foreign_field_accesses_for(child, source, SELF_NAMES, &mut metrics.foreign_field_accesses);
        }

        if has_self_param(child) && metrics.arg_count > 0 {
            metrics.arg_count -= 1;
        }
        functions.push(metrics);
    }
}

fn extract_trait_for_impl(impl_node: Node, source: &str) -> Option<String> {
    let trait_node = impl_node.child_by_field_name("trait")?;
    Some(node_text(trait_node, source).to_string())
}

fn extract_impl_type_name(impl_node: Node, source: &str) -> String {
    if let Some(type_node) = impl_node.child_by_field_name("type") {
        return node_text(type_node, source).to_string();
    }
    find_child_by_kind(impl_node, "type_identifier")
        .map(|n| node_text(n, source).to_string())
        .unwrap_or_default()
}

fn analyze_function(node: Node, source: &str) -> Option<FunctionMetrics> {
    let name = find_child_by_kind(node, "identifier")
        .map_or_else(|| "<anonymous>".into(), |n| node_text(n, source).to_string());

    let start_line = node.start_position().row as u32 + 1;
    let end_line = node.end_position().row as u32 + 1;
    let loc = end_line.saturating_sub(start_line) + 1;

    let (arg_count, primitive_type_count, typed_param_count, max_same_primitive_count) =
        count_parameters(node, source);

    let body = find_child_by_kind(node, "block")?;
    let mut s = WalkState::new();
    walk_body(body, source, 0, &mut s);

    let mut structural_hash = 0;
    let mut distinct_node_kinds = 0;
    let mut skeleton_hash = 0;
    let mut consecutive_asserts = 0;
    let mut assert_hash = 0;
    let mut short_var_count = 0;
    let mut string_match_arms = 0;
    if super::extras_enabled(node.start_byte(), node.end_byte()) {
        structural_hash = compute_structural_fingerprint(body);
        distinct_node_kinds = count_distinct_node_kinds(body);
        skeleton_hash = compute_skeleton_hash(body);
        consecutive_asserts = count_consecutive_asserts(body, "expression_statement");
        assert_hash = compute_assert_fingerprint(body, "expression_statement");
        short_var_count = count_short_variables(body, source, &["let_declaration"]);
        string_match_arms = count_string_match_arms(body, "match_expression", "match_arm", &["string_literal", "raw_string_literal"], &[]);
    }
    let cpg = super::cpg_for(body, node, &crate::cpg::RUST);

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
        short_var_count,
        string_match_arms,
        cpg,
    })
}

fn walk_body(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "if_expression" => {
                s.track_if(depth);
                s.track_cogc_branch();
                count_boolean_ops(child, &mut s.cc, BOOL_OPS, BOOL_STOPS);
                count_cogc_sequences(child, &mut s.cogc, BOOL_OPS, BOOL_STOPS);
                shared::check_condition_complexity(child, &mut s.compound_condition_count, COND_KINDS, BOOL_OPS, BOOL_STOPS);
                walk_children(child, source, depth + 1, s);
            }
            "for_expression" | "while_expression" | "loop_expression" => {
                s.track_loop(depth);
                s.track_cogc_branch();
                walk_children(child, source, depth + 1, s);
            }
            "match_expression" => {
                s.track_nesting(depth);
                s.track_cogc_branch();
                let saved = s.cogc_nesting;
                s.cogc_nesting += 1;
                walk_match_arms(child, source, depth + 1, s);
                s.cogc_nesting = saved;
            }
            "closure_expression" => {}
            "string_literal" | "raw_string_literal" => track_embedded_block(&mut s.max_embedded_block_loc, child),
            _ => walk_body(child, source, depth, s),
        }
    }
}

fn walk_match_arms(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let Some(body) = find_child_by_kind(node, "match_block") else {
        return;
    };
    let mut arm_cursor = body.walk();
    for arm in body.children(&mut arm_cursor) {
        if arm.kind() != "match_arm" {
            continue;
        }
        let is_wildcard = find_child_by_kind(arm, "match_pattern").is_some_and(|p| {
            let mut pc = p.walk();
            // cursor must outlive the iterator — binding extends the borrow
            let result = p.children(&mut pc).any(|c| c.kind() == "_");
            result
        });
        if !is_wildcard {
            s.cc += 1;
        }
        walk_children(arm, source, depth, s);
    }
}

const BRANCH_KINDS: BranchKinds = BranchKinds {
    blocks: &["block"],
    else_clause: "else_clause",
    catch_clause: None,
    finally_clause: None,
    catch_body_kind: "block",
};

const ELSE_CFG: ElseBranchCfg = ElseBranchCfg {
    block_kind: "block",
    if_kind: "if_expression",
    cond_kinds: COND_KINDS,
    bool_ops: BOOL_OPS,
    bool_stops: BOOL_STOPS,
};

const BRANCH_HANDLERS: BranchHandlers = BranchHandlers {
    kinds: &BRANCH_KINDS,
    walk_body,
    walk_else: walk_else_clause,
};

const ELSE_HANDLERS: ElseHandlers = ElseHandlers {
    cfg: &ELSE_CFG,
    walk_body,
    walk_children,
};

fn walk_children(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    shared::walk_branches(node, &mut BlockWalkCtx { source, depth, state: s }, &BRANCH_HANDLERS);
}

fn walk_else_clause(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    shared::walk_else_branch(node, &mut BlockWalkCtx { source, depth, state: s }, &ELSE_HANDLERS);
}

fn count_parameters(func_node: Node, source: &str) -> (u32, u32, u32, u32) {
    let Some(params) = find_child_by_kind(func_node, "parameters") else {
        return (0, 0, 0, 0);
    };
    let mut cursor = params.walk();
    let mut arg_count = 0;
    let mut typed = 0;
    let mut prims: Vec<&str> = Vec::new();
    for child in params.children(&mut cursor) {
        match child.kind() {
            "parameter" => {
                arg_count += 1;
                typed += 1;
                if let Some(ty) = primitive_type_of(child, source) {
                    prims.push(ty);
                }
            }
            "self_parameter" => arg_count += 1,
            _ => {}
        }
    }
    (arg_count, prims.len() as u32, typed, max_same_primitive(&prims))
}

fn primitive_type_of<'a>(param_node: Node, source: &'a str) -> Option<&'a str> {
    let type_node = find_type_leaf(param_node)
        .or_else(|| find_child_by_kind(param_node, "reference_type").and_then(find_type_leaf))?;
    let name = &source[type_node.byte_range()];
    PRIMITIVE_TYPES.contains(&name).then_some(name)
}

fn find_type_leaf(node: Node) -> Option<Node> {
    let mut cursor = node.walk();
    // cursor must outlive the iterator — binding extends the borrow
    let result = node
        .children(&mut cursor)
        .find(|c| c.kind() == "type_identifier" || c.kind() == "primitive_type");
    result
}

fn has_self_param(func_node: Node) -> bool {
    let Some(params) = find_child_by_kind(func_node, "parameters") else {
        return false;
    };
    let mut cursor = params.walk();
    // cursor must outlive the iterator — binding extends the borrow
    let result = params
        .children(&mut cursor)
        .any(|c| c.kind() == "self_parameter");
    result
}

fn count_declarations_and_fields(root: Node, source: &str) -> (u32, Vec<(String, u32)>) {
    let mut decl_count: u32 = 0;
    let mut struct_fields = Vec::new();
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        match child.kind() {
            "struct_item" => {
                decl_count += 1;
                let name = find_child_by_kind(child, "type_identifier")
                    .map_or_else(|| "<anon>".into(), |n| node_text(n, source).to_string());
                let fields = find_child_by_kind(child, "field_declaration_list").map_or(0, |fdl| {
                    let mut c = fdl.walk();
                    fdl.children(&mut c).filter(|n| n.kind() == "field_declaration").count() as u32
                });
                if fields > 0 { struct_fields.push((name, fields)); }
            }
            "enum_item" | "trait_item" | "type_item" => { decl_count += 1; }
            _ => {}
        }
    }
    (decl_count, struct_fields)
}
