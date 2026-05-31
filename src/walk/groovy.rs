use tree_sitter::{Node, Tree};

use super::counters::{count_short_variables, count_string_match_arms, max_same_primitive};
use super::shared::{self, count_boolean_ops, count_cogc_sequences, GlobalMetricsConfig};
use super::{
    collect_field_accesses_for, collect_foreign_field_accesses_for, compute_assert_fingerprint, compute_skeleton_hash,
    compute_structural_fingerprint, count_code_lines, count_consecutive_asserts,
    count_distinct_node_kinds, find_child_by_kind, is_catch_body_empty, node_text, track_embedded_block, FileMetrics,
    FunctionMetrics, ModuleMetrics, WalkState,
};

const COMMENT_PREFIXES: &[&str] = &["//", "/*", "*"];
const SELF_NAMES: &[&str] = &["this"];
const PRIMITIVE_TYPES: &[&str] = &[
    "int", "long", "short", "byte", "float", "double", "boolean", "char", "void", "String",
];
const NESTING_BRANCH_KINDS: &[&str] = &[
    "if_statement",
    "for_statement",
    "enhanced_for_statement",
    "while_statement",
    "do_statement",
    "switch_expression",
];
const BOOL_OPS: &[&str] = &["&&", "||"];
const BOOL_STOPS: &[&str] = &[
    "block",
    "method_declaration",
    "class_declaration",
    "closure",
    "lambda_expression",
];
const GLOBAL_CFG: GlobalMetricsConfig = GlobalMetricsConfig {
    cond: &["if_statement"],
    loops: &[
        "for_statement",
        "enhanced_for_statement",
        "while_statement",
        "do_statement",
    ],
    branches: NESTING_BRANCH_KINDS,
    recurse: &[],
};
const COND_KINDS: &[&str] = &["parenthesized_expression", "condition"];
const SCOPE_BOUNDARY: &[&str] = &["closure", "lambda_expression"];
const LOOP_KINDS: &[&str] = &[
    "for_statement",
    "enhanced_for_statement",
    "while_statement",
    "do_statement",
];
const TRY_KINDS: &[&str] = &["try_statement", "try_with_resources_statement"];
const EMBEDDED_STR: &[&str] = &["string_literal", "template_expression"];

struct CallableConfig {
    body_kind: &'static str,
    fallback_name: &'static str,
}

const METHOD_CFG: CallableConfig = CallableConfig {
    body_kind: "block",
    fallback_name: "<anonymous>",
};
const CTOR_CFG: CallableConfig = CallableConfig {
    body_kind: "constructor_body",
    fallback_name: "<constructor>",
};
const FUNC_DEF_CFG: CallableConfig = CallableConfig {
    body_kind: "closure",
    fallback_name: "<anonymous>",
};

pub fn walk(tree: &Tree, source: &str) -> FileMetrics {
    let root = tree.root_node();
    let total_loc = count_code_lines(source, COMMENT_PREFIXES);

    let mut functions = Vec::new();
    let mut gcond: u32 = 0;
    let mut gnest: u32 = 0;

    collect_functions(root, source, &mut functions);
    shared::collect_global_metrics(root, &mut gcond, &mut gnest, &GLOBAL_CFG);

    let module = ModuleMetrics {
        total_loc,
        total_functions: functions.len() as u32,
        sum_cc: functions.iter().map(|f| f.cc).sum(),
        global_conditional_count: gcond,
        global_max_nesting: gnest,
        declaration_count: count_declarations(root),
        struct_fields: Vec::new(),
    };

    FileMetrics { functions, module }
}

fn collect_functions(node: Node, source: &str, fns: &mut Vec<FunctionMetrics>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "class_declaration" => collect_class_methods(child, source, fns),
            "method_declaration" => fns.extend(analyze_callable(child, source, &METHOD_CFG)),
            "function_definition" => fns.extend(analyze_callable(child, source, &FUNC_DEF_CFG)),
            _ => collect_functions(child, source, fns),
        }
    }
}

