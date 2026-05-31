use tree_sitter::{Node, Tree};

use super::counters::{count_short_variables, count_string_match_arms, max_same_primitive};
use super::shared::{self, count_boolean_ops, count_cogc_sequences, GlobalMetricsConfig};
use super::{
    collect_field_accesses_for, collect_foreign_field_accesses_for, compute_assert_fingerprint, compute_skeleton_hash,
    compute_structural_fingerprint, count_code_lines, count_consecutive_asserts,
    count_distinct_node_kinds, find_child_by_kind, node_text, track_embedded_block, FileMetrics, FunctionMetrics,
    ModuleMetrics, WalkState,
};

const COMMENT_PREFIXES: &[&str] = &["//", "/*", "*", "/+", "+"];
const SELF_NAMES: &[&str] = &["this"];
const PRIMITIVE_TYPES: &[&str] = &[
    "int", "uint", "byte", "ubyte", "short", "ushort", "long", "ulong", "float", "double", "real",
    "char", "wchar", "dchar", "bool", "size_t", "string", "void",
];
const NESTING_BRANCH_KINDS: &[&str] = &[
    "if_statement", "for_statement", "foreach_statement",
    "while_statement", "do_statement", "switch_statement",
];
const BOOL_OPS: &[&str] = &["&&", "||"];
const BOOL_STOPS: &[&str] = &[
    "block_statement", "function_declaration", "class_declaration",
    "struct_declaration", "function_literal",
];
const GLOBAL_CFG: GlobalMetricsConfig = GlobalMetricsConfig {
    cond: &["if_statement"],
    loops: &["for_statement", "foreach_statement", "while_statement", "do_statement"],
    branches: NESTING_BRANCH_KINDS,
    recurse: &["module_def"],
};
const COND_KINDS: &[&str] = &["if_condition"];
const LOOP_KINDS: &[&str] = &[
    "for_statement", "foreach_statement", "while_statement", "do_statement",
];

struct ParamInfo {
    args: u32,
    primitives: u32,
    typed: u32,
    max_same: u32,
}

pub fn walk(tree: &Tree, source: &str) -> FileMetrics {
    let root = tree.root_node();
    let total_loc = count_code_lines(source, COMMENT_PREFIXES);
    let mut functions = Vec::new();
    let mut gcc: u32 = 0;
    let mut gmn: u32 = 0;
    collect_functions(root, source, &mut functions, None);
    shared::collect_global_metrics(root, &mut gcc, &mut gmn, &GLOBAL_CFG);
    let mut dc: u32 = 0;
    let mut sf = Vec::new();
    count_decls(root, source, &mut dc, &mut sf);
    FileMetrics {
        module: ModuleMetrics {
            total_loc, total_functions: functions.len() as u32,
            sum_cc: functions.iter().map(|f| f.cc).sum(),
            global_conditional_count: gcc, global_max_nesting: gmn,
            declaration_count: dc, struct_fields: sf,
        },
        functions,
    }
}

fn collect_functions(node: Node, source: &str, fns: &mut Vec<FunctionMetrics>, class: Option<&str>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let kind = child.kind();
        match kind {
            "function_declaration" | "constructor" | "destructor"
            | "class_declaration" | "struct_declaration" => {
                dispatch_member(child, source, kind, fns, class);
            }
            "unittest_declaration" => {
                let line = child.start_position().row as u32 + 1;
                let Some(body) = find_child_by_kind(child, "block_statement") else { continue };
                let mut s = WalkState::new();
                walk_body(body, source, 0, &mut s);
                fns.push(finish(format!("unittest_L{line}"), child, &s, body, ParamInfo { args: 0, primitives: 0, typed: 0, max_same: 0 }));
            }
            "module_declaration" | "import_declaration"
            | "interface_declaration" | "enum_declaration" => {}
            _ => { if class.is_none() { collect_functions(child, source, fns, None); } }
        }
    }
}

fn dispatch_member(child: Node, source: &str, kind: &str, fns: &mut Vec<FunctionMetrics>, class: Option<&str>) {
    match kind {
        "function_declaration" => {
            let name = find_child_by_kind(child, "identifier")
                .map_or_else(|| "<anonymous>".into(), |n| node_text(n, source).to_string());
            let Some(mut m) = build_fn(child, source, name, count_params(child, source)) else { return };
            apply_class_ctx(&mut m, child, source, class);
            fns.push(m);
        }
        "constructor" | "destructor" => {
            let Some(cn) = class else { return };
            let is_ctor = kind == "constructor";
            let pi = if is_ctor { count_params(child, source) } else { ParamInfo { args: 0, primitives: 0, typed: 0, max_same: 0 } };
            let name = if is_ctor { format!("{cn}.this") } else { format!("{cn}.~this") };
            let Some(mut m) = build_fn(child, source, name, pi) else { return };
            m.is_constructor = is_ctor;
            fns.push(m);
        }
        _ => {
            let cn = find_child_by_kind(child, "identifier")
                .map(|n| node_text(n, source).to_string()).unwrap_or_default();
            if let Some(body) = find_child_by_kind(child, "aggregate_body") {
                collect_functions(body, source, fns, Some(&cn));
            }
        }
    }
}

