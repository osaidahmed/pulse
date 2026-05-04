use std::path::PathBuf;

use crate::thresholds::AuditThresholds;

use super::finding::{ImportConfidence, MartinMetrics, MartinTier};
use super::graph::{ImportGraph, NodeIndex};

#[derive(Debug, Clone, Copy)]
pub struct AbstractnessRecord {
    pub abstractness: f64,
    pub confidence: ImportConfidence,
}

pub fn instability(afferent: u32, efferent: u32) -> f64 {
    let total = u64::from(afferent) + u64::from(efferent);
    if total == 0 {
        return 0.0;
    }
    f64::from(efferent) / total as f64
}

pub fn distance(abstractness: f64, instability: f64) -> f64 {
    (abstractness + instability - 1.0).abs()
}

pub fn classify(distance: f64, thresholds: &AuditThresholds) -> MartinTier {
    if distance >= thresholds.package_metrics.martin_distance_alert {
        return MartinTier::Alert;
    }
    if distance >= thresholds.package_metrics.martin_distance_warning {
        return MartinTier::Warning;
    }
    MartinTier::Healthy
}

pub fn compute(
    graph: &ImportGraph,
    node: NodeIndex,
    abstractness: AbstractnessRecord,
    import_confidence: ImportConfidence,
    thresholds: &AuditThresholds,
) -> MartinMetrics {
    let afferent = graph.adjacency.afferent(node);
    let efferent = graph.adjacency.efferent(node);
    let i = instability(afferent, efferent);
    let d = distance(abstractness.abstractness, i);
    let module = path_buf_of(graph, node);
    let confidence = import_confidence.min(abstractness.confidence);
    MartinMetrics {
        module,
        afferent,
        efferent,
        instability: i,
        abstractness: abstractness.abstractness,
        distance: d,
        tier: classify(d, thresholds),
        confidence,
    }
}

fn path_buf_of(graph: &ImportGraph, node: NodeIndex) -> PathBuf {
    graph.registry.path_of(node).to_path_buf()
}
