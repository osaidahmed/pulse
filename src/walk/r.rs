use tree_sitter::{Node, Tree};

use super::counters::count_short_variables;
use super::shared::{self, GlobalMetricsConfig};
use super::{
    compute_assert_fingerprint, compute_skeleton_hash, compute_structural_fingerprint,
    count_code_lines, count_consecutive_asserts, count_distinct_node_kinds, node_text,
    track_embedded_block, FileMetrics,
    FunctionMetrics, ModuleMetrics, WalkState,
};

const COMMENT_PREFIXES: &[&str] = &["#"];
const NESTING_BRANCH_KINDS: &[&str] =
    &["if_statement", "for_statement", "while_statement", "repeat_statement"];
const GLOBAL_CFG: GlobalMetricsConfig = GlobalMetricsConfig {
    cond: &["if_statement"],
    loops: &["for_statement", "while_statement", "repeat_statement"],
    branches: NESTING_BRANCH_KINDS,
    recurse: &[],
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
        declaration_count: 0,
        struct_fields: Vec::new(),
    };
    FileMetrics { functions, module }
}

fn collect_functions(node: Node, source: &str, fns: &mut Vec<FunctionMetrics>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "function_definition" => {
                if let Some(m) = build_metrics(child, source, "<anonymous>".into()) {
                    fns.push(m);
                }
            }
            "binary_operator" => handle_assignment(child, source, fns),
            _ => {}
        }
    }
}

fn handle_assignment(node: Node, source: &str, fns: &mut Vec<FunctionMetrics>) {
    let Some(op_node) = node.child_by_field_name("operator") else { return };
    let op = node_text(op_node, source);
    let (name_node, func_node) = match op {
        "<-" | "=" | "<<-" => (node.child_by_field_name("lhs"), node.child_by_field_name("rhs")),
        "->" | "->>" => (node.child_by_field_name("rhs"), node.child_by_field_name("lhs")),
        _ => return,
    };
    let Some(func_node) = func_node else { return };
    if func_node.kind() != "function_definition" { return; }
    let name = name_node.map_or_else(
        || "<anonymous>".into(),
        |n| if n.kind() == "identifier" {
            node_text(n, source).to_string()
        } else {
            node_text(n, source).rsplit(&['$', '@'][..]).next().unwrap_or("<anonymous>").to_string()
        },
    );
    if let Some(m) = build_metrics(func_node, source, name) {
        fns.push(m);
    }
}

#[allow(clippy::unnecessary_wraps)]
fn build_metrics(node: Node, source: &str, name: String) -> Option<FunctionMetrics> {
    let start_line = node.start_position().row as u32 + 1;
    let end_line = node.end_position().row as u32 + 1;
    let body = node.child_by_field_name("body");
    let mut s = WalkState::new();
    if let Some(b) = body { walk_body(b, source, 0, &mut s); }
    let params = node.child_by_field_name("parameters");
    let arg_count = params.map_or(0, |p| {
        let mut c = p.walk();
        p.children(&mut c).filter(|c| c.kind() == "parameter" || c.kind() == "dots").count() as u32
    });
    Some(FunctionMetrics {
        name, start_line, end_line,
        loc: end_line.saturating_sub(start_line) + 1,
        cc: s.cc, cognitive_complexity: s.cogc,
        max_nesting: s.max_nesting, bump_count: s.bump_count, arg_count,
        compound_condition_count: s.compound_condition_count,
        is_constructor: false,
        max_embedded_block_loc: s.max_embedded_block_loc,
        structural_hash: body.map_or(0, compute_structural_fingerprint),
        distinct_node_kinds: body.map_or(0, count_distinct_node_kinds),
        skeleton_hash: body.map_or(0, compute_skeleton_hash),
        consecutive_asserts: body.map_or(0, |b| count_consecutive_asserts(b, "call")),
        assert_hash: body.map_or(0, |b| compute_assert_fingerprint(b, "call")),
        primitive_type_count: 0, typed_param_count: 0, max_same_primitive_count: 0,
        empty_catch_count: s.empty_catch_count,
        field_accesses: Vec::new(),
 foreign_field_accesses: Vec::new(),
 class_name: None,
 parent_class: None,
        short_var_count: body.map_or(0, |b| count_short_variables(b, source, &["binary_operator"])),
        string_match_arms: 0,
        cpg: None,
    })
}

fn walk_body(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        dispatch(child, source, depth, s);
    }
}

fn dispatch(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    match node.kind() {
        "if_statement" => handle_if(node, source, depth, s),
        "for_statement" | "while_statement" | "repeat_statement" => handle_loop(node, source, depth, s),
        "call" => handle_call(node, source, depth, s),
        "binary_operator" => {
            if let Some(l) = node.child_by_field_name("lhs") { dispatch(l, source, depth, s); }
            if let Some(r) = node.child_by_field_name("rhs") { dispatch(r, source, depth, s); }
        }
        "string" | "function_definition" => track_leaf(node, s),
        _ => walk_body(node, source, depth, s),
    }
}

