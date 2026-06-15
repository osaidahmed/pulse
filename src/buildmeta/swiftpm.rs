use std::path::Path;

use serde_json::Value;
use tree_sitter::Node;

use crate::parse::{parse_guarded, Language};
use crate::walk::node_text;

use super::{DeclaredDep, DepScope, Ecosystem, Lockfile, Manifest};

pub(super) fn parse_manifest(path: &Path, source: &str) -> Option<Manifest> {
    let tree = parse_guarded(source, Language::Swift)?;
    let mut deps = Vec::new();
    walk_packages(tree.root_node(), source, &mut deps);
    Some(Manifest { path: path.to_path_buf(), ecosystem: Ecosystem::Swift, deps, workspace_members: Vec::new() })
}

fn walk_packages(node: Node, source: &str, deps: &mut Vec<DeclaredDep>) {
    if node.kind() == "call_expression" && node_text(node, source).trim_start().starts_with(".package") {
        if let Some(dep) = package_dep(node, source) {
            deps.push(dep);
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.is_named() {
            walk_packages(child, source, deps);
        }
    }
}

fn package_dep(call: Node, source: &str) -> Option<DeclaredDep> {
    let mut args = Vec::new();
    collect_labeled_strings(call, source, &mut args);
    if args.iter().any(|(label, _)| label == "path") {
        return None;
    }
    let name = args
        .iter()
        .find(|(label, _)| label == "name")
        .map(|(_, v)| v.clone())
        .or_else(|| args.iter().find(|(label, _)| label == "url").map(|(_, v)| name_from_url(v)))?;
    let constraint = args
        .iter()
        .find(|(label, _)| matches!(label.as_str(), "from" | "exact" | "branch" | "revision"))
        .map(|(_, v)| v.clone())
        .unwrap_or_default();
    Some(DeclaredDep {
        name,
        constraint,
        scope: DepScope::Deployed,
        line: call.start_position().row as u32 + 1,
        own: false,
    })
}

fn collect_labeled_strings(node: Node, source: &str, out: &mut Vec<(String, String)>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "value_argument" {
            if let Some(pair) = labeled_string(child, source) {
                out.push(pair);
            }
        }
        if child.is_named() {
            collect_labeled_strings(child, source, out);
        }
    }
}

fn labeled_string(arg: Node, source: &str) -> Option<(String, String)> {
    let label = arg.child_by_field_name("name")?;
    let value = arg.child_by_field_name("value")?;
    if value.kind() != "line_string_literal" {
        return None;
    }
    Some((node_text(label, source).trim_end_matches(':').trim().to_string(), string_content(value, source)))
}

fn string_content(literal: Node, source: &str) -> String {
    let mut cursor = literal.walk();
    literal.children(&mut cursor).filter(|c| c.kind() == "line_str_text").map(|c| node_text(c, source)).collect()
}

fn name_from_url(url: &str) -> String {
    url.trim_end_matches('/').rsplit('/').next().unwrap_or(url).trim_end_matches(".git").to_string()
}

pub(super) fn parse_lockfile(path: &Path, source: &str) -> Option<Lockfile> {
    let json: Value = serde_json::from_str(source).ok()?;
    let pins = json.get("pins").or_else(|| json.get("object").and_then(|o| o.get("pins")))?.as_array()?;
    let mut resolved = Vec::new();
    for pin in pins {
        let name = pin.get("identity").or_else(|| pin.get("package")).and_then(Value::as_str);
        let version = pin.get("state").and_then(|s| s.get("version")).and_then(Value::as_str);
        if let Some(name) = name {
            resolved.push((name.to_string(), version.unwrap_or("").to_string()));
        }
    }
    (!resolved.is_empty()).then(|| Lockfile { path: path.to_path_buf(), ecosystem: Ecosystem::Swift, resolved })
}
