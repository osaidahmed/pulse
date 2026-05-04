use std::path::{Path, PathBuf};

use tree_sitter::Node;

use crate::parse::Language;

use super::imports::{line_of, node_text, RawImport};

const LUA_CALLEES: &[&str] = &["require"];
const R_CALLEES: &[&str] = &["library", "require", "source"];
const RUBY_CALLEES: &[&str] = &["require_relative", "require"];

pub fn match_node(node: Node, source: &str, lang: Language) -> Option<RawImport> {
    let callees = callees_for(lang)?;
    if !is_call_kind(node.kind()) {
        return None;
    }
    let mut cursor = node.walk();
    let callee = node.children(&mut cursor).find(Node::is_named)?;
    if callee.kind() != "identifier" {
        return None;
    }
    let name = node_text(callee, source);
    if !callees.contains(&name.as_str()) {
        return None;
    }
    let target = first_string_arg(node, source)?;
    Some(RawImport { target, line: line_of(node) })
}

pub fn candidates(raw: &str, source_file: &Path, project_root: &Path, lang: Language) -> Vec<PathBuf> {
    let parent = source_file.parent().unwrap_or(project_root);
    let exts: &[&str] = match lang {
        Language::Lua => &["lua"],
        Language::R => &["R", "r"],
        Language::Ruby => &["rb"],
        _ => return Vec::new(),
    };
    script_paths(raw, parent, project_root, exts)
}

fn callees_for(lang: Language) -> Option<&'static [&'static str]> {
    match lang {
        Language::Lua => Some(LUA_CALLEES),
        Language::R => Some(R_CALLEES),
        Language::Ruby => Some(RUBY_CALLEES),
        _ => None,
    }
}

fn is_call_kind(kind: &str) -> bool {
    matches!(kind, "function_call" | "call" | "call_expression")
}

fn first_string_arg(node: Node, source: &str) -> Option<String> {
    let mut node_cursor = node.walk();
    let args = node
        .children(&mut node_cursor)
        .find(|c| c.is_named() && matches!(c.kind(), "arguments" | "argument_list"))?;
    let mut arg_cursor = args.walk();
    for child in args.children(&mut arg_cursor) {
        if let Some(text) = string_descend(child, source) {
            return Some(text);
        }
    }
    None
}

fn string_descend(node: Node, source: &str) -> Option<String> {
    let kind = node.kind();
    if matches!(kind, "string" | "string_literal") {
        return literal_text_of(node, source);
    }
    if kind != "argument" {
        return None;
    }
    descend_argument(node, source)
}

fn literal_text_of(node: Node, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if matches!(child.kind(), "string_content" | "string_fragment") {
            return Some(node_text(child, source));
        }
    }
    strip_quotes(&node_text(node, source))
}

fn descend_argument(node: Node, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(text) = string_descend(child, source) {
            return Some(text);
        }
    }
    None
}

fn strip_quotes(s: &str) -> Option<String> {
    let stripped = s
        .strip_prefix('"')
        .and_then(|r| r.strip_suffix('"'))
        .or_else(|| s.strip_prefix('\'').and_then(|r| r.strip_suffix('\'')))
        .or_else(|| s.strip_prefix('`').and_then(|r| r.strip_suffix('`')))?;
    Some(stripped.to_string())
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
