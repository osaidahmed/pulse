use tree_sitter::{Node, Tree};

use super::counters::{count_short_variables, count_string_match_arms};
use super::shared::{self, GlobalMetricsConfig};
use super::{
    compute_assert_fingerprint, compute_skeleton_hash, compute_structural_fingerprint,
    count_code_lines, count_consecutive_asserts, find_child_by_kind, node_text,
    track_embedded_block, FileMetrics, FunctionMetrics, ModuleMetrics, WalkState,
};

const COMMENT_PREFIXES: &[&str] = &["#"];
const NESTING_BRANCH_KINDS: &[&str] = &["if", "unless", "while", "until", "for", "case"];
const GLOBAL_CFG: GlobalMetricsConfig = GlobalMetricsConfig {
    cond: &["if", "unless"],
    loops: &["while", "until", "for"],
    branches: NESTING_BRANCH_KINDS,
    recurse: &[],
};
const BOOL_STOPS: &[&str] = &[
    "method", "class", "module", "lambda", "block", "do_block", "body_statement", "then",
];

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
    let mut cursor = root.walk();
    let declaration_count = root.children(&mut cursor)
        .filter(|c| matches!(c.kind(), "class" | "module")).count() as u32;
    let module = ModuleMetrics {
        total_loc, total_functions, sum_cc,
        global_conditional_count: gcc, global_max_nesting: gmn,
        declaration_count,
        struct_fields: Vec::new(),
    };
    FileMetrics { functions, module }
}

fn collect_functions(node: Node, source: &str, fns: &mut Vec<FunctionMetrics>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "method" => {
                if let Some(m) = build_metrics(child, source, extract_name(child, source)) {
                    fns.push(m);
                }
            }
            "class" | "module" => collect_type_body(child, source, fns),
            _ => {}
        }
    }
}

fn collect_type_body(type_node: Node, source: &str, fns: &mut Vec<FunctionMetrics>) {
    let type_name = find_child_by_kind(type_node, "constant")
        .or_else(|| find_child_by_kind(type_node, "scope_resolution"))
        .map(|n| node_text(n, source).to_string())
        .unwrap_or_default();
    let Some(body) = find_child_by_kind(type_node, "body_statement") else { return };
    let mut child_opt = body.child(0);
    while let Some(child) = child_opt {
        child_opt = child.next_sibling();
        if matches!(child.kind(), "class" | "module") {
            collect_type_body(child, source, fns);
            continue;
        }
        if !matches!(child.kind(), "method" | "singleton_method") { continue; }
        let raw = extract_name(child, source);
        let Some(mut m) = build_metrics(child, source, raw.clone()) else { continue };
        m.name = format!("{type_name}.{raw}");
        m.class_name = Some(type_name.clone());
        m.is_constructor = raw == "initialize";
        if !m.is_constructor {
            collect_field_accesses(child, source, &mut m.field_accesses);
            m.field_accesses.sort();
            m.field_accesses.dedup();
        }
        fns.push(m);
    }
}

fn extract_name(node: Node, source: &str) -> String {
    if node.kind() == "singleton_method" {
        let mut cursor = node.walk();
        for c in node.children(&mut cursor) {
            if c.kind() == "identifier" && c.prev_sibling().map(|s| s.kind()) == Some(".") {
                return node_text(c, source).to_string();
            }
        }
    }
    find_child_by_kind(node, "identifier")
        .map_or_else(|| "<anonymous>".into(), |n| node_text(n, source).to_string())
}

#[allow(clippy::unnecessary_wraps)]
fn build_metrics(node: Node, source: &str, name: String) -> Option<FunctionMetrics> {
    let start_line = node.start_position().row as u32 + 1;
    let end_line = node.end_position().row as u32 + 1;
    let body = find_child_by_kind(node, "body_statement");
    let mut s = WalkState::new();
    if let Some(b) = body { walk_body(b, source, 0, &mut s); }
    let sh = body.map_or(0, compute_structural_fingerprint);
    let sk = body.map_or(0, compute_skeleton_hash);
    let ca = body.map_or(0, |b| count_consecutive_asserts(b, "call"));
    let ah = body.map_or(0, |b| compute_assert_fingerprint(b, "call"));
    Some(FunctionMetrics {
        name, start_line, end_line,
        loc: end_line.saturating_sub(start_line) + 1,
        cc: s.cc, cognitive_complexity: s.cogc,
        max_nesting: s.max_nesting, bump_count: s.bump_count,
        arg_count: count_parameters(node),
        compound_condition_count: s.compound_condition_count,
        is_constructor: false,
        max_embedded_block_loc: s.max_embedded_block_loc,
        structural_hash: sh, skeleton_hash: sk,
        consecutive_asserts: ca, assert_hash: ah,
        primitive_type_count: 0, typed_param_count: 0,
        empty_catch_count: s.empty_catch_count,
        field_accesses: Vec::new(), class_name: None,
        short_var_count: body.map_or(0, |b| count_short_variables(b, source, &["assignment", "operator_assignment"])),
        string_match_arms: body.map_or(0, |b| count_string_match_arms(b, "case", "when", &["string"])),
    })
}

