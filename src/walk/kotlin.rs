use tree_sitter::{Node, Tree};

use super::counters::{count_short_variables, count_string_match_arms, max_same_primitive};
use super::shared::{self, count_boolean_ops, count_cogc_sequences, GlobalMetricsConfig};
use super::{
    collect_field_accesses_for, collect_foreign_field_accesses_for, compute_assert_fingerprint, compute_skeleton_hash,
    compute_structural_fingerprint, count_code_lines, count_consecutive_asserts, count_distinct_node_kinds,
    find_child_by_kind, is_catch_body_empty, node_text, track_embedded_block, FileMetrics, FunctionMetrics,
    ModuleMetrics, WalkState,
};

const COMMENT_PREFIXES: &[&str] = &["//", "/*", "*"];
const SELF_NAMES: &[&str] = &["this"];
const PRIMITIVE_TYPES: &[&str] =
    &["Int", "Long", "Short", "Byte", "Float", "Double", "Boolean", "Char", "String", "Unit", "Nothing"];
const NESTING_BRANCH_KINDS: &[&str] =
    &["if_expression", "for_statement", "while_statement", "do_while_statement", "when_expression"];
const BOOL_OPS: &[&str] = &["&&", "||"];
const BOOL_STOPS: &[&str] =
    &["block", "function_declaration", "class_declaration", "lambda_literal", "anonymous_function"];
const GLOBAL_CFG: GlobalMetricsConfig = GlobalMetricsConfig {
    cond: &["if_expression"],
    loops: &["for_statement", "while_statement", "do_while_statement"],
    branches: NESTING_BRANCH_KINDS,
    recurse: &[],
};
const COND_KINDS: &[&str] = &["binary_expression"];
const WHEN_ENTRY_LEAF: &[&str] =
    &["else", "->", "string_literal", "number_literal", "identifier", "binary_expression", ","];
const SCOPE_BOUNDARY: &[&str] = &["lambda_literal", "anonymous_function", "function_declaration"];

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
        declaration_count: {
            let mut dc = root.walk();
            root.children(&mut dc)
                .filter(|c| c.kind() == "class_declaration" || c.kind() == "object_declaration")
                .count() as u32
        },
        struct_fields: Vec::new(),
    };
    FileMetrics { functions, module }
}

fn collect_functions(node: Node, source: &str, fns: &mut Vec<FunctionMetrics>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "class_declaration" | "object_declaration" => {
                collect_class_methods(child, source, fns);
            }
            "function_declaration" => {
                if let Some(m) = analyze_callable(child, source, &extract_name(child, source)) {
                    fns.push(m);
                }
            }
            _ => collect_functions(child, source, fns),
        }
    }
}

fn collect_class_methods(node: Node, source: &str, fns: &mut Vec<FunctionMetrics>) {
    let cls = find_child_by_kind(node, "identifier").map(|n| node_text(n, source).to_string()).unwrap_or_default();

    emit_primary_ctor(node, source, &cls, fns);

    let Some(body) = find_child_by_kind(node, "class_body") else {
        return;
    };
    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        dispatch_member(child, source, &cls, fns);
    }
}

fn dispatch_member(child: Node, source: &str, cls: &str, fns: &mut Vec<FunctionMetrics>) {
    match child.kind() {
        "function_declaration" => emit_method(child, source, cls, fns),
        "secondary_constructor" | "anonymous_initializer" => {
            emit_ctor_or_init(child, source, cls, fns);
        }
        "companion_object" => emit_companion_methods(child, source, cls, fns),
        "class_declaration" | "object_declaration" => collect_class_methods(child, source, fns),
        _ => {}
    }
}

fn emit_method(child: Node, source: &str, cls: &str, fns: &mut Vec<FunctionMetrics>) {
    let id = find_child_by_kind(child, "identifier").map_or("<anonymous>", |n| node_text(n, source));
    if let Some(mut m) = analyze_callable(child, source, &format!("{cls}.{id}")) {
        m.class_name = Some(cls.to_string());
        collect_field_accesses_for(child, source, SELF_NAMES, &mut m.field_accesses);

        collect_foreign_field_accesses_for(child, source, SELF_NAMES, &mut m.foreign_field_accesses);

        fns.push(m);
    }
}

fn emit_companion_methods(node: Node, source: &str, cls: &str, fns: &mut Vec<FunctionMetrics>) {
    let Some(body) = find_child_by_kind(node, "class_body") else {
        return;
    };
    let prefix = format!("{cls}.Companion");
    let mut cursor = body.walk();
    for member in body.children(&mut cursor) {
        if member.kind() == "function_declaration" {
            emit_method(member, source, &prefix, fns);
        }
    }
}

