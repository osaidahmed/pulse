use std::path::Path;

use tree_sitter::Node;

use crate::parse::{parse_guarded, Language};
use crate::walk::{find_child_by_kind, find_child_by_kinds, node_text};

use super::{DeclaredDep, DepScope, Ecosystem, Manifest};

const DEPLOYED_CONFIGS: &[&str] = &[
    "implementation",
    "api",
    "compileOnly",
    "compileOnlyApi",
    "runtimeOnly",
    "annotationProcessor",
    "kapt",
    "compile",
    "runtime",
];

pub(super) fn parse_kotlin(path: &Path, source: &str) -> Option<Manifest> {
    parse(path, source, Language::Kotlin)
}

pub(super) fn parse_groovy(path: &Path, source: &str) -> Option<Manifest> {
    parse(path, source, Language::Groovy)
}

fn parse(path: &Path, source: &str, lang: Language) -> Option<Manifest> {
    let tree = parse_guarded(source, lang)?;
    let mut deps = Vec::new();
    walk(tree.root_node(), source, &mut deps);
    (!deps.is_empty()).then(|| Manifest {
        path: path.to_path_buf(),
        ecosystem: Ecosystem::Gradle,
        deps,
        workspace_members: Vec::new(),
    })
}

fn walk(node: Node, source: &str, deps: &mut Vec<DeclaredDep>) {
    if let Some(dep) = call_dep(node, source) {
        deps.push(dep);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk(child, source, deps);
    }
}

fn call_dep(node: Node, source: &str) -> Option<DeclaredDep> {
    let (config, args) = call_parts(node, source)?;
    let scope = scope_for(&config)?;
    let (positional, labeled) = collect_args(args, source);
    let (name, constraint) = coordinate(&positional, &labeled)?;
    Some(DeclaredDep { name, constraint, scope, line: node.start_position().row as u32 + 1, own: false })
}

fn call_parts<'a>(node: Node<'a>, source: &str) -> Option<(String, Node<'a>)> {
    match node.kind() {
        "call_expression" => Some((
            node_text(find_child_by_kind(node, "identifier")?, source).to_string(),
            find_child_by_kind(node, "value_arguments")?,
        )),
        "juxt_function_call" => {
            Some((node_text(node.child_by_field_name("name")?, source).to_string(), node.child_by_field_name("args")?))
        }
        _ => None,
    }
}

fn scope_for(config: &str) -> Option<DepScope> {
    if config.starts_with("test") || config.starts_with("androidTest") {
        return Some(DepScope::Dev);
    }
    DEPLOYED_CONFIGS.contains(&config).then_some(DepScope::Deployed)
}

fn collect_args(container: Node, source: &str) -> (Vec<String>, Vec<(String, String)>) {
    let mut positional = Vec::new();
    let mut labeled = Vec::new();
    let mut cursor = container.walk();
    for child in container.children(&mut cursor) {
        push_arg(child, source, &mut positional, &mut labeled);
    }
    (positional, labeled)
}

fn push_arg(child: Node, source: &str, positional: &mut Vec<String>, labeled: &mut Vec<(String, String)>) {
    match child.kind() {
        "value_argument" => push_value_argument(child, source, positional, labeled),
        "map_item" => {
            if let (Some(key), Some(value)) = (child.child_by_field_name("key"), value_string(child, source)) {
                labeled.push((node_text(key, source).to_string(), value));
            }
        }
        "character_literal" | "string_literal" => {
            if let Some(value) = literal_text(child, source) {
                positional.push(value);
            }
        }
        _ => {}
    }
}

fn push_value_argument(arg: Node, source: &str, positional: &mut Vec<String>, labeled: &mut Vec<(String, String)>) {
    let Some(value) =
        find_child_by_kinds(arg, &["string_literal", "character_literal"]).and_then(|v| literal_text(v, source))
    else {
        return;
    };
    match find_child_by_kind(arg, "identifier") {
        Some(label) => labeled.push((node_text(label, source).to_string(), value)),
        None => positional.push(value),
    }
}

fn value_string(map_item: Node, source: &str) -> Option<String> {
    literal_text(map_item.child_by_field_name("value")?, source)
}

fn coordinate(positional: &[String], labeled: &[(String, String)]) -> Option<(String, String)> {
    if let Some(coord) = positional.iter().find(|s| is_coordinate(s)) {
        return Some(split_coord(coord));
    }
    let get = |key: &str| labeled.iter().find(|(l, _)| l == key).map(|(_, v)| v.clone());
    let group = get("group")?;
    let artifact = get("name")?;
    Some((format!("{group}:{artifact}"), get("version").unwrap_or_default()))
}

fn is_coordinate(s: &str) -> bool {
    !s.starts_with(':') && !s.contains(' ') && s.contains(':')
}

fn split_coord(coord: &str) -> (String, String) {
    let mut parts = coord.splitn(3, ':');
    let group = parts.next().unwrap_or_default();
    let artifact = parts.next().unwrap_or_default();
    (format!("{group}:{artifact}"), parts.next().unwrap_or_default().to_string())
}

fn literal_text(node: Node, source: &str) -> Option<String> {
    match node.kind() {
        "character_literal" => Some(node_text(node, source).trim_matches(['\'', '"']).to_string()),
        "string_literal" => {
            let mut cursor = node.walk();
            let inner: String = node
                .children(&mut cursor)
                .filter(|c| matches!(c.kind(), "string_content" | "string_fragment"))
                .map(|c| node_text(c, source))
                .collect();
            (!inner.is_empty()).then_some(inner)
        }
        _ => None,
    }
}
