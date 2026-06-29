use std::collections::BTreeSet;

use tree_sitter::Node;

use pulse_syntax::walk::{find_child_by_kind, node_text};

use super::binding::TypeEnv;
use super::binding_extract::{bind_c_decl, collect_c_locals, head_of, EnvBuilder};

const TYPE_KINDS: &[&str] = &["type_identifier", "qualified_identifier", "template_type"];

const SCOPE_BOUNDARIES: &[&str] =
    &["lambda_expression", "function_definition", "class_specifier", "struct_specifier", "field_declaration_list"];

pub fn method_var_types(method_node: Node, source: &str) -> TypeEnv {
    let mut builder = EnvBuilder::default();
    collect_params(method_node, source, &mut builder);
    if let Some(body) = find_child_by_kind(method_node, "compound_statement") {
        collect_c_locals(body, source, &mut builder, type_head, SCOPE_BOUNDARIES);
    }
    builder.into_env(&BTreeSet::new())
}

pub fn class_field_types(class_node: Node, source: &str) -> TypeEnv {
    let mut builder = EnvBuilder::default();
    if let Some(body) = find_child_by_kind(class_node, "field_declaration_list") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if child.kind() == "field_declaration" {
                bind_c_decl(child, source, &mut builder, type_head);
            }
        }
    }
    builder.into_env(&BTreeSet::new())
}

pub fn class_parents(class_node: Node, source: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let Some(clause) = find_child_by_kind(class_node, "base_class_clause") else {
        return out;
    };
    let mut cursor = clause.walk();
    for child in clause.children(&mut cursor) {
        if TYPE_KINDS.contains(&child.kind()) {
            if let Some(name) = type_head(child, source) {
                out.push(name);
            }
        }
    }
    out
}

fn collect_params(method_node: Node, source: &str, builder: &mut EnvBuilder) {
    let declarator = find_child_by_kind(method_node, "function_declarator")
        .or_else(|| find_child_by_kind(method_node, "pointer_declarator").and_then(function_declarator_in));
    let Some(params) = declarator.and_then(|d| find_child_by_kind(d, "parameter_list")) else {
        return;
    };
    let mut cursor = params.walk();
    for p in params.children(&mut cursor) {
        if p.kind() == "parameter_declaration" {
            bind_c_decl(p, source, builder, type_head);
        }
    }
}

fn function_declarator_in(node: Node) -> Option<Node> {
    find_child_by_kind(node, "function_declarator")
}

fn type_head(node: Node, source: &str) -> Option<String> {
    match node.kind() {
        "type_identifier" | "qualified_identifier" => head_of(node_text(node, source)),
        "template_type" => node.child_by_field_name("name").and_then(|n| head_of(node_text(n, source))),
        _ => None,
    }
}
