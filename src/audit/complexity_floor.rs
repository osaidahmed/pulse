use std::collections::HashMap;

use crate::thresholds::ComplexityFloorThresholds;

use super::discovery::RawCluster;
use super::walker::{ShapeMetrics, SubtreeRecord};

pub fn shape_index(records: &[SubtreeRecord]) -> HashMap<u64, ShapeMetrics> {
    let mut out: HashMap<u64, ShapeMetrics> = HashMap::new();
    for r in records {
        out.entry(r.fingerprint).or_insert(r.shape);
    }
    out
}

pub fn passes_floor(metrics: &ShapeMetrics, thresholds: ComplexityFloorThresholds) -> bool {
    metrics.distinct_kinds >= thresholds.distinct_kind_floor
        && metrics.branching_factor >= thresholds.branching_factor_floor
}

pub fn filter_clusters(
    clusters: Vec<RawCluster>,
    shapes: &HashMap<u64, ShapeMetrics>,
    thresholds: ComplexityFloorThresholds,
) -> Vec<RawCluster> {
    clusters
        .into_iter()
        .filter(|c| {
            shapes
                .get(&c.fingerprint)
                .is_some_and(|m| passes_floor(m, thresholds))
        })
        .collect()
}
