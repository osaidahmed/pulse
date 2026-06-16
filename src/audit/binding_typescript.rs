use std::collections::BTreeSet;

use tree_sitter::Node;

use crate::walk::{find_child_by_kind, find_child_by_kinds, node_text, DepthGuard};

use super::binding::TypeEnv;
use super::binding_extract::{head_of, EnvBuilder};

const NAMED_TYPE_KINDS: &[&str] = &["type_identifier", "generic_type", "nested_type_identifier"];
const UNWRAP_KINDS: &[&str] = &["flow_maybe_type", "parenthesized_type"];
const RECEIVER_KINDS: &[&str] = &["identifier", "member_expression"];
const PARAM_KINDS: &[&str] = &["required_parameter", "optional_parameter"];
const DECL_KINDS: &[&str] = &["lexical_declaration", "variable_declaration"];
const SCOPE_BOUNDARIES: &[&str] = &[
    "arrow_function",
    "function",
    "function_declaration",
    "function_expression",
    "generator_function",
    "generator_function_declaration",
    "method_definition",
    "class_declaration",
    "class_body",
];

pub fn method_var_types(method_node: Node, source: &str) -> TypeEnv {
    let mut builder = EnvBuilder::default();
    collect_params(method_node, source, &mut builder);
    if let Some(body) = find_child_by_kind(method_node, "statement_block") {
        collect_locals(body, source, &mut builder);
    }
    builder.into_env(&scope_type_param_names(method_node, source))
}

fn scope_type_param_names(method_node: Node, source: &str) -> BTreeSet<String> {
    let mut names = type_param_names(method_node, source);
    if let Some(class) = enclosing_class(method_node) {
        names.extend(type_param_names(class, source));
    }
    names
}

fn enclosing_class(method_node: Node) -> Option<Node> {
    let mut node = method_node.parent();
    let mut depth = 0;
    while let Some(n) = node {
        if depth > 12 {
            break;
        }
        if n.kind() == "class_body" {
            return n.parent();
        }
        node = n.parent();
        depth += 1;
    }
    None
}

pub fn class_field_types(class_node: Node, source: &str) -> TypeEnv {
    let mut builder = EnvBuilder::default();
    if let Some(body) = find_child_by_kind(class_node, "class_body") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if child.kind() == "public_field_definition" {
                bind_named_field(child, source, &mut builder);
            }
        }
    }
    builder.into_env(&type_param_names(class_node, source))
}

pub fn class_parents(class_node: Node, source: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let Some(heritage) = find_child_by_kind(class_node, "class_heritage") else {
        return out;
    };
    if let Some(name) = extends_name(heritage, source) {
        out.push(name);
    }
    if let Some(implements) = find_child_by_kind(heritage, "implements_clause") {
        collect_implements(implements, source, &mut out);
    }
    out
}

fn extends_name(heritage: Node, source: &str) -> Option<String> {
    let extends = find_child_by_kind(heritage, "extends_clause")?;
    let value = find_child_by_kinds(extends, RECEIVER_KINDS)?;
    head_of(node_text(value, source))
}

fn collect_implements(implements: Node, source: &str, out: &mut Vec<String>) {
    let mut cursor = implements.walk();
    for child in implements.children(&mut cursor) {
        if let Some(name) = type_head_name(child, source) {
            out.push(name);
        }
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
            if let Some(id) = find_child_by_kind(tp, "type_identifier") {
                names.insert(node_text(id, source).to_string());
            }
        }
    }
    names
}

fn collect_params(method_node: Node, source: &str, builder: &mut EnvBuilder) {
    let Some(params) = find_child_by_kind(method_node, "formal_parameters") else {
        return;
    };
    let mut cursor = params.walk();
    for p in params.children(&mut cursor) {
        if PARAM_KINDS.contains(&p.kind()) {
            bind_named_field(p, source, builder);
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
    if DECL_KINDS.contains(&kind) {
        collect_declarators(child, source, builder);
        return;
    }
    collect_locals(child, source, builder);
}

fn collect_declarators(decl: Node, source: &str, builder: &mut EnvBuilder) {
    let mut cursor = decl.walk();
    for child in decl.children(&mut cursor) {
        if child.kind() == "variable_declarator" {
            bind_named_field(child, source, builder);
        }
    }
}

fn bind_named_field(node: Node, source: &str, builder: &mut EnvBuilder) {
    let Some(name) = find_child_by_kind(node, "identifier").or_else(|| find_child_by_kind(node, "property_identifier"))
    else {
        return;
    };
    let Some(ty) = field_type_name(node, source) else {
        return;
    };
    builder.bind(node_text(name, source).to_string(), ty);
}

fn field_type_name(node: Node, source: &str) -> Option<String> {
    let ann = find_child_by_kind(node, "type_annotation")?;
    let ty = find_child_by_kinds(ann, &type_node_kinds())?;
    type_head_name(ty, source)
}

fn type_node_kinds() -> Vec<&'static str> {
    let mut kinds = vec!["predefined_type", "union_type", "array_type"];
    kinds.extend_from_slice(NAMED_TYPE_KINDS);
    kinds.extend_from_slice(UNWRAP_KINDS);
    kinds
}

fn type_head_name(node: Node, source: &str) -> Option<String> {
    let kind = node.kind();
    if NAMED_TYPE_KINDS.contains(&kind) {
        return head_of(node_text(node, source));
    }
    if UNWRAP_KINDS.contains(&kind) {
        let inner = find_child_by_kinds(node, &type_node_kinds())?;
        return type_head_name(inner, source);
    }
    None
}