fn apply_class_ctx(m: &mut FunctionMetrics, node: Node, source: &str, class: Option<&str>) {
    let Some(cn) = class else { return };
    let method = m.name.clone();
    m.name = format!("{cn}.{method}");
    m.class_name = Some(cn.to_string());
    collect_field_accesses_for(node, source, SELF_NAMES, &mut m.field_accesses);

    collect_foreign_field_accesses_for(node, source, SELF_NAMES, &mut m.foreign_field_accesses);

}

fn build_fn(node: Node, source: &str, name: String, pi: ParamInfo) -> Option<FunctionMetrics> {
    let fb = find_child_by_kind(node, "function_body")?;
    let body = find_child_by_kind(fb, "block_statement")?;
    let mut s = WalkState::new();
    walk_body(body, source, 0, &mut s);
    let mut m = finish(name, node, &s, body, pi);
    m.short_var_count = count_short_variables(body, source, &["variable_declaration", "auto_declaration"]);
    m.string_match_arms = count_string_match_arms(body, "switch_statement", "case_statement", &["string_literal"], &[]);
    Some(m)
}

fn finish(name: String, node: Node, s: &WalkState, body: Node, pi: ParamInfo) -> FunctionMetrics {
    let sl = node.start_position().row as u32 + 1;
    let el = node.end_position().row as u32 + 1;
    FunctionMetrics {
        name, start_line: sl, end_line: el,
        loc: el.saturating_sub(sl) + 1,
        cc: s.cc, cognitive_complexity: s.cogc,
        max_nesting: s.max_nesting, bump_count: s.bump_count,
        arg_count: pi.args, compound_condition_count: s.compound_condition_count,
        is_constructor: false, max_embedded_block_loc: s.max_embedded_block_loc,
        structural_hash: compute_structural_fingerprint(body),
        distinct_node_kinds: count_distinct_node_kinds(body),
        skeleton_hash: compute_skeleton_hash(body),
        consecutive_asserts: count_consecutive_asserts(body, "expression_statement"),
        assert_hash: compute_assert_fingerprint(body, "expression_statement"),
        primitive_type_count: pi.primitives, typed_param_count: pi.typed,
        max_same_primitive_count: pi.max_same,
        empty_catch_count: s.empty_catch_count,
        field_accesses: Vec::new(),
 foreign_field_accesses: Vec::new(),
 class_name: None,
 parent_class: None,
        short_var_count: 0, string_match_arms: 0,
    }
}

fn walk_body(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    node.children(&mut cursor).for_each(|child| walk_node(child, source, depth, s));
}

const SKIP_KINDS: &[&str] = &["function_literal", "scope_guard_statement"];

fn walk_node(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    let kind = child.kind();
    if LOOP_KINDS.contains(&kind) { return handle_loop(child, source, depth, s); }
    if SKIP_KINDS.contains(&kind) { return; }
    match kind {
        "if_statement" => handle_if(child, source, depth, s),
        "switch_statement" => handle_switch(child, source, depth, s),
        "try_statement" => walk_try(child, source, depth, s),
        "ternary_expression" => { s.cc += 1; s.track_cogc_branch(); }
        "string_literal" => track_embedded_block(&mut s.max_embedded_block_loc, child),
        _ => walk_body(child, source, depth, s),
    }
}

fn handle_if(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.track_if(depth);
    s.track_cogc_branch();
    track_condition(node, s);
    walk_if_scopes(node, source, depth + 1, s);
}

fn walk_if_scopes(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut saw_then = false;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() != "scope_statement" { continue; }
        if let Some(elif) = find_child_by_kind(child, "if_statement") {
            s.cc += 1;
            s.track_cogc_branch();
            track_condition(elif, s);
            walk_if_scopes(elif, source, depth, s);
            continue;
        }
        let Some(block) = find_child_by_kind(child, "block_statement") else { continue };
        if saw_then { s.track_cogc_flat(); }
        saw_then = true;
        let saved = s.cogc_nesting;
        s.cogc_nesting += 1;
        walk_body(block, source, depth, s);
        s.cogc_nesting = saved;
    }
}

fn track_condition(node: Node, s: &mut WalkState) {
    count_boolean_ops(node, &mut s.cc, BOOL_OPS, BOOL_STOPS);
    count_cogc_sequences(node, &mut s.cogc, BOOL_OPS, BOOL_STOPS);
    shared::check_condition_complexity(node, &mut s.compound_condition_count, COND_KINDS, BOOL_OPS, BOOL_STOPS);
}

