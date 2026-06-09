use tree_sitter::{Node, Tree};

use super::counters::{count_short_variables, count_string_match_arms, max_same_primitive};
use super::shared::{self, count_boolean_ops, count_cogc_sequences, GlobalMetricsConfig};
use super::{
    compute_assert_fingerprint, compute_skeleton_hash, compute_structural_fingerprint, count_code_lines,
    count_consecutive_asserts, count_distinct_node_kinds, find_child_by_kind, node_text, track_embedded_block,
    FileMetrics, FunctionMetrics, ModuleMetrics, WalkState,
};

const COMMENT_PREFIXES: &[&str] = &["//", "/*", "*"];
const SELF_NAMES: &[&str] = &["self"];
const PRIMITIVE_TYPES: &[&str] = &[
    "Int",
    "Int8",
    "Int16",
    "Int32",
    "Int64",
    "UInt",
    "UInt8",
    "UInt16",
    "UInt32",
    "UInt64",
    "Float",
    "Double",
    "Bool",
    "String",
    "Character",
    "Void",
];
const NESTING_BRANCH_KINDS: &[&str] = &[
    "if_statement",
    "guard_statement",
    "for_statement",
    "while_statement",
    "repeat_while_statement",
    "switch_statement",
];
const BOOL_OPS: &[&str] = &["&&", "||"];
const BOOL_STOPS: &[&str] =
    &["statements", "function_declaration", "class_declaration", "lambda_literal", "init_declaration"];
const GLOBAL_CFG: GlobalMetricsConfig = GlobalMetricsConfig {
    cond: &["if_statement", "guard_statement"],
    loops: &["for_statement", "while_statement", "repeat_while_statement"],
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
    let children: Vec<_> = node.children(&mut cursor).collect();
    for child in children {
        match child.kind() {
            "function_declaration" => {
                if let Some(m) = analyze_callable(child, source, false) {
                    functions.push(m);
                }
            }
            "class_declaration" => collect_type_or_extension(child, source, functions),
            _ => {}
        }
    }
}

fn collect_type_or_extension(node: Node, source: &str, functions: &mut Vec<FunctionMetrics>) {
    let is_extension = has_keyword_child(node, "extension");
    let name = if is_extension {
        find_child_by_kind(node, "user_type")
            .and_then(|ut| find_child_by_kind(ut, "type_identifier"))
            .map(|n| node_text(n, source).to_string())
            .unwrap_or_default()
    } else {
        find_child_by_kind(node, "type_identifier").map(|n| node_text(n, source).to_string()).unwrap_or_default()
    };
    let body = find_child_by_kind(node, "class_body").or_else(|| find_child_by_kind(node, "enum_class_body"));
    if let Some(body) = body {
        collect_methods_in_body(body, source, &name, functions);
    }
}

fn has_keyword_child(node: Node, keyword: &str) -> bool {
    let mut cursor = node.walk();
    let result = node.children(&mut cursor).any(|c| c.kind() == keyword);
    result
}

fn collect_methods_in_body(body: Node, source: &str, type_name: &str, functions: &mut Vec<FunctionMetrics>) {
    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        match child.kind() {
            "function_declaration" => add_method(child, source, type_name, false, functions),
            "init_declaration" => add_method(child, source, type_name, true, functions),
            "class_declaration" => collect_type_or_extension(child, source, functions),
            _ => {}
        }
    }
}

fn add_method(node: Node, source: &str, type_name: &str, is_init: bool, functions: &mut Vec<FunctionMetrics>) {
    let Some(mut m) = analyze_callable(node, source, is_init) else { return };
    if is_init {
        m.name = format!("{type_name}.init");
        m.is_constructor = true;
    } else {
        let method_name = m.name.clone();
        m.name = format!("{type_name}.{method_name}");
        collect_swift_field_accesses(node, source, &mut m.field_accesses);
        m.field_accesses.sort();
        m.field_accesses.dedup();
    }
    m.class_name = Some(type_name.to_string());
    functions.push(m);
}

