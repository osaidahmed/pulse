use std::collections::BTreeSet;

use tree_sitter::Node;

use crate::walk::{find_child_by_kind, node_text, DepthGuard};

use super::binding::TypeEnv;
use super::binding_extract::{head_of, EnvBuilder};

const TYPE_KINDS: &[&str] = &["type_identifier", "qualified_type", "pointer_type", "generic_type"];

const PRIMITIVE_NAMES: &[&str] = &[
    "int",
    "int8",
    "int16",
    "int32",
    "int64",
    "uint",
    "uint8",
    "uint16",
    "uint32",
    "uint64",
    "uintptr",
    "float32",
    "float64",
    "complex64",
    "complex128",
    "string",
    "bool",
    "byte",
    "rune",
    "error",
    "any",
];

const SCOPE_BOUNDARIES: &[&str] = &["func_literal", "function_declaration", "method_declaration"];

pub fn method_var_types(method_node: Node, source: &str) -> TypeEnv {
    let mut builder = EnvBuilder::default();
    if let Some(recv) = method_node.child_by_field_name("receiver") {
        collect_param_list(recv, source, &mut builder);
    }
    if let Some(params) = method_node.child_by_field_name("parameters") {
        collect_param_list(params, source, &mut builder);
    }
    if let Some(body) = method_node.child_by_field_name("body") {
        collect_locals(body, source, &mut builder);
    }
    builder.into_env(&type_param_names(method_node, source))
}

pub fn class_field_types(_class_node: Node, _source: &str) -> TypeEnv {
    TypeEnv::new()
}

pub fn class_parents(_class_node: Node, _source: &str) -> Vec<String> {
    Vec::new()
}

fn type_param_names(method_node: Node, source: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let Some(tps) = method_node.child_by_field_name("type_parameters") else {
        return names;
    };
    let mut cursor = tps.walk();
    for decl in tps.children(&mut cursor) {
        if decl.kind() == "type_parameter_declaration" {
            if let Some(id) = find_child_by_kind(decl, "identifier") {
                names.insert(node_text(id, source).to_string());
            }
        }
    }
    names
}

fn collect_param_list(list: Node, source: &str, builder: &mut EnvBuilder) {
    let mut cursor = list.walk();
    for decl in list.children(&mut cursor) {
        if decl.kind() == "parameter_declaration" {
            bind_names_before_type(decl, source, builder);
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
    let kind = child.kind();
    if SCOPE_BOUNDARIES.contains(&kind) {
        return;
    }
    if kind == "var_declaration" {
        let mut cursor = child.walk();
        for spec in child.children(&mut cursor) {
            if spec.kind() == "var_spec" {
                bind_names_before_type(spec, source, builder);
            }
        }
        return;
    }
    collect_locals(child, source, builder);
}

fn bind_names_before_type(decl: Node, source: &str, builder: &mut EnvBuilder) {
    let Some(ty_node) = type_node_of(decl) else {
        return;
    };
    let Some(ty) = type_head(ty_node, source) else {
        return;
    };
    let ty_start = ty_node.start_byte();
    let mut cursor = decl.walk();
    for c in decl.children(&mut cursor) {
        if c.kind() == "identifier" && c.start_byte() < ty_start {
            builder.bind(node_text(c, source).to_string(), ty.clone());
        }
    }
}

fn type_node_of(node: Node) -> Option<Node> {
    let mut cursor = node.walk();
    let found = node.children(&mut cursor).find(|c| TYPE_KINDS.contains(&c.kind()));
    found
}

fn type_head(node: Node, source: &str) -> Option<String> {
    match node.kind() {
        "pointer_type" => type_node_of(node).and_then(|inner| type_head(inner, source)),
        "generic_type" => find_child_by_kind(node, "type_identifier").and_then(|t| simple_head(t, source)),
        _ => simple_head(node, source),
    }
}

fn simple_head(node: Node, source: &str) -> Option<String> {
    match head_of(node_text(node, source)) {
        Some(head) if PRIMITIVE_NAMES.contains(&head.as_str()) => None,
        other => other,
    }
}
