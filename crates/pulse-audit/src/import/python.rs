use std::path::{Path, PathBuf};

use tree_sitter::Node;

use crate::imports::{line_of, node_text, RawImport};

pub fn match_node(node: Node, source: &str) -> Option<RawImport> {
    if node.kind() == "import_statement" {
        return absolute_import(node, source);
    }
    if node.kind() == "import_from_statement" {
        return from_import(node, source);
    }
    None
}

pub fn candidates(raw: &str, source_file: &Path, project_root: &Path) -> Vec<PathBuf> {
    let parent = source_file.parent().unwrap_or(project_root);
    if raw.starts_with('.') {
        return relative_dotted_paths(raw, parent);
    }
    absolute_dotted_paths(raw, project_root)
}

pub fn path_suffix(raw: &str) -> Option<String> {
    let parts: Vec<&str> = raw.split('.').filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        return None;
    }
    let joined = parts.join(std::path::MAIN_SEPARATOR_STR);
    Some(format!("{}{}.py", std::path::MAIN_SEPARATOR_STR, joined))
}

fn absolute_import(node: Node, source: &str) -> Option<RawImport> {
    let mut cursor = node.walk();
    let dotted = node.children(&mut cursor).find(|c| c.is_named() && c.kind() == "dotted_name")?;
    Some(RawImport { target: node_text(dotted, source), line: line_of(node) })
}

fn from_import(node: Node, source: &str) -> Option<RawImport> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if !child.is_named() {
            continue;
        }
        if matches!(child.kind(), "dotted_name" | "relative_import") {
            return Some(RawImport { target: node_text(child, source), line: line_of(node) });
        }
    }
    None
}

fn relative_dotted_paths(raw: &str, parent: &Path) -> Vec<PathBuf> {
    let trimmed = raw.trim_start_matches('.');
    let dot_count = raw.len() - trimmed.len();
    let mut start = parent.to_path_buf();
    let walk_up = dot_count.saturating_sub(1);
    for _ in 0..walk_up {
        if let Some(p) = start.parent() {
            start = p.to_path_buf();
        }
    }
    let segments: Vec<&str> = trimmed.split('.').filter(|s| !s.is_empty()).collect();
    let mut p = start;
    for seg in &segments {
        p.push(seg);
    }
    vec![p.with_extension("py"), p.join("__init__.py")]
}

fn absolute_dotted_paths(raw: &str, project_root: &Path) -> Vec<PathBuf> {
    let segments: Vec<&str> = raw.split('.').filter(|s| !s.is_empty()).collect();
    if segments.is_empty() {
        return Vec::new();
    }
    let base_dirs: &[&str] = &["", "src"];
    let mut out: Vec<PathBuf> = Vec::new();
    for dir in base_dirs {
        let mut p = project_root.to_path_buf();
        if !dir.is_empty() {
            p.push(dir);
        }
        for seg in &segments {
            p.push(seg);
        }
        out.push(p.with_extension("py"));
        out.push(p.join("__init__.py"));
    }
    out
}
