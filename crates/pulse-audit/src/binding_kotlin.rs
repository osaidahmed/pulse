use std::collections::BTreeSet;

use tree_sitter::Node;

use pulse_syntax::walk::{find_child_by_kind, find_child_by_kinds, node_text, DepthGuard};

use super::binding::TypeEnv;
use super::binding_extract::{head_of, EnvBuilder};

const TYPE_KINDS: &[&str] = &["user_type", "nullable_type", "non_nullable_type", "parenthesized_type"];

const PRIMITIVE_TYPES: &[&str] =
    &["Int", "Long", "Short", "Byte", "Float", "Double", "Boolean", "Char", "String", "Unit", "Nothing"];

const SCOPE_BOUNDARIES: &[&str] = &[
    "lambda_literal",
    "annotated_lambda",
    "anonymous_function",
    "function_declaration",
    "class_declaration",
    "object_declaration",
    "object_literal",
    "companion_object",
];

pub fn method_var_types(method_node: Node, source: &str) -> TypeEnv {
    let mut builder = EnvBuilder::default();
    collect_params(method_node, source, &mut builder);
    let func_body = find_child_by_kind(method_node, "function_body");
    let body =
        func_body.and_then(|b| find_child_by_kind(b, "block")).or_else(|| find_child_by_kind(method_node, "block"));
    if let Some(body) = body {
        collect_locals(body, source, &mut builder);
    }
    builder.into_env(&type_param_names(method_node, source))
}

pub fn class_field_types(class_node: Node, source: &str) -> TypeEnv {
    let mut builder = EnvBuilder::default();
    collect_ctor_fields(class_node, source, &mut builder);
    if let Some(body) = find_child_by_kind(class_node, "class_body") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if child.kind() == "property_declaration" {
                collect_property(child, source, &mut builder);
            }
        }
    }
    builder.into_env(&type_param_names(class_node, source))
}

pub fn class_parents(class_node: Node, source: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let Some(specs) = find_child_by_kind(class_node, "delegation_specifiers") else {
        return out;
    };
    let mut cursor = specs.walk();
    for spec in specs.children(&mut cursor) {
        if spec.kind() == "delegation_specifier" {
            if let Some(name) = delegation_type_name(spec, source) {
                out.push(name);
            }
        }
    }
    out
}

fn delegation_type_name(spec: Node, source: &str) -> Option<String> {
    let direct = find_child_by_kinds(spec, TYPE_KINDS);
    let nested = direct.or_else(|| {
        find_child_by_kinds(spec, &["constructor_invocation", "explicit_delegation"])
            .and_then(|inner| find_child_by_kinds(inner, TYPE_KINDS))
    });
    nested.and_then(|t| type_head_name(t, source))
}

fn collect_ctor_fields(class_node: Node, source: &str, builder: &mut EnvBuilder) {
    let Some(params) = find_child_by_kind(class_node, "primary_constructor")
        .and_then(|ctor| find_child_by_kind(ctor, "class_parameters"))
    else {
        return;
    };
    let mut cursor = params.walk();
    for p in params.children(&mut cursor) {
        if p.kind() == "class_parameter" && is_property_param(p) && !has_vararg(p, source) {
            bind_named(p, source, builder);
        }
    }
}

fn has_vararg(node: Node, source: &str) -> bool {
    find_child_by_kinds(node, &["modifiers", "parameter_modifiers"])
        .is_some_and(|m| node_text(m, source).contains("vararg"))
}

fn is_property_param(param: Node) -> bool {
    let mut cursor = param.walk();
    let result = param.children(&mut cursor).any(|c| matches!(c.kind(), "val" | "var"));
    result
}

fn collect_params(method_node: Node, source: &str, builder: &mut EnvBuilder) {
    let Some(params) = find_child_by_kind(method_node, "function_value_parameters") else {
        return;
    };
    let mut cursor = params.walk();
    let mut vararg = false;
    for p in params.children(&mut cursor) {
        match p.kind() {
            "parameter_modifiers" => vararg = node_text(p, source).contains("vararg"),
            "parameter" => {
                if !vararg {
                    bind_named(p, source, builder);
                }
                vararg = false;
            }
            _ => vararg = false,
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
    if kind == "property_declaration" {
        collect_property(child, source, builder);
        return;
    }
    if kind == "for_statement" {
        bind_for_variable(child, source, builder);
    }
    collect_locals(child, source, builder);
}

fn bind_for_variable(node: Node, source: &str, builder: &mut EnvBuilder) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "variable_declaration" {
            bind_declaration(child, source, builder);
        }
    }
}

fn collect_property(node: Node, source: &str, builder: &mut EnvBuilder) {
    if let Some(decl) = find_child_by_kind(node, "variable_declaration") {
        bind_declaration(decl, source, builder);
    }
}

fn bind_declaration(decl: Node, source: &str, builder: &mut EnvBuilder) {
    bind_named(decl, source, builder);
}

fn bind_named(node: Node, source: &str, builder: &mut EnvBuilder) {
    let Some(ty) = type_node_of(node).and_then(|t| type_head_name(t, source)) else {
        return;
    };
    if let Some(name) = find_child_by_kind(node, "identifier") {
        builder.bind(node_text(name, source).to_string(), ty);
    }
}

fn type_node_of(node: Node) -> Option<Node> {
    find_child_by_kinds(node, TYPE_KINDS)
}

fn type_head_name(node: Node, source: &str) -> Option<String> {
    match node.kind() {
        "nullable_type" | "parenthesized_type" => type_node_of(node).and_then(|inner| type_head_name(inner, source)),
        "non_nullable_type" | "function_type" => None,
        _ => simple_head(node, source),
    }
}

fn simple_head(node: Node, source: &str) -> Option<String> {
    match head_of(node_text(node, source)) {
        Some(head) if PRIMITIVE_TYPES.contains(&head.as_str()) => None,
        other => other,
    }
}

fn type_param_names(node: Node, source: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let Some(tps) = find_child_by_kind(node, "type_parameters") else {
        return names;
    };
    let mut cursor = tps.walk();
    for tp in tps.children(&mut cursor) {
        if tp.kind() == "type_parameter" {
            if let Some(id) = find_child_by_kind(tp, "identifier") {
                names.insert(node_text(id, source).to_string());
            }
        }
    }
    names
}
