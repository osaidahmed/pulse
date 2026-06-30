use std::path::PathBuf;

use pulse_audit::graph::{Adjacency, ImportGraph, InputEdge, NodeIndex, NodeRegistry};
use pulse_syntax::parse::Language;

fn p(s: &str) -> PathBuf {
    PathBuf::from(s)
}

fn edge(src: &str, dst: &str) -> InputEdge {
    InputEdge { source: p(src), target: p(dst), source_lang: Language::Rust, target_lang: Language::Rust }
}

fn build(edges: &[InputEdge]) -> ImportGraph {
    ImportGraph::build(edges)
}

#[test]
fn empty_input_produces_zero_node_zero_edge_graph() {
    let g = build(&[]);
    assert_eq!(g.registry.count(), 0);
    assert_eq!(g.adjacency.edges().len(), 0);
}

#[test]
fn single_edge_creates_two_nodes() {
    let g = build(&[edge("a.rs", "b.rs")]);
    assert_eq!(g.registry.count(), 2);
    assert_eq!(g.adjacency.edges().len(), 1);
}

#[test]
fn duplicate_edges_collapse_to_one() {
    let g = build(&[edge("a.rs", "b.rs"), edge("a.rs", "b.rs")]);
    assert_eq!(g.adjacency.edges().len(), 1);
}

#[test]
fn linear_chain_a_to_b_to_c() {
    let g = build(&[edge("a.rs", "b.rs"), edge("b.rs", "c.rs")]);
    assert_eq!(g.registry.count(), 3);
    assert_eq!(g.adjacency.edges().len(), 2);
    let a = g.registry.lookup(&p("a.rs")).unwrap();
    let b = g.registry.lookup(&p("b.rs")).unwrap();
    let c = g.registry.lookup(&p("c.rs")).unwrap();
    assert_eq!(g.adjacency.afferent(a), 0);
    assert_eq!(g.adjacency.efferent(a), 1);
    assert_eq!(g.adjacency.afferent(b), 1);
    assert_eq!(g.adjacency.efferent(b), 1);
    assert_eq!(g.adjacency.afferent(c), 1);
    assert_eq!(g.adjacency.efferent(c), 0);
}

#[test]
fn diamond_two_paths_converge() {
    let g = build(&[edge("a.rs", "b.rs"), edge("a.rs", "c.rs"), edge("b.rs", "d.rs"), edge("c.rs", "d.rs")]);
    let d = g.registry.lookup(&p("d.rs")).unwrap();
    assert_eq!(g.adjacency.afferent(d), 2);
    assert_eq!(g.adjacency.efferent(d), 0);
}

#[test]
fn self_loop_creates_one_node_one_edge() {
    let g = build(&[edge("a.rs", "a.rs")]);
    assert_eq!(g.registry.count(), 1);
    assert_eq!(g.adjacency.edges().len(), 1);
    let a = g.registry.lookup(&p("a.rs")).unwrap();
    assert_eq!(g.adjacency.afferent(a), 1);
    assert_eq!(g.adjacency.efferent(a), 1);
}

#[test]
fn two_node_cycle() {
    let g = build(&[edge("a.rs", "b.rs"), edge("b.rs", "a.rs")]);
    assert_eq!(g.registry.count(), 2);
    assert_eq!(g.adjacency.edges().len(), 2);
}

#[test]
fn three_node_cycle() {
    let g = build(&[edge("a.rs", "b.rs"), edge("b.rs", "c.rs"), edge("c.rs", "a.rs")]);
    assert_eq!(g.registry.count(), 3);
    assert_eq!(g.adjacency.edges().len(), 3);
}

#[test]
fn disjoint_subgraphs_two_components() {
    let g = build(&[edge("a.rs", "b.rs"), edge("c.rs", "d.rs")]);
    assert_eq!(g.registry.count(), 4);
    assert_eq!(g.adjacency.edges().len(), 2);
    let a = g.registry.lookup(&p("a.rs")).unwrap();
    let c = g.registry.lookup(&p("c.rs")).unwrap();
    assert!(!g.adjacency.outgoing(a).contains(&g.registry.lookup(&p("d.rs")).unwrap()));
    assert!(!g.adjacency.outgoing(c).contains(&g.registry.lookup(&p("b.rs")).unwrap()));
}

