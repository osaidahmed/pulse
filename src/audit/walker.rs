use std::collections::HashMap;
use std::path::{Path, PathBuf};

use tree_sitter::{Node, Tree};

use crate::parse::Language;
use crate::thresholds::AuditThresholds;
use crate::walk::fingerprint::compute_subtree_fingerprint_seeded;

use super::corpus_stats::{KindHistogram, PerFileFeatures, WelfordIdentifierStats, line_length_stats};
use super::lang_kinds;

pub type KindIndex = HashMap<u64, Vec<Box<str>>>;

#[derive(Debug, Clone, Copy, Default)]
pub struct ShapeMetrics {
    pub distinct_kinds: u32,
    pub branching_factor: f32,
    pub linear_chain_len: u32,
}

#[derive(Debug, Clone)]
pub struct SubtreeRecord {
    pub fingerprint: u64,
    pub parent_fingerprint: Option<u64>,
    pub file: PathBuf,
    pub line: u32,
    pub depth: u32,
    pub named_node_count: u32,
    pub snippet: String,
    pub shape: ShapeMetrics,
    #[allow(dead_code)]
    pub simhash: u64,
    #[allow(dead_code)]
    pub loc: u32,
}

pub struct WalkOutput {
    pub subtrees: Vec<SubtreeRecord>,
    pub features: PerFileFeatures,
    pub kinds_by_fp: KindIndex,
}

pub fn extract_records(
    tree: &Tree,
    source: &str,
    lang: Language,
    file: &Path,
    thresholds: &AuditThresholds,
) -> WalkOutput {
    let ctx = WalkCtx { source, lang, file, thresholds };
    let mut state = VisitState::default();
    visit(tree.root_node(), None, &ctx, &mut state);
    WalkOutput {
        subtrees: state.out,
        features: state.accum.finalize(file, source),
        kinds_by_fp: state.kinds_by_fp,
    }
}

pub fn extract_subtrees(
    tree: &Tree,
    source: &str,
    lang: Language,
    file: &Path,
    thresholds: &AuditThresholds,
) -> Vec<SubtreeRecord> {
    extract_records(tree, source, lang, file, thresholds).subtrees
}

struct WalkCtx<'a> {
    source: &'a str,
    lang: Language,
    file: &'a Path,
    thresholds: &'a AuditThresholds,
}

#[derive(Default)]
struct VisitState {
    out: Vec<SubtreeRecord>,
    accum: FeatureAccum,
    kinds_by_fp: KindIndex,
}

#[derive(Default)]
struct FeatureAccum {
    identifiers: WelfordIdentifierStats,
    histogram: KindHistogram,
    ast_node_count: u64,
}

impl FeatureAccum {
    fn observe(&mut self, node: Node, source: &str, lang: Language) {
        self.ast_node_count += 1;
        self.histogram.observe(node.kind());
        if lang_kinds::is_identifier_kind(lang, node.kind()) {
            let text = &source[node.byte_range()];
            self.identifiers.observe(text.chars().count() as u32);
        }
    }

    fn finalize(self, file: &Path, source: &str) -> PerFileFeatures {
        let (mean_id_len, var_id_len) = self.identifiers.finalize();
        let size_bytes = source.len() as u64;
        let nodes_per_byte = if size_bytes == 0 {
            0.0
        } else {
            self.ast_node_count as f64 / size_bytes as f64
        };
        let (max_line_len, median_line_len) = line_length_stats(source);
        PerFileFeatures {
            file: file.to_path_buf(),
            mean_id_len,
            var_id_len,
            ast_nodes_per_byte: nodes_per_byte,
            max_line_len,
            median_line_len,
            kind_histogram: self.histogram,
            size_bytes,
        }
    }
}

fn visit(
    node: Node,
    parent_fp: Option<u64>,
    ctx: &WalkCtx,
    state: &mut VisitState,
) {
    let mut next_parent = parent_fp;
    if node.is_named() {
        state.accum.observe(node, ctx.source, ctx.lang);
        if let Some(record) = consider(node, parent_fp, ctx) {
            next_parent = Some(record.fingerprint);
            state.kinds_by_fp.entry(record.fingerprint).or_insert_with(|| collect_kind_list(node));
            state.out.push(record);
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit(child, next_parent, ctx, state);
    }
}

fn collect_kind_list(node: Node) -> Vec<Box<str>> {
    let mut kinds: Vec<Box<str>> = Vec::new();
    push_kinds(node, &mut kinds);
    kinds
}

fn push_kinds(node: Node, out: &mut Vec<Box<str>>) {
    if node.is_named() {
        out.push(node.kind().into());
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        push_kinds(child, out);
    }
}

fn consider(node: Node, parent_fp: Option<u64>, ctx: &WalkCtx) -> Option<SubtreeRecord> {
    if lang_kinds::is_skippable_root(ctx.lang, node.kind()) {
        return None;
    }
    let depth = subtree_depth(node);
    let named_count = count_named(node);
    if (depth as usize) < ctx.thresholds.pattern_mining.subtree_min_depth
        || (named_count as usize) < ctx.thresholds.pattern_mining.subtree_min_nodes
    {
        return None;
    }
    let fingerprint = compute_subtree_fingerprint_seeded(node, ctx.lang as u64);
    Some(SubtreeRecord {
        fingerprint,
        parent_fingerprint: parent_fp,
        file: ctx.file.to_path_buf(),
        line: node.start_position().row as u32 + 1,
        depth,
        named_node_count: named_count,
        snippet: snippet_for(node, ctx.source),
        shape: shape_metrics_for(node, named_count),
        simhash: crate::walk::compute_simhash(node),
        loc: node.end_position().row.saturating_sub(node.start_position().row) as u32 + 1,
    })
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
    let mut combined = String::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if combined.is_empty() {
            combined.push_str(trimmed);
        } else {
            combined.push(' ');
            combined.push_str(trimmed);
        }
        if combined.chars().count() >= 12 {
            break;
        }
    }
    combined.chars().take(80).collect()
}

fn shape_metrics_for(node: Node, named_count: u32) -> ShapeMetrics {
    let mut distinct: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let chain = collect_shape(node, &mut distinct);
    let interior = named_count.saturating_sub(1);
    let branching = if chain == 0 {
        0.0
    } else {
        f32::from(u16::try_from(interior).unwrap_or(u16::MAX))
            / f32::from(u16::try_from(chain).unwrap_or(u16::MAX))
    };
    ShapeMetrics {
        distinct_kinds: distinct.len() as u32,
        branching_factor: branching,
        linear_chain_len: chain,
    }
}

fn collect_shape<'a>(node: Node<'a>, distinct: &mut std::collections::HashSet<&'a str>) -> u32 {
    if node.is_named() {
        distinct.insert(node.kind());
    }
    let mut max_chain = 1;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.is_named() {
            let c = 1 + collect_shape(child, distinct);
            if c > max_chain {
                max_chain = c;
            }
        }
    }
    max_chain
}
