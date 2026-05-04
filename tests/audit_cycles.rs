use std::path::PathBuf;

use pulse::audit::cycles::{find_cycles, Scc};
use pulse::audit::graph::{ImportGraph, InputEdge, NodeIndex};
use pulse::parse::Language;
use pulse::thresholds::Thresholds;

fn t() -> Thresholds {
    Thresholds::default()
}

fn p(s: &str) -> PathBuf {
    PathBuf::from(s)
}

fn edge(src: &str, dst: &str) -> InputEdge {
    InputEdge {
        source: p(src),
        target: p(dst),
        source_lang: Language::Rust,
        target_lang: Language::Rust,
    }
}

fn build(edges: &[InputEdge]) -> ImportGraph {
    ImportGraph::build(edges)
}

fn cycles(graph: &ImportGraph) -> Vec<Scc> {
    find_cycles(graph, t().audit.martin_cycle_min_size)
}

#[test]
fn empty_graph_has_no_cycles() {
    let g = build(&[]);
    assert!(cycles(&g).is_empty());
}

#[test]
fn dag_has_no_cycles() {
    let g = build(&[edge("a.rs", "b.rs"), edge("b.rs", "c.rs")]);
    assert!(cycles(&g).is_empty());
}

#[test]
fn diamond_has_no_cycles() {
    let g = build(&[
        edge("a.rs", "b.rs"),
        edge("a.rs", "c.rs"),
        edge("b.rs", "d.rs"),
        edge("c.rs", "d.rs"),
    ]);
    assert!(cycles(&g).is_empty());
}

#[test]
fn two_node_cycle_returns_one_scc_size_two() {
    let g = build(&[edge("a.rs", "b.rs"), edge("b.rs", "a.rs")]);
    let result = cycles(&g);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].members.len(), 2);
}

#[test]
fn three_node_cycle_returns_one_scc_size_three() {
    let g = build(&[
        edge("a.rs", "b.rs"),
        edge("b.rs", "c.rs"),
        edge("c.rs", "a.rs"),
    ]);
    let result = cycles(&g);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].members.len(), 3);
}

#[test]
fn two_disjoint_cycles_returns_two_sccs() {
    let g = build(&[
        edge("a.rs", "b.rs"),
        edge("b.rs", "a.rs"),
        edge("c.rs", "d.rs"),
        edge("d.rs", "c.rs"),
    ]);
    let result = cycles(&g);
    assert_eq!(result.len(), 2);
}

#[test]
fn nested_cycle_collapses_to_single_scc() {
    let g = build(&[
        edge("a.rs", "b.rs"),
        edge("b.rs", "c.rs"),
        edge("c.rs", "a.rs"),
        edge("b.rs", "a.rs"),
    ]);
    let result = cycles(&g);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].members.len(), 3);
}

#[test]
fn isolated_self_loop_yields_one_scc_size_one() {
    let g = build(&[edge("a.rs", "a.rs"), edge("b.rs", "c.rs")]);
    let result = cycles(&g);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].members.len(), 1);
}

#[test]
fn cycle_min_size_three_skips_two_node_cycle() {
    let g = build(&[edge("a.rs", "b.rs"), edge("b.rs", "a.rs")]);
    let result = find_cycles(&g, 3);
    assert!(result.is_empty());
}

#[test]
fn cycle_min_size_three_keeps_three_node_cycle() {
    let g = build(&[
        edge("a.rs", "b.rs"),
        edge("b.rs", "c.rs"),
        edge("c.rs", "a.rs"),
    ]);
    let result = find_cycles(&g, 3);
    assert_eq!(result.len(), 1);
}

#[test]
fn isolated_nodes_are_not_in_sccs() {
    let g = build(&[edge("a.rs", "b.rs")]);
    let result = cycles(&g);
    assert!(result.is_empty());
}

#[test]
fn scc_member_order_is_path_alphabetical() {
    let g = build(&[
        edge("z_late.rs", "a_first.rs"),
        edge("a_first.rs", "m_mid.rs"),
        edge("m_mid.rs", "z_late.rs"),
    ]);
    let result = cycles(&g);
    let paths: Vec<_> = result[0]
        .members
        .iter()
        .map(|n| g.registry.path_of(*n).to_string_lossy().into_owned())
        .collect();
    let mut sorted = paths.clone();
    sorted.sort();
    assert_eq!(paths, sorted);
}

#[test]
fn scc_edges_only_include_intra_member_edges() {
    let g = build(&[
        edge("a.rs", "b.rs"),
        edge("b.rs", "a.rs"),
        edge("a.rs", "outside.rs"),
    ]);
    let result = cycles(&g);
    assert_eq!(result[0].edges.len(), 2);
}

