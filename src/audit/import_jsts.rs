use std::path::{Path, PathBuf};

use tree_sitter::Node;

use crate::parse::Language;

use super::imports::{line_of, node_text, RawImport};

const JS_EXTENSIONS: &[&str] = &["js", "jsx", "mjs", "cjs", "ts", "tsx"];
const TS_EXTENSIONS: &[&str] = &["ts", "tsx", "d.ts", "js", "jsx", "mjs", "cjs"];

pub fn match_node(node: Node, source: &str) -> Option<RawImport> {
    if node.kind() == "import_statement" {
        return import_statement_target(node, source);
    }
    if node.kind() == "call_expression" {
        return call_expression_target(node, source);
    }
    None
}

pub fn candidates(raw: &str, source_file: &Path, project_root: &Path, lang: Language) -> Vec<PathBuf> {
    let parent = source_file.parent().unwrap_or(project_root);
    let exts = extensions_for(lang);
    let mut out = direct_paths(raw, parent, exts);
    out.extend(direct_paths(raw, project_root, exts));
    out.extend(tsconfig_path_mappings(raw, project_root, exts));
    out
}

fn extensions_for(lang: Language) -> &'static [&'static str] {
    if lang == Language::TypeScript {
        return TS_EXTENSIONS;
    }
    JS_EXTENSIONS
}

fn import_statement_target(node: Node, source: &str) -> Option<RawImport> {
    let mut cursor = node.walk();
    let string_node = node
        .children(&mut cursor)
        .find(|c| c.is_named() && c.kind() == "string")?;
    let target = literal_text_of(string_node, source)?;
    Some(RawImport { target, line: line_of(node) })
}

fn call_expression_target(node: Node, source: &str) -> Option<RawImport> {
    let mut cursor = node.walk();
    let callee = node.children(&mut cursor).find(Node::is_named)?;
    let is_require = callee.kind() == "identifier" && node_text(callee, source) == "require";
    let is_dynamic_import = callee.kind() == "import";
    if !is_require && !is_dynamic_import {
        return None;
    }
    let target = string_arg_text(node, source)?;
    Some(RawImport { target, line: line_of(node) })
}

fn string_arg_text(node: Node, source: &str) -> Option<String> {
    let mut node_cursor = node.walk();
    let args = node
        .children(&mut node_cursor)
        .find(|c| c.is_named() && c.kind() == "arguments")?;
    let mut arg_cursor = args.walk();
    let string_node = args
        .children(&mut arg_cursor)
        .find(|c| c.is_named() && c.kind() == "string")?;
    literal_text_of(string_node, source)
}

fn literal_text_of(string_node: Node, source: &str) -> Option<String> {
    let mut cursor = string_node.walk();
    for child in string_node.children(&mut cursor) {
        if matches!(child.kind(), "string_content" | "string_fragment") {
            return Some(node_text(child, source));
        }
    }
    None
}

fn direct_paths(raw: &str, base: &Path, exts: &[&str]) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    let direct = base.join(raw);
    if direct.extension().is_some() {
        out.push(direct.clone());
    }
    for ext in exts {
        out.push(direct.with_extension(ext));
    }
    for ext in exts {
        out.push(direct.join(format!("index.{ext}")));
    }
    out
}

fn tsconfig_path_mappings(raw: &str, project_root: &Path, exts: &[&str]) -> Vec<PathBuf> {
    let tsconfig = project_root.join("tsconfig.json");
    let Ok(text) = std::fs::read_to_string(&tsconfig) else { return Vec::new() };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&strip_json_comments(&text)) else {
        return Vec::new();
    };
    let Some(map) = json
        .pointer("/compilerOptions/paths")
        .and_then(|v| v.as_object())
    else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = Vec::new();
    for (pattern, replacements) in map {
        let Some(arr) = replacements.as_array() else { continue };
        let matched = match pattern.strip_suffix('*') {
            Some(prefix) => match raw.strip_prefix(prefix) {
                Some(rest) => rest,
                None => continue,
            },
            None if pattern == raw => "",
            None => continue,
        };
        for replacement in arr {
            let template = replacement.as_str().unwrap_or("");
            let resolved = template.replace('*', matched);
            let mut p = project_root.to_path_buf();
            p.push(&resolved);
            out.push(p.clone());
            for ext in exts {
                out.push(p.with_extension(ext));
            }
        }
    }
    out
}

fn strip_json_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '/' && chars.peek() == Some(&'/') {
            consume_line_comment(&mut chars, &mut out);
            continue;
        }
        out.push(c);
    }
    out
}

fn consume_line_comment(
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
    out: &mut String,
) {
    for c in chars.by_ref() {
        if c == '\n' {
            out.push('\n');
            return;
        }
    }
}
