use tree_sitter::Node;

use crate::cpg::cfg::CfgLang;
use crate::walk::{node_text, DepthGuard};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefUse {
    Def,
    Use,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefUseRecord {
    pub name: String,
    pub block: u32,
    pub kind: DefUse,
    pub line: u32,
}

pub(crate) fn collect(node: Node, source: &str, block: u32, lang: &CfgLang, out: &mut Vec<DefUseRecord>) {
    let Some(_g) = DepthGuard::enter() else { return };
    let k = node.kind();
    if lang.nested_fn_kinds.contains(&k) {
        push_idents(node, source, block, DefUse::Use, out);
        return;
    }
    if lang.def_kinds.contains(&k) {
        handle_binding(node, source, block, lang, out);
        return;
    }
    if k == "identifier" {
        out.push(rec(node, source, block, DefUse::Use));
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.is_named() {
            collect(child, source, block, lang, out);
        }
    }
}

pub(crate) fn loop_header(node: Node, source: &str, block: u32, lang: &CfgLang, out: &mut Vec<DefUseRecord>) {
    let mut cursor = node.walk();
    for cond in node.children_by_field_name("condition", &mut cursor) {
        collect(cond, source, block, lang, out);
    }
    for field in ["initializer", "increment"] {
        if let Some(n) = node.child_by_field_name(field) {
            collect(n, source, block, lang, out);
        }
    }
    if let Some(p) = binding_left(node) {
        push_idents(p, source, block, DefUse::Def, out);
    }
    if let Some(it) = binding_right(node) {
        collect(it, source, block, lang, out);
    }
}

pub(crate) fn seed_hoisted(node: Node, source: &str, entry: u32, lang: &CfgLang, out: &mut Vec<DefUseRecord>) {
    if lang.hoist_kinds.is_empty() {
        return;
    }
    let Some(_g) = DepthGuard::enter() else { return };
    let k = node.kind();
    if lang.nested_fn_kinds.contains(&k) {
        return;
    }
    if lang.hoist_kinds.contains(&k) {
        seed_hoist_names(node, source, entry, out);
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.is_named() {
            seed_hoisted(child, source, entry, lang, out);
        }
    }
}

fn seed_hoist_names(node: Node, source: &str, entry: u32, out: &mut Vec<DefUseRecord>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let Some(name) = child.child_by_field_name("name") else { continue };
        if !is_destructure_pattern(name.kind()) {
            push_idents(name, source, entry, DefUse::Def, out);
        }
    }
}

fn handle_binding(node: Node, source: &str, block: u32, lang: &CfgLang, out: &mut Vec<DefUseRecord>) {
    let right = binding_right(node);
    if let Some(l) = binding_left(node) {
        if is_field_or_index_target(l.kind()) {
            collect(l, source, block, lang, out);
        } else if right.is_some() && !is_destructure_pattern(l.kind()) {
            push_idents(l, source, block, DefUse::Def, out);
            if is_augmented(node, source, lang) {
                push_idents(l, source, block, DefUse::Use, out);
            }
        }
    }
    if let Some(r) = right {
        collect(r, source, block, lang, out);
    }
}

fn is_augmented(node: Node, source: &str, lang: &CfgLang) -> bool {
    lang.aug_kinds.contains(&node.kind())
        || node.child_by_field_name("operator").is_some_and(|op| node_text(op, source) != "=")
}

fn is_field_or_index_target(kind: &str) -> bool {
    matches!(
        kind,
        "attribute"
            | "subscript"
            | "subscript_expression"
            | "field_expression"
            | "index_expression"
            | "member_expression"
            | "field_access"
            | "array_access"
    )
}

fn is_destructure_pattern(kind: &str) -> bool {
    matches!(kind, "array_pattern" | "object_pattern" | "tuple_pattern" | "list_pattern")
}

pub(crate) fn seed_params(fn_node: Node, source: &str, entry: u32, out: &mut Vec<DefUseRecord>) {
    if let Some(params) = fn_node.child_by_field_name("parameters") {
        push_idents(params, source, entry, DefUse::Def, out);
    }
}

fn binding_left(node: Node) -> Option<Node> {
    node.child_by_field_name("left")
        .or_else(|| node.child_by_field_name("pattern"))
        .or_else(|| node.child_by_field_name("name"))
}

fn binding_right(node: Node) -> Option<Node> {
    node.child_by_field_name("right").or_else(|| node.child_by_field_name("value"))
}

fn push_idents(node: Node, source: &str, block: u32, kind: DefUse, out: &mut Vec<DefUseRecord>) {
    let Some(_g) = DepthGuard::enter() else { return };
    if node.kind() == "identifier" {
        out.push(rec(node, source, block, kind));
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.is_named() {
            push_idents(child, source, block, kind, out);
        }
    }
}

fn rec(node: Node, source: &str, block: u32, kind: DefUse) -> DefUseRecord {
    DefUseRecord { name: node_text(node, source).to_string(), block, kind, line: node.start_position().row as u32 + 1 }
}