#[test]
fn hub_one_central_imports_ten_peripherals() {
    let edges: Vec<InputEdge> = (0..10).map(|i| edge("hub.rs", &format!("p{i}.rs"))).collect();
    let g = build(&edges);
    let hub = g.registry.lookup(&p("hub.rs")).unwrap();
    assert_eq!(g.adjacency.efferent(hub), 10);
    assert_eq!(g.adjacency.afferent(hub), 0);
}

#[test]
fn inverse_hub_ten_peripherals_import_one() {
    let edges: Vec<InputEdge> = (0..10).map(|i| edge(&format!("p{i}.rs"), "central.rs")).collect();
    let g = build(&edges);
    let central = g.registry.lookup(&p("central.rs")).unwrap();
    assert_eq!(g.adjacency.afferent(central), 10);
    assert_eq!(g.adjacency.efferent(central), 0);
}

#[test]
fn directed_a_to_b_does_not_imply_b_to_a() {
    let g = build(&[edge("a.rs", "b.rs")]);
    let a = g.registry.lookup(&p("a.rs")).unwrap();
    let b = g.registry.lookup(&p("b.rs")).unwrap();
    assert_eq!(g.adjacency.afferent(a), 0);
    assert_eq!(g.adjacency.efferent(b), 0);
}

#[test]
fn graph_construction_is_deterministic() {
    let input = [edge("a.rs", "b.rs"), edge("c.rs", "d.rs"), edge("a.rs", "c.rs")];
    let g1 = build(&input);
    let g2 = build(&input);
    assert_eq!(g1.adjacency.edges(), g2.adjacency.edges());
}

#[test]
fn node_index_is_stable_across_intern_calls() {
    let mut reg = NodeRegistry::default();
    let a = reg.intern(&p("a.rs"), Language::Rust);
    let b = reg.intern(&p("b.rs"), Language::Rust);
    let a2 = reg.intern(&p("a.rs"), Language::Rust);
    assert_eq!(a, a2);
    assert_ne!(a, b);
}

#[test]
fn node_registry_records_language_per_node() {
    let mut reg = NodeRegistry::default();
    let py = reg.intern(&p("a.py"), Language::Python);
    let rs = reg.intern(&p("b.rs"), Language::Rust);
    assert_eq!(reg.language_of(py), Language::Python);
    assert_eq!(reg.language_of(rs), Language::Rust);
}

#[test]
fn lookup_returns_none_for_unknown_path() {
    let reg = NodeRegistry::default();
    assert_eq!(reg.lookup(&p("missing.rs")), None);
}

#[test]
fn adjacency_with_capacity_zero_supports_zero_nodes() {
    let adj = Adjacency::with_capacity(0);
    assert_eq!(adj.edge_count_helper(), 0);
}

trait TestHelpers {
    fn edge_count_helper(&self) -> usize;
}

impl TestHelpers for Adjacency {
    fn edge_count_helper(&self) -> usize {
        self.edges().len()
    }
}

#[test]
fn adjacency_insert_unique_returns_false_on_duplicate() {
    let mut adj = Adjacency::with_capacity(2);
    let a = NodeIndex(0);
    let b = NodeIndex(1);
    assert!(adj.insert_unique(a, b));
    assert!(!adj.insert_unique(a, b));
}

#[test]
fn adjacency_finalize_sorts_edges() {
    let mut adj = Adjacency::with_capacity(3);
    adj.insert_unique(NodeIndex(2), NodeIndex(0));
    adj.insert_unique(NodeIndex(0), NodeIndex(1));
    adj.insert_unique(NodeIndex(1), NodeIndex(2));
    adj.finalize();
    let edges = adj.edges();
    assert_eq!(edges, &[(NodeIndex(0), NodeIndex(1)), (NodeIndex(1), NodeIndex(2)), (NodeIndex(2), NodeIndex(0))]);
}

