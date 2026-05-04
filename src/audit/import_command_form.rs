use std::path::{Path, PathBuf};

use tree_sitter::Node;

use crate::parse::Language;

use super::imports::{line_of, node_text, RawImport};

pub fn match_node(node: Node, source: &str, lang: Language) -> Option<RawImport> {
    if lang == Language::Tcl {
        return match_tcl_command(node, source);
    }
    if lang == Language::Cobol {
        return match_cobol_statement(node, source);
    }
    None
}

pub fn candidates(raw: &str, source_file: &Path, project_root: &Path, lang: Language) -> Vec<PathBuf> {
    let parent = source_file.parent().unwrap_or(project_root);
    let exts: &[&str] = match lang {
        Language::Tcl => &["tcl"],
        Language::Cobol => &["cbl", "cob", "cpy"],
        _ => return Vec::new(),
    };
    script_paths(raw, parent, project_root, exts)
}

fn match_tcl_command(node: Node, source: &str) -> Option<RawImport> {
    if node.kind() != "command" {
        return None;
    }
    let words = collect_simple_words(node, source);
    let head = words.first()?.as_str();
    if head == "source" && words.len() >= 2 {
        return Some(RawImport { target: words[1].clone(), line: line_of(node) });
    }
    if head == "package" && words.len() >= 3 && words[1] == "require" {
        return Some(RawImport { target: words[2].clone(), line: line_of(node) });
    }
    None
}

fn collect_simple_words(node: Node, source: &str) -> Vec<String> {
    if node.kind() == "simple_word" {
        return vec![node_text(node, source)];
    }
    let mut out: Vec<String> = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.is_named() {
            out.extend(collect_simple_words(child, source));
        }
    }
    out
}

fn match_cobol_statement(node: Node, source: &str) -> Option<RawImport> {
    let kind = node.kind();
    if kind == "copy_statement" {
        let mut cursor = node.walk();
        let word = node
            .children(&mut cursor)
            .find(|c| c.is_named() && c.kind() == "WORD")?;
        return Some(RawImport { target: node_text(word, source), line: line_of(node) });
    }
    if kind != "call_statement" {
        return None;
    }
    let mut cursor = node.walk();
    let string_node = node
        .children(&mut cursor)
        .find(|c| c.is_named() && c.kind() == "string")?;
    let raw = node_text(string_node, source);
    let stripped = strip_double_quotes(&raw)?;
    Some(RawImport { target: stripped, line: line_of(node) })
}

fn strip_double_quotes(s: &str) -> Option<String> {
    s.strip_prefix('"')
        .and_then(|r| r.strip_suffix('"'))
        .map(str::to_string)
}

fn script_paths(raw: &str, parent: &Path, project_root: &Path, exts: &[&str]) -> Vec<PathBuf> {
    let normalized = raw.replace('.', std::path::MAIN_SEPARATOR_STR);
    let mut out: Vec<PathBuf> = Vec::new();
    let bases: [&Path; 2] = [parent, project_root];
    for base in bases {
        let candidate = base.join(&normalized);
        for ext in exts {
            out.push(candidate.with_extension(ext));
        }
        let raw_path = base.join(raw);
        if raw_path.extension().is_some() {
            out.push(raw_path.clone());
        }
        for ext in exts {
            out.push(raw_path.with_extension(ext));
        }
    }
    out
}
