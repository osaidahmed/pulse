mod analysis;

use tree_sitter::{Node, Tree};

use super::counters::count_short_variables;
use super::shared::{self, GlobalMetricsConfig};
use super::{
    compute_assert_fingerprint, compute_skeleton_hash, compute_structural_fingerprint,
    count_code_lines, find_child_by_kind, node_text,
    track_embedded_block, FileMetrics, FunctionMetrics, ModuleMetrics, WalkState,
};
use analysis::{
    braced_in_wordlist, cmd_name, collect_field_accesses, count_boolean_ops,
    count_named_consecutive_asserts, count_switch_arms, walk_cogc,
};

const COMMENT_PREFIXES: &[&str] = &["#"];
const NESTING_BRANCH_KINDS: &[&str] = &["if", "while", "foreach"];
const GLOBAL_CFG: GlobalMetricsConfig = GlobalMetricsConfig {
    cond: &["if"],
    loops: &["while", "foreach"],
    branches: NESTING_BRANCH_KINDS,
    recurse: &["braced_word"],
};

pub fn walk(tree: &Tree, source: &str) -> FileMetrics {
    let root = tree.root_node();
    let total_loc = count_code_lines(source, COMMENT_PREFIXES);
    let mut functions = Vec::new();
    let mut gcc: u32 = 0;
    let mut gmn: u32 = 0;
    collect_functions(root, source, &mut functions, None);
    shared::collect_global_metrics(root, &mut gcc, &mut gmn, &GLOBAL_CFG);
    let total_functions = functions.len() as u32;
    let sum_cc: u32 = functions.iter().map(|f| f.cc).sum();
    let mut cursor = root.walk();
    let declaration_count = root
        .children(&mut cursor)
        .filter(|c| c.kind() == "namespace")
        .count() as u32;
    let module = ModuleMetrics {
        total_loc,
        total_functions,
        sum_cc,
        global_conditional_count: gcc,
        global_max_nesting: gmn,
        declaration_count,
        struct_fields: Vec::new(),
    };
    FileMetrics { functions, module }
}

fn collect_functions(node: Node, source: &str, fns: &mut Vec<FunctionMetrics>, ns: Option<&str>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "procedure" {
            push_proc(child, source, ns, fns);
        } else if child.kind() == "namespace" {
            if let Some((name, body)) = namespace_eval_body(child, source) {
                collect_functions(body, source, fns, Some(name));
            }
        }
    }
}

fn namespace_eval_body<'a>(ns: Node<'a>, source: &'a str) -> Option<(&'a str, Node<'a>)> {
    let wl = find_child_by_kind(ns, "word_list")?;
    let mut cursor = wl.walk();
    let words: Vec<Node> = wl.children(&mut cursor).filter(Node::is_named).collect();
    if words.len() < 3 || node_text(words[0], source) != "eval" { return None; }
    if words[2].kind() != "braced_word" { return None; }
    Some((node_text(words[1], source), words[2]))
}

fn push_proc(node: Node, source: &str, ns: Option<&str>, fns: &mut Vec<FunctionMetrics>) {
    let raw = node.child_by_field_name("name")
        .map_or_else(|| "<anonymous>".into(), |n| node_text(n, source).to_string());
    let Some(mut m) = build_metrics(node, source, raw.clone()) else { return };
    if let Some(ns_name) = ns {
        m.name = format!("{ns_name}.{raw}");
        m.class_name = Some(ns_name.to_string());
        m.is_constructor = matches!(raw.as_str(), "init" | "new" | "constructor");
        if !m.is_constructor {
            collect_field_accesses(node, source, &mut m.field_accesses);
            m.field_accesses.sort();
            m.field_accesses.dedup();
        }
    }
    fns.push(m);
}

#[allow(clippy::unnecessary_wraps)]
fn build_metrics(node: Node, source: &str, name: String) -> Option<FunctionMetrics> {
    let start_line = node.start_position().row as u32 + 1;
    let end_line = node.end_position().row as u32 + 1;
    let body = node.child_by_field_name("body");
    let mut s = WalkState::new();
    if let Some(b) = body { walk_body(b, source, 0, &mut s); }
    let sh = body.map_or(0, compute_structural_fingerprint);
    let sk = body.map_or(0, compute_skeleton_hash);
    let ca = body.map_or(0, count_named_consecutive_asserts);
    let ah = body.map_or(0, |b| compute_assert_fingerprint(b, "command"));
    let sma = body.map_or(0, |b| count_switch_arms(b, source));
    let arg_count = node.child_by_field_name("arguments").map_or(0, |a| {
        let mut c = a.walk();
        a.children(&mut c).filter(|c| matches!(c.kind(), "argument" | "simple_word")).count() as u32
    });
    Some(FunctionMetrics {
        name, start_line, end_line,
        loc: end_line.saturating_sub(start_line) + 1,
        cc: s.cc, cognitive_complexity: s.cogc,
        max_nesting: s.max_nesting, bump_count: s.bump_count,
        arg_count,
        compound_condition_count: s.compound_condition_count,
        is_constructor: false,
        max_embedded_block_loc: s.max_embedded_block_loc,
        structural_hash: sh, skeleton_hash: sk,
        consecutive_asserts: ca, assert_hash: ah,
        primitive_type_count: 0, typed_param_count: 0, max_same_primitive_count: 0,
        empty_catch_count: s.empty_catch_count,
        field_accesses: Vec::new(),
 foreign_field_accesses: Vec::new(),
 class_name: None,
 parent_class: None,
        short_var_count: body.map_or(0, |b| count_short_variables(b, source, &["set"])),
        string_match_arms: sma,
    })
}