fn track_leaf(node: Node, s: &mut WalkState) {
    if node.kind() == "string" { track_embedded_block(&mut s.max_embedded_block_loc, node); }
}

fn handle_if(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.track_if(depth);
    s.track_cogc_branch();
    analyze_condition(node, source, s);
    if let Some(cons) = node.child_by_field_name("consequence") {
        s.cogc_nesting += 1;
        walk_body(cons, source, depth + 1, s);
        s.cogc_nesting -= 1;
    }
    if let Some(alt) = node.child_by_field_name("alternative") {
        handle_alternative(alt, source, depth, s);
    }
}

fn handle_alternative(alt: Node, source: &str, depth: u32, s: &mut WalkState) {
    if alt.kind() != "if_statement" {
        s.track_cogc_flat();
        s.cogc_nesting += 1;
        walk_body(alt, source, depth + 1, s);
        s.cogc_nesting -= 1;
        return;
    }
    s.cc += 1;
    s.track_cogc_branch();
    analyze_condition(alt, source, s);
    if let Some(cons) = alt.child_by_field_name("consequence") {
        s.cogc_nesting += 1;
        walk_body(cons, source, depth + 1, s);
        s.cogc_nesting -= 1;
    }
    if let Some(inner) = alt.child_by_field_name("alternative") {
        handle_alternative(inner, source, depth, s);
    }
}

fn analyze_condition(node: Node, source: &str, s: &mut WalkState) {
    let Some(cond) = node.child_by_field_name("condition") else { return };
    walk_bool_tree(cond, source, &mut |_| { s.cc += 1; });
    let mut last_op: Option<&str> = None;
    walk_bool_tree(cond, source, &mut |op| {
        if last_op != Some(op) {
            s.cogc += 1;
            last_op = Some(if op == "&&" { "&&" } else { "||" });
        }
    });
    let text = node_text(cond, source);
    let line = text.lines().next().unwrap_or("");
    if line.matches("&&").count() + line.matches("||").count() >= 2 {
        s.compound_condition_count += 1;
    }
}

fn handle_loop(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.track_loop(depth);
    s.track_cogc_branch();
    if let Some(body) = node.child_by_field_name("body") {
        s.cogc_nesting += 1;
        walk_body(body, source, depth + 1, s);
        s.cogc_nesting -= 1;
    }
}

fn handle_call(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let func_name = node.child_by_field_name("function").map(|n| node_text(n, source));
    match func_name {
        Some("switch") => handle_switch(node, source, depth, s),
        Some("tryCatch") => handle_try_catch(node, source, depth, s),
        _ => {
            if let Some(args) = node.child_by_field_name("arguments") {
                walk_body(args, source, depth, s);
            }
        }
    }
}

fn handle_switch(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.track_nesting(depth);
    s.track_cogc_branch();
    s.cogc_nesting += 1;
    let Some(args) = node.child_by_field_name("arguments") else { s.cogc_nesting -= 1; return };
    for_each_argument(args, |child, is_first| {
        if is_first { return; }
        s.cc += 1;
        if let Some(val) = child.child_by_field_name("value") {
            walk_body(val, source, depth + 1, s);
        }
    });
    s.cogc_nesting -= 1;
}

fn handle_try_catch(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let Some(args) = node.child_by_field_name("arguments") else { return };
    for_each_argument(args, |child, is_first| {
        if is_first { walk_body(child, source, depth, s); return; }
        s.cc += 1;
        s.track_cogc_branch();
        let Some(val) = child.child_by_field_name("value") else { return };
        if val.kind() != "function_definition" { walk_body(val, source, depth, s); return; }
        if is_handler_empty(val) { s.empty_catch_count += 1; }
        if let Some(body) = val.child_by_field_name("body") {
            s.cogc_nesting += 1;
            walk_body(body, source, depth + 1, s);
            s.cogc_nesting -= 1;
        }
    });
}

fn for_each_argument(args: Node, mut f: impl FnMut(Node, bool)) {
    let mut cursor = args.walk();
    let mut first = true;
    for child in args.children(&mut cursor) {
        if child.kind() != "argument" { continue; }
        f(child, first);
        first = false;
    }
}

fn is_handler_empty(func: Node) -> bool {
    let Some(body) = func.child_by_field_name("body") else { return true };
    if body.kind() != "braced_expression" { return false; }
    let mut cursor = body.walk();
    let has_content = body.children(&mut cursor).any(|c| !matches!(c.kind(), "comment" | "{" | "}"));
    !has_content
}

fn walk_bool_tree(node: Node, source: &str, on_op: &mut dyn FnMut(&str)) {
    if node.kind() == "binary_operator" {
        if let Some(op) = node.child_by_field_name("operator") {
            let t = node_text(op, source);
            if t == "&&" || t == "||" { on_op(t); }
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() != "function_definition" { walk_bool_tree(child, source, on_op); }
    }
}