fn count_parameters(func: Node) -> u32 {
    let Some(params) = find_child_by_kind(func, "method_parameters") else { return 0 };
    let mut cursor = params.walk();
    params.children(&mut cursor).filter(|c| matches!(c.kind(),
        "identifier" | "optional_parameter" | "splat_parameter" | "hash_splat_parameter"
        | "keyword_parameter" | "optional_keyword_parameter" | "block_parameter"
    )).count() as u32
}

fn walk_body(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    s.reset_bump();
    for child in node.children(&mut cursor) {
        dispatch(child, source, depth, s);
    }
}

fn dispatch(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let k = node.kind();
    if matches!(k, "if" | "unless" | "if_modifier" | "unless_modifier") {
        handle_if(node, source, depth, k.ends_with("modifier"), s);
    } else if matches!(k, "while" | "until" | "for" | "while_modifier" | "until_modifier") {
        handle_loop(node, k, source, depth, s);
    } else if k == "case" {
        handle_case(node, source, depth, s);
    } else if k == "begin" {
        walk_begin_children(node, source, depth, s);
    } else if k == "rescue" {
        handle_rescue(node, source, depth, s);
    } else {
        handle_expression(node, k, source, depth, s);
    }
}

fn handle_if(node: Node, source: &str, depth: u32, postfix: bool, s: &mut WalkState) {
    s.track_if(depth);
    s.track_cogc_branch();
    count_boolean_ops(node, &mut s.cc);
    check_condition(node, source, &mut s.compound_condition_count);
    let mut last: Option<String> = None;
    walk_cogc(node, &mut s.cogc, &mut last);
    if postfix { return; }
    walk_if_clauses(node, source, depth + 1, s);
}

fn walk_if_clauses(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut child_opt = node.child(0);
    while let Some(child) = child_opt {
        child_opt = child.next_sibling();
        match child.kind() {
            "then" => { s.cogc_nesting += 1; walk_body(child, source, depth, s); s.cogc_nesting -= 1; }
            "elsif" => {
                s.cc += 1; s.track_cogc_branch();
                if depth > s.max_nesting { s.max_nesting = depth; }
                count_boolean_ops(child, &mut s.cc);
                check_condition(child, source, &mut s.compound_condition_count);
                let mut last: Option<String> = None;
                walk_cogc(child, &mut s.cogc, &mut last);
                walk_if_clauses(child, source, depth, s);
            }
            "else" => { s.track_cogc_flat(); s.cogc_nesting += 1; walk_body(child, source, depth, s); s.cogc_nesting -= 1; }
            _ => {}
        }
    }
}

fn handle_loop(node: Node, kind: &str, source: &str, depth: u32, s: &mut WalkState) {
    s.track_loop(depth);
    s.track_cogc_branch();
    if kind.ends_with("modifier") { return; }
    let mut child_opt = node.child(0);
    while let Some(child) = child_opt {
        child_opt = child.next_sibling();
        if !matches!(child.kind(), "do" | "body_statement") { continue; }
        s.cogc_nesting += 1; walk_body(child, source, depth + 1, s); s.cogc_nesting -= 1;
    }
}

fn handle_case(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.track_nesting(depth); s.track_cogc_branch(); s.cogc_nesting += 1;
    let mut c = node.child(0);
    while let Some(child) = c {
        c = child.next_sibling();
        match child.kind() {
            "when" => { s.cc += 1; walk_body(child, source, depth + 1, s); }
            "else" => walk_body(child, source, depth + 1, s),
            _ => {}
        }
    }
    s.cogc_nesting -= 1;
}

fn walk_begin_children(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let n = node.child_count();
    for i in 0..n {
        let child = node.child(i).unwrap();
        match child.kind() {
            "rescue" => handle_rescue(child, source, depth, s),
            "ensure" | "else" => walk_body(child, source, depth, s),
            _ => dispatch(child, source, depth, s),
        }
    }
}

