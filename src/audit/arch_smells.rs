use std::path::PathBuf;

use crate::thresholds::AuditThresholds;

use super::components::{Component, ComponentGraph};
use super::finding::{AuditFinding, AuditKind, AuditLocation, ImportConfidence, UnstableDepEvidence};

const MIN_DEPS: usize = 2;

pub fn unstable_dependencies(cg: &ComponentGraph, thresholds: &AuditThresholds) -> Vec<AuditFinding> {
    let threshold = thresholds.package_metrics.unstable_dep_strength;
    let mut out: Vec<AuditFinding> = cg
        .components
        .iter()
        .filter_map(|c| unstable_dep_finding(cg, c, threshold))
        .collect();
    out.sort_by(|a, b| severity(b).partial_cmp(&severity(a)).unwrap_or(std::cmp::Ordering::Equal));
    out.truncate(thresholds.package_metrics.max_arch_findings_reported);
    out
}

fn unstable_dep_finding(cg: &ComponentGraph, c: &Component, threshold: f64) -> Option<AuditFinding> {
    let total = c.deps.len();
    if total < MIN_DEPS {
        return None;
    }
    let higher: Vec<f64> = c
        .deps
        .iter()
        .map(|&d| cg.components[d].instability)
        .filter(|&i| i > c.instability)
        .collect();
    let strength = higher.len() as f64 / total as f64;
    if strength < threshold {
        return None;
    }
    let mean_higher = higher.iter().sum::<f64>() / higher.len() as f64;
    let evidence = UnstableDepEvidence {
        component: c.path.clone(),
        instability: c.instability,
        strength,
        gap: c.instability - mean_higher,
        unstable_deps: higher.len() as u32,
        total_deps: total as u32,
        confidence: ImportConfidence::Medium,
    };
    Some(arch_finding(AuditKind::UnstableDependency(evidence), c.path.clone(), c.file_count, higher.len() as u32))
}

fn arch_finding(kind: AuditKind, path: PathBuf, file_count: u32, support: u32) -> AuditFinding {
    AuditFinding {
        kind,
        representative_snippet: String::new(),
        support,
        file_count,
        idf_score: None,
        action_label: None,
        pattern_category: None,
        locality_entropy: None,
        p_value: None,
        locations: vec![AuditLocation { file: path, line: 1 }],
    }
}

fn severity(f: &AuditFinding) -> f64 {
    match &f.kind {
        AuditKind::UnstableDependency(e) => e.strength * e.gap.abs(),
        _ => 0.0,
    }
}