fn analyze_callable(node: Node, source: &str, is_init: bool) -> Option<FunctionMetrics> {
    let name = if is_init {
        "init".into()
    } else {
        find_child_by_kind(node, "simple_identifier")
            .map_or_else(|| "<anonymous>".into(), |n| node_text(n, source).to_string())
    };

    let start_line = node.start_position().row as u32 + 1;
    let end_line = node.end_position().row as u32 + 1;
    let loc = end_line.saturating_sub(start_line) + 1;
    let (arg_count, primitive_type_count, typed_param_count, max_same_primitive_count) = count_parameters(node, source);

    let body = find_child_by_kind(node, "function_body")?;
    let stmts = find_child_by_kind(body, "statements");
    let mut s = WalkState::new();
    if let Some(stmts) = stmts {
        walk_body(stmts, source, 0, &mut s);
    }

    let hash_node = stmts.unwrap_or(body);
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
        is_constructor: is_init,
        max_embedded_block_loc: s.max_embedded_block_loc,
        structural_hash: compute_structural_fingerprint(hash_node),
        distinct_node_kinds: count_distinct_node_kinds(hash_node),
        skeleton_hash: compute_skeleton_hash(hash_node),
        consecutive_asserts: count_consecutive_asserts(hash_node, "call_expression"),
        assert_hash: compute_assert_fingerprint(hash_node, "call_expression"),
        primitive_type_count,
        typed_param_count,
        max_same_primitive_count,
        empty_catch_count: s.empty_catch_count,
        field_accesses: Vec::new(),
        foreign_field_accesses: Vec::new(),
        class_name: None,
        parent_class: None,
        short_var_count: count_short_variables(body, source, &["property_declaration"]),
        string_match_arms: count_string_match_arms(
            body,
            "switch_statement",
            "switch_entry",
            &["line_string_literal"],
            &[],
        ),
        cpg: super::cpg_for(hash_node, node, source, &crate::cpg::SWIFT),
    })
}

fn count_parameters(node: Node, source: &str) -> (u32, u32, u32, u32) {
    let mut cursor = node.walk();
    let mut count = 0;
    let mut typed = 0;
    let mut prims: Vec<&str> = Vec::new();
    for child in node.children(&mut cursor).filter(|c| c.kind() == "parameter") {
        count += 1;
        let ut = find_child_by_kind(child, "user_type").or_else(|| {
            find_child_by_kind(child, "type_annotation").and_then(|ta| find_child_by_kind(ta, "user_type"))
        });
        let has_type = find_child_by_kind(child, "type_annotation").is_some() || ut.is_some();
        if !has_type {
            continue;
        }
        typed += 1;
        let prim = ut
            .and_then(|u| find_child_by_kind(u, "type_identifier"))
            .map(|ti| node_text(ti, source))
            .filter(|n| PRIMITIVE_TYPES.contains(n));
        if let Some(name) = prim {
            prims.push(name);
        }
    }
    (count, prims.len() as u32, typed, max_same_primitive(&prims))
}

// ─── Body walking ──────────────────────────────────────────────────────

fn walk_body(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_node(child, source, depth, s);
    }
}

type NodeHandler = fn(Node, &str, u32, &mut WalkState);

const NODE_HANDLERS: &[(&[&str], NodeHandler)] = &[
    (&["if_statement", "guard_statement"], handle_conditional),
    (&["for_statement", "while_statement", "repeat_while_statement"], handle_loop),
    (&["switch_statement"], handle_switch),
    (&["do_statement"], handle_do),
];

const STRING_KINDS: &[&str] = &["line_string_literal", "multi_line_string_literal", "raw_string_literal"];

