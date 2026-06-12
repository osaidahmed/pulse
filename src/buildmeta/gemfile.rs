use std::path::Path;

use tree_sitter::Node;

use crate::parse::{parse_guarded, Language};
use crate::walk::node_text;

use super::{DeclaredDep, DepScope, Ecosystem, Lockfile, Manifest};

pub(super) fn parse_manifest(path: &Path, source: &str) -> Option<Manifest> {
    let tree = parse_guarded(source, Language::Ruby)?;
    let mut deps = Vec::new();
    walk_calls(tree.root_node(), source, DepScope::Deployed, &mut deps);
    Some(Manifest { path: path.to_path_buf(), ecosystem: Ecosystem::RubyGems, deps, workspace_members: Vec::new() })
}

fn walk_calls(node: Node, source: &str, scope: DepScope, deps: &mut Vec<DeclaredDep>) {
    if node.kind() == "call" {
        match method_name(node, source) {
            Some("gem") => {
                push_gem(node, source, scope, deps);
                return;
            }
            Some("group") => {
                let group_scope = if is_dev_group(node, source) { DepScope::Dev } else { scope };
                walk_block(node, source, group_scope, deps);
                return;
            }
            _ => {}
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.is_named() {
            walk_calls(child, source, scope, deps);
        }
    }
}

fn walk_block(node: Node, source: &str, scope: DepScope, deps: &mut Vec<DeclaredDep>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if matches!(child.kind(), "do_block" | "block") {
            walk_calls(child, source, scope, deps);
        }
    }
}

fn method_name<'a>(call: Node, source: &'a str) -> Option<&'a str> {
    if call.child_by_field_name("receiver").is_some() {
        return None;
    }
    let method = call.child_by_field_name("method")?;
    (method.kind() == "identifier").then(|| node_text(method, source))
}

fn push_gem(call: Node, source: &str, scope: DepScope, deps: &mut Vec<DeclaredDep>) {
    let strings = argument_strings(call, source);
    let Some((name, constraints)) = strings.split_first() else { return };
    deps.push(DeclaredDep {
        name: name.clone(),
        constraint: constraints.join(", "),
        scope,
        line: call.start_position().row as u32 + 1,
    });
}

fn argument_strings(call: Node, source: &str) -> Vec<String> {
    let Some(args) = call.child_by_field_name("arguments") else { return Vec::new() };
    let mut strings = Vec::new();
    let mut child = args.named_child(0);
    while let Some(c) = child {
        if c.kind() == "string" {
            strings.push(string_content(c, source));
        }
        child = c.next_named_sibling();
    }
    strings
}

fn string_content(string_node: Node, source: &str) -> String {
    let mut cursor = string_node.walk();
    string_node.children(&mut cursor).filter(|c| c.kind() == "string_content").map(|c| node_text(c, source)).collect()
}

fn is_dev_group(call: Node, source: &str) -> bool {
    let Some(args) = call.child_by_field_name("arguments") else { return false };
    let mut child = args.named_child(0);
    while let Some(c) = child {
        if c.kind() == "simple_symbol" && matches!(node_text(c, source), ":development" | ":test") {
            return true;
        }
        child = c.next_named_sibling();
    }
    false
}

pub(super) fn parse_lockfile(path: &Path, source: &str) -> Option<Lockfile> {
    let mut resolved = Vec::new();
    let mut in_specs = false;
    for line in source.lines() {
        if line.trim_end() == "  specs:" {
            in_specs = true;
            continue;
        }
        if in_specs && !line.starts_with("    ") {
            in_specs = false;
        }
        if !in_specs || line.starts_with("      ") {
            continue;
        }
        let entry = line.trim();
        let (name, rest) = entry.split_once(' ').unwrap_or((entry, ""));
        let version = rest.trim_start_matches('(').trim_end_matches(')').to_string();
        if !name.is_empty() {
            resolved.push((name.to_string(), version));
        }
    }
    (!resolved.is_empty()).then(|| Lockfile { path: path.to_path_buf(), ecosystem: Ecosystem::RubyGems, resolved })
}
