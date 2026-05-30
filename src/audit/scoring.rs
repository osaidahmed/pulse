use std::collections::HashMap;

use crate::thresholds::AuditThresholds;

use super::categorize;
use super::discovery::RawCluster;
use super::finding::{AuditFinding, AuditKind, AuditLocation, PatternCategory};
use super::mdl::{self, CorpusScale};
use super::walker::KindIndex;

pub struct ScoringCtx<'a> {
    pub kinds_by_fp: &'a KindIndex,
    pub size_by_fp: &'a HashMap<u64, u32>,
    pub corpus: CorpusScale,
    pub floor: f64,
    pub max_findings: usize,
}

pub fn build_findings(clusters: Vec<RawCluster>, ctx: &ScoringCtx) -> Vec<AuditFinding> {
    let mut findings: Vec<AuditFinding> = clusters
        .into_iter()
        .filter_map(|c| build_categorized_finding(c, ctx))
        .collect();
    findings.sort_by(rank_descending);
    findings.truncate(ctx.max_findings);
    findings
}

fn build_categorized_finding(cluster: RawCluster, ctx: &ScoringCtx) -> Option<AuditFinding> {
    let size = ctx.size_by_fp.get(&cluster.fingerprint).copied().unwrap_or(0);
    let gain = mdl::mdl_gain(cluster.support, size, ctx.corpus);
    if gain < ctx.floor {
        return None;
    }
    let category = ctx
        .kinds_by_fp
        .get(&cluster.fingerprint)
        .map_or(PatternCategory::Other, |kinds| categorize::categorize(kinds));
    Some(finding_from_cluster(cluster, Some(gain), category))
}

fn finding_from_cluster(
    cluster: RawCluster,
    score: Option<f64>,
    category: PatternCategory,
) -> AuditFinding {
    let locations = cluster
        .locations
        .into_iter()
        .map(|(file, line)| AuditLocation { file, line })
        .collect();
    AuditFinding {
        kind: AuditKind::UncategorizedPattern { fingerprint: cluster.fingerprint },
        representative_snippet: cluster.representative_snippet,
        support: cluster.support,
        file_count: cluster.file_count,
        idf_score: score,
        action_label: None,
        locations,
        pattern_category: Some(category),
    }
}

fn rank_descending(a: &AuditFinding, b: &AuditFinding) -> std::cmp::Ordering {
    let sa = a.idf_score.unwrap_or(0.0);
    let sb = b.idf_score.unwrap_or(0.0);
    sb.partial_cmp(&sa)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| b.support.cmp(&a.support))
        .then_with(|| fingerprint_of(a).cmp(&fingerprint_of(b)))
}

pub fn apply_idf(
    clusters: Vec<RawCluster>,
    total_files: usize,
    thresholds: &AuditThresholds,
) -> Vec<AuditFinding> {
    if total_files == 0 {
        return Vec::new();
    }
    let mut findings: Vec<AuditFinding> = clusters
        .into_iter()
        .filter(|c| !is_idiom(c, total_files, thresholds))
        .map(|c| legacy_finding(c, total_files))
        .collect();
    findings.sort_by(legacy_order);
    findings.truncate(thresholds.pattern_mining.max_findings_reported);
    findings
}

fn is_idiom(cluster: &RawCluster, total_files: usize, thresholds: &AuditThresholds) -> bool {
    let ratio = f64::from(cluster.file_count) / total_files as f64;
    ratio > thresholds.pattern_mining.idiom_suppression_threshold
}

fn legacy_finding(cluster: RawCluster, total_files: usize) -> AuditFinding {
    let idf = idf_score(cluster.file_count, total_files);
    finding_from_cluster(cluster, Some(idf), PatternCategory::Other)
}

fn idf_score(file_count: u32, total_files: usize) -> f64 {
    if file_count == 0 || total_files == 0 {
        return 0.0;
    }
    (total_files as f64 / f64::from(file_count)).ln()
}

fn legacy_order(a: &AuditFinding, b: &AuditFinding) -> std::cmp::Ordering {
    b.support
        .cmp(&a.support)
        .then(b.file_count.cmp(&a.file_count))
        .then(fingerprint_of(a).cmp(&fingerprint_of(b)))
}

fn fingerprint_of(f: &AuditFinding) -> u64 {
    let AuditKind::UncategorizedPattern { fingerprint } = f.kind else {
        return 0;
    };
    fingerprint
}
