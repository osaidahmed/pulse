use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::thresholds::{AuditThresholds, CommunityThresholds};

use super::community::{louvain, CommunityParams};
use super::components::component_of;
use super::finding::{AuditFinding, AuditKind, AuditLocation, ImportConfidence, SplitComponentEvidence};
use super::graph::{ImportGraph, NodeIndex};

pub fn detect(graph: &ImportGraph, thresholds: &AuditThresholds) -> Vec<AuditFinding> {
    let community = thresholds.package_metrics.community;
    let adjacency = undirected_adjacency(graph);
    let result = louvain(&adjacency, CommunityParams { resolution: community.resolution, max_passes: community.max_passes });
    let tally = tally_by_directory(graph, &result.assignment);
    let out: Vec<AuditFinding> =
        tally.into_iter().filter_map(|(dir, counts)| split_finding(dir, &counts, &community)).collect();
    rank_and_cap(out, thresholds.package_metrics.max_arch_findings_reported)
}

fn undirected_adjacency(graph: &ImportGraph) -> Vec<Vec<(usize, f64)>> {
    let n = graph.registry.count();
    let mut adjacency = vec![Vec::new(); n];
    for u in 0..n {
        for &v in graph.adjacency.outgoing(NodeIndex(u as u32)) {
            let target = v.0 as usize;
            if target == u {
                continue;
            }
            adjacency[u].push((target, 1.0));
            adjacency[target].push((u, 1.0));
        }
    }
    adjacency
}

fn tally_by_directory(graph: &ImportGraph, assignment: &[usize]) -> BTreeMap<PathBuf, BTreeMap<usize, u32>> {
    let mut tally: BTreeMap<PathBuf, BTreeMap<usize, u32>> = BTreeMap::new();
    for (i, &community) in assignment.iter().enumerate() {
        let dir = component_of(graph.registry.path_of(NodeIndex(i as u32)));
        *tally.entry(dir).or_default().entry(community).or_insert(0) += 1;
    }
    tally
}

fn split_finding(
    dir: PathBuf,
    counts: &BTreeMap<usize, u32>,
    thresholds: &CommunityThresholds,
) -> Option<AuditFinding> {
    let file_count: u32 = counts.values().sum();
    if file_count < thresholds.min_split_files || counts.len() < 2 {
        return None;
    }
    let largest = counts.values().copied().max().unwrap_or(0);
    let cohesion = f64::from(largest) / f64::from(file_count);
    if cohesion >= thresholds.split_cohesion {
        return None;
    }
    let evidence = SplitComponentEvidence {
        component: dir.clone(),
        file_count,
        community_count: counts.len() as u32,
        cohesion,
        confidence: ImportConfidence::Medium,
    };
    Some(AuditFinding {
        kind: AuditKind::SplitComponent(evidence),
        representative_snippet: String::new(),
        support: file_count,
        file_count,
        idf_score: None,
        action_label: None,
        pattern_category: None,
        locality_entropy: None,
        p_value: None,
        locations: vec![AuditLocation { file: dir, line: 1 }],
    })
}

fn rank_and_cap(mut findings: Vec<AuditFinding>, cap: usize) -> Vec<AuditFinding> {
    findings.sort_by(|a, b| severity(b).partial_cmp(&severity(a)).unwrap_or(std::cmp::Ordering::Equal));
    findings.truncate(cap);
    findings
}

fn severity(f: &AuditFinding) -> f64 {
    match &f.kind {
        AuditKind::SplitComponent(e) => f64::from(e.file_count) * (1.0 - e.cohesion),
        _ => 0.0,
    }
}
