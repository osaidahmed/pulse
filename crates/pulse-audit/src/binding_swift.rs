use std::collections::BTreeSet;

use tree_sitter::Node;

use pulse_syntax::walk::{find_child_by_kind, node_text, DepthGuard};

use super::binding::TypeEnv;
use super::binding_extract::{head_of, EnvBuilder};

const TYPE_KINDS: &[&str] = &["user_type", "optional_type", "array_type", "dictionary_type", "tuple_type", "metatype"];

const SKIP_TYPE_KINDS: &[&str] = &["array_type", "dictionary_type", "tuple_type", "metatype"];

const PRIMITIVE_NAMES: &[&str] = &[
    "Int",
    "Int8",
    "Int16",
    "Int32",
    "Int64",
    "UInt",
    "UInt8",
    "UInt16",
    "UInt32",
    "UInt64",
    "Float",
    "Double",
    "Bool",
    "String",
    "Character",
    "Void",
];

const SCOPE_BOUNDARIES: &[&str] = &[
    "lambda_literal",
    "function_declaration",
    "init_declaration",
    "deinit_declaration",
    "subscript_declaration",
    "computed_property",
    "class_declaration",
    "protocol_declaration",
];

const BODY_KINDS: &[&str] = &["class_body", "enum_class_body"];

pub fn method_var_types(method_node: Node, source: &str) -> TypeEnv {
    let mut builder = EnvBuilder::default();
    collect_params(method_node, source, &mut builder);
    if let Some(body) = find_child_by_kind(method_node, "function_body") {
        collect_locals(body, source, &mut builder);
    }
    builder.into_env(&type_param_names(method_node, source))
}

pub fn class_field_types(class_node: Node, source: &str) -> TypeEnv {
    let mut builder = EnvBuilder::default();
    if let Some(body) = body_of(class_node) {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if child.kind() == "property_declaration" {
                bind_property(child, source, &mut builder);
            }
        }
    }
    builder.into_env(&type_param_names(class_node, source))
}

pub fn class_parents(class_node: Node, source: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cursor = class_node.walk();
    for child in class_node.children(&mut cursor) {
        if child.kind() != "inheritance_specifier" {
            continue;
        }
        let ty = child.child_by_field_name("inherits_from").or_else(|| type_node_of(child));
        if let Some(name) = ty.and_then(|t| type_head_name(t, source)) {
            out.push(name);
        }
    }
    out
}

fn body_of(class_node: Node) -> Option<Node> {
    BODY_KINDS.iter().find_map(|k| find_child_by_kind(class_node, k))
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
    let mut cursor = method_node.walk();
    for p in method_node.children(&mut cursor) {
        if p.kind() == "parameter" {
            bind_param(p, source, builder);
        }
    }
}

fn bind_param(param: Node, source: &str, builder: &mut EnvBuilder) {
    if is_variadic(param) {
        return;
    }
    let Some(name) = param.child_by_field_name("name").filter(|n| n.kind() == "simple_identifier") else {
        return;
    };
    let ty = param.child_by_field_name("type").or_else(|| type_node_of(param));
    if let Some(head) = ty.and_then(|t| type_head_name(t, source)) {
        builder.bind(node_text(name, source).to_string(), head);
    }
}

fn is_variadic(param: Node) -> bool {
    let mut cursor = param.walk();
    let found = param.children(&mut cursor).any(|c| c.kind() == "...");
    found
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
        "property_declaration" => bind_property(child, source, builder),
        "for_statement" => bind_for_item(child, source, builder),
        _ => {}
    }
    collect_locals(child, source, builder);
}

fn bind_for_item(stmt: Node, source: &str, builder: &mut EnvBuilder) {
    if let Some(pat) = stmt.child_by_field_name("item") {
        bind_pattern_with_annotation(pat, stmt, source, builder);
    }
}

fn bind_property(decl: Node, source: &str, builder: &mut EnvBuilder) {
    if is_multi_declarator(decl) {
        return;
    }
    let Some(pat) = decl.child_by_field_name("name") else {
        return;
    };
    bind_pattern_with_annotation(pat, decl, source, builder);
}

fn is_multi_declarator(decl: Node) -> bool {
    let mut cursor = decl.walk();
    let multi = decl.children(&mut cursor).any(|c| c.kind() == ",");
    multi
}

fn bind_pattern_with_annotation(pattern: Node, owner: Node, source: &str, builder: &mut EnvBuilder) {
    let Some(name) = pattern_name(pattern, source) else {
        return;
    };
    let Some(ann) = find_child_by_kind(owner, "type_annotation") else {
        return;
    };
    if let Some(head) = type_node_of(ann).and_then(|t| type_head_name(t, source)) {
        builder.bind(name, head);
    }
}

fn pattern_name(pattern: Node, source: &str) -> Option<String> {
    let id =
        pattern.child_by_field_name("bound_identifier").or_else(|| find_child_by_kind(pattern, "simple_identifier"))?;
    Some(node_text(id, source).to_string())
}

fn type_node_of(node: Node) -> Option<Node> {
    let mut cursor = node.walk();
    let found = node.children(&mut cursor).find(|c| TYPE_KINDS.contains(&c.kind()));
    found
}

fn type_head_name(node: Node, source: &str) -> Option<String> {
    match node.kind() {
        k if SKIP_TYPE_KINDS.contains(&k) => None,
        "optional_type" => {
            let wrapped = node.child_by_field_name("wrapped").or_else(|| type_node_of(node))?;
            type_head_name(wrapped, source)
        }
        _ => head_of(node_text(node, source)).filter(|h| !PRIMITIVE_NAMES.contains(&h.as_str())),
    }
}
