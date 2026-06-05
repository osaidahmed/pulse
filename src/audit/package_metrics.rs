use std::path::PathBuf;

use crate::thresholds::AuditThresholds;

use super::cycle_shapes;
use super::cycles::{find_cycles, Scc};
use super::finding::{
    AuditFinding, AuditKind, AuditLocation, CycleMembership, CycleShape, ImportConfidence, MartinTier,
};
use super::graph::{ImportGraph, InputEdge, NodeIndex};
use super::martin::{compute, AbstractnessRecord};

#[derive(Debug, Clone)]
pub struct ModuleProfile {
    pub abstractness: AbstractnessRecord,
    pub import_confidence: ImportConfidence,
}

pub fn run_from_edges(
    edges: &[InputEdge],
    profile_lookup: impl Fn(&std::path::Path) -> ModuleProfile,
    thresholds: &AuditThresholds,
) -> Vec<AuditFinding> {
    run_with_module_count(edges, profile_lookup, thresholds, edges.len() as u32)
}

pub fn run_with_module_count(
    edges: &[InputEdge],
    profile_lookup: impl Fn(&std::path::Path) -> ModuleProfile,
    thresholds: &AuditThresholds,
    total_modules: u32,
) -> Vec<AuditFinding> {
    let graph = ImportGraph::build(edges);
    if graph.adjacency.edges().is_empty() {
        return zero_edge_finding(total_modules);
    }
    let mut findings: Vec<AuditFinding> = Vec::new();
    findings.extend(martin_findings(&graph, &profile_lookup, thresholds));
    findings.extend(cycle_findings(&graph, &profile_lookup, thresholds));
    findings
}

fn zero_edge_finding(module_count: u32) -> Vec<AuditFinding> {
    vec![AuditFinding {
        kind: AuditKind::ZeroEdgeProject { module_count },
        representative_snippet: String::new(),
        support: 0,
        file_count: module_count,
        idf_score: None,
        action_label: None,
        pattern_category: None,
        locality_entropy: None,
        p_value: None,
        locations: Vec::new(),
    }]
}

fn martin_findings(
    graph: &ImportGraph,
    profile_lookup: &impl Fn(&std::path::Path) -> ModuleProfile,
    thresholds: &AuditThresholds,
) -> Vec<AuditFinding> {
    let mut interesting: Vec<AuditFinding> = (0..graph.registry.count() as u32)
        .map(NodeIndex)
        .filter_map(|node| martin_finding_for(graph, node, profile_lookup, thresholds))
        .collect();
    interesting.sort_by(sort_martin);
    interesting.truncate(thresholds.package_metrics.max_martin_findings_reported);
    interesting
}

fn martin_finding_for(
    graph: &ImportGraph,
    node: NodeIndex,
    profile_lookup: &impl Fn(&std::path::Path) -> ModuleProfile,
    thresholds: &AuditThresholds,
) -> Option<AuditFinding> {
    let path = graph.registry.path_of(node);
    let profile = profile_lookup(path);
    let metrics = compute(graph, node, profile.abstractness, profile.import_confidence, thresholds);
    if metrics.tier == MartinTier::Healthy {
        return None;
    }
    let module_path: PathBuf = path.to_path_buf();
    Some(AuditFinding {
        kind: AuditKind::DistanceFromMainSequence(metrics.clone()),
        representative_snippet: format!(
            "instability: {:.3} | abstractness: {:.3} | distance: {:.3}",
            metrics.instability, metrics.abstractness, metrics.distance
        ),
        support: metrics.afferent + metrics.efferent,
        file_count: 1,
        idf_score: None,
        action_label: None,
        pattern_category: None,
        locality_entropy: None,
        p_value: None,
        locations: vec![AuditLocation { file: module_path, line: 1 }],
    })
}

fn cycle_findings(
    graph: &ImportGraph,
    profile_lookup: &impl Fn(&std::path::Path) -> ModuleProfile,
    thresholds: &AuditThresholds,
) -> Vec<AuditFinding> {
    let sccs = find_cycles(graph, thresholds.package_metrics.martin_cycle_min_size);
    let mut shaped: Vec<(Scc, CycleShape)> = sccs
        .into_iter()
        .map(|scc| {
            let shape = cycle_shapes::classify(&scc.members, &scc.edges);
            (scc, shape)
        })
        .collect();
    shaped.sort_by(|a, b| cycle_severity(b).partial_cmp(&cycle_severity(a)).unwrap_or(std::cmp::Ordering::Equal));
    shaped.truncate(thresholds.package_metrics.max_cycle_findings_reported);
    shaped
        .into_iter()
        .map(|(scc, shape)| cycle_finding(graph, scc, shape, profile_lookup))
        .collect()
}

fn cycle_severity(item: &(Scc, CycleShape)) -> f64 {
    cycle_shapes::shape_weight(item.1) * item.0.members.len() as f64
}

fn cycle_finding(
    graph: &ImportGraph,
    scc: Scc,
    shape: CycleShape,
    profile_lookup: &impl Fn(&std::path::Path) -> ModuleProfile,
) -> AuditFinding {
    let members: Vec<PathBuf> = scc
        .members
        .iter()
        .map(|n| graph.registry.path_of(*n).to_path_buf())
        .collect();
    let edges: Vec<(PathBuf, PathBuf)> = scc
        .edges
        .iter()
        .map(|(a, b)| {
            (
                graph.registry.path_of(*a).to_path_buf(),
                graph.registry.path_of(*b).to_path_buf(),
            )
        })
        .collect();
    let confidence = members
        .iter()
        .map(|p| profile_lookup(p).import_confidence)
        .min()
        .unwrap_or(ImportConfidence::High);
    let locations = members
        .iter()
        .map(|p| AuditLocation { file: p.clone(), line: 1 })
        .collect();
    let support = edges.len() as u32;
    let file_count = members.len() as u32;
    AuditFinding {
        kind: AuditKind::ImportCycle(CycleMembership { members, edges, confidence, shape }),
        representative_snippet: format!("cycle of {file_count} modules"),
        support,
        file_count,
        idf_score: None,
        action_label: None,
        pattern_category: None,
        locality_entropy: None,
        p_value: None,
        locations,
    }
}

fn sort_martin(a: &AuditFinding, b: &AuditFinding) -> std::cmp::Ordering {
    let da = distance_of(a);
    let db = distance_of(b);
    db.partial_cmp(&da).unwrap_or(std::cmp::Ordering::Equal)
}

fn distance_of(f: &AuditFinding) -> f64 {
    let AuditKind::DistanceFromMainSequence(m) = &f.kind else {
        return 0.0;
    };
    m.distance
}
