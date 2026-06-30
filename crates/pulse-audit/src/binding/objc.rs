use std::collections::BTreeSet;

use tree_sitter::Node;

use pulse_syntax::walk::{find_child_by_kind, node_text};

use crate::binding::extract::{collect_c_locals, head_of, EnvBuilder};
use crate::binding::TypeEnv;

const SCOPE_BOUNDARIES: &[&str] =
    &["block_literal", "method_definition", "function_definition", "class_implementation", "class_interface"];

pub fn method_var_types(method_node: Node, source: &str) -> TypeEnv {
    let mut builder = EnvBuilder::default();
    collect_params(method_node, source, &mut builder);
    if let Some(body) = find_child_by_kind(method_node, "compound_statement") {
        collect_c_locals(body, source, &mut builder, type_head, SCOPE_BOUNDARIES);
    }
    builder.into_env(&BTreeSet::new())
}

pub fn class_field_types(_class_node: Node, _source: &str) -> TypeEnv {
    TypeEnv::new()
}

pub fn class_parents(_class_node: Node, _source: &str) -> Vec<String> {
    Vec::new()
}

fn collect_params(method_node: Node, source: &str, builder: &mut EnvBuilder) {
    let mut cursor = method_node.walk();
    for child in method_node.children(&mut cursor) {
        if child.kind() != "method_parameter" {
            continue;
        }
        let ty = find_child_by_kind(child, "method_type").and_then(|mt| type_head(mt, source));
        let name = find_child_by_kind(child, "identifier");
        if let (Some(name), Some(ty)) = (name, ty) {
            builder.bind(node_text(name, source).to_string(), ty);
        }
    }
}

fn type_head(node: Node, source: &str) -> Option<String> {
    match node.kind() {
        "type_identifier" => head_of(node_text(node, source)),
        "method_type" => find_child_by_kind(node, "type_name").and_then(|tn| type_head_in(tn, source)),
        _ => None,
    }
}

fn type_head_in(type_name: Node, source: &str) -> Option<String> {
    find_child_by_kind(type_name, "type_identifier").and_then(|t| head_of(node_text(t, source)))
}
