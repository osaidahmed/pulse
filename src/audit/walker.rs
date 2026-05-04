use std::path::{Path, PathBuf};

use tree_sitter::{Node, Tree};

use crate::parse::Language;
use crate::thresholds::AuditThresholds;
use crate::walk::fingerprint::compute_subtree_fingerprint_seeded;

use super::lang_kinds;

#[derive(Debug, Clone)]
pub struct SubtreeRecord {
    pub fingerprint: u64,
    pub file: PathBuf,
    pub line: u32,
    pub depth: u32,
    pub named_node_count: u32,
    pub snippet: String,
}

pub fn extract_subtrees(
    tree: &Tree,
    source: &str,
    lang: Language,
    file: &Path,
    thresholds: &AuditThresholds,
) -> Vec<SubtreeRecord> {
    let mut out = Vec::new();
    let ctx = WalkCtx { source, lang, file, thresholds };
    visit(tree.root_node(), &ctx, &mut out);
    out
}

struct WalkCtx<'a> {
    source: &'a str,
    lang: Language,
    file: &'a Path,
    thresholds: &'a AuditThresholds,
}

fn visit(node: Node, ctx: &WalkCtx, out: &mut Vec<SubtreeRecord>) {
    if node.is_named() {
        consider(node, ctx, out);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit(child, ctx, out);
    }
}

fn consider(node: Node, ctx: &WalkCtx, out: &mut Vec<SubtreeRecord>) {
    if lang_kinds::is_skippable_root(ctx.lang, node.kind()) {
        return;
    }
    let depth = subtree_depth(node);
    let named_count = count_named(node);
    if (depth as usize) < ctx.thresholds.pattern_mining.subtree_min_depth
        || (named_count as usize) < ctx.thresholds.pattern_mining.subtree_min_nodes
    {
        return;
    }
    let fingerprint = compute_subtree_fingerprint_seeded(node, ctx.lang as u64);
    out.push(SubtreeRecord {
        fingerprint,
        file: ctx.file.to_path_buf(),
        line: node.start_position().row as u32 + 1,
        depth,
        named_node_count: named_count,
        snippet: snippet_for(node, ctx.source),
    });
}

fn subtree_depth(node: Node) -> u32 {
    let mut max = 1;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.is_named() {
            let d = 1 + subtree_depth(child);
            if d > max {
                max = d;
            }
        }
    }
    max
}

fn count_named(node: Node) -> u32 {
    let mut count = u32::from(node.is_named());
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        count += count_named(child);
    }
    count
}

fn snippet_for(node: Node, source: &str) -> String {
    let text = &source[node.byte_range()];
    text.lines()
        .next()
        .unwrap_or("")
        .trim()
        .chars()
        .take(80)
        .collect()
}
