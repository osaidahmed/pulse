use std::path::{Path, PathBuf};

use tree_sitter::{Node, Tree};

use pulse_syntax::parse::Language;

use super::finding::ImportConfidence;
use super::import;

#[derive(Debug, Clone)]
pub struct RawImport {
    pub target: String,
    pub line: u32,
}

pub fn extract_imports(tree: &Tree, source: &str, lang: Language) -> Vec<RawImport> {
    let mut out = Vec::new();
    visit(tree.root_node(), source, lang, &mut out);
    out
}

pub fn confidence_for(lang: Language) -> ImportConfidence {
    if is_high_confidence(lang) {
        return ImportConfidence::High;
    }
    if is_medium_confidence(lang) {
        return ImportConfidence::Medium;
    }
    if is_low_confidence(lang) {
        return ImportConfidence::Low;
    }
    if is_best_effort(lang) {
        return ImportConfidence::BestEffort;
    }
    ImportConfidence::BestEffort
}

fn is_high_confidence(lang: Language) -> bool {
    matches!(
        lang,
        Language::Rust
            | Language::Go
            | Language::Java
            | Language::CSharp
            | Language::Kotlin
            | Language::Swift
            | Language::Haskell
            | Language::Zig
            | Language::D
    )
}

fn is_medium_confidence(lang: Language) -> bool {
    matches!(
        lang,
        Language::Php
            | Language::Ruby
            | Language::ObjectiveC
            | Language::Groovy
            | Language::Python
            | Language::JavaScript
            | Language::TypeScript
    )
}

fn is_low_confidence(lang: Language) -> bool {
    matches!(lang, Language::C | Language::Cpp)
}

fn is_best_effort(lang: Language) -> bool {
    matches!(lang, Language::Lua | Language::R | Language::Tcl | Language::Cobol)
}

pub fn has_extractor(lang: Language) -> bool {
    is_high_confidence(lang) || is_medium_confidence(lang) || is_low_confidence(lang) || is_best_effort(lang)
}

#[allow(clippy::manual_find)]
pub fn resolve_target(raw: &str, source_file: &Path, project_root: &Path, lang: Language) -> Option<PathBuf> {
    for candidate in candidate_paths(raw, source_file, project_root, lang) {
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

pub fn resolve_by_suffix(raw: &str, lang: Language, typed_set: &std::collections::HashSet<PathBuf>) -> Option<PathBuf> {
    let suffix = path_suffix_for(raw, lang)?;
    typed_set.iter().filter(|p| p.to_string_lossy().ends_with(&suffix)).min().cloned()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DispatchGroup {
    CallForm,
    CommandForm,
    Preprocessor,
    Python,
    Php,
    JsTs,
    StructuredImport,
}

const GROUP_TABLE: &[(Language, DispatchGroup)] = &[
    (Language::Lua, DispatchGroup::CallForm),
    (Language::R, DispatchGroup::CallForm),
    (Language::Ruby, DispatchGroup::CallForm),
    (Language::Tcl, DispatchGroup::CommandForm),
    (Language::Cobol, DispatchGroup::CommandForm),
    (Language::C, DispatchGroup::Preprocessor),
    (Language::Cpp, DispatchGroup::Preprocessor),
    (Language::ObjectiveC, DispatchGroup::Preprocessor),
    (Language::Python, DispatchGroup::Python),
    (Language::Php, DispatchGroup::Php),
    (Language::JavaScript, DispatchGroup::JsTs),
    (Language::TypeScript, DispatchGroup::JsTs),
];

fn dispatch_group(lang: Language) -> DispatchGroup {
    GROUP_TABLE.iter().find(|(l, _)| *l == lang).map_or(DispatchGroup::StructuredImport, |(_, g)| *g)
}

fn candidate_paths(raw: &str, source_file: &Path, project_root: &Path, lang: Language) -> Vec<PathBuf> {
    match dispatch_group(lang) {
        DispatchGroup::CallForm => import::call_form::candidates(raw, source_file, project_root, lang),
        DispatchGroup::CommandForm => import::command_form::candidates(raw, source_file, project_root, lang),
        DispatchGroup::Preprocessor => import::preprocessor::candidates(raw, source_file, project_root, lang),
        DispatchGroup::Python => import::python::candidates(raw, source_file, project_root),
        DispatchGroup::Php => import::php::candidates(raw, source_file, project_root),
        DispatchGroup::JsTs => import::jsts::candidates(raw, source_file, project_root, lang),
        DispatchGroup::StructuredImport => import::kinds::candidates(raw, source_file, project_root, lang),
    }
}

fn path_suffix_for(raw: &str, lang: Language) -> Option<String> {
    if lang == Language::Python {
        return import::python::path_suffix(raw);
    }
    if lang == Language::Php {
        return import::php::path_suffix(raw);
    }
    import::kinds::path_suffix_for(raw, lang)
}

fn visit(node: Node, source: &str, lang: Language, out: &mut Vec<RawImport>) {
    if let Some(raw) = match_node_for_lang(node, source, lang) {
        out.push(raw);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit(child, source, lang, out);
    }
}

fn match_node_for_lang(node: Node, source: &str, lang: Language) -> Option<RawImport> {
    let group = dispatch_group(lang);
    if group == DispatchGroup::Python {
        return import::python::match_node(node, source);
    }
    if group == DispatchGroup::Php {
        return import::php::match_node(node, source);
    }
    if group == DispatchGroup::JsTs {
        return import::jsts::match_node(node, source);
    }
    if group == DispatchGroup::CallForm {
        return import::call_form::match_node(node, source, lang);
    }
    if group == DispatchGroup::CommandForm {
        return import::command_form::match_node(node, source, lang);
    }
    if group == DispatchGroup::Preprocessor {
        return import::preprocessor::match_node(node, source, lang);
    }
    import::kinds::match_node(node, source, lang)
}

pub(super) fn node_text(node: Node, source: &str) -> String {
    source[node.byte_range()].to_string()
}

pub(super) fn line_of(node: Node) -> u32 {
    node.start_position().row as u32 + 1
}