fn walk_node(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    let kind = child.kind();
    if STRING_KINDS.contains(&kind) {
        track_embedded_block(&mut s.max_embedded_block_loc, child);
        return;
    }
    if kind == "lambda_literal" {
        return;
    }
    if kind == "ternary_expression" {
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

fn handle_conditional(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.track_if(depth);
    s.track_cogc_branch();
    count_boolean_ops(child, &mut s.cc, BOOL_OPS, BOOL_STOPS);
    count_cogc_sequences(child, &mut s.cogc, BOOL_OPS, BOOL_STOPS);
    check_condition_complexity(child, source, &mut s.compound_condition_count);
    if child.kind() == "guard_statement" {
        let mut cursor = child.walk();
        for gc in child.children(&mut cursor) {
            if gc.kind() == "statements" {
                let saved = s.cogc_nesting;
                s.cogc_nesting += 1;
                walk_body(gc, source, depth + 1, s);
                s.cogc_nesting = saved;
            }
        }
    } else {
        walk_if_children(child, source, depth + 1, s);
    }
}

fn handle_loop(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.track_loop(depth);
    s.track_cogc_branch();
    let mut cursor = child.walk();
    for lc in child.children(&mut cursor) {
        if lc.kind() == "statements" {
            let saved = s.cogc_nesting;
            s.cogc_nesting += 1;
            walk_body(lc, source, depth + 1, s);
            s.cogc_nesting = saved;
        }
    }
}

fn handle_switch(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.track_nesting(depth);
    s.track_cogc_branch();
    let saved = s.cogc_nesting;
    s.cogc_nesting += 1;
    let mut cursor = child.walk();
    for sc in child.children(&mut cursor) {
        if sc.kind() != "switch_entry" {
            continue;
        }
        if find_child_by_kind(sc, "default_keyword").is_none() {
            s.cc += 1;
        }
        if let Some(stmts) = find_child_by_kind(sc, "statements") {
            walk_body(stmts, source, depth + 1, s);
        }
    }
    s.cogc_nesting = saved;
}

fn handle_do(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut dc_opt = child.child(0);
    while let Some(dc) = dc_opt {
        dc_opt = dc.next_sibling();
        match dc.kind() {
            "statements" => walk_body(dc, source, depth, s),
            "catch_block" => walk_catch(dc, source, depth, s),
            _ => {}
        }
    }
}

fn walk_catch(dc: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.cc += 1;
    s.track_cogc_branch();
    let empty = !find_child_by_kind(dc, "statements").is_some_and(|stmts| {
        let mut c = stmts.walk();
        stmts.children(&mut c).count() > 0
    });
    if empty {
        s.empty_catch_count += 1;
    }
    if let Some(stmts) = find_child_by_kind(dc, "statements") {
        walk_body(stmts, source, depth, s);
    }
}

fn walk_if_children(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    let mut saw_else = false;
    for child in node.children(&mut cursor) {
        match child.kind() {
            "statements" => {
                if saw_else {
                    s.track_cogc_flat();
                }
                let saved = s.cogc_nesting;
                s.cogc_nesting += 1;
                walk_body(child, source, depth, s);
                s.cogc_nesting = saved;
                saw_else = false;
            }
            "else" => {
                saw_else = true;
            }
            "if_statement" => {
                s.cc += 1;
                s.track_cogc_branch();
                count_boolean_ops(child, &mut s.cc, BOOL_OPS, BOOL_STOPS);
                count_cogc_sequences(child, &mut s.cogc, BOOL_OPS, BOOL_STOPS);
                check_condition_complexity(child, source, &mut s.compound_condition_count);
                walk_if_children(child, source, depth, s);
                saw_else = false;
            }
            _ => {}
        }
    }
}

fn check_condition_complexity(node: Node, source: &str, compound_conditions: &mut u32) {
    let text = node_text(node, source);
    let cond_text = text.split('{').next().unwrap_or("");
    let ops = cond_text.matches("&&").count() + cond_text.matches("||").count();
    if ops >= 2 {
        *compound_conditions += 1;
    }
}

fn count_declarations(root: Node) -> u32 {
    let mut count: u32 = 0;
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        let dominated = (child.kind() == "class_declaration" && !has_keyword_child(child, "extension"))
            || child.kind() == "protocol_declaration";
        if dominated {
            count += 1;
        }
    }
    count
}

// ─── Field access tracking ─────────────────────────────────────────────

fn collect_swift_field_accesses(node: Node, source: &str, fields: &mut Vec<String>) {
    let mut cursor = node.walk();
    node.children(&mut cursor).for_each(|child| {
        if child.kind() == "navigation_expression" {
            if let Some(field) = try_extract_self_field(child, source) {
                fields.push(field);
            }
        }
        collect_swift_field_accesses(child, source, fields);
    });
}

fn try_extract_self_field(nav: Node, source: &str) -> Option<String> {
    let mut cursor = nav.walk();
    let children: Vec<_> = nav.children(&mut cursor).collect();
    if children.len() < 2 {
        return None;
    }
    let obj = children[0];
    let is_self = obj.kind() == "self_expression"
        || (obj.kind() == "simple_identifier" && SELF_NAMES.contains(&node_text(obj, source)));
    if !is_self {
        return None;
    }
    let suffix = children.last()?;
    if suffix.kind() != "navigation_suffix" {
        return None;
    }
    find_child_by_kind(*suffix, "simple_identifier").map(|id| node_text(id, source).to_string())
}
