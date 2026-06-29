use tree_sitter::{Node, Tree};

use super::{
    collect_field_accesses_for, collect_foreign_field_accesses_for, compute_assert_fingerprint, compute_skeleton_hash,
    compute_structural_fingerprint, count_code_lines, count_consecutive_asserts, count_distinct_node_kinds,
    find_child_by_kind, node_text, track_embedded_block, FileMetrics, FunctionMetrics, ModuleMetrics, WalkState,
};
const COND_KINDS: &[&str] = &["binary_expression", "parenthesized_expression"];

use super::counters::{count_short_variables, count_string_match_arms, max_same_primitive};
use super::shared::{self, count_boolean_ops, count_cogc_sequences, GlobalMetricsConfig};

const COMMENT_PREFIXES: &[&str] = &["//", "/*", "*"];
const _SELF_NAMES: &[&str] = &[];
const PRIMITIVE_TYPES: &[&str] = &[
    "int",
    "int8",
    "int16",
    "int32",
    "int64",
    "uint",
    "uint8",
    "uint16",
    "uint32",
    "uint64",
    "uintptr",
    "float32",
    "float64",
    "bool",
    "string",
    "byte",
    "rune",
    "complex64",
    "complex128",
    "error",
];
const NESTING_BRANCH_KINDS: &[&str] =
    &["if_statement", "for_statement", "expression_switch_statement", "type_switch_statement", "select_statement"];
const BOOL_OPS: &[&str] = &["&&", "||"];
const BOOL_STOPS: &[&str] = &["block", "function_declaration", "method_declaration", "func_literal"];
const GLOBAL_CFG: GlobalMetricsConfig = GlobalMetricsConfig {
    cond: &["if_statement"],
    loops: &["for_statement"],
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
        let analyzed = match child.kind() {
            "function_declaration" => analyze_function(child, source),
            "method_declaration" => analyze_method(child, source),
            _ => None,
        };
        if let Some(m) = analyzed {
            functions.push(m);
        }
    }
}

struct MethodContext {
    name: String,
    arg_count: u32,
    primitive_type_count: u32,
    typed_param_count: u32,
    max_same_primitive_count: u32,
    field_accesses: Vec<String>,
    foreign_field_accesses: Vec<(String, String)>,
    class_name: Option<String>,
}

fn analyze_function(node: Node, source: &str) -> Option<FunctionMetrics> {
    let name = find_child_by_kind(node, "identifier")
        .map_or_else(|| "<anonymous>".into(), |n| node_text(n, source).to_string());
    let (arg_count, primitive_type_count, typed_param_count, max_same_primitive_count) =
        count_parameters_from_node(node, source);
    let info = MethodContext {
        name,
        arg_count,
        primitive_type_count,
        typed_param_count,
        max_same_primitive_count,
        field_accesses: Vec::new(),
        foreign_field_accesses: Vec::new(),
        class_name: None,
    };
    build_metrics(node, source, info)
}

fn analyze_method(node: Node, source: &str) -> Option<FunctionMetrics> {
    let mut pl_cursor = node.walk();
    let param_lists: Vec<Node> = node.children(&mut pl_cursor).filter(|c| c.kind() == "parameter_list").collect();

    let receiver_type = param_lists.first().and_then(|r| extract_receiver_type(*r, source));

    let method_name = find_child_by_kind(node, "field_identifier")
        .map_or_else(|| "<anonymous>".into(), |n| node_text(n, source).to_string());

    let name = match receiver_type {
        Some(ref t) => format!("{t}.{method_name}"),
        None => method_name,
    };

    let (arg_count, prim, typed, max_same) =
        if param_lists.len() >= 2 { count_param_children(param_lists[1], source) } else { (0, 0, 0, 0) };

    let self_names: Vec<&str> = param_lists
        .first()
        .and_then(|r| find_child_by_kind(*r, "parameter_declaration"))
        .and_then(|p| find_child_by_kind(p, "identifier"))
        .map(|id| vec![node_text(id, source)])
        .unwrap_or_default();
    let mut field_accesses = Vec::new();
    let mut foreign_field_accesses = Vec::new();
    if !self_names.is_empty() {
        collect_field_accesses_for(node, source, &self_names, &mut field_accesses);
        collect_foreign_field_accesses_for(node, source, &self_names, &mut foreign_field_accesses);
    }

    let info = MethodContext {
        name,
        arg_count,
        primitive_type_count: prim,
        typed_param_count: typed,
        max_same_primitive_count: max_same,
        field_accesses,
        foreign_field_accesses,
        class_name: receiver_type,
    };
    build_metrics(node, source, info)
}

