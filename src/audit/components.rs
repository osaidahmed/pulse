#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use super::graph::{ImportGraph, NodeIndex};

pub struct Component {
    pub path: PathBuf,
    pub file_count: u32,
    pub afferent: u32,
    pub efferent: u32,
    pub instability: f64,
    pub abstractness: f64,
    pub loc: u32,
    pub centrality: f64,
    pub deps: Vec<usize>,
}

pub struct MemberMetrics {
    pub abstractness: f64,
    pub loc: u32,
}

pub struct ComponentGraph {
    pub components: Vec<Component>,
}

pub fn build(graph: &ImportGraph, member_of: impl Fn(&Path) -> MemberMetrics) -> ComponentGraph {
    let (paths, file_comp) = assign_components(graph);
    let count = paths.len();
    let edges = component_edges(graph, &file_comp);
    let mut afferent = vec![0u32; count];
    let mut efferent = vec![0u32; count];
    let mut deps: Vec<Vec<usize>> = vec![Vec::new(); count];
    for &(from, to) in &edges {
        efferent[from] += 1;
        afferent[to] += 1;
        deps[from].push(to);
    }
    let (file_counts, abstractness, loc) = aggregate_members(graph, &file_comp, count, &member_of);
    let mut components = Vec::with_capacity(count);
    for c in 0..count {
        components.push(Component {
            path: paths[c].clone(),
            file_count: file_counts[c],
            afferent: afferent[c],
            efferent: efferent[c],
            instability: instability(afferent[c], efferent[c]),
            abstractness: abstractness[c],
            loc: loc[c],
            centrality: 0.0,
            deps: std::mem::take(&mut deps[c]),
        });
    }
    ComponentGraph { components }
}

fn assign_components(graph: &ImportGraph) -> (Vec<PathBuf>, Vec<usize>) {
    let n = graph.registry.count();
    let mut set: BTreeSet<PathBuf> = BTreeSet::new();
    for i in 0..n {
        set.insert(component_of(graph.registry.path_of(NodeIndex(i as u32))));
    }
    let paths: Vec<PathBuf> = set.into_iter().collect();
    let index: BTreeMap<PathBuf, usize> =
        paths.iter().cloned().enumerate().map(|(i, p)| (p, i)).collect();
    let file_comp = (0..n)
        .map(|i| index[&component_of(graph.registry.path_of(NodeIndex(i as u32)))])
        .collect();
    (paths, file_comp)
}

fn component_of(path: &Path) -> PathBuf {
    path.parent().map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}

fn component_edges(graph: &ImportGraph, file_comp: &[usize]) -> BTreeSet<(usize, usize)> {
    let mut edges = BTreeSet::new();
    for (i, &from) in file_comp.iter().enumerate() {
        for &t in graph.adjacency.outgoing(NodeIndex(i as u32)) {
            let to = file_comp[t.0 as usize];
            if from != to {
                edges.insert((from, to));
            }
        }
    }
    edges
}

fn aggregate_members(
    graph: &ImportGraph,
    file_comp: &[usize],
    count: usize,
    member_of: &impl Fn(&Path) -> MemberMetrics,
) -> (Vec<u32>, Vec<f64>, Vec<u32>) {
    let mut counts = vec![0u32; count];
    let mut sum_abs = vec![0.0f64; count];
    let mut sum_loc = vec![0u32; count];
    for (i, &c) in file_comp.iter().enumerate() {
        let m = member_of(graph.registry.path_of(NodeIndex(i as u32)));
        counts[c] += 1;
        sum_abs[c] += m.abstractness;
        sum_loc[c] = sum_loc[c].saturating_add(m.loc);
    }
    let abstractness = counts
        .iter()
        .zip(&sum_abs)
        .map(|(&n, &s)| if n == 0 { 0.0 } else { s / f64::from(n) })
        .collect();
    (counts, abstractness, sum_loc)
}

fn instability(afferent: u32, efferent: u32) -> f64 {
    let total = afferent + efferent;
    if total == 0 {
        0.0
    } else {
        f64::from(efferent) / f64::from(total)
    }
}
