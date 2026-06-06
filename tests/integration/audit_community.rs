use pulse::audit::community::{louvain, CommunityParams};

fn params() -> CommunityParams {
    CommunityParams { resolution: 1.0, max_passes: 10 }
}

fn undirected(edges: &[(usize, usize)], n: usize) -> Vec<Vec<(usize, f64)>> {
    let mut adjacency = vec![Vec::new(); n];
    for &(a, b) in edges {
        adjacency[a].push((b, 1.0));
        adjacency[b].push((a, 1.0));
    }
    adjacency
}

fn two_triangles_bridge() -> Vec<Vec<(usize, f64)>> {
    undirected(&[(0, 1), (0, 2), (1, 2), (3, 4), (3, 5), (4, 5), (2, 3)], 6)
}

#[test]
fn separates_two_loosely_connected_clusters() {
    let result = louvain(&two_triangles_bridge(), params());
    assert_eq!(result.community_count, 2, "two triangles joined by one edge form two communities");
    assert_eq!(result.assignment[0], result.assignment[1]);
    assert_eq!(result.assignment[1], result.assignment[2]);
    assert_eq!(result.assignment[3], result.assignment[4]);
    assert_eq!(result.assignment[4], result.assignment[5]);
    assert_ne!(result.assignment[2], result.assignment[3]);
    assert!(result.modularity > 0.0, "clear structure has positive modularity");
}

#[test]
fn a_clique_stays_one_community() {
    let result = louvain(&undirected(&[(0, 1), (0, 2), (1, 2)], 3), params());
    assert_eq!(result.community_count, 1);
    assert!(result.assignment.iter().all(|&c| c == result.assignment[0]));
}

#[test]
fn isolated_nodes_each_form_their_own_community() {
    let result = louvain(&vec![Vec::new(); 3], params());
    assert_eq!(result.community_count, 3);
    assert!((result.modularity).abs() < 1e-12, "no edges means zero modularity");
}

#[test]
fn empty_graph_is_handled() {
    let result = louvain(&[], params());
    assert!(result.assignment.is_empty());
    assert_eq!(result.community_count, 0);
}

#[test]
fn assignment_is_deterministic() {
    let graph = two_triangles_bridge();
    let first = louvain(&graph, params());
    let second = louvain(&graph, params());
    assert_eq!(first.assignment, second.assignment);
    assert_eq!(first.modularity.to_bits(), second.modularity.to_bits());
}

#[test]
fn an_extreme_resolution_penalty_prevents_any_merge() {
    let graph = two_triangles_bridge();
    let result = louvain(&graph, CommunityParams { resolution: 100.0, max_passes: 10 });
    assert_eq!(result.community_count, 6, "a large resolution makes every merge unprofitable");
}
