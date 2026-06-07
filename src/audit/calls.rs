use tree_sitter::{Node, Tree};

use crate::parse::Language;

use super::call_method_dotted;
use super::call_path_qualified;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawCall {
    pub callee_name: String,
    pub receiver_hint: Option<String>,
    pub line: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallDispatch {
    MethodDotted,
    PathQualified,
}

const DISPATCH_TABLE: &[(Language, &[CallDispatch])] = &[
    (Language::Python, &[CallDispatch::MethodDotted]),
    (Language::JavaScript, &[CallDispatch::MethodDotted]),
    (Language::TypeScript, &[CallDispatch::MethodDotted]),
    (Language::Java, &[CallDispatch::MethodDotted]),
    (Language::CSharp, &[CallDispatch::MethodDotted]),
    (Language::Kotlin, &[CallDispatch::MethodDotted]),
    (Language::Swift, &[CallDispatch::MethodDotted]),
    (Language::Ruby, &[CallDispatch::MethodDotted]),
    (Language::Php, &[CallDispatch::MethodDotted, CallDispatch::PathQualified]),
    (Language::Groovy, &[CallDispatch::MethodDotted]),
    (Language::Rust, &[CallDispatch::PathQualified, CallDispatch::MethodDotted]),
    (Language::Go, &[CallDispatch::MethodDotted]),
    (Language::Cpp, &[CallDispatch::PathQualified, CallDispatch::MethodDotted]),
    (Language::C, &[CallDispatch::MethodDotted]),
    (Language::Haskell, &[CallDispatch::MethodDotted]),
    (Language::D, &[CallDispatch::MethodDotted, CallDispatch::PathQualified]),
];

pub fn extract_calls(tree: &Tree, source: &str, lang: Language) -> Vec<RawCall> {
    let dispatchers = dispatchers_for_lang(lang);
    if dispatchers.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    visit(tree.root_node(), source, dispatchers, &mut out);
    out
}

pub fn dispatchers_for_lang(lang: Language) -> &'static [CallDispatch] {
    DISPATCH_TABLE.iter().find(|(l, _)| *l == lang).map_or(&[][..], |(_, d)| *d)
}

fn visit(node: Node, source: &str, dispatchers: &[CallDispatch], out: &mut Vec<RawCall>) {
    if let Some(call) = match_single(node, source, dispatchers) {
        out.push(call);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit(child, source, dispatchers, out);
    }
}

pub fn match_single(node: Node, source: &str, dispatchers: &[CallDispatch]) -> Option<RawCall> {
    for d in dispatchers {
        let result = match d {
            CallDispatch::MethodDotted => call_method_dotted::match_node(node, source),
            CallDispatch::PathQualified => call_path_qualified::match_node(node, source),
        };
        if let Some(call) = result {
            return Some(call);
        }
    }
    None
}

pub(super) fn line_of(node: Node) -> u32 {
    node.start_position().row as u32 + 1
}

pub(super) fn node_text<'a>(node: Node, source: &'a str) -> &'a str {
    &source[node.byte_range()]
}