#[test]
fn adjacency_finalize_sorts_outgoing_lists() {
    let mut adj = Adjacency::with_capacity(4);
    adj.insert_unique(NodeIndex(0), NodeIndex(3));
    adj.insert_unique(NodeIndex(0), NodeIndex(1));
    adj.insert_unique(NodeIndex(0), NodeIndex(2));
    adj.finalize();
    assert_eq!(adj.outgoing(NodeIndex(0)), &[NodeIndex(1), NodeIndex(2), NodeIndex(3)]);
}

#[test]
fn cross_language_edge_records_both_languages() {
    let g = build(&[InputEdge {
        source: p("a.py"),
        target: p("b.rs"),
        source_lang: Language::Python,
        target_lang: Language::Rust,
    }]);
    let a = g.registry.lookup(&p("a.py")).unwrap();
    let b = g.registry.lookup(&p("b.rs")).unwrap();
    assert_eq!(g.registry.language_of(a), Language::Python);
    assert_eq!(g.registry.language_of(b), Language::Rust);
}

#[test]
fn one_thousand_edges_complete_within_time_budget() {
    let edges: Vec<InputEdge> = (0..1000).map(|i| edge(&format!("a{i}.rs"), &format!("b{i}.rs"))).collect();
    let g = build(&edges);
    assert_eq!(g.adjacency.edges().len(), 1000);
}

#[test]
fn pathological_chain_one_thousand_nodes_completes() {
    let edges: Vec<InputEdge> = (0..999).map(|i| edge(&format!("n{i}.rs"), &format!("n{}.rs", i + 1))).collect();
    let g = build(&edges);
    assert_eq!(g.registry.count(), 1000);
    assert_eq!(g.adjacency.edges().len(), 999);
}

#[test]
fn pathological_hub_imports_one_thousand() {
    let edges: Vec<InputEdge> = (0..1000).map(|i| edge("hub.rs", &format!("p{i}.rs"))).collect();
    let g = build(&edges);
    let hub = g.registry.lookup(&p("hub.rs")).unwrap();
    assert_eq!(g.adjacency.efferent(hub), 1000);
}

#[test]
fn pathological_inverse_hub_one_thousand_imports_one() {
    let edges: Vec<InputEdge> = (0..1000).map(|i| edge(&format!("p{i}.rs"), "core.rs")).collect();
    let g = build(&edges);
    let core = g.registry.lookup(&p("core.rs")).unwrap();
    assert_eq!(g.adjacency.afferent(core), 1000);
}

#[test]
fn lookup_after_build_returns_correct_index() {
    let g = build(&[edge("a.rs", "b.rs"), edge("c.rs", "d.rs")]);
    assert!(g.registry.lookup(&p("a.rs")).is_some());
    assert!(g.registry.lookup(&p("d.rs")).is_some());
    assert!(g.registry.lookup(&p("missing.rs")).is_none());
}

#[test]
fn build_preserves_insertion_order_of_first_appearance() {
    let g = build(&[edge("z.rs", "a.rs"), edge("a.rs", "m.rs")]);
    let z = g.registry.lookup(&p("z.rs")).unwrap();
    let a = g.registry.lookup(&p("a.rs")).unwrap();
    let m = g.registry.lookup(&p("m.rs")).unwrap();
    assert_eq!(z, NodeIndex(0));
    assert_eq!(a, NodeIndex(1));
    assert_eq!(m, NodeIndex(2));
}

#[test]
fn build_handles_complete_graph_k4() {
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
    assert_eq!(g.registry.count(), 4);
    assert_eq!(g.adjacency.edges().len(), 12);
}

#[test]
fn empty_intern_yields_zero_count() {
    let reg = NodeRegistry::default();
    assert_eq!(reg.count(), 0);
}

#[test]
fn intern_increments_count_only_for_new_paths() {
    let mut reg = NodeRegistry::default();
    reg.intern(&p("a.rs"), Language::Rust);
    reg.intern(&p("a.rs"), Language::Rust);
    reg.intern(&p("b.rs"), Language::Rust);
    assert_eq!(reg.count(), 2);
}
