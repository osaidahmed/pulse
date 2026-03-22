use tree_sitter::Node;

use super::complexity::{check_compound, count_bool_ops, count_cogc_sequences, is_otherwise_guard};
use super::super::{find_child_by_kind, track_embedded_block, WalkState};

const SCOPE_BOUNDARY: &[&str] = &["lambda", "lambda_case"];

pub fn walk_function_matches(func_node: Node, source: &str, depth: u32, s: &mut WalkState) {
    for_children_of_kind(func_node, "match", |child| walk_match(child, source, depth, s));
}

pub fn walk_body(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    s.reset_bump();
    for child in node.children(&mut cursor) {
        walk_node(child, source, depth, s);
    }
}

fn walk_node(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    let kind = child.kind();
    if SCOPE_BOUNDARY.contains(&kind) {
        return;
    }
    if kind == "literal" || kind == "string" {
        check_string_embed(child, s);
        return;
    }
    match kind {
        "conditional" => handle_branching(child, source, depth, s, true),
        "case" => handle_branching(child, source, depth, s, false),
        "do" => walk_do_stmts(child, source, depth, s),
        "let_in" => walk_let_in(child, source, depth, s),
        _ => walk_body(child, source, depth, s),
    }
}

fn check_string_embed(node: Node, s: &mut WalkState) {
    if node.kind() == "string" {
        track_embedded_block(&mut s.max_embedded_block_loc, node);
        return;
    }
    for_children_of_kind(node, "string", |c| {
        track_embedded_block(&mut s.max_embedded_block_loc, c);
    });
}

fn handle_branching(node: Node, source: &str, depth: u32, s: &mut WalkState, is_if: bool) {
    if is_if {
        s.track_if(depth);
    } else {
        s.track_nesting(depth);
    }
    s.track_cogc_branch();
    let saved = s.cogc_nesting;
    s.cogc_nesting += 1;
    if is_if {
        walk_conditional_parts(node, source, depth, s);
    } else {
        walk_case_alternatives(node, source, depth, s);
    }
    s.cogc_nesting = saved;
}

fn walk_conditional_parts(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = node.walk();
    let mut phase = 0u8;
    for child in node.children(&mut cursor) {
        phase = advance_conditional(child, phase, source, depth, s);
    }
}

fn advance_conditional(child: Node, phase: u8, source: &str, depth: u32, s: &mut WalkState) -> u8 {
    match child.kind() {
        "if" => 1,
        "then" => 2,
        "else" => { s.track_cogc_flat(); 3 }
        _ if phase == 1 => { analyze_condition(child, source, s); 2 }
        _ if phase == 2 => { walk_node(child, source, depth + 1, s); phase }
        _ if phase == 3 => { walk_else(child, source, depth, s); phase }
        _ => phase,
    }
}

fn analyze_condition(node: Node, source: &str, s: &mut WalkState) {
    count_bool_ops(node, source, &mut s.cc);
    count_cogc_sequences(node, source, &mut s.cogc);
    check_compound(node, source, &mut s.compound_condition_count);
}

fn walk_else(child: Node, source: &str, depth: u32, s: &mut WalkState) {
    if child.kind() != "conditional" {
        walk_node(child, source, depth + 1, s);
        return;
    }
    s.cc += 1;
    s.track_cogc_branch();
    walk_conditional_parts(child, source, depth, s);
}

fn walk_case_alternatives(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let Some(alts) = find_child_by_kind(node, "alternatives") else { return };
    for_children_of_kind(alts, "alternative", |alt| {
        if !is_wildcard_alternative(alt) {
            s.cc += 1;
        }
        s.reset_bump();
        for_children_of_kind(alt, "match", |m| walk_match(m, source, depth + 1, s));
    });
}

fn is_wildcard_alternative(alt: Node) -> bool {
    let mut cursor = alt.walk();
    let result = alt.children(&mut cursor)
        .find(|c| !matches!(c.kind(), "match" | "->" | "|" | "guards" | "where" | "local_binds"))
        .is_some_and(|p| p.kind() == "wildcard");
    result
}

fn walk_match(match_node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut gcursor = match_node.walk();
    let has_guards = match_node.children(&mut gcursor).any(|c| c.kind() == "guards");
    drop(gcursor);
    let mut cursor = match_node.walk();
    for child in match_node.children(&mut cursor) {
        match child.kind() {
            "guards" if has_guards => {
                walk_guards(child, source, s);
                walk_match_rhs(match_node, source, depth, s);
            }
            "|" | "=" | "->" | "guards" => {}
            _ if !has_guards => walk_node(child, source, depth, s),
            _ => {}
        }
    }
}

fn walk_guards(guards_node: Node, source: &str, s: &mut WalkState) {
    for_children_of_kind(guards_node, "boolean", |child| {
        if !is_otherwise_guard(child, source) {
            s.cc += 1;
            s.track_cogc_branch();
        }
        count_bool_ops(child, source, &mut s.cc);
        count_cogc_sequences(child, source, &mut s.cogc);
        check_compound(child, source, &mut s.compound_condition_count);
    });
}

fn walk_match_rhs(match_node: Node, source: &str, depth: u32, s: &mut WalkState) {
    let mut cursor = match_node.walk();
    let mut past_eq = false;
    for child in match_node.children(&mut cursor) {
        if child.kind() == "=" || child.kind() == "->" {
            past_eq = true;
        } else if past_eq && child.kind() != "guards" {
            walk_node(child, source, depth, s);
        }
    }
}

fn walk_do_stmts(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    const DO_KINDS: &[&str] = &["exp", "bind", "let", "rec"];
    for_matching_children(node, |k| DO_KINDS.contains(&k), |child| {
        walk_body(child, source, depth, s);
    });
}

fn walk_let_in(node: Node, source: &str, depth: u32, s: &mut WalkState) {
    for_matching_children(node, |k| k != "let" && k != "in", |child| {
        walk_node(child, source, depth, s);
    });
}

fn for_children_of_kind(node: Node, kind: &str, f: impl FnMut(Node)) {
    for_matching_children(node, |k| k == kind, f);
}

fn for_matching_children(node: Node, predicate: impl Fn(&str) -> bool, mut f: impl FnMut(Node)) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if predicate(child.kind()) {
            f(child);
        }
    }
}