fn handle_rescue(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.cc += 1;
    s.track_cogc_branch();
    if is_rescue_empty(node) { s.empty_catch_count += 1; }
    s.cogc_nesting += 1;
    let mut c = node.child(0);
    while let Some(child) = c {
        c = child.next_sibling();
        match child.kind() {
            "exceptions" | "exception_variable" => {}
            "then" | "body_statement" => walk_body(child, source, depth, s),
            _ => dispatch(child, source, depth, s),
        }
    }
    s.cogc_nesting -= 1;
}

fn is_rescue_empty(node: Node) -> bool {
    let mut c = node.child(0);
    while let Some(child) = c {
        c = child.next_sibling();
        let k = child.kind();
        if matches!(k, "exceptions" | "exception_variable" | "comment" | "then") || child.is_extra() {
            continue;
        }
        if k != "body_statement" { return false; }
        let mut inner = child.walk();
        let has_content = child.children(&mut inner).any(|gc| gc.kind() != "comment" && !gc.is_extra());
        if has_content { return false; }
    }
    true
}

fn handle_expression(node: Node, kind: &str, source: &str, depth: u32, s: &mut WalkState) {
    match kind {
        "conditional" => { s.cc += 1; s.track_cogc_branch(); walk_body(node, source, depth, s); }
        "binary" => {
            let mut c = node.child(0);
            while let Some(child) = c {
                c = child.next_sibling();
                if !matches!(child.kind(), "&&" | "||" | "and" | "or") {
                    dispatch(child, source, depth, s);
                }
            }
        }
        "do_block" | "block" => { let sv = s.cogc_nesting; walk_body(node, source, depth, s); s.cogc_nesting = sv; }
        "string" | "heredoc_body" => track_embedded_block(&mut s.max_embedded_block_loc, node),
        _ => walk_body(node, source, depth, s),
    }
}

fn count_boolean_ops(node: Node, cc: &mut u32) {
    let n = node.child_count();
    for i in 0..n {
        let child = node.child(i).unwrap();
        let k = child.kind();
        if k == "binary" && !bool_op_category(child).is_empty() {
            *cc += 1;
            count_boolean_ops(child, cc);
        } else if !BOOL_STOPS.contains(&k) {
            count_boolean_ops(child, cc);
        }
    }
}

fn walk_cogc(node: Node, cogc: &mut u32, last: &mut Option<String>) {
    let mut c = node.child(0);
    while let Some(child) = c {
        c = child.next_sibling();
        let k = child.kind();
        if BOOL_STOPS.contains(&k) { continue; }
        if k != "binary" { walk_cogc(child, cogc, last); continue; }
        let op = bool_op_category(child);
        if op.is_empty() { continue; }
        if last.as_deref() != Some(op) { *cogc += 1; *last = Some(op.to_string()); }
        walk_cogc(child, cogc, last);
    }
}

fn bool_op_category(node: Node) -> &'static str {
    let n = node.child_count();
    for i in 0..n {
        if let Some(c) = node.child(i) {
            match c.kind() {
                "&&" | "and" => return "and",
                "||" | "or" => return "or",
                _ => {}
            }
        }
    }
    ""
}

fn check_condition(node: Node, source: &str, count: &mut u32) {
    let text = node_text(node, source);
    let line = text.split('\n').next().unwrap_or("");
    let ops = line.matches("&&").count() + line.matches("||").count()
        + line.matches(" and ").count() + line.matches(" or ").count();
    if ops >= 2 { *count += 1; }
}

fn collect_field_accesses(node: Node, source: &str, fields: &mut Vec<String>) {
    let mut stack = vec![node];
    let mut cursor = node.walk();
    while let Some(current) = stack.pop() {
        for child in current.children(&mut cursor) {
            push_ruby_self_ref(child, source, fields);
            stack.push(child);
        }
    }
}

fn push_ruby_self_ref(node: Node, source: &str, fields: &mut Vec<String>) {
    let kind = node.kind();
    if kind == "instance_variable" {
        if let Some(f) = node_text(node, source).strip_prefix('@') {
            fields.push(f.to_string());
        }
        return;
    }
    if kind != "call" {
        return;
    }
    let Some(receiver) = node.child_by_field_name("receiver") else { return; };
    if receiver.kind() != "self" { return; }
    if let Some(method) = node.child_by_field_name("method") {
        fields.push(node_text(method, source).to_string());
    }
}
