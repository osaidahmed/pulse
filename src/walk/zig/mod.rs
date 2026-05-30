mod methods;

use tree_sitter::{Node, Tree};

use super::shared::{self, count_boolean_ops, count_cogc_sequences, GlobalMetricsConfig};
use super::{
    count_code_lines, find_child_by_kind, node_text, track_embedded_block, FileMetrics,
    FunctionMetrics, ModuleMetrics, WalkState,
};
use methods::{analyze_function, analyze_test, try_collect_struct_methods};

const COMMENT_PREFIXES: &[&str] = &["//", "/*", "*"];
const NESTING_BRANCH_KINDS: &[&str] = &[
    "if_statement",
    "for_statement",
    "while_statement",
    "switch_expression",
];
const BOOL_OPS: &[&str] = &["and", "or"];
const BOOL_STOPS: &[&str] = &[
    "block",
    "function_declaration",
    "test_declaration",
    "struct_declaration",
];
const GLOBAL_CFG: GlobalMetricsConfig = GlobalMetricsConfig {
    cond: &["if_statement"],
    loops: &["while_statement", "for_statement"],
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
    shared::collect_global_metrics(
        root,
        &mut global_conditional_count,
        &mut global_max_nesting,
        &GLOBAL_CFG,
    );

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
    for child in node.children(&mut cursor) { dispatch_top_level(child, source, functions); }
}

fn dispatch_top_level(child: Node, source: &str, functions: &mut Vec<FunctionMetrics>) {
    let result = match child.kind() {
        "function_declaration" => analyze_function(child, source),
        "test_declaration" => analyze_test(child, source),
        "variable_declaration" => { try_collect_struct_methods(child, source, functions); return; }
        _ => return,
    };
    if let Some(m) = result { functions.push(m); }
}

// ─── Body walking ──────────────────────────────────────────────────────

pub(super) fn walk_body_pub(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    walk_body(node, source, depth, s);
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
    (&["labeled_statement"], walk_labeled_statement),
    (&["for_statement", "while_statement"], handle_loop),
    (&["switch_expression"], handle_switch),
    (&["catch_expression"], handle_catch),
    (&["binary_expression"], handle_binary),
    (&["defer_statement", "errdefer_statement", "labeled_type_expression"], walk_body),
];

const STRING_KINDS: &[&str] = &["string", "multiline_string"];

fn walk_node(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    let kind = child.kind();
    if STRING_KINDS.contains(&kind) {
        track_embedded_block(&mut s.max_embedded_block_loc, child);
        return;
    }
    if kind == "comptime_statement" { return; }
    for (kinds, handler) in NODE_HANDLERS {
        if kinds.contains(&kind) {
            handler(child, source, depth, s);
            return;
        }
    }
    walk_body(child, source, depth, s);
}

fn walk_labeled_statement(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let inner = find_child_by_kind(node, "for_statement")
        .or_else(|| find_child_by_kind(node, "while_statement"));
    if let Some(loop_node) = inner {
        handle_loop(loop_node, source, depth, s);
    } else {
        walk_body(node, source, depth, s);
    }
}

fn handle_binary(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();
    let has_orelse = children.iter().any(|c| c.kind() == "orelse");
    if has_orelse {
        s.cc += 1;
        s.track_cogc_branch();
    }
    for child in children {
        walk_node(child, source, depth, s);
    }
}

fn handle_catch(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.cc += 1;
    s.track_cogc_branch();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "block" {
            let saved = s.cogc_nesting;
            s.cogc_nesting += 1;
            walk_body(child, source, depth + 1, s);
            s.cogc_nesting = saved;
        }
    }
}

fn handle_if(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.track_if(depth);
    s.track_cogc_branch();
    count_boolean_ops(node, &mut s.cc, BOOL_OPS, BOOL_STOPS);
    count_cogc_sequences(node, &mut s.cogc, BOOL_OPS, BOOL_STOPS);
    check_condition_complexity(node, source, &mut s.compound_condition_count);
    walk_if_children(node, source, depth + 1, s);
}

fn walk_if_children(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    node.children(&mut cursor).for_each(|child| match child.kind() {
        "block_expression" => {
            let saved = s.cogc_nesting;
            s.cogc_nesting += 1;
            walk_body(child, source, depth, s);
            s.cogc_nesting = saved;
        }
        "else_clause" => walk_else_clause(child, source, depth, s),
        _ => {}
    });
}

fn walk_else_clause(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "if_statement" => {
                s.cc += 1;
                s.track_cogc_branch();
                count_boolean_ops(child, &mut s.cc, BOOL_OPS, BOOL_STOPS);
                count_cogc_sequences(child, &mut s.cogc, BOOL_OPS, BOOL_STOPS);
                check_condition_complexity(child, source, &mut s.compound_condition_count);
                walk_if_children(child, source, depth, s);
            }
            "labeled_statement" | "block_expression" => {
                s.track_cogc_flat();
                let saved = s.cogc_nesting;
                s.cogc_nesting += 1;
                walk_body(child, source, depth, s);
                s.cogc_nesting = saved;
            }
            _ => {}
        }
    }
}

fn handle_loop(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.track_loop(depth);
    s.track_cogc_branch();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "block_expression" || child.kind() == "block" {
            let saved = s.cogc_nesting;
            s.cogc_nesting += 1;
            walk_body(child, source, depth + 1, s);
            s.cogc_nesting = saved;
        }
    }
}

fn handle_switch(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    s.track_nesting(depth);
    s.track_cogc_branch();
    let saved = s.cogc_nesting;
    s.cogc_nesting += 1;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() != "switch_case" {
            continue;
        }
        let mut dc_cursor = child.walk();
        let is_default = child.children(&mut dc_cursor).any(|c| c.kind() == "else");
        if !is_default {
            s.cc += 1;
        }
        walk_case_body(child, source, depth + 1, s);
    }
    s.cogc_nesting = saved;
}

fn walk_case_body(case: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut child_opt = case.child(0);
    while let Some(child) = child_opt {
        child_opt = child.next_sibling();
        match child.kind() {
            "block" | "block_expression" => walk_body(child, source, depth, s),
            "if_statement" | "binary_expression" | "call_expression" | "return_expression"
            | "catch_expression" | "switch_expression" => {
                walk_node(child, source, depth, s);
            }
            _ => {}
        }
    }
}

fn check_condition_complexity(node: Node, source: &str, compound_conditions: &mut u32) {
    let text = node_text(node, source);
    let cond_text = text.split('{').next().unwrap_or("");
    let ops = cond_text.matches(" and ").count() + cond_text.matches(" or ").count();
    if ops >= 2 {
        *compound_conditions += 1;
    }
}

fn count_declarations(root: Node) -> u32 {
    let mut count: u32 = 0;
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if child.kind() == "variable_declaration"
            && (find_child_by_kind(child, "struct_declaration").is_some()
                || find_child_by_kind(child, "enum_declaration").is_some()
                || find_child_by_kind(child, "union_declaration").is_some()) {
            count += 1;
        }
    }
    count
}