fn handle_loop(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.track_loop(depth);
    s.track_cogc_branch();
    let scope = find_child_by_kind(node, "scope_statement");
    let Some(block) = scope.and_then(|sc| find_child_by_kind(sc, "block_statement")) else { return };
    let saved = s.cogc_nesting;
    s.cogc_nesting += 1;
    walk_body(block, source, depth + 1, s);
    s.cogc_nesting = saved;
}

fn handle_switch(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.track_nesting(depth);
    s.track_cogc_branch();
    let saved = s.cogc_nesting;
    s.cogc_nesting += 1;
    let scope = find_child_by_kind(node, "scope_statement");
    if let Some(block) = scope.and_then(|sc| find_child_by_kind(sc, "block_statement")) {
        walk_cases(block, source, depth + 1, s);
    }
    s.cogc_nesting = saved;
}

fn walk_cases(block: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut child_opt = block.child(0);
    while let Some(child) = child_opt {
        child_opt = child.next_sibling();
        if child.kind() != "case_statement" { continue; }
        if find_child_by_kind(child, "default").is_none() { s.cc += 1; }
        let mut ic = child.walk();
        for gc in child.children(&mut ic) {
            if !matches!(gc.kind(), "case" | "default" | ":" | "expression_list") {
                walk_node(gc, source, depth, s);
            }
        }
    }
}

fn walk_try(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    let kids: Vec<Node> = node.children(&mut cursor).collect();
    for child in kids {
        let kind = child.kind();
        if kind == "catch_statement" { handle_catch(child, source, depth, s); continue; }
        let block = match kind {
            "scope_statement" => find_child_by_kind(child, "block_statement"),
            "finally_statement" => find_child_by_kind(child, "scope_statement")
                .and_then(|sc| find_child_by_kind(sc, "block_statement")),
            _ => None,
        };
        if let Some(b) = block { walk_body(b, source, depth, s); }
    }
}

fn handle_catch(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.cc += 1;
    s.track_cogc_branch();
    let scope = find_child_by_kind(node, "scope_statement");
    let Some(block) = scope.and_then(|sc| find_child_by_kind(sc, "block_statement")) else {
        s.empty_catch_count += 1;
        return;
    };
    let mut bc = block.walk();
    if block.children(&mut bc).all(|c| matches!(c.kind(), "{" | "}")) { s.empty_catch_count += 1; }
    let saved = s.cogc_nesting;
    s.cogc_nesting += 1;
    walk_body(block, source, depth, s);
    s.cogc_nesting = saved;
}

fn count_params(node: Node, source: &str) -> ParamInfo {
    let Some(params) = find_child_by_kind(node, "parameters") else {
        return ParamInfo { args: 0, primitives: 0, typed: 0, max_same: 0 };
    };
    let mut cursor = params.walk();
    let mut a = 0;
    let mut t = 0;
    let mut prims: Vec<&str> = Vec::new();
    for child in params
        .children(&mut cursor)
        .filter(|c| c.kind() == "parameter" || c.kind() == "variadic_parameter")
    {
        a += 1;
        t += 1;
        if let Some(ty) = d_primitive_type(child, source) {
            prims.push(ty);
        }
    }
    ParamInfo { args: a, primitives: prims.len() as u32, typed: t, max_same: max_same_primitive(&prims) }
}

fn d_primitive_type<'a>(param: Node, source: &'a str) -> Option<&'a str> {
    let tn = find_child_by_kind(param, "type")?;
    let mut tc = tn.walk();
    for c in tn.children(&mut tc) {
        if PRIMITIVE_TYPES.contains(&c.kind()) {
            return Some(c.kind());
        }
        let name = node_text(c, source);
        if c.kind() == "identifier" && PRIMITIVE_TYPES.contains(&name) {
            return Some(name);
        }
    }
    None
}

fn count_decls(node: Node, source: &str, dc: &mut u32, sf: &mut Vec<(String, u32)>) {
    let mut cursor = node.walk();
    node.children(&mut cursor).for_each(|child| match child.kind() {
        "class_declaration" | "struct_declaration" => {
            *dc += 1;
            count_type_fields(child, source, sf);
        }
        "interface_declaration" | "enum_declaration" => *dc += 1,
        "module_def" => count_decls(child, source, dc, sf),
        _ => {}
    });
}

fn count_type_fields(type_node: Node, source: &str, sf: &mut Vec<(String, u32)>) {
    let name = find_child_by_kind(type_node, "identifier")
        .map_or_else(|| "<anon>".into(), |n| node_text(n, source).to_string());
    let Some(body) = find_child_by_kind(type_node, "aggregate_body") else { return };
    let mut total: u32 = 0;
    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        if child.kind() != "variable_declaration" { continue; }
        let mut n: u32 = 0;
        let mut vc = child.walk();
        for gc in child.children(&mut vc) { if gc.kind() == "declarator" { n += 1; } }
        total += if n == 0 { 1 } else { n };
    }
    if total > 0 { sf.push((name, total)); }
}
