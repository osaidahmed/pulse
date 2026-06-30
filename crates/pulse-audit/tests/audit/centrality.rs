use pulse_audit::centrality::{pagerank, PageRankParams};

use crate::audit_common::*;

fn params() -> PageRankParams {
    let pr = t().audit.package_metrics.pagerank;
    PageRankParams { damping: pr.damping, max_iters: pr.max_iters, epsilon: pr.epsilon }
}

fn sum(ranks: &[f64]) -> f64 {
    ranks.iter().sum()
}

#[test]
fn ranks_form_a_probability_distribution() {
    let adjacency = vec![vec![1, 2], vec![2], vec![0], vec![0, 1]];
    let (ranks, converged) = pagerank(&adjacency, params());
    assert!(converged, "a small connected graph converges");
    assert!((sum(&ranks) - 1.0).abs() < 1e-6, "ranks sum to one");
}

#[test]
fn most_depended_upon_node_ranks_highest() {
    let adjacency = vec![vec![], vec![0], vec![0], vec![0]];
    let (ranks, _) = pagerank(&adjacency, params());
    let hub = ranks[0];
    assert!(ranks[1..].iter().all(|&r| hub > r), "the node every other depends on carries the most rank");
}

#[test]
fn empty_graph_has_no_ranks() {
    let (ranks, converged) = pagerank(&[], params());
    assert!(ranks.is_empty());
    assert!(converged);
}

#[test]
fn isolated_nodes_share_rank_uniformly() {
    let adjacency = vec![vec![], vec![], vec![]];
    let (ranks, converged) = pagerank(&adjacency, params());
    assert!(converged, "an all-dangling graph reaches the uniform fixpoint immediately");
    assert!(ranks.iter().all(|&r| (r - 1.0 / 3.0).abs() < 1e-9));
}

#[test]
fn ranks_are_deterministic() {
    let adjacency = vec![vec![1, 2], vec![2, 0], vec![0], vec![1]];
    let first = pagerank(&adjacency, params()).0;
    let second = pagerank(&adjacency, params()).0;
    let bits = |v: &[f64]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
    assert_eq!(bits(&first), bits(&second));
}
