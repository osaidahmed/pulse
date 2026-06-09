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
    if matches!(k, "identifier" | "variable_name") {
        if node_text(node, source) != "_" {
            out.push(rec(node, source, block, DefUse::Use));
        }
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
    let mut hc = node.walk();
    let header = node.children(&mut hc).find(|c| matches!(c.kind(), "for_clause" | "range_clause")).unwrap_or(node);
    let mut cursor = header.walk();
    for cond in header.children_by_field_name("condition", &mut cursor) {
        collect(cond, source, block, lang, out);
    }
    for field in ["initializer", "increment", "update"] {
        if let Some(n) = header.child_by_field_name(field) {
            collect(n, source, block, lang, out);
        }
    }
    if let Some(p) = binding_left(header) {
        push_idents(p, source, block, DefUse::Def, out);
    }
    if let Some(it) = binding_right(header) {
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

pub(crate) fn seed_case_bindings(case: Node, source: &str, entry: u32, out: &mut Vec<DefUseRecord>) {
    seed_children(case, source, entry, out, pick_case_pattern);
    let mut cursor = case.walk();
    for child in case.children(&mut cursor) {
        if child.kind() == "switch_label" {
            seed_children(child, source, entry, out, pick_case_pattern);
        }
    }
}

fn seed_children(
    node: Node,
    source: &str,
    entry: u32,
    out: &mut Vec<DefUseRecord>,
    pick: impl Fn(Node) -> Option<Node>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(target) = pick(child) {
            push_idents(target, source, entry, DefUse::Def, out);
        }
    }
}

fn pick_case_pattern(child: Node) -> Option<Node> {
    is_case_pattern(child.kind()).then_some(child)
}

fn is_case_pattern(kind: &str) -> bool {
    matches!(
        kind,
        "pattern" | "type_pattern" | "record_pattern" | "declaration_pattern" | "recursive_pattern" | "var_pattern"
    )
}

fn seed_hoist_names(node: Node, source: &str, entry: u32, out: &mut Vec<DefUseRecord>) {
    seed_children(node, source, entry, out, |child| {
        child.child_by_field_name("name").filter(|n| !is_destructure_pattern(n.kind()))
    });
}

fn handle_binding(node: Node, source: &str, block: u32, lang: &CfgLang, out: &mut Vec<DefUseRecord>) {
    let Some(r) = binding_right(node) else { return };
    collect(r, source, block, lang, out);
    let aug = is_augmented(node, source, lang);
    let mut targets: Vec<Node> = Vec::new();
    collect_binding_targets(node, &mut targets);
    for t in targets {
        if is_field_or_index_target(t.kind()) {
            collect(t, source, block, lang, out);
        } else if !is_destructure_pattern(t.kind()) {
            push_idents(t, source, block, DefUse::Def, out);
            if aug {
                push_idents(t, source, block, DefUse::Use, out);
            }
        }
    }
}

fn collect_binding_targets<'a>(node: Node<'a>, out: &mut Vec<Node<'a>>) {
    for field in ["left", "pattern", "name"] {
        let mut cursor = node.walk();
        let mut any = false;
        for child in node.children_by_field_name(field, &mut cursor) {
            any = true;
            if child.kind() == "expression_list" {
                let mut inner = child.walk();
                out.extend(child.children(&mut inner).filter(tree_sitter::Node::is_named));
            } else {
                out.push(child);
            }
        }
        if any {
            return;
        }
    }
}

pub(crate) fn seed_defs(node: Node, source: &str, block: u32, out: &mut Vec<DefUseRecord>) {
    push_idents(node, source, block, DefUse::Def, out);
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
            | "member_access_expression"
            | "element_access_expression"
            | "selector_expression"
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
    node.child_by_field_name("right").or_else(|| node.child_by_field_name("value")).or_else(|| initializer_child(node))
}

fn initializer_child(node: Node) -> Option<Node> {
    if node.kind() != "variable_declarator" {
        return None;
    }
    let name_id = node.child_by_field_name("name").map(|n| n.id());
    let mut cursor = node.walk();
    let mut found = None;
    for child in node.children(&mut cursor) {
        if child.is_named() && Some(child.id()) != name_id {
            found = Some(child);
        }
    }
    found
}

fn push_idents(node: Node, source: &str, block: u32, kind: DefUse, out: &mut Vec<DefUseRecord>) {
    let Some(_g) = DepthGuard::enter() else { return };
    if matches!(node.kind(), "identifier" | "variable_name") {
        if node_text(node, source) != "_" {
            out.push(rec(node, source, block, kind));
        }
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
