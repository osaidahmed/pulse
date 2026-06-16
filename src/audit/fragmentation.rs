use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::thresholds::AuditThresholds;

use super::components::component_of;
use super::finding::{AuditFinding, AuditKind, AuditLocation, ImportConfidence};
use super::graph::{ImportGraph, NodeIndex};

#[derive(Debug, Clone, PartialEq)]
pub struct OverFragmentationEvidence {
    pub component: PathBuf,
    pub coupled_files: u32,
    pub internal_edges: u32,
    pub avg_loc: u32,
    pub density: f64,
    pub confidence: ImportConfidence,
}

#[derive(Default)]
struct DirStats {
    files: u32,
    internal_edges: u32,
    loc: u32,
}

pub fn detect(graph: &ImportGraph, loc_of: impl Fn(&Path) -> u32, thresholds: &AuditThresholds) -> Vec<AuditFinding> {
    let mut dirs: HashMap<PathBuf, DirStats> = HashMap::new();
    for i in 0..graph.registry.count() {
        let node = NodeIndex(i as u32);
        let path = graph.registry.path_of(node);
        let dir = component_of(path);
        let stats = dirs.entry(dir.clone()).or_default();
        stats.files += 1;
        stats.loc = stats.loc.saturating_add(loc_of(path));
        for &target in graph.adjacency.outgoing(node) {
            if component_of(graph.registry.path_of(target)) == dir {
                stats.internal_edges += 1;
            }
        }
    }
    let mut findings: Vec<AuditFinding> = dirs.iter().filter_map(|(dir, s)| finding_for(dir, s, thresholds)).collect();
    findings.sort_by(|a, b| density_of(b).partial_cmp(&density_of(a)).unwrap_or(std::cmp::Ordering::Equal));
    findings.truncate(thresholds.package_metrics.max_arch_findings_reported);
    findings
}

fn finding_for(dir: &Path, s: &DirStats, t: &AuditThresholds) -> Option<AuditFinding> {
    let frag = &t.package_metrics.fragmentation;
    if s.files < frag.min_coupled_files {
        return None;
    }
    let density = f64::from(s.internal_edges) / f64::from(s.files);
    if density < frag.min_density {
        return None;
    }
    let avg_loc = s.loc / s.files;
    if avg_loc > frag.max_avg_loc {
        return None;
    }
    Some(AuditFinding {
        kind: AuditKind::OverFragmentation(OverFragmentationEvidence {
            component: dir.to_path_buf(),
            coupled_files: s.files,
            internal_edges: s.internal_edges,
            avg_loc,
            density,
            confidence: ImportConfidence::Low,
        }),
        representative_snippet: format!(
            "{} small coupled files (avg {avg_loc} LOC), {} intra-directory imports (density {density:.2})",
            s.files, s.internal_edges
        ),
        support: s.internal_edges,
        file_count: s.files,
        idf_score: None,
        action_label: None,
        pattern_category: None,
        locality_entropy: None,
        p_value: None,
        locations: vec![AuditLocation { file: dir.to_path_buf(), line: 1 }],
    })
}

fn density_of(f: &AuditFinding) -> f64 {
    if let AuditKind::OverFragmentation(e) = &f.kind {
        e.density
    } else {
        0.0
    }
}
