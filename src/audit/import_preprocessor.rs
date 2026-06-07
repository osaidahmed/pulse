use std::path::{Path, PathBuf};

use tree_sitter::Node;

use crate::parse::Language;

use super::imports::{line_of, node_text, RawImport};

pub fn match_node(node: Node, source: &str, lang: Language) -> Option<RawImport> {
    if matches!(lang, Language::C | Language::Cpp | Language::ObjectiveC) && node.kind() == "preproc_include" {
        return preproc_include_target(node, source);
    }
    if lang == Language::ObjectiveC && node.kind() == "module_import" {
        return objc_module_import(node, source);
    }
    None
}

pub fn candidates(raw: &str, source_file: &Path, project_root: &Path, lang: Language) -> Vec<PathBuf> {
    let parent = source_file.parent().unwrap_or(project_root);
    let exts: &[&str] = match lang {
        Language::C => &["h", "c"],
        Language::Cpp => &["h", "hpp", "cpp", "cc", "cppm"],
        Language::ObjectiveC => &["h", "m"],
        _ => return Vec::new(),
    };
    relative_paths(raw, parent, project_root, exts)
}

fn preproc_include_target(node: Node, source: &str) -> Option<RawImport> {
    let mut cursor = node.walk();
    let literal = node.children(&mut cursor).find(|c| c.is_named() && c.kind() == "string_literal")?;
    let target = string_content_text(literal, source)?;
    Some(RawImport { target, line: line_of(node) })
}

fn objc_module_import(node: Node, source: &str) -> Option<RawImport> {
    let mut cursor = node.walk();
    let id = node.children(&mut cursor).find(|c| c.is_named() && c.kind() == "identifier")?;
    Some(RawImport { target: node_text(id, source), line: line_of(node) })
}

fn string_content_text(string_literal: Node, source: &str) -> Option<String> {
    let mut cursor = string_literal.walk();
    for child in string_literal.children(&mut cursor) {
        if child.kind() == "string_content" {
            return Some(node_text(child, source));
        }
    }
    let raw = node_text(string_literal, source);
    raw.strip_prefix('"').and_then(|r| r.strip_suffix('"')).map(str::to_string)
}

fn relative_paths(raw: &str, parent: &Path, project_root: &Path, exts: &[&str]) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    let bases: [&Path; 4] = [parent, project_root, project_root, project_root];
    let prefixes: [&str; 4] = ["", "", "include", "src"];
    for (base, prefix) in bases.iter().zip(prefixes.iter()) {
        let mut p = base.to_path_buf();
        if !prefix.is_empty() {
            p.push(prefix);
        }
        let direct = p.join(raw);
        if direct.extension().is_some() {
            out.push(direct.clone());
        }
        for ext in exts {
            out.push(direct.with_extension(ext));
        }
    }
    out
}