#[test]
fn complete_graph_k4_returns_one_scc_of_size_four() {
    let nodes = ["a.rs", "b.rs", "c.rs", "d.rs"];
    let mut edges: Vec<InputEdge> = Vec::new();
    for src in &nodes {
        for dst in &nodes {
            if src != dst {
                edges.push(edge(src, dst));
            }
        }
    }
    let g = build(&edges);
    let result = cycles(&g);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].members.len(), 4);
}

#[test]
fn iterative_tarjan_handles_ten_thousand_node_chain_without_stack_overflow() {
    let edges: Vec<InputEdge> = (0..9_999)
        .map(|i| edge(&format!("n{i}.rs"), &format!("n{}.rs", i + 1)))
        .collect();
    let g = build(&edges);
    let result = cycles(&g);
    assert!(result.is_empty());
}

#[test]
fn maximum_size_cycle_one_thousand_nodes() {
    let mut edges: Vec<InputEdge> = (0..999)
        .map(|i| edge(&format!("n{i}.rs"), &format!("n{}.rs", i + 1)))
        .collect();
    edges.push(edge("n999.rs", "n0.rs"));
    let g = build(&edges);
    let result = cycles(&g);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].members.len(), 1000);
}

#[test]
fn butterfly_two_cycles_share_one_node() {
    let g = build(&[
        edge("a.rs", "b.rs"),
        edge("b.rs", "a.rs"),
        edge("b.rs", "c.rs"),
        edge("c.rs", "b.rs"),
    ]);
    let result = cycles(&g);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].members.len(), 3);
}

#[test]
fn cycles_with_external_referrer_does_not_inflate_scc() {
    let g = build(&[
        edge("ext.rs", "a.rs"),
        edge("a.rs", "b.rs"),
        edge("b.rs", "a.rs"),
    ]);
    let result = cycles(&g);
    assert_eq!(result[0].members.len(), 2);
}

#[test]
fn cycles_with_external_destination_does_not_inflate_scc() {
    let g = build(&[
        edge("a.rs", "b.rs"),
        edge("b.rs", "a.rs"),
        edge("a.rs", "ext.rs"),
    ]);
    let result = cycles(&g);
    assert_eq!(result[0].members.len(), 2);
}

#[test]
fn cycles_are_deterministic_across_runs() {
    let input = [
        edge("a.rs", "b.rs"),
        edge("b.rs", "c.rs"),
        edge("c.rs", "a.rs"),
        edge("d.rs", "e.rs"),
        edge("e.rs", "d.rs"),
    ];
    let g1 = build(&input);
    let g2 = build(&input);
    let r1 = cycles(&g1);
    let r2 = cycles(&g2);
    assert_eq!(r1.len(), r2.len());
    for (a, b) in r1.iter().zip(r2.iter()) {
        assert_eq!(a.members.len(), b.members.len());
        assert_eq!(a.edges.len(), b.edges.len());
    }
}

#[test]
fn long_chain_into_cycle_does_not_extend_scc() {
    let g = build(&[
        edge("ext1.rs", "ext2.rs"),
        edge("ext2.rs", "ext3.rs"),
        edge("ext3.rs", "a.rs"),
        edge("a.rs", "b.rs"),
        edge("b.rs", "a.rs"),
    ]);
    let result = cycles(&g);
    assert_eq!(result[0].members.len(), 2);
}

#[test]
fn self_loop_kept_regardless_of_min_size() {
    let g = build(&[edge("a.rs", "a.rs"), edge("b.rs", "c.rs")]);
    let high_min = find_cycles(&g, 5);
    assert_eq!(high_min.len(), 1);
    assert_eq!(high_min[0].members.len(), 1);
}

#[test]
fn isolated_nodes_without_self_loops_are_not_cycles() {
    let g = build(&[edge("b.rs", "c.rs")]);
    let result = find_cycles(&g, 1);
    assert!(result.is_empty());
}

#[test]
fn empty_min_size_zero_is_treated_as_zero_or_one_no_panic() {
    let g = build(&[edge("a.rs", "b.rs"), edge("b.rs", "a.rs")]);
    let result = find_cycles(&g, 0);
    assert_eq!(result.len(), 1);
}

#[test]
fn path_lookup_after_cycle_returns_member_paths() {
    let g = build(&[
        edge("foo.rs", "bar.rs"),
        edge("bar.rs", "foo.rs"),
    ]);
    let result = cycles(&g);
    let names: Vec<_> = result[0]
        .members
        .iter()
        .map(|n| g.registry.path_of(*n).to_string_lossy().into_owned())
        .collect();
    assert!(names.contains(&"foo.rs".to_string()));
    assert!(names.contains(&"bar.rs".to_string()));
}

#[test]
fn scc_with_node_index_can_be_resolved_to_path() {
    let g = build(&[edge("a.rs", "b.rs"), edge("b.rs", "a.rs")]);
    let result = cycles(&g);
    for &m in &result[0].members {
        assert!(matches!(m, NodeIndex(_)));
        let path = g.registry.path_of(m);
        assert!(path == p("a.rs") || path == p("b.rs"));
    }
}
