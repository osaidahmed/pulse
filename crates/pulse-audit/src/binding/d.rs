use std::collections::BTreeSet;

use tree_sitter::Node;

use pulse_syntax::walk::{find_child_by_kind, node_text, DepthGuard};

use crate::binding::extract::{head_of, EnvBuilder};
use crate::binding::TypeEnv;

const PRIMITIVE_KINDS: &[&str] = &[
    "bool",
    "byte",
    "cdouble",
    "cent",
    "cfloat",
    "char",
    "creal",
    "cstring",
    "dchar",
    "double",
    "dstring",
    "float",
    "idouble",
    "ifloat",
    "int",
    "ireal",
    "long",
    "noreturn",
    "ptrdiff_t",
    "real",
    "short",
    "size_t",
    "string",
    "ubyte",
    "ucent",
    "uint",
    "ulong",
    "ushort",
    "void",
    "wchar",
    "wstring",
];

const SCOPE_BOUNDARIES: &[&str] = &[
    "function_literal",
    "function_declaration",
    "constructor",
    "destructor",
    "class_declaration",
    "struct_declaration",
    "interface_declaration",
    "union_declaration",
    "unittest_declaration",
];

pub fn method_var_types(method_node: Node, source: &str) -> TypeEnv {
    let mut builder = EnvBuilder::default();
    collect_params(method_node, source, &mut builder);
    if let Some(body) = method_body(method_node) {
        collect_locals(body, source, &mut builder);
    }
    builder.into_env(&type_param_names(method_node, source))
}

pub fn class_field_types(class_node: Node, source: &str) -> TypeEnv {
    let mut builder = EnvBuilder::default();
    if let Some(body) = find_child_by_kind(class_node, "aggregate_body") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if child.kind() == "variable_declaration" {
                collect_decl(child, source, &mut builder);
            }
        }
    }
    builder.into_env(&type_param_names(class_node, source))
}

pub fn class_parents(class_node: Node, source: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cursor = class_node.walk();
    for child in class_node.children(&mut cursor) {
        if child.kind() == "base_class" {
            if let Some(name) = base_parent_name(child, source) {
                out.push(name);
            }
        }
    }
    out
}

fn base_parent_name(base: Node, source: &str) -> Option<String> {
    if let Some(ti) = find_child_by_kind(base, "template_instance") {
        return find_child_by_kind(ti, "identifier").map(|id| node_text(id, source).to_string());
    }
    head_of(node_text(base, source))
}

fn method_body(method_node: Node) -> Option<Node> {
    find_child_by_kind(method_node, "function_body").and_then(|fb| find_child_by_kind(fb, "block_statement"))
}

fn type_param_names(node: Node, source: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let Some(tps) = find_child_by_kind(node, "template_parameters") else {
        return names;
    };
    let mut cursor = tps.walk();
    for tp in tps.children(&mut cursor) {
        if tp.kind() == "template_parameter" {
            if let Some(id) = find_child_by_kind(tp, "identifier") {
                names.insert(node_text(id, source).to_string());
            }
        }
    }
    names
}

fn collect_params(method_node: Node, source: &str, builder: &mut EnvBuilder) {
    let Some(params) = find_child_by_kind(method_node, "parameters") else {
        return;
    };
    let mut cursor = params.walk();
    for p in params.children(&mut cursor) {
        if p.kind() == "parameter" {
            bind_type_and_id(p, source, builder);
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
    if kind == "variable_declaration" {
        collect_decl(child, source, builder);
        return;
    }
    if kind == "foreach_type" {
        bind_type_and_id(child, source, builder);
    }
    collect_locals(child, source, builder);
}

fn collect_decl(decl: Node, source: &str, builder: &mut EnvBuilder) {
    let Some(ty) = find_child_by_kind(decl, "type").and_then(|t| type_node_head(t, source)) else {
        return;
    };
    let mut cursor = decl.walk();
    for child in decl.children(&mut cursor) {
        if child.kind() == "declarator" {
            if let Some(name) = find_child_by_kind(child, "identifier") {
                builder.bind(node_text(name, source).to_string(), ty.clone());
            }
        }
    }
}

fn bind_type_and_id(node: Node, source: &str, builder: &mut EnvBuilder) {
    let Some(ty) = find_child_by_kind(node, "type").and_then(|t| type_node_head(t, source)) else {
        return;
    };
    if let Some(name) = find_child_by_kind(node, "identifier") {
        builder.bind(node_text(name, source).to_string(), ty);
    }
}

fn type_node_head(node: Node, source: &str) -> Option<String> {
    let _guard = DepthGuard::enter()?;
    if has_type_suffix(node) {
        return None;
    }
    let mut cursor = node.walk();
    let inner = node.children(&mut cursor).find(|c| c.is_named() && c.kind() != "type_ctor")?;
    let kind = inner.kind();
    if PRIMITIVE_KINDS.contains(&kind) {
        return None;
    }
    match kind {
        "identifier" => head_of(node_text(inner, source)),
        "template_instance" => find_child_by_kind(inner, "identifier").map(|id| node_text(id, source).to_string()),
        "type" => type_node_head(inner, source),
        _ => None,
    }
}

fn has_type_suffix(node: Node) -> bool {
    let mut cursor = node.walk();
    let suffix = node
        .children(&mut cursor)
        .any(|c| matches!(c.kind(), "*" | "[" | "]" | "delegate" | "function" | "parameters"));
    suffix
}
