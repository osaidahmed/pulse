use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::thresholds::{AuditThresholds, CommunityThresholds};

use super::community::{louvain, CommunityParams};
use super::components::component_of;
use super::finding::{
    AuditFinding, AuditKind, AuditLocation, ImportConfidence, MergeComponentsEvidence, MoveFileEvidence,
    SplitComponentEvidence,
};
use super::graph::{ImportGraph, NodeIndex};

pub fn detect(graph: &ImportGraph, thresholds: &AuditThresholds) -> Vec<AuditFinding> {
    let community = thresholds.package_metrics.community;
    let adjacency = undirected_adjacency(graph);
    let result =
        louvain(&adjacency, CommunityParams { resolution: community.resolution, max_passes: community.max_passes });
    let by_dir = tally_by_directory(graph, &result.assignment);
    let mut out: Vec<AuditFinding> =
        by_dir.iter().filter_map(|(dir, counts)| split_finding(dir.clone(), counts, &community)).collect();
    out.extend(merge_findings(&by_dir, &community));
    out.extend(move_findings(graph, &result.assignment, &community));
    rank_and_cap(out, thresholds.package_metrics.max_arch_findings_reported)
}

struct MoveTarget {
    home: PathBuf,
    total: u32,
    share: f64,
    lone_dirs: BTreeSet<PathBuf>,
}

fn move_findings(graph: &ImportGraph, assignment: &[usize], thresholds: &CommunityThresholds) -> Vec<AuditFinding> {
    let by_community = tally_by_community(graph, assignment);
    let mut out = Vec::new();
    for (community_id, counts) in &by_community {
        if let Some(target) = move_target(counts, thresholds) {
            collect_strays(&mut out, graph, assignment, *community_id, &target);
        }
    }
    out
}

fn tally_by_community(graph: &ImportGraph, assignment: &[usize]) -> BTreeMap<usize, BTreeMap<PathBuf, u32>> {
    assignment.iter().enumerate().fold(BTreeMap::new(), |mut tally, (i, &community)| {
        let dir = component_of(graph.registry.path_of(NodeIndex(i as u32)));
        *tally.entry(community).or_default().entry(dir).or_insert(0) += 1;
        tally
    })
}

fn move_target(counts: &BTreeMap<PathBuf, u32>, thresholds: &CommunityThresholds) -> Option<MoveTarget> {
    let total: u32 = counts.values().sum();
    if total < thresholds.min_split_files || counts.len() < 2 {
        return None;
    }
    let (home, home_count) = home_dir(counts);
    let share = f64::from(home_count) / f64::from(total);
    if share < thresholds.split_cohesion {
        return None;
    }
    let lone_dirs = counts.iter().filter(|&(_, &count)| count == 1).map(|(dir, _)| dir.clone()).collect();
    Some(MoveTarget { home, total, share, lone_dirs })
}

fn home_dir(counts: &BTreeMap<PathBuf, u32>) -> (PathBuf, u32) {
    let mut best = (PathBuf::new(), 0u32);
    for (dir, &count) in counts {
        if count > best.1 {
            best = (dir.clone(), count);
        }
    }
    best
}

fn collect_strays(
    out: &mut Vec<AuditFinding>,
    graph: &ImportGraph,
    assignment: &[usize],
    community_id: usize,
    target: &MoveTarget,
) {
    for (i, &community) in assignment.iter().enumerate() {
        if community != community_id {
            continue;
        }
        let path = graph.registry.path_of(NodeIndex(i as u32));
        let dir = component_of(path);
        if target.lone_dirs.contains(&dir) {
            out.push(move_finding(path, dir, target));
        }
    }
}

fn move_finding(file: &Path, current_dir: PathBuf, target: &MoveTarget) -> AuditFinding {
    let evidence = MoveFileEvidence {
        file: file.to_path_buf(),
        current_dir,
        target_dir: target.home.clone(),
        community_size: target.total,
        home_share: target.share,
        confidence: ImportConfidence::Medium,
    };
    arch_finding(
        AuditKind::MoveFile(evidence),
        target.total,
        1,
        vec![AuditLocation { file: file.to_path_buf(), line: 1 }],
    )
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
    Some(arch_finding(
        AuditKind::SplitComponent(evidence),
        file_count,
        file_count,
        vec![AuditLocation { file: dir, line: 1 }],
    ))
}

fn arch_finding(kind: AuditKind, support: u32, file_count: u32, locations: Vec<AuditLocation>) -> AuditFinding {
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
        locations,
    }
}

fn merge_findings(
    by_dir: &BTreeMap<PathBuf, BTreeMap<usize, u32>>,
    thresholds: &CommunityThresholds,
) -> Vec<AuditFinding> {
    let mut groups: BTreeMap<usize, Vec<(PathBuf, u32)>> = BTreeMap::new();
    for (dir, counts) in by_dir {
        if let Some((community, count)) = dominant_community(counts, thresholds) {
            groups.entry(community).or_default().push((dir.clone(), count));
        }
    }
    groups
        .values()
        .filter(|dirs| dirs.len() >= 2)
        .filter(|dirs| dirs.iter().map(|(_, count)| count).sum::<u32>() >= thresholds.min_split_files)
        .map(|dirs| merge_finding(dirs))
        .collect()
}

fn dominant_community(counts: &BTreeMap<usize, u32>, thresholds: &CommunityThresholds) -> Option<(usize, u32)> {
    let total: u32 = counts.values().sum();
    let (&community, &count) = counts.iter().max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0)))?;
    if count >= 2 && f64::from(count) / f64::from(total) >= thresholds.split_cohesion {
        Some((community, count))
    } else {
        None
    }
}

fn merge_finding(dirs: &[(PathBuf, u32)]) -> AuditFinding {
    let components: Vec<PathBuf> = dirs.iter().map(|(dir, _)| dir.clone()).collect();
    let community_files: u32 = dirs.iter().map(|(_, count)| count).sum();
    let locations = components.iter().map(|c| AuditLocation { file: c.clone(), line: 1 }).collect();
    let evidence = MergeComponentsEvidence {
        components: components.clone(),
        community_files,
        confidence: ImportConfidence::Medium,
    };
    arch_finding(AuditKind::MergeComponents(evidence), community_files, community_files, locations)
}

fn rank_and_cap(mut findings: Vec<AuditFinding>, cap: usize) -> Vec<AuditFinding> {
    findings.sort_by(|a, b| severity(b).partial_cmp(&severity(a)).unwrap_or(std::cmp::Ordering::Equal));
    findings.truncate(cap);
    findings
}

fn severity(f: &AuditFinding) -> f64 {
    match &f.kind {
        AuditKind::SplitComponent(e) => f64::from(e.file_count) * (1.0 - e.cohesion),
        AuditKind::MoveFile(e) => f64::from(e.community_size) * e.home_share,
        AuditKind::MergeComponents(e) => f64::from(e.community_files),
        _ => 0.0,
    }
}
