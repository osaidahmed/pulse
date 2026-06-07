use std::path::PathBuf;

use crate::thresholds::AuditThresholds;

use super::components::{Component, ComponentGraph};
use super::finding::{
    AuditFinding, AuditKind, AuditLocation, GodComponentEvidence, HubLikeEvidence, ImportConfidence,
    UnstableDepEvidence,
};

const MIN_DEPS: usize = 2;
const MIN_COMPONENTS_FOR_GC: usize = 3;

pub fn unstable_dependencies(cg: &ComponentGraph, thresholds: &AuditThresholds) -> Vec<AuditFinding> {
    let threshold = thresholds.package_metrics.unstable_dep_strength;
    let out: Vec<AuditFinding> = cg.components.iter().filter_map(|c| unstable_dep_finding(cg, c, threshold)).collect();
    rank_and_cap(out, thresholds.package_metrics.max_arch_findings_reported)
}

struct HubBounds {
    median_in: f64,
    median_out: f64,
    ratio: f64,
}

pub fn hub_like_dependencies(cg: &ComponentGraph, thresholds: &AuditThresholds) -> Vec<AuditFinding> {
    let bounds = HubBounds {
        median_in: median(cg.components.iter().map(|c| c.afferent)),
        median_out: median(cg.components.iter().map(|c| c.efferent)),
        ratio: thresholds.package_metrics.hublike_imbalance_ratio,
    };
    let out: Vec<AuditFinding> = cg.components.iter().filter_map(|c| hub_finding(c, &bounds)).collect();
    rank_and_cap(out, thresholds.package_metrics.max_arch_findings_reported)
}

fn hub_finding(c: &Component, bounds: &HubBounds) -> Option<AuditFinding> {
    if f64::from(c.afferent) <= bounds.median_in || f64::from(c.efferent) <= bounds.median_out {
        return None;
    }
    let total = c.afferent + c.efferent;
    let imbalance = c.afferent.abs_diff(c.efferent);
    if total == 0 || f64::from(imbalance) >= bounds.ratio * f64::from(total) {
        return None;
    }
    let evidence = HubLikeEvidence {
        component: c.path.clone(),
        afferent: c.afferent,
        efferent: c.efferent,
        imbalance,
        centrality: c.centrality,
        confidence: ImportConfidence::Medium,
    };
    Some(arch_finding(AuditKind::HubLikeDependency(evidence), c.path.clone(), c.file_count, total))
}

pub fn god_components(cg: &ComponentGraph, thresholds: &AuditThresholds) -> Vec<AuditFinding> {
    if cg.components.len() < MIN_COMPONENTS_FOR_GC {
        return Vec::new();
    }
    let cutoff =
        percentile(cg.components.iter().map(|c| c.loc), thresholds.package_metrics.god_component_loc_percentile);
    let out: Vec<AuditFinding> = cg.components.iter().filter_map(|c| god_finding(c, cutoff)).collect();
    rank_and_cap(out, thresholds.package_metrics.max_arch_findings_reported)
}

fn god_finding(c: &Component, cutoff: f64) -> Option<AuditFinding> {
    if f64::from(c.loc) <= cutoff {
        return None;
    }
    let density = if c.file_count == 0 { f64::from(c.loc) } else { f64::from(c.loc) / f64::from(c.file_count) };
    let evidence = GodComponentEvidence {
        component: c.path.clone(),
        loc: c.loc,
        file_count: c.file_count,
        density,
        centrality: c.centrality,
        confidence: ImportConfidence::Medium,
    };
    Some(arch_finding(AuditKind::GodComponent(evidence), c.path.clone(), c.file_count, c.loc))
}

fn percentile(values: impl Iterator<Item = u32>, p: f64) -> f64 {
    let mut v: Vec<u32> = values.collect();
    v.sort_unstable();
    let n = v.len();
    if n == 0 {
        return 0.0;
    }
    let rank = p.clamp(0.0, 1.0) * (n - 1) as f64;
    let lo = rank.floor() as usize;
    let hi = rank.ceil() as usize;
    let frac = rank - lo as f64;
    f64::from(v[lo]) + frac * (f64::from(v[hi]) - f64::from(v[lo]))
}

fn median(values: impl Iterator<Item = u32>) -> f64 {
    let mut v: Vec<u32> = values.collect();
    v.sort_unstable();
    let n = v.len();
    if n == 0 {
        return 0.0;
    }
    if n % 2 == 1 {
        f64::from(v[n / 2])
    } else {
        f64::midpoint(f64::from(v[n / 2 - 1]), f64::from(v[n / 2]))
    }
}

fn rank_and_cap(mut findings: Vec<AuditFinding>, cap: usize) -> Vec<AuditFinding> {
    findings.sort_by(|a, b| arch_severity(b).partial_cmp(&arch_severity(a)).unwrap_or(std::cmp::Ordering::Equal));
    findings.truncate(cap);
    findings
}

fn unstable_dep_finding(cg: &ComponentGraph, c: &Component, threshold: f64) -> Option<AuditFinding> {
    let total = c.deps.len();
    if total < MIN_DEPS {
        return None;
    }
    let higher: Vec<f64> =
        c.deps.iter().map(|&d| cg.components[d].instability).filter(|&i| i > c.instability).collect();
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
        centrality: c.centrality,
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

pub fn arch_severity(f: &AuditFinding) -> f64 {
    match &f.kind {
        AuditKind::UnstableDependency(e) => e.centrality * e.strength * e.gap.abs(),
        AuditKind::HubLikeDependency(e) => e.centrality * f64::from(e.afferent + e.efferent),
        AuditKind::GodComponent(e) => e.centrality * e.density,
        _ => 0.0,
    }
}