fn collect_class_methods(class_node: Node, source: &str, fns: &mut Vec<FunctionMetrics>) {
    let cls = find_child_by_kind(class_node, "identifier")
        .map(|n| node_text(n, source).to_string())
        .unwrap_or_default();

    let Some(body) = find_child_by_kind(class_node, "class_body") else {
        return;
    };
    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        match child.kind() {
            "method_declaration" => emit_method(child, source, &cls, fns),
            "constructor_declaration" => emit_ctor(child, source, &cls, fns),
            "class_declaration" => collect_class_methods(child, source, fns),
            _ => {}
        }
    }
}

fn emit_method(child: Node, source: &str, cls: &str, fns: &mut Vec<FunctionMetrics>) {
    let Some(mut m) = analyze_callable(child, source, &METHOD_CFG) else { return };
    let method_name = m.name.clone();
    m.name = format!("{cls}.{method_name}");
    m.class_name = Some(cls.to_string());
    collect_field_accesses_for(child, source, SELF_NAMES, &mut m.field_accesses);

    collect_foreign_field_accesses_for(child, source, SELF_NAMES, &mut m.foreign_field_accesses);

    fns.push(m);
}

fn emit_ctor(child: Node, source: &str, cls: &str, fns: &mut Vec<FunctionMetrics>) {
    let Some(mut m) = analyze_callable(child, source, &CTOR_CFG) else { return };
    m.name = format!("{cls}.{cls}");
    m.is_constructor = true;
    m.class_name = Some(cls.to_string());
    fns.push(m);
}

fn analyze_callable(node: Node, source: &str, cfg: &CallableConfig) -> Option<FunctionMetrics> {
    let name = find_child_by_kind(node, "identifier")
        .map_or_else(|| cfg.fallback_name.into(), |n| node_text(n, source).to_string());

    let sl = node.start_position().row as u32 + 1;
    let el = node.end_position().row as u32 + 1;

    let (arg_count, prim, typed, max_same) = count_parameters(node, source);
    let body = find_child_by_kind(node, cfg.body_kind)
        .or_else(|| find_child_by_kind(node, "block"))?;

    let mut s = WalkState::new();
    walk_body(body, source, 0, &mut s);

    Some(FunctionMetrics {
        name,
        start_line: sl,
        end_line: el,
        loc: el.saturating_sub(sl) + 1,
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
        primitive_type_count: prim,
        typed_param_count: typed,
        max_same_primitive_count: max_same,
        empty_catch_count: s.empty_catch_count,
        field_accesses: Vec::new(),
        foreign_field_accesses: Vec::new(),
        class_name: None,
        parent_class: None,
        short_var_count: count_short_variables(body, source, &["local_variable_declaration"]),
        string_match_arms: count_string_match_arms(
            body,
            "switch_expression",
            "switch_block_statement_group",
            &["string_literal"],
            &[],
        ),
    })
}

fn walk_body(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    node.children(&mut cursor).for_each(|child| walk_node(child, source, depth, s));
}

fn walk_node(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    let kind = child.kind();
    if SCOPE_BOUNDARY.contains(&kind) {
        return;
    }
    if EMBEDDED_STR.contains(&kind) {
        track_embedded_block(&mut s.max_embedded_block_loc, child);
        return;
    }
    if LOOP_KINDS.contains(&kind) {
        return handle_loop(child, source, depth, s);
    }
    if TRY_KINDS.contains(&kind) {
        return walk_children(child, source, depth, s);
    }
    walk_node_inner(child, kind, source, depth, s);
}

fn walk_node_inner(child: Node, kind: &str, source: &str, depth: u32, s: &mut WalkState) {
    match kind {
        "if_statement" => handle_if(child, source, depth, s),
        "switch_expression" => handle_switch(child, source, depth, s),
        "switch_block_statement_group" => handle_switch_case(child, source, depth, s),
        "catch_clause" => handle_catch(child, source, depth, s),
        "ternary_expression" => { s.cc += 1; s.track_cogc_branch(); }
        _ => walk_body(child, source, depth, s),
    }
}

