use tree_sitter::Node;

use crate::cpg::cfg::CfgLang;
use crate::walk::{find_child_by_kind, node_text, DepthGuard};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefUse {
    Def,
    Use,
}

#[derive(Clone, Copy)]
pub(crate) enum Mark {
    Use,
    Assign,
    Decl,
}

const DEF_MARKS: [Mark; 2] = [Mark::Assign, Mark::Decl];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefUseRecord {
    pub name: String,
    pub block: u32,
    pub kind: DefUse,
    pub line: u32,
    pub decl: bool,
}

pub(crate) const DECL_KINDS: &[&str] = &[
    "let_declaration",
    "variable_declarator",
    "init_declarator",
    "var_spec",
    "short_var_declaration",
    "property_declaration",
];

pub(crate) fn collect(node: Node, source: &str, block: u32, lang: &CfgLang, out: &mut Vec<DefUseRecord>) {
    let Some(_g) = DepthGuard::enter() else { return };
    let k = node.kind();
    let ctx = super::nested::FreeCtx { source, block, lang };
    if super::nested::dispatch(k, node, &ctx, out) {
        return;
    }
    if lang.def_kinds.contains(&k) {
        handle_binding(node, source, block, lang, out);
        return;
    }
    if matches!(k, "declaration" | "property_declaration") {
        collect_declaration(node, source, block, lang, out);
        return;
    }
    if matches!(k, "identifier" | "variable_name" | "simple_identifier") {
        if node_text(node, source) != "_" {
            out.push(rec(node, source, block, Mark::Use));
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

fn collect_declaration(node: Node, source: &str, block: u32, lang: &CfgLang, out: &mut Vec<DefUseRecord>) {
    if node.kind() == "property_declaration" {
        collect_property_decl(node, source, block, lang, out);
        return;
    }
    let mut cursor = node.walk();
    for d in node.children_by_field_name("declarator", &mut cursor) {
        if d.kind() == "init_declarator" {
            handle_binding(d, source, block, lang, out);
        } else if let Some(size) = d.child_by_field_name("size") {
            collect(size, source, block, lang, out);
        }
    }
}

fn collect_property_decl(node: Node, source: &str, block: u32, lang: &CfgLang, out: &mut Vec<DefUseRecord>) {
    let mut c = node.walk();
    for child in node.children(&mut c) {
        if child.is_named() && !matches!(child.kind(), "variable_declaration" | "multi_variable_declaration") {
            collect(child, source, block, lang, out);
        }
    }
    if let Some(m) = find_child_by_kind(node, "multi_variable_declaration") {
        push_idents(m, source, block, Mark::Decl, out);
        push_idents(m, source, block, Mark::Use, out);
    } else if let Some(v) = find_child_by_kind(node, "variable_declaration") {
        if v.next_sibling().as_ref().is_some_and(|s| !s.is_extra()) {
            push_idents(v, source, block, Mark::Decl, out);
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
        push_idents(p, source, block, Mark::Decl, out);
    }
    if let Some(it) = binding_right(header) {
        collect(it, source, block, lang, out);
    }
    if let Some(it) = crate::cpg::cfg_nodes::kotlin_for_iterable(header) {
        collect(it, source, block, lang, out);
    }
}

pub(crate) fn seed_hoisted(node: Node, source: &str, entry: u32, lang: &CfgLang, out: &mut Vec<DefUseRecord>) {
    if lang.hoist_kinds.is_empty() {
        return;
    }
    let Some(_g) = DepthGuard::enter() else { return };
    let k = node.kind();
    if lang.nested.fns.contains(&k) || lang.nested.items.contains(&k) {
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

const CASE_PATTERN_KINDS: &[&str] =
    &["pattern", "type_pattern", "record_pattern", "declaration_pattern", "recursive_pattern", "var_pattern"];

pub(crate) fn seed_case_bindings(case: Node, source: &str, entry: u32, out: &mut Vec<DefUseRecord>) {
    seed_children(case, source, entry, out, |c| CASE_PATTERN_KINDS.contains(&c.kind()).then_some(c));
    let mut cursor = case.walk();
    for child in case.children(&mut cursor) {
        if child.kind() == "switch_label" {
            seed_children(child, source, entry, out, |c| CASE_PATTERN_KINDS.contains(&c.kind()).then_some(c));
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
            push_idents(target, source, entry, Mark::Decl, out);
        }
    }
}

pub(crate) fn seed_escaping(node: Node, source: &str, entry: u32, exit: u32, out: &mut Vec<DefUseRecord>) {
    push_idents(node, source, entry, Mark::Decl, out);
    push_idents(node, source, exit, Mark::Use, out);
}

fn seed_hoist_names(node: Node, source: &str, entry: u32, out: &mut Vec<DefUseRecord>) {
    seed_children(node, source, entry, out, |child| {
        child.child_by_field_name("name").filter(|n| !is_destructure_pattern(n.kind()))
    });
}

fn handle_binding(node: Node, source: &str, block: u32, lang: &CfgLang, out: &mut Vec<DefUseRecord>) {
    let Some(r) = binding_right(node) else { return };
    collect(r, source, block, lang, out);
    let aug = lang.aug_kinds.contains(&node.kind())
        || node.child_by_field_name("operator").is_some_and(|op| node_text(op, source) != "=");
    let cfg_gated = node
        .prev_named_sibling()
        .filter(|p| p.kind() == "attribute_item")
        .is_some_and(|p| node_text(p, source).contains("cfg"));
    let def_mark = DEF_MARKS[usize::from(DECL_KINDS.contains(&node.kind()) && !cfg_gated)];
    let mut targets: Vec<Node> = Vec::new();
    collect_binding_targets(node, &mut targets);
    for t in targets {
        if is_field_or_index_target(t.kind()) {
            collect(t, source, block, lang, out);
        } else {
            let mark = if is_destructure_pattern(t.kind()) { Mark::Assign } else { def_mark };
            push_idents(t, source, block, mark, out);
            if aug {
                push_idents(t, source, block, Mark::Use, out);
            }
        }
    }
}

pub(crate) fn collect_binding_targets<'a>(node: Node<'a>, out: &mut Vec<Node<'a>>) {
    for field in ["left", "pattern", "name", "declarator", "target"] {
        let mut cursor = node.walk();
        let mut any = false;
        for child in node.children_by_field_name(field, &mut cursor) {
            any = true;
            if matches!(child.kind(), "expression_list" | "directly_assignable_expression" | "left_assignment_list") {
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

fn is_destructure_pattern(kind: &str) -> bool {
    matches!(kind, "array_pattern" | "object_pattern" | "tuple_pattern" | "list_pattern")
}

pub(crate) fn is_field_or_index_target(kind: &str) -> bool {
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
            | "dynamic_variable_name"
            | "pointer_expression"
            | "unary_expression"
            | "navigation_expression"
            | "call_expression"
            | "call"
            | "element_reference"
            | "scope_resolution"
    )
}

pub(crate) fn seed_params(fn_node: Node, source: &str, entry: u32, out: &mut Vec<DefUseRecord>) {
    let params = fn_node
        .child_by_field_name("parameters")
        .or_else(|| fn_node.child_by_field_name("declarator").and_then(|d| d.child_by_field_name("parameters")))
        .or_else(|| find_child_by_kind(fn_node, "function_value_parameters"));
    if let Some(params) = params {
        push_idents(params, source, entry, Mark::Decl, out);
    } else {
        let mut cursor = fn_node.walk();
        for p in fn_node.children(&mut cursor).filter(|n| n.kind() == "parameter") {
            push_idents(p, source, entry, Mark::Decl, out);
        }
    }
}

fn first_field<'a>(node: Node<'a>, fields: &[&str]) -> Option<Node<'a>> {
    fields.iter().find_map(|f| node.child_by_field_name(f))
}

fn binding_left(node: Node) -> Option<Node> {
    first_field(node, &["left", "pattern", "name", "declarator", "item"])
}

fn binding_right(node: Node) -> Option<Node> {
    first_field(node, &["right", "value", "result", "collection"]).or_else(|| initializer_child(node))
}

fn initializer_child(node: Node) -> Option<Node> {
    if node.kind() != "variable_declarator" {
        return None;
    }
    let name_id = node.child_by_field_name("name").map(|n| n.id());
    let mut cursor = node.walk();
    let mut found = None;
    for child in node.children(&mut cursor) {
        if child.is_named() && Some(child.id()) != name_id && child.kind() != "type_annotation" {
            found = Some(child);
        }
    }
    found
}

pub(crate) fn push_idents(node: Node, source: &str, block: u32, mark: Mark, out: &mut Vec<DefUseRecord>) {
    let Some(_g) = DepthGuard::enter() else { return };
    if matches!(node.kind(), "identifier" | "variable_name" | "simple_identifier" | "shorthand_field_identifier") {
        if node_text(node, source) != "_" {
            out.push(rec(node, source, block, mark));
        }
        return;
    }
    if matches!(node.kind(), "user_type" | "function_type") {
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.is_named() {
            push_idents(child, source, block, mark, out);
        }
    }
}

pub(crate) fn rec(node: Node, source: &str, block: u32, mark: Mark) -> DefUseRecord {
    let (kind, decl) = match mark {
        Mark::Use => (DefUse::Use, false),
        Mark::Assign => (DefUse::Def, false),
        Mark::Decl => (DefUse::Def, true),
    };
    DefUseRecord {
        name: node_text(node, source).to_string(),
        block,
        kind,
        line: node.start_position().row as u32 + 1,
        decl,
    }
}
