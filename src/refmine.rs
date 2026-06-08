use std::collections::HashMap;

use tree_sitter::Node;

use crate::parse::{self, Language};

const MAX_DEPTH: u32 = 2000;

pub fn store_body(key: u64, body: &str) {
    let dir = crate::baselines::baseline_dir();
    let path = dir.join(format!("{key:016x}.body"));
    if path.exists() {
        return;
    }
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(path, body);
}

pub fn load_body(key: u64) -> Option<String> {
    std::fs::read_to_string(crate::baselines::baseline_dir().join(format!("{key:016x}.body"))).ok()
}

pub fn body_match_ratio(pre: &str, cur: &str, lang: Language) -> Option<f64> {
    let block_kinds = block_kinds(lang)?;
    let pre_stmts = statements(pre, lang, block_kinds);
    if pre_stmts.is_empty() {
        return None;
    }
    let matched = count_matches(&pre_stmts, &statements(cur, lang, block_kinds));
    Some(matched as f64 / pre_stmts.len() as f64)
}

fn block_kinds(lang: Language) -> Option<&'static [&'static str]> {
    let kinds = crate::langkinds::block_kinds(lang);
    (!kinds.is_empty()).then_some(kinds)
}

fn statements(src: &str, lang: Language, block_kinds: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(tree) = parse::parse_guarded(src, lang) {
        collect(tree.root_node(), src, block_kinds, &mut out, 0);
    }
    out
}

fn collect(node: Node, src: &str, block_kinds: &[&str], out: &mut Vec<String>, depth: u32) {
    if depth > MAX_DEPTH {
        return;
    }
    let is_block = block_kinds.contains(&node.kind());
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if is_block {
            if let Ok(text) = child.utf8_text(src.as_bytes()) {
                out.push(normalize(text));
            }
        }
        collect(child, src, block_kinds, out, depth + 1);
    }
}

fn normalize(text: &str) -> String {
    text.split_whitespace().collect::<Vec<&str>>().join(" ")
}

fn count_matches(pre: &[String], cur: &[String]) -> usize {
    let mut available: HashMap<&str, usize> = HashMap::new();
    for s in cur {
        *available.entry(s.as_str()).or_default() += 1;
    }
    let mut matched = 0;
    for s in pre {
        if let Some(count) = available.get_mut(s.as_str()) {
            if *count > 0 {
                *count -= 1;
                matched += 1;
            }
        }
    }
    matched
}