fn handle_if(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.track_if(depth);
    s.track_cogc_branch();
    count_boolean_ops(child, &mut s.cc, BOOL_OPS, BOOL_STOPS);
    count_cogc_sequences(child, &mut s.cogc, BOOL_OPS, BOOL_STOPS);
    shared::check_condition_complexity(
        child,
        &mut s.compound_condition_count,
        COND_KINDS,
        BOOL_OPS,
        BOOL_STOPS,
    );
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

fn handle_switch_case(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    let label_text = find_child_by_kind(child, "switch_label")
        .map(|l| node_text(l, source))
        .unwrap_or_default();
    if !label_text.contains("default") {
        s.cc += 1;
    }
    walk_body(child, source, depth, s);
}

fn handle_catch(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.cc += 1;
    s.track_cogc_branch();
    if is_catch_body_empty(child, "block", None) {
        s.empty_catch_count += 1;
    }
    shared::walk_block_children(
        child,
        &mut shared::BlockWalkCtx { source, depth, state: s },
        "block",
        walk_body,
    );
}

const BLOCK_KINDS: &[&str] = &[
    "block", "switch_block", "constructor_body", "closure", "expression_statement",
];

fn walk_children(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    let mut saw_else = false;
    for child in node.children(&mut cursor) {
        let kind = child.kind();
        if BLOCK_KINDS.contains(&kind) {
            walk_groovy_consequence(child, source, depth, saw_else, s);
            saw_else = false;
        } else {
            match kind {
                "else" => saw_else = true,
                "if_statement" => {
                    handle_elif(child, source, depth, s);
                    saw_else = false;
                }
                "catch_clause" => handle_catch(child, source, depth, s),
                "finally_clause" => shared::walk_block_children(
                    child,
                    &mut shared::BlockWalkCtx { source, depth, state: s },
                    "block",
                    walk_body,
                ),
                _ => {}
            }
        }
    }
}

fn walk_groovy_consequence(node: Node, source: &str, depth: u32, saw_else: bool, s: &mut WalkState) {
    if saw_else {
        s.track_cogc_flat();
    }
    let saved = s.cogc_nesting;
    s.cogc_nesting += 1;
    let target = if node.kind() == "expression_statement" {
        find_child_by_kind(node, "closure")
            .or_else(|| find_child_by_kind(node, "block"))
            .unwrap_or(node)
    } else {
        node
    };
    walk_body(target, source, depth, s);
    s.cogc_nesting = saved;
}

fn handle_elif(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.cc += 1;
    s.track_cogc_branch();
    count_boolean_ops(child, &mut s.cc, BOOL_OPS, BOOL_STOPS);
    count_cogc_sequences(child, &mut s.cogc, BOOL_OPS, BOOL_STOPS);
    shared::check_condition_complexity(
        child,
        &mut s.compound_condition_count,
        COND_KINDS,
        BOOL_OPS,
        BOOL_STOPS,
    );
    walk_children(child, source, depth, s);
}

fn count_parameters(node: Node, source: &str) -> (u32, u32, u32, u32) {
    let Some(params) = find_child_by_kind(node, "formal_parameters") else {
        return (0, 0, 0, 0);
    };
    let mut cursor = params.walk();
    let mut count = 0;
    let mut typed = 0;
    let mut prims: Vec<&str> = Vec::new();
    for child in params
        .children(&mut cursor)
        .filter(|c| c.kind() == "formal_parameter" || c.kind() == "spread_parameter")
    {
        count += 1;
        let (has_type, prim_ty) = classify_param(child, source);
        typed += u32::from(has_type);
        if let Some(ty) = prim_ty {
            prims.push(ty);
        }
    }
    (count, prims.len() as u32, typed, max_same_primitive(&prims))
}

fn classify_param<'a>(param: Node, source: &'a str) -> (bool, Option<&'a str>) {
    let mut cursor = param.walk();
    for child in param.children(&mut cursor) {
        match child.kind() {
            "integral_type" | "floating_point_type" | "boolean_type" | "void_type" => {
                return (true, Some(&source[child.byte_range()]));
            }
            "type_identifier" => {
                let name = &source[child.byte_range()];
                return (true, PRIMITIVE_TYPES.contains(&name).then_some(name));
            }
            "generic_type" | "array_type" | "scoped_type_identifier" => return (true, None),
            _ => {}
        }
    }
    (false, None)
}

fn count_declarations(root: Node) -> u32 {
    let mut cursor = root.walk();
    root.children(&mut cursor)
        .filter(|c| {
            matches!(
                c.kind(),
                "class_declaration" | "interface_declaration" | "enum_declaration"
            )
        })
        .count() as u32
}
