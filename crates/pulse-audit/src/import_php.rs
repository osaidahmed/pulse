use std::path::{Path, PathBuf};

use tree_sitter::Node;

use super::imports::{line_of, node_text, RawImport};

struct KindHandler {
    parent_kinds: &'static [&'static str],
    child_kinds: &'static [&'static str],
    extract_inner: bool,
}

const KIND_HANDLERS: &[KindHandler] = &[
    KindHandler {
        parent_kinds: &["namespace_use_declaration"],
        child_kinds: &["qualified_name", "namespace_name"],
        extract_inner: false,
    },
    KindHandler {
        parent_kinds: &[
            "require_once_expression",
            "require_expression",
            "include_expression",
            "include_once_expression",
        ],
        child_kinds: &["string", "encapsed_string"],
        extract_inner: true,
    },
];

pub fn match_node(node: Node, source: &str) -> Option<RawImport> {
    let handler = KIND_HANDLERS.iter().find(|h| h.parent_kinds.contains(&node.kind()))?;
    let target = descend_apply(node, &|n| extract_for(n, source, handler))?;
    Some(RawImport { target, line: line_of(node) })
}

fn extract_for(node: Node, source: &str, handler: &KindHandler) -> Option<String> {
    if !handler.child_kinds.contains(&node.kind()) {
        return None;
    }
    if handler.extract_inner {
        return literal_text_of(node, source);
    }
    Some(node_text(node, source))
}

pub fn candidates(raw: &str, source_file: &Path, project_root: &Path) -> Vec<PathBuf> {
    let parent = source_file.parent().unwrap_or(project_root);
    let mut out = direct_php_paths(raw, parent, project_root);
    out.extend(psr4_paths_from_composer(raw, project_root));
    out
}

pub fn path_suffix(raw: &str) -> Option<String> {
    let parts: Vec<&str> = raw.split(&['\\', '/'][..]).filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        return None;
    }
    let joined = parts.join(std::path::MAIN_SEPARATOR_STR);
    Some(format!("{}{}.php", std::path::MAIN_SEPARATOR_STR, joined))
}

fn descend_apply<R>(node: Node, accept_extract: &dyn Fn(Node) -> Option<R>) -> Option<R> {
    if let Some(value) = accept_extract(node) {
        return Some(value);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if !child.is_named() {
            continue;
        }
        if let Some(value) = descend_apply(child, accept_extract) {
            return Some(value);
        }
    }
    None
}

fn literal_text_of(node: Node, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if matches!(child.kind(), "string_content" | "string_fragment") {
            return Some(node_text(child, source));
        }
    }
    let raw = node_text(node, source);
    let inner = raw
        .strip_prefix('"')
        .and_then(|r| r.strip_suffix('"'))
        .or_else(|| raw.strip_prefix('\'').and_then(|r| r.strip_suffix('\'')))?;
    Some(inner.to_string())
}

fn direct_php_paths(raw: &str, parent: &Path, project_root: &Path) -> Vec<PathBuf> {
    let normalized = raw.replace('\\', std::path::MAIN_SEPARATOR_STR);
    let mut out: Vec<PathBuf> = Vec::new();
    let bases: [&Path; 2] = [parent, project_root];
    let prefixes: [&str; 2] = ["", "src"];
    for base in bases {
        for prefix in prefixes {
            let mut p = base.to_path_buf();
            if !prefix.is_empty() {
                p.push(prefix);
            }
            p.push(&normalized);
            out.push(p.with_extension("php"));
        }
        let raw_path = base.join(raw);
        if raw_path.extension().is_some() {
            out.push(raw_path);
        }
    }
    out
}

fn psr4_paths_from_composer(raw: &str, project_root: &Path) -> Vec<PathBuf> {
    let composer = project_root.join("composer.json");
    let Ok(text) = std::fs::read_to_string(&composer) else { return Vec::new() };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else { return Vec::new() };
    let Some(map) = json.pointer("/autoload/psr-4").and_then(|v| v.as_object()) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = Vec::new();
    for (prefix, dir) in map {
        let dir_str = dir.as_str().unwrap_or("");
        let trimmed_prefix = prefix.trim_end_matches('\\');
        let Some(remainder) = raw.strip_prefix(trimmed_prefix) else {
            continue;
        };
        let cleaned = remainder.trim_start_matches('\\');
        let normalized = cleaned.replace('\\', std::path::MAIN_SEPARATOR_STR);
        let mut p = project_root.join(dir_str);
        p.push(normalized);
        out.push(p.with_extension("php"));
    }
    out
}
