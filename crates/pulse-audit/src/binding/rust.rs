use std::collections::BTreeSet;

use tree_sitter::Node;

use pulse_syntax::walk::{find_child_by_kind, node_text, DepthGuard};

use crate::binding::extract::{head_of, EnvBuilder};
use crate::binding::TypeEnv;

const NAMED_TYPE_KINDS: &[&str] = &["type_identifier", "scoped_type_identifier", "generic_type"];

const PEEL_KINDS: &[&str] = &["reference_type", "pointer_type"];

const PRIMITIVE_KINDS: &[&str] = &[
    "primitive_type",
    "tuple_type",
    "unit_type",
    "array_type",
    "function_type",
    "never_type",
    "dynamic_type",
    "abstract_type",
];

const SCOPE_BOUNDARIES: &[&str] =
    &["closure_expression", "function_item", "impl_item", "struct_item", "enum_item", "trait_item", "union_item"];

pub fn method_var_types(method_node: Node, source: &str) -> TypeEnv {
    let mut builder = EnvBuilder::default();
    collect_params(method_node, source, &mut builder);
    if let Some(body) = method_node.child_by_field_name("body") {
        collect_locals(body, source, &mut builder);
    }
    let mut tvars = type_param_names(method_node, source);
    tvars.extend(enclosing_impl_type_params(method_node, source));
    builder.into_env(&tvars)
}

fn enclosing_impl_type_params(method_node: Node, source: &str) -> BTreeSet<String> {
    let mut node = method_node.parent();
    let mut depth = 0;
    while let Some(n) = node {
        if depth > 6 {
            break;
        }
        if matches!(n.kind(), "impl_item" | "trait_item") {
            return type_param_names(n, source);
        }
        node = n.parent();
        depth += 1;
    }
    BTreeSet::new()
}

pub fn class_field_types(class_node: Node, source: &str) -> TypeEnv {
    let mut builder = EnvBuilder::default();
    if let Some(body) = find_child_by_kind(class_node, "field_declaration_list") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if child.kind() == "field_declaration" {
                collect_field(child, source, &mut builder);
            }
        }
    }
    builder.into_env(&type_param_names(class_node, source))
}

pub fn class_parents(class_node: Node, source: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    if let Some(tr) = class_node.child_by_field_name("trait") {
        if let Some(name) = type_head_name(tr, source) {
            out.push(name);
        }
    }
    if let Some(bounds) = class_node.child_by_field_name("bounds") {
        collect_bound_names(bounds, source, &mut out);
    }
    out
}

fn collect_bound_names(bounds: Node, source: &str, out: &mut Vec<String>) {
    let mut cursor = bounds.walk();
    for child in bounds.children(&mut cursor) {
        if NAMED_TYPE_KINDS.contains(&child.kind()) {
            if let Some(name) = type_head_name(child, source) {
                out.push(name);
            }
        }
    }
}

fn type_param_names(node: Node, source: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let Some(tps) = node.child_by_field_name("type_parameters") else {
        return names;
    };
    let mut cursor = tps.walk();
    for tp in tps.children(&mut cursor) {
        if tp.kind() == "type_parameter" {
            if let Some(id) = tp.child_by_field_name("name") {
                names.insert(node_text(id, source).to_string());
            }
        }
    }
    names
}

fn collect_params(method_node: Node, source: &str, builder: &mut EnvBuilder) {
    let Some(params) = method_node.child_by_field_name("parameters") else {
        return;
    };
    let mut cursor = params.walk();
    for p in params.children(&mut cursor) {
        if p.kind() == "parameter" {
            bind_typed(p, source, builder);
        }
    }
}

fn collect_locals(node: Node, source: &str, builder: &mut EnvBuilder) {
    let Some(_guard) = DepthGuard::enter() else {
        return;
    };
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        process_local_node(child, source, builder);
    }
}

fn process_local_node(child: Node, source: &str, builder: &mut EnvBuilder) {
    if SCOPE_BOUNDARIES.contains(&child.kind()) {
        return;
    }
    if child.kind() == "let_declaration" {
        bind_typed(child, source, builder);
    }
    collect_locals(child, source, builder);
}

fn collect_field(decl: Node, source: &str, builder: &mut EnvBuilder) {
    let Some(name) = decl.child_by_field_name("name") else {
        return;
    };
    bind_with_name(decl, name, source, builder);
}

fn bind_typed(node: Node, source: &str, builder: &mut EnvBuilder) {
    let Some(pattern) = node.child_by_field_name("pattern") else {
        return;
    };
    if pattern.kind() != "identifier" {
        return;
    }
    bind_with_name(node, pattern, source, builder);
}

fn bind_with_name(node: Node, name: Node, source: &str, builder: &mut EnvBuilder) {
    let Some(ty_node) = node.child_by_field_name("type") else {
        return;
    };
    if let Some(ty) = type_head_name(ty_node, source) {
        builder.bind(node_text(name, source).to_string(), ty);
    }
}

fn type_head_name(node: Node, source: &str) -> Option<String> {
    let inner = peel_type(node);
    let kind = inner.kind();
    if PRIMITIVE_KINDS.contains(&kind) {
        return None;
    }
    if kind == "generic_type" {
        let head = inner.child_by_field_name("type")?;
        return type_head_name(head, source);
    }
    if !NAMED_TYPE_KINDS.contains(&kind) {
        return None;
    }
    head_of(node_text(inner, source))
}

fn peel_type(node: Node) -> Node {
    let mut cur = node;
    while PEEL_KINDS.contains(&cur.kind()) {
        let Some(inner) = cur.child_by_field_name("type") else {
            break;
        };
        cur = inner;
    }
    cur
}