fn emit_ctor_or_init(child: Node, source: &str, cls: &str, fns: &mut Vec<FunctionMetrics>) {
    let is_ctor = child.kind() == "secondary_constructor";
    let opt = if is_ctor {
        analyze_callable(child, source, &format!("{cls}.{cls}"))
    } else {
        let Some(body) = find_child_by_kind(child, "block") else {
            return;
        };
        let mut s = WalkState::new();
        walk_body(body, source, 0, &mut s);
        Some(walked_metrics(child, body, source, &s))
    };
    let Some(mut m) = opt else { return };
    m.is_constructor = true;
    m.class_name = Some(cls.to_string());
    if !is_ctor {
        m.name = format!("{cls}.init");
    }
    fns.push(m);
}

fn emit_primary_ctor(class_node: Node, source: &str, cls: &str, fns: &mut Vec<FunctionMetrics>) {
    let Some(ctor) = find_child_by_kind(class_node, "primary_constructor") else {
        return;
    };
    let Some(params) = find_child_by_kind(ctor, "class_parameters") else {
        return;
    };
    let mut cursor = params.walk();
    let mut cnt = 0;
    let mut typed = 0;
    let mut prims: Vec<&str> = Vec::new();
    for child in params.children(&mut cursor).filter(|c| c.kind() == "class_parameter") {
        cnt += 1;
        typed += 1;
        if let Some(ty) = primitive_type_of(child, source) {
            prims.push(ty);
        }
    }
    let prim = prims.len() as u32;
    let max_same = max_same_primitive(&prims);
    if cnt <= 5 {
        return;
    }
    let sl = ctor.start_position().row as u32 + 1;
    let el = ctor.end_position().row as u32 + 1;
    fns.push(FunctionMetrics {
        name: format!("{cls}.{cls}"),
        start_line: sl,
        end_line: el,
        loc: el.saturating_sub(sl) + 1,
        cc: 1,
        arg_count: cnt,
        is_constructor: true,
        primitive_type_count: prim,
        typed_param_count: typed,
        max_same_primitive_count: max_same,
        foreign_field_accesses: Vec::new(),
        class_name: Some(cls.to_string()),
        parent_class: None,
        cognitive_complexity: 0,
        max_nesting: 0,
        bump_count: 0,
        compound_condition_count: 0,
        max_embedded_block_loc: 0,
        structural_hash: 0,
        distinct_node_kinds: 0,
        skeleton_hash: 0,
        consecutive_asserts: 0,
        assert_hash: 0,
        empty_catch_count: 0,
        field_accesses: Vec::new(),
        short_var_count: 0,
        string_match_arms: 0,
        cpg: None,
    });
}

fn extract_name(node: Node, source: &str) -> String {
    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();
    let start = match children.iter().position(|c| c.kind() == "fun") {
        Some(i) => i + 1,
        None => return "<anonymous>".into(),
    };
    let mut receiver: Option<&str> = None;
    for child in &children[start..] {
        match child.kind() {
            "user_type" => receiver = Some(node_text(*child, source)),
            "identifier" if !node_text(*child, source).is_empty() => {
                let n = node_text(*child, source);
                return receiver.map_or_else(|| n.to_string(), |r| format!("{r}.{n}"));
            }
            "function_value_parameters" => break,
            _ => {}
        }
    }
    receiver
        .and_then(|r| r.rfind('.').map(|i| format!("{}.{}", &r[..i], &r[i + 1..])))
        .unwrap_or_else(|| "<anonymous>".into())
}

fn analyze_callable(node: Node, source: &str, name: &str) -> Option<FunctionMetrics> {
    let func_body = find_child_by_kind(node, "function_body")?;
    let body = find_child_by_kind(func_body, "block").unwrap_or(func_body);
    let mut s = WalkState::new();
    walk_body(body, source, 0, &mut s);
    let p = count_parameters(node, source);
    let mut m = walked_metrics(node, body, source, &s);
    m.name = name.to_string();
    m.arg_count = p.0;
    m.primitive_type_count = p.1;
    m.typed_param_count = p.2;
    m.max_same_primitive_count = p.3;
    Some(m)
}

fn walked_metrics(node: Node, body: Node, source: &str, s: &WalkState) -> FunctionMetrics {
    let sl = node.start_position().row as u32 + 1;
    let el = node.end_position().row as u32 + 1;
    FunctionMetrics {
        name: String::new(),
        start_line: sl,
        end_line: el,
        loc: el.saturating_sub(sl) + 1,
        cc: s.cc,
        cognitive_complexity: s.cogc,
        max_nesting: s.max_nesting,
        bump_count: s.bump_count,
        arg_count: 0,
        compound_condition_count: s.compound_condition_count,
        is_constructor: false,
        max_embedded_block_loc: s.max_embedded_block_loc,
        structural_hash: compute_structural_fingerprint(body),
        distinct_node_kinds: count_distinct_node_kinds(body),
        skeleton_hash: compute_skeleton_hash(body),
        consecutive_asserts: count_consecutive_asserts(body, "call_expression"),
        assert_hash: compute_assert_fingerprint(body, "call_expression"),
        primitive_type_count: 0,
        typed_param_count: 0,
        max_same_primitive_count: 0,
        empty_catch_count: s.empty_catch_count,
        field_accesses: Vec::new(),
        foreign_field_accesses: Vec::new(),
        class_name: None,
        parent_class: None,
        short_var_count: count_short_variables(body, source, &["property_declaration"]),
        string_match_arms: count_string_match_arms(body, "when_expression", "when_entry", &["string_literal"], &[]),
        cpg: super::cpg_for(body, node, source, &crate::cpg::KOTLIN).map(|mut c| {
            let exit = c.cfg.exit;
            crate::cpg::implicit_return::seed_string_interpolation(body, source, exit, &mut c.def_use);
            c
        }),
    }
}

