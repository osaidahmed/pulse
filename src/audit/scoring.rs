use crate::thresholds::AuditThresholds;

use super::discovery::RawCluster;
use super::finding::{AuditFinding, AuditKind, AuditLocation};

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
        .map(|c| build_finding(c, total_files))
        .collect();
    findings.sort_by(order_findings);
    findings.truncate(thresholds.max_findings_reported);
    findings
}

fn is_idiom(cluster: &RawCluster, total_files: usize, thresholds: &AuditThresholds) -> bool {
    let ratio = f64::from(cluster.file_count) / total_files as f64;
    ratio > thresholds.idiom_suppression_threshold
}

fn build_finding(cluster: RawCluster, total_files: usize) -> AuditFinding {
    let idf = idf_score(cluster.file_count, total_files);
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
        idf_score: Some(idf),
        action_label: None,
        locations,
    }
}

fn idf_score(file_count: u32, total_files: usize) -> f64 {
    if file_count == 0 || total_files == 0 {
        return 0.0;
    }
    (total_files as f64 / f64::from(file_count)).ln()
}

fn order_findings(a: &AuditFinding, b: &AuditFinding) -> std::cmp::Ordering {
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
