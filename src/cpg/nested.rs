use std::collections::HashSet;

use tree_sitter::Node;

use crate::cpg::cfg::CfgLang;
use crate::cpg::defuse::{collect_binding_targets, is_field_or_index_target, rec, DefUseRecord, Mark, DECL_KINDS};
use crate::walk::{node_text, DepthGuard};

pub(crate) struct FreeCtx<'a> {
    pub source: &'a str,
    pub block: u32,
    pub lang: &'a CfgLang,
}

pub(crate) fn dispatch(kind: &str, node: Node, ctx: &FreeCtx, out: &mut Vec<DefUseRecord>) -> bool {
    if ctx.lang.nested.items.contains(&kind) {
        return true;
    }
    if ctx.lang.nested.fns.contains(&kind) {
        collect_free_uses(node, ctx, out);
        return true;
    }
    false
}

pub(crate) fn captured_names(body: Node, source: &str, lang: &CfgLang) -> HashSet<String> {
    let ctx = FreeCtx { source, block: 0, lang };
    let mut recs = Vec::new();
    walk_for_nested(body, &ctx, &mut recs);
    recs.into_iter().map(|r| r.name).collect()
}

fn walk_for_nested(node: Node, ctx: &FreeCtx, out: &mut Vec<DefUseRecord>) {
    let Some(_g) = DepthGuard::enter() else { return };
    let k = node.kind();
    if ctx.lang.nested.items.contains(&k) {
        return;
    }
    if ctx.lang.nested.fns.contains(&k) {
        collect_free_uses(node, ctx, out);
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.is_named() {
            walk_for_nested(child, ctx, out);
        }
    }
}

fn collect_free_uses(node: Node, ctx: &FreeCtx, out: &mut Vec<DefUseRecord>) {
    let Some(_g) = DepthGuard::enter() else { return };
    let bound = scope_bound(node, ctx);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.is_named() {
            push_free_idents(child, ctx, &bound, out);
        }
    }
}

fn scope_bound(node: Node, ctx: &FreeCtx) -> HashSet<String> {
    let mut bound = HashSet::new();
    extend_scope_bound(node, ctx, &mut bound);
    bound
}

fn extend_scope_bound(node: Node, ctx: &FreeCtx, bound: &mut HashSet<String>) {
    seed_implicit(node, bound);
    collect_params(node, ctx.source, bound);
    collect_locals(node, ctx, bound);
}

fn collect_params(node: Node, source: &str, bound: &mut HashSet<String>) {
    if let Some(p) = node.child_by_field_name("parameter") {
        collect_ident_names(p, source, bound);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind().contains("param") {
            collect_ident_names(child, source, bound);
        }
    }
}

fn collect_locals(node: Node, ctx: &FreeCtx, bound: &mut HashSet<String>) {
    let Some(_g) = DepthGuard::enter() else { return };
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let k = child.kind();
        if !child.is_named() || is_nested_scope(ctx.lang, k) {
            continue;
        }
        if DECL_KINDS.contains(&k) {
            add_binding_names(child, ctx.source, bound);
        }
        collect_locals(child, ctx, bound);
    }
}

fn add_binding_names(node: Node, source: &str, bound: &mut HashSet<String>) {
    let mut targets = Vec::new();
    collect_binding_targets(node, &mut targets);
    for t in &targets {
        if !is_field_or_index_target(t.kind()) {
            collect_ident_names(*t, source, bound);
        }
    }
}

fn is_nested_scope(lang: &CfgLang, kind: &str) -> bool {
    lang.nested.fns.contains(&kind) || lang.nested.items.contains(&kind)
}

fn collect_ident_names(node: Node, source: &str, bound: &mut HashSet<String>) {
    let Some(_g) = DepthGuard::enter() else { return };
    let k = node.kind();
    if matches!(k, "identifier" | "variable_name" | "simple_identifier") {
        let t = node_text(node, source);
        if t != "_" {
            bound.insert(t.to_string());
        }
        return;
    }
    if matches!(k, "user_type" | "function_type") {
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.is_named() {
            collect_ident_names(child, source, bound);
        }
    }
}

fn push_free_idents(node: Node, ctx: &FreeCtx, bound: &HashSet<String>, out: &mut Vec<DefUseRecord>) {
    let Some(_g) = DepthGuard::enter() else { return };
    let k = node.kind();
    if matches!(k, "identifier" | "variable_name" | "simple_identifier") {
        let t = node_text(node, ctx.source);
        if is_free_use(t, bound) {
            out.push(rec(node, ctx.source, ctx.block, Mark::Use));
        }
        return;
    }
    if matches!(k, "user_type" | "function_type") || ctx.lang.nested.items.contains(&k) {
        return;
    }
    if ctx.lang.nested.fns.contains(&k) {
        push_nested_free(node, ctx, bound, out);
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.is_named() {
            push_free_idents(child, ctx, bound, out);
        }
    }
}

fn is_free_use(name: &str, bound: &HashSet<String>) -> bool {
    name != "_" && !is_closure_arg(name) && !bound.contains(name)
}

fn push_nested_free(node: Node, ctx: &FreeCtx, bound: &HashSet<String>, out: &mut Vec<DefUseRecord>) {
    let mut inner = bound.clone();
    extend_scope_bound(node, ctx, &mut inner);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.is_named() {
            push_free_idents(child, ctx, &inner, out);
        }
    }
}

fn seed_implicit(node: Node, bound: &mut HashSet<String>) {
    if node.kind() == "lambda_literal" {
        bound.insert("it".to_string());
    }
}

fn is_closure_arg(name: &str) -> bool {
    name.starts_with('$') && name.len() > 1 && name[1..].bytes().all(|b| b.is_ascii_digit())
}
