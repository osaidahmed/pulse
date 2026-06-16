use tree_sitter::Node;

use crate::walk::{find_child_by_kind, node_text, DepthGuard};

use super::binding::TypeEnv;

const TYPE_KINDS: &[&str] = &[
    "type_identifier",
    "scoped_type_identifier",
    "generic_type",
    "annotated_type",
    "integral_type",
    "floating_point_type",
    "boolean_type",
    "void_type",
    "array_type",
];

const PRIMITIVE_KINDS: &[&str] = &["integral_type", "floating_point_type", "boolean_type", "void_type", "array_type"];

const SCOPE_BOUNDARIES: &[&str] =
    &["lambda_expression", "class_declaration", "method_declaration", "constructor_declaration", "class_body"];

pub fn method_var_types(method_node: Node, source: &str) -> TypeEnv {
    let mut env = TypeEnv::new();
    collect_params(method_node, source, &mut env);
    let body = find_child_by_kind(method_node, "block").or_else(|| find_child_by_kind(method_node, "constructor_body"));
    if let Some(body) = body {
        collect_locals(body, source, &mut env);
    }
    env
}

pub fn class_field_types(class_node: Node, source: &str) -> TypeEnv {
    let mut env = TypeEnv::new();
    let Some(body) = find_child_by_kind(class_node, "class_body") else {
        return env;
    };
    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        if child.kind() == "field_declaration" {
            collect_decl(child, source, &mut env);
        }
    }
    env
}

pub fn class_parents(class_node: Node, source: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    if let Some(sc) = find_child_by_kind(class_node, "superclass") {
        if let Some(name) = type_node_of(sc).and_then(|t| type_head_name(t, source)) {
            out.push(name);
        }
    }
    if let Some(si) = find_child_by_kind(class_node, "super_interfaces") {
        collect_interface_names(si, source, &mut out);
    }
    out
}

fn collect_interface_names(super_interfaces: Node, source: &str, out: &mut Vec<String>) {
    let Some(list) = find_child_by_kind(super_interfaces, "type_list") else {
        return;
    };
    let mut cursor = list.walk();
    for child in list.children(&mut cursor) {
        if let Some(name) = type_head_name(child, source) {
            out.push(name);
        }
    }
}

fn collect_params(method_node: Node, source: &str, env: &mut TypeEnv) {
    let Some(params) = find_child_by_kind(method_node, "formal_parameters") else {
        return;
    };
    let mut cursor = params.walk();
    for p in params.children(&mut cursor) {
        if matches!(p.kind(), "formal_parameter" | "spread_parameter") {
            if let Some((name, ty)) = type_and_name(p, source) {
                env.entry(name).or_insert(ty);
            }
        }
    }
}

fn collect_locals(node: Node, source: &str, env: &mut TypeEnv) {
    let Some(_guard) = DepthGuard::enter() else {
        return;
    };
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        process_local_node(child, source, env);
    }
}

fn process_local_node(child: Node, source: &str, env: &mut TypeEnv) {
    let kind = child.kind();
    if SCOPE_BOUNDARIES.contains(&kind) {
        return;
    }
    if kind == "local_variable_declaration" {
        collect_decl(child, source, env);
        return;
    }
    if kind == "enhanced_for_statement" {
        if let Some((name, ty)) = type_and_name(child, source) {
            env.entry(name).or_insert(ty);
        }
    } else if kind == "catch_formal_parameter" {
        collect_catch(child, source, env);
    }
    collect_locals(child, source, env);
}

fn collect_decl(decl: Node, source: &str, env: &mut TypeEnv) {
    let Some(ty) = type_node_of(decl).and_then(|t| type_head_name(t, source)) else {
        return;
    };
    let mut cursor = decl.walk();
    for child in decl.children(&mut cursor) {
        if child.kind() == "variable_declarator" {
            if let Some(name) = find_child_by_kind(child, "identifier") {
                env.entry(node_text(name, source).to_string()).or_insert_with(|| ty.clone());
            }
        }
    }
}

fn collect_catch(node: Node, source: &str, env: &mut TypeEnv) {
    let Some(name) = find_child_by_kind(node, "identifier") else {
        return;
    };
    let ty_node = find_child_by_kind(node, "catch_type").and_then(type_node_of).or_else(|| type_node_of(node));
    if let Some(ty) = ty_node.and_then(|t| type_head_name(t, source)) {
        env.entry(node_text(name, source).to_string()).or_insert(ty);
    }
}

fn type_and_name(node: Node, source: &str) -> Option<(String, String)> {
    let ty = type_node_of(node).and_then(|t| type_head_name(t, source))?;
    let name = find_child_by_kind(node, "identifier")?;
    Some((node_text(name, source).to_string(), ty))
}

fn type_node_of(node: Node) -> Option<Node> {
    let mut cursor = node.walk();
    let found = node.children(&mut cursor).find(|c| TYPE_KINDS.contains(&c.kind()));
    found
}

fn type_head_name(node: Node, source: &str) -> Option<String> {
    let kind = node.kind();
    if PRIMITIVE_KINDS.contains(&kind) {
        return None;
    }
    if kind == "annotated_type" {
        return type_node_of(node).and_then(|c| type_head_name(c, source));
    }
    head_of(node_text(node, source))
}

fn head_of(text: &str) -> Option<String> {
    let before_generic = text.split('<').next()?.trim();
    let simple = before_generic.rsplit('.').next()?.trim();
    if simple.is_empty() {
        None
    } else {
        Some(simple.to_string())
    }
}
