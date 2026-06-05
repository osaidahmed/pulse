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

pub(crate) fn collect(
    node: Node,
    source: &str,
    block: u32,
    lang: &CfgLang,
    out: &mut Vec<DefUseRecord>,
) {
    let Some(_g) = DepthGuard::enter() else { return };
    let k = node.kind();
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

pub(crate) fn loop_header(
    node: Node,
    source: &str,
    block: u32,
    lang: &CfgLang,
    out: &mut Vec<DefUseRecord>,
) {
    if let Some(c) = node.child_by_field_name("condition") {
        collect(c, source, block, lang, out);
    }
    if let Some(p) = binding_left(node) {
        push_idents(p, source, block, DefUse::Def, out);
    }
    if let Some(it) = binding_right(node) {
        collect(it, source, block, lang, out);
    }
}

fn handle_binding(node: Node, source: &str, block: u32, lang: &CfgLang, out: &mut Vec<DefUseRecord>) {
    if let Some(l) = binding_left(node) {
        if is_field_or_index_target(l.kind()) {
            collect(l, source, block, lang, out);
        } else {
            push_idents(l, source, block, DefUse::Def, out);
            if lang.aug_kinds.contains(&node.kind()) {
                push_idents(l, source, block, DefUse::Use, out);
            }
        }
    }
    if let Some(r) = binding_right(node) {
        collect(r, source, block, lang, out);
    }
}

fn is_field_or_index_target(kind: &str) -> bool {
    matches!(kind, "attribute" | "subscript" | "field_expression" | "index_expression")
}

pub(crate) fn seed_params(fn_node: Node, source: &str, entry: u32, out: &mut Vec<DefUseRecord>) {
    if let Some(params) = fn_node.child_by_field_name("parameters") {
        push_idents(params, source, entry, DefUse::Def, out);
    }
}

fn binding_left(node: Node) -> Option<Node> {
    node.child_by_field_name("left").or_else(|| node.child_by_field_name("pattern"))
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
    DefUseRecord {
        name: node_text(node, source).to_string(),
        block,
        kind,
        line: node.start_position().row as u32 + 1,
    }
}