fn build_metrics(node: Node, source: &str, info: MethodContext) -> Option<FunctionMetrics> {
    let start_line = node.start_position().row as u32 + 1;
    let end_line = node.end_position().row as u32 + 1;
    let loc = crate::walk::span_code_lines(node, source, COMMENT_PREFIXES);

    let body = find_child_by_kind(node, "block")?;
    let cpg = super::cpg_for(body, node, source, &crate::cpg::GO);
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
        max_embedded_block_loc: s.max_embedded_block_loc,
        structural_hash: compute_structural_fingerprint(body),
        distinct_node_kinds: count_distinct_node_kinds(body),
        skeleton_hash: compute_skeleton_hash(body),
        consecutive_asserts: count_consecutive_asserts(body, "expression_statement"),
        assert_hash: compute_assert_fingerprint(body, "expression_statement"),
        primitive_type_count: info.primitive_type_count,
        typed_param_count: info.typed_param_count,
        max_same_primitive_count: info.max_same_primitive_count,
        field_accesses: info.field_accesses,
        foreign_field_accesses: info.foreign_field_accesses,
        class_name: info.class_name,
        short_var_count: count_short_variables(body, source, &["short_var_declaration", "var_declaration"]),
        string_match_arms: count_string_match_arms(
            body,
            "expression_switch_statement",
            "expression_case",
            &["interpreted_string_literal", "raw_string_literal"],
            &["default_case"],
        ),
        cpg,
        ..Default::default()
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

type NodeHandler = fn(Node, &str, u32, &mut WalkState);

const NODE_HANDLERS: &[(&[&str], NodeHandler)] = &[
    (&["if_statement"], handle_if),
    (&["for_statement"], handle_for),
    (&["expression_switch_statement", "type_switch_statement", "select_statement"], handle_switch),
    (&["go_statement", "defer_statement"], walk_body),
];

const STRING_KINDS: &[&str] = &["interpreted_string_literal", "raw_string_literal"];

fn walk_body(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        dispatch_body_child(child, source, depth, s);
    }
}

fn dispatch_body_child(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    let kind = child.kind();
    if STRING_KINDS.contains(&kind) {
        track_embedded_block(&mut s.max_embedded_block_loc, child);
        return;
    }
    if kind == "func_literal" {
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
    walk_if_children(child, source, depth + 1, s);
}

fn handle_for(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.track_loop(depth);
    s.track_cogc_branch();
    walk_for_body(child, source, depth + 1, s);
}

fn handle_switch(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.track_nesting(depth);
    s.track_cogc_branch();
    let saved = s.cogc_nesting;
    s.cogc_nesting += 1;
    walk_switch_cases(child, source, depth + 1, s);
    s.cogc_nesting = saved;
}

fn walk_for_body(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let Some(block) = find_child_by_kind(node, "block") else {
        return;
    };
    let saved = s.cogc_nesting;
    s.cogc_nesting += 1;
    walk_body(block, source, depth, s);
    s.cogc_nesting = saved;
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
                count_boolean_ops(child, &mut s.cc, BOOL_OPS, BOOL_STOPS);
                count_cogc_sequences(child, &mut s.cogc, BOOL_OPS, BOOL_STOPS);
                shared::check_condition_complexity(
                    child,
                    &mut s.compound_condition_count,
                    COND_KINDS,
                    BOOL_OPS,
                    BOOL_STOPS,
                );
                walk_if_children(child, source, depth, s);
            }
            _ => {}
        }
    }
}

fn walk_switch_cases(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    node.children(&mut cursor).for_each(|child| match child.kind() {
        "expression_case" | "type_case" | "communication_case" => {
            s.cc += 1;
            walk_case_body(child, source, depth, s);
        }
        "default_case" => {
            walk_case_body(child, source, depth, s);
        }
        _ => {}
    });
}

fn walk_case_body(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut child_opt = node.child(0);
    while let Some(child) = child_opt {
        child_opt = child.next_sibling();
        match child.kind() {
            "expression_case" | "type_case" | "communication_case" | "default_case" | ":" | "case" | "default" => {}
            _ => walk_body(child, source, depth, s),
        }
    }
}

fn count_parameters_from_node(func_node: Node, source: &str) -> (u32, u32, u32, u32) {
    let Some(params) = find_child_by_kind(func_node, "parameter_list") else {
        return (0, 0, 0, 0);
    };
    count_param_children(params, source)
}

fn count_param_children(params: Node, source: &str) -> (u32, u32, u32, u32) {
    let mut cursor = params.walk();
    let mut count = 0;
    let mut typed = 0;
    let mut prims: Vec<&str> = Vec::new();
    for child in params.children(&mut cursor) {
        let (n, prim_ty) = match child.kind() {
            "parameter_declaration" => {
                let mut name_cursor = child.walk();
                let names = child.children(&mut name_cursor).filter(|c| c.kind() == "identifier").count() as u32;
                (names.max(1), primitive_type_of(child, source))
            }
            "variadic_parameter_declaration" => (1, primitive_type_of(child, source)),
            _ => continue,
        };
        count += n;
        typed += n;
        if let Some(ty) = prim_ty {
            prims.extend(std::iter::repeat_n(ty, n as usize));
        }
    }
    (count, prims.len() as u32, typed, max_same_primitive(&prims))
}

fn primitive_type_of<'a>(param: Node, source: &'a str) -> Option<&'a str> {
    let ti = find_child_by_kind(param, "type_identifier")
        .or_else(|| find_child_by_kind(param, "pointer_type").and_then(|p| find_child_by_kind(p, "type_identifier")))
        .or_else(|| find_child_by_kind(param, "slice_type").and_then(|s| find_child_by_kind(s, "type_identifier")))?;
    let name = node_text(ti, source);
    PRIMITIVE_TYPES.contains(&name).then_some(name)
}

fn count_declarations(root: Node) -> u32 {
    let mut count: u32 = 0;
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if child.kind() == "type_declaration" {
            count += count_type_specs(child);
        }
    }
    count
}

fn count_type_specs(type_decl: Node) -> u32 {
    let mut cursor = type_decl.walk();
    type_decl
        .children(&mut cursor)
        .filter(|s| {
            s.kind() == "type_spec"
                && (find_child_by_kind(*s, "struct_type").is_some()
                    || find_child_by_kind(*s, "interface_type").is_some())
        })
        .count() as u32
}
