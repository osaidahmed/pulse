use std::collections::BTreeSet;

use tree_sitter::Node;

use pulse_syntax::walk::{find_child_by_kind, node_text, DepthGuard};

use super::binding::TypeEnv;
use super::binding_extract::{head_of, EnvBuilder};

const NAMED_TYPE_KINDS: &[&str] =
    &["identifier", "qualified_name", "generic_name", "alias_qualified_name", "scoped_type"];

const PEEL_KINDS: &[&str] = &["nullable_type", "ref_type", "scoped_type"];

const SKIP_TYPE_KINDS: &[&str] =
    &["predefined_type", "implicit_type", "array_type", "pointer_type", "tuple_type", "function_pointer_type"];

const SCOPE_BOUNDARIES: &[&str] = &[
    "lambda_expression",
    "local_function_statement",
    "class_declaration",
    "struct_declaration",
    "interface_declaration",
    "record_declaration",
    "method_declaration",
    "constructor_declaration",
    "declaration_list",
];

pub fn method_var_types(method_node: Node, source: &str) -> TypeEnv {
    let mut builder = EnvBuilder::default();
    collect_params(method_node, source, &mut builder);
    if let Some(body) = find_child_by_kind(method_node, "block") {
        collect_locals(body, source, &mut builder);
    }
    builder.into_env(&type_param_names(method_node, source))
}

pub fn class_field_types(class_node: Node, source: &str) -> TypeEnv {
    let mut builder = EnvBuilder::default();
    if let Some(body) = find_child_by_kind(class_node, "declaration_list") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            collect_member(child, source, &mut builder);
        }
    }
    builder.into_env(&type_param_names(class_node, source))
}

pub fn class_parents(class_node: Node, source: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let Some(base_list) = find_child_by_kind(class_node, "base_list") else {
        return out;
    };
    let mut cursor = base_list.walk();
    for child in base_list.children(&mut cursor) {
        collect_base(child, source, &mut out);
    }
    out
}

fn collect_base(child: Node, source: &str, out: &mut Vec<String>) {
    let ty = if child.kind() == "primary_constructor_base_type" {
        child.child_by_field_name("type")
    } else if NAMED_TYPE_KINDS.contains(&child.kind()) {
        Some(child)
    } else {
        None
    };
    if let Some(name) = ty.and_then(|t| type_head_name(t, source)) {
        out.push(name);
    }
}

fn collect_member(child: Node, source: &str, builder: &mut EnvBuilder) {
    match child.kind() {
        "field_declaration" => {
            if let Some(decl) = find_child_by_kind(child, "variable_declaration") {
                collect_var_declaration(decl, source, builder);
            }
        }
        "property_declaration" => bind_field(child, source, builder),
        _ => {}
    }
}

fn type_param_names(node: Node, source: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let Some(tps) = find_child_by_kind(node, "type_parameter_list") else {
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
    let Some(params) = find_child_by_kind(method_node, "parameter_list") else {
        return;
    };
    let mut cursor = params.walk();
    for p in params.children(&mut cursor) {
        if p.kind() == "parameter" {
            bind_field(p, source, builder);
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
    match kind {
        "local_declaration_statement" | "using_statement" => {
            if let Some(decl) = find_child_by_kind(child, "variable_declaration") {
                collect_var_declaration(decl, source, builder);
            }
        }
        "foreach_statement" => bind_foreach(child, source, builder),
        _ => {}
    }
    collect_locals(child, source, builder);
}

fn bind_foreach(node: Node, source: &str, builder: &mut EnvBuilder) {
    let (Some(ty_node), Some(name)) = (node.child_by_field_name("type"), node.child_by_field_name("left")) else {
        return;
    };
    if name.kind() != "identifier" {
        return;
    }
    if let Some(ty) = type_head_name(ty_node, source) {
        builder.bind(node_text(name, source).to_string(), ty);
    }
}

fn collect_var_declaration(decl: Node, source: &str, builder: &mut EnvBuilder) {
    let Some(ty) = decl.child_by_field_name("type").and_then(|t| type_head_name(t, source)) else {
        return;
    };
    let mut cursor = decl.walk();
    for child in decl.children(&mut cursor) {
        if child.kind() == "variable_declarator" {
            if let Some(name) = child.child_by_field_name("name") {
                builder.bind(node_text(name, source).to_string(), ty.clone());
            }
        }
    }
}

fn bind_field(node: Node, source: &str, builder: &mut EnvBuilder) {
    let (Some(ty_node), Some(name)) = (node.child_by_field_name("type"), node.child_by_field_name("name")) else {
        return;
    };
    if let Some(ty) = type_head_name(ty_node, source) {
        builder.bind(node_text(name, source).to_string(), ty);
    }
}

fn type_head_name(node: Node, source: &str) -> Option<String> {
    let inner = peel_type(node);
    let kind = inner.kind();
    if SKIP_TYPE_KINDS.contains(&kind) {
        return None;
    }
    if !NAMED_TYPE_KINDS.contains(&kind) {
        return None;
    }
    match head_of(node_text(inner, source)) {
        Some(head) if head == "dynamic" => None,
        other => other,
    }
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