fn walk_body(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_node(child, source, depth, s);
    }
}

const LOOP_KINDS: &[&str] = &["for_statement", "while_statement", "do_while_statement"];
const EMBEDDED_STR: &[&str] = &["string_literal", "multiline_string_literal"];

fn walk_node(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    let kind = child.kind();
    if SCOPE_BOUNDARY.contains(&kind) || EMBEDDED_STR.contains(&kind) {
        if EMBEDDED_STR.contains(&kind) {
            track_embedded_block(&mut s.max_embedded_block_loc, child);
        }
        return;
    }
    if LOOP_KINDS.contains(&kind) {
        s.track_loop(depth);
        s.track_cogc_branch();
        walk_branches(child, source, depth + 1, s);
        return;
    }
    match kind {
        "if_expression" => handle_if(child, source, depth, s),
        "when_expression" => handle_when(child, source, depth, s),
        "catch_block" => handle_catch(child, source, depth, s),
        _ => walk_default(child, source, depth, s),
    }
}

fn walk_default(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    if child.kind() == "binary_expression" && child.children(&mut child.walk()).any(|c| c.kind() == "?:") {
        s.cc += 1;
        s.track_cogc_branch();
    } else {
        walk_body(child, source, depth, s);
    }
}

fn handle_if(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.track_if(depth);
    s.track_cogc_branch();
    count_boolean_ops(child, &mut s.cc, BOOL_OPS, BOOL_STOPS);
    count_cogc_sequences(child, &mut s.cogc, BOOL_OPS, BOOL_STOPS);
    shared::check_condition_complexity(child, &mut s.compound_condition_count, COND_KINDS, BOOL_OPS, BOOL_STOPS);
    walk_branches(child, source, depth + 1, s);
}

fn walk_branches(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    let mut saw_else = false;
    for child in node.children(&mut cursor) {
        match child.kind() {
            "block" => {
                if saw_else {
                    s.track_cogc_flat();
                }
                let saved = s.cogc_nesting;
                s.cogc_nesting += 1;
                walk_body(child, source, depth, s);
                s.cogc_nesting = saved;
                saw_else = false;
            }
            "else" => saw_else = true,
            "if_expression" => {
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
                walk_branches(child, source, depth, s);
                saw_else = false;
            }
            _ => {}
        }
    }
}

fn handle_when(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.track_nesting(depth);
    s.track_cogc_branch();
    let saved = s.cogc_nesting;
    s.cogc_nesting += 1;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() != "when_entry" {
            continue;
        }
        if !child.children(&mut child.walk()).any(|c| c.kind() == "else") {
            s.cc += 1;
        }
        let mut cur2 = child.walk();
        for gc in child.children(&mut cur2) {
            if gc.kind() == "block" {
                walk_body(gc, source, depth + 1, s);
            } else if !WHEN_ENTRY_LEAF.contains(&gc.kind()) {
                walk_node(gc, source, depth + 1, s);
            }
        }
    }
    s.cogc_nesting = saved;
}

fn handle_catch(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.cc += 1;
    s.track_cogc_branch();
    if is_catch_body_empty(child, "block", None) {
        s.empty_catch_count += 1;
    }
    shared::walk_block_children(child, &mut shared::BlockWalkCtx { source, depth, state: s }, "block", walk_body);
}

fn count_parameters(node: Node, source: &str) -> (u32, u32, u32, u32) {
    let Some(params) = find_child_by_kind(node, "function_value_parameters") else {
        return (0, 0, 0, 0);
    };
    let mut cursor = params.walk();
    let mut count = 0;
    let mut typed = 0;
    let mut prims: Vec<&str> = Vec::new();
    for child in params.children(&mut cursor).filter(|c| c.kind() == "parameter") {
        count += 1;
        let has_type =
            find_child_by_kind(child, "user_type").is_some() || find_child_by_kind(child, "nullable_type").is_some();
        typed += u32::from(has_type);
        if let Some(ty) = primitive_type_of(child, source) {
            prims.push(ty);
        }
    }
    (count, prims.len() as u32, typed, max_same_primitive(&prims))
}

fn primitive_type_of<'a>(param: Node, source: &'a str) -> Option<&'a str> {
    let ut = find_child_by_kind(param, "user_type")
        .or_else(|| find_child_by_kind(param, "nullable_type").and_then(|n| find_child_by_kind(n, "user_type")))?;
    let id = find_child_by_kind(ut, "identifier")?;
    let name = &source[id.byte_range()];
    PRIMITIVE_TYPES.contains(&name).then_some(name)
}
