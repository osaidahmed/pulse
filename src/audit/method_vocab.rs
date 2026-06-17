use std::collections::HashMap;
use std::path::PathBuf;

use tree_sitter::Node;

use crate::parse::Language;
use crate::walk::{node_text, DepthGuard};

use super::call_walker::FUNCTION_KINDS;
use super::corpus::CorpusFile;
use super::lang_kinds::is_identifier_kind;

const MIN_TOKEN_LEN: usize = 3;

pub type VocabByMethod = HashMap<(PathBuf, u32), Vec<String>>;

pub fn method_vocab_from(file: &CorpusFile) -> VocabByMethod {
    let Some((source, tree)) = file.parsed() else { return VocabByMethod::new() };
    let mut out = VocabByMethod::new();
    collect(tree.root_node(), source, file.lang, &file.path, &mut out);
    out
}

fn collect(node: Node, source: &str, lang: Language, path: &PathBuf, out: &mut VocabByMethod) {
    let Some(_guard) = DepthGuard::enter() else { return };
    if FUNCTION_KINDS.contains(&node.kind()) {
        let line = node.start_position().row as u32 + 1;
        let mut tokens = Vec::new();
        gather_identifiers(node, source, lang, &mut tokens);
        if !tokens.is_empty() {
            out.entry((path.clone(), line)).or_default().extend(tokens);
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect(child, source, lang, path, out);
    }
}

fn gather_identifiers(node: Node, source: &str, lang: Language, out: &mut Vec<String>) {
    let Some(_guard) = DepthGuard::enter() else { return };
    if is_identifier_kind(lang, node.kind()) {
        split_into_tokens(node_text(node, source), out);
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        gather_identifiers(child, source, lang, out);
    }
}

fn split_into_tokens(raw: &str, out: &mut Vec<String>) {
    let trimmed = raw.trim_start_matches(['$', '@', '&', '_']);
    let mut current = String::new();
    let mut prev_lower = false;
    for ch in trimmed.chars() {
        if ch.is_alphanumeric() {
            if ch.is_uppercase() && prev_lower {
                flush(&mut current, out);
            }
            current.push(ch.to_ascii_lowercase());
            prev_lower = ch.is_lowercase() || ch.is_numeric();
        } else {
            flush(&mut current, out);
            prev_lower = false;
        }
    }
    flush(&mut current, out);
}

fn flush(current: &mut String, out: &mut Vec<String>) {
    if current.len() >= MIN_TOKEN_LEN {
        out.push(std::mem::take(current));
    } else {
        current.clear();
    }
}