fn walk_body(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        dispatch(child, source, depth, s);
    }
}

fn dispatch(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let k = node.kind();
    if k == "if" {
        handle_if(node, source, depth, s);
    } else if matches!(k, "while" | "foreach") {
        handle_loop(node, source, depth, s, last_braced(node));
    } else if matches!(k, "try" | "catch") {
        handle_exception(node, source, depth, s);
    } else if k == "command" {
        dispatch_cmd(node, source, depth, s);
    } else if k != "quoted_word" {
        walk_body(node, source, depth, s);
    } else {
        track_embedded_block(&mut s.max_embedded_block_loc, node);
    }
}

fn handle_exception(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    if node.kind() == "try" { handle_try(node, source, depth, s); return; }
    s.cc += 1;
    s.track_cogc_branch();
    if let Some(b) = find_child_by_kind(node, "braced_word") {
        if braced_empty(b) { s.empty_catch_count += 1; }
    }
}

fn dispatch_cmd(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let name = cmd_name(node, source);
    if name == "for" {
        handle_loop(node, source, depth, s, braced_in_wordlist(node, source).get(3).copied());
        return;
    }
    if name == "switch" { handle_switch(node, source, depth, s); return; }
    walk_wl_bodies(node, source, depth, s);
}

fn handle_if(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.track_if(depth);
    s.track_cogc_branch();
    process_condition(node, source, s);
    walk_consequence(node, source, depth + 1, s);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "elseif" {
            s.cc += 1;
            s.track_cogc_branch();
            process_condition(child, source, s);
            walk_consequence(child, source, depth + 1, s);
        } else if child.kind() == "else" {
            s.track_cogc_flat();
            walk_consequence(child, source, depth + 1, s);
        }
    }
}

#[allow(unused_variables)]
fn handle_loop(node: Node, source: &str, depth: u32, s: &mut WalkState, body: Option<Node>) {
    s.track_loop(depth);
    s.track_cogc_branch();
    let Some(b) = body else { return };
    s.cogc_nesting += 1;
    walk_body(b, source, depth + 1, s);
    s.cogc_nesting -= 1;
}

fn handle_switch(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.track_nesting(depth);
    s.track_cogc_branch();
    s.cogc_nesting += 1;
    for bw in &braced_in_wordlist(node, source) {
        walk_switch_cases(*bw, source, depth, s);
    }
    s.cogc_nesting -= 1;
}

fn walk_switch_cases(bw: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut child_opt = bw.child(0);
    while let Some(child) = child_opt {
        child_opt = child.next_sibling();
        if child.kind() != "command" { continue; }
        if cmd_name(child, source) != "default" { s.cc += 1; }
        walk_wl_bodies(child, source, depth + 1, s);
    }
}

fn handle_try(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    let children: Vec<Node> = node.children(&mut cursor).collect();
    for (i, child) in children.iter().enumerate() {
        if child.kind() == "braced_word" && i == 1 {
            walk_body(*child, source, depth, s);
        } else if child.kind() == "on" {
            try_on_error(&children, i, source, depth, s);
        } else if child.kind() == "finally" {
            if let Some(fb) = find_child_by_kind(*child, "braced_word") {
                walk_body(fb, source, depth, s);
            }
        }
    }
}

fn try_on_error(children: &[Node], i: usize, source: &str, depth: u32, s: &mut WalkState) {
    if i + 1 >= children.len() || children[i + 1].kind() != "error" { return; }
    s.cc += 1;
    s.track_cogc_branch();
    let Some(hb) = children.iter().skip(i + 2).find(|c| c.kind() == "braced_word") else { return };
    if braced_empty(*hb) { s.empty_catch_count += 1; }
    s.cogc_nesting += 1;
    walk_body(*hb, source, depth, s);
    s.cogc_nesting -= 1;
}

fn process_condition(node: Node, source: &str, s: &mut WalkState) {
    let Some(cond) = node.child_by_field_name("condition") else { return };
    count_boolean_ops(cond, &mut s.cc);
    let text = node_text(cond, source);
    let line = text.split('\n').next().unwrap_or("");
    if line.matches("&&").count() + line.matches("||").count() >= 2 {
        s.compound_condition_count += 1;
    }
    let mut last: Option<String> = None;
    walk_cogc(cond, &mut s.cogc, &mut last);
}

fn walk_consequence(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let Some(cons) = node.child_by_field_name("consequence") else { return };
    s.cogc_nesting += 1;
    walk_body(cons, source, depth, s);
    s.cogc_nesting -= 1;
}

fn walk_wl_bodies(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let Some(wl) = find_child_by_kind(node, "word_list") else { return };
    let mut cursor = wl.walk();
    for child in wl.children(&mut cursor) {
        if child.kind() == "braced_word" { walk_body(child, source, depth, s); }
    }
}

fn last_braced(node: Node) -> Option<Node> {
    let mut result = None;
    let mut c = node.child(0);
    while let Some(child) = c {
        c = child.next_sibling();
        if child.kind() == "braced_word" { result = Some(child); }
    }
    result
}

fn braced_empty(node: Node) -> bool {
    let mut cursor = node.walk();
    let has_content = node.children(&mut cursor).any(|c| c.is_named() && c.kind() != "comment");
    !has_content
}
