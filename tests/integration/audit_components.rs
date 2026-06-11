use std::path::{Path, PathBuf};

use pulse::audit::components::{build, Component, ComponentGraph, MemberMetrics};
use pulse::audit::graph::{ImportGraph, InputEdge};
use pulse::parse::Language;

fn edge(src: &str, dst: &str) -> InputEdge {
    InputEdge {
        source: PathBuf::from(src),
        target: PathBuf::from(dst),
        source_lang: Language::Rust,
        target_lang: Language::Rust,
    }
}

fn graph_of(edges: &[InputEdge]) -> ComponentGraph {
    let graph = ImportGraph::build(edges);
    build(&graph, |_| MemberMetrics { abstractness: Some(0.0), loc: 0 })
}

fn component<'a>(cg: &'a ComponentGraph, dir: &str) -> &'a Component {
    cg.components
        .iter()
        .find(|c| c.path.as_path() == Path::new(dir))
        .unwrap_or_else(|| panic!("component {dir} not found"))
}

#[test]
fn files_group_into_directory_components() {
    let cg = graph_of(&[edge("src/a/foo.rs", "src/b/bar.rs"), edge("src/b/bar.rs", "src/a/baz.rs")]);
    assert_eq!(cg.components.len(), 2);
    assert_eq!(component(&cg, "src/a").file_count, 2);
    assert_eq!(component(&cg, "src/b").file_count, 1);
}

#[test]
fn acyclic_component_dependency_sets_instability_extremes() {
    let cg = graph_of(&[edge("src/a/foo.rs", "src/b/bar.rs")]);
    let a = component(&cg, "src/a");
    let b = component(&cg, "src/b");
    assert_eq!((a.efferent, a.afferent), (1, 0));
    assert!((a.instability - 1.0).abs() < 1e-9);
    assert_eq!((b.efferent, b.afferent), (0, 1));
    assert!(b.instability.abs() < 1e-9);
}

#[test]
fn mutual_component_dependency_is_balanced() {
    let cg = graph_of(&[edge("src/a/foo.rs", "src/b/bar.rs"), edge("src/b/bar.rs", "src/a/baz.rs")]);
    for dir in ["src/a", "src/b"] {
        let c = component(&cg, dir);
        assert_eq!((c.afferent, c.efferent), (1, 1));
        assert!((c.instability - 0.5).abs() < 1e-9);
    }
}

#[test]
fn intra_directory_edges_do_not_create_component_edges() {
    let cg = graph_of(&[edge("src/a/foo.rs", "src/a/bar.rs")]);
    let a = component(&cg, "src/a");
    assert_eq!((a.afferent, a.efferent), (0, 0));
    assert!(a.deps.is_empty());
}

#[test]
fn abstractness_is_the_member_mean() {
    let graph = ImportGraph::build(&[edge("src/a/foo.rs", "src/b/bar.rs"), edge("src/b/bar.rs", "src/a/baz.rs")]);
    let member_of = |p: &Path| -> MemberMetrics {
        let abstractness = if p.ends_with("foo.rs") { 1.0 } else { 0.0 };
        MemberMetrics { abstractness: Some(abstractness), loc: 0 }
    };
    let cg = build(&graph, member_of);
    let a = cg.components.iter().find(|c| c.path.as_path() == Path::new("src/a")).unwrap();
    assert!((a.abstractness - 0.5).abs() < 1e-9, "src/a has foo(1.0) + baz(0.0) -> mean 0.5");
}

#[test]
fn loc_is_the_member_sum() {
    let graph = ImportGraph::build(&[edge("src/a/foo.rs", "src/b/bar.rs"), edge("src/b/bar.rs", "src/a/baz.rs")]);
    let member_of = |p: &Path| -> MemberMetrics {
        let loc = if p.ends_with("foo.rs") { 120 } else { 30 };
        MemberMetrics { abstractness: Some(0.0), loc }
    };
    let cg = build(&graph, member_of);
    let a = cg.components.iter().find(|c| c.path.as_path() == Path::new("src/a")).unwrap();
    assert_eq!(a.loc, 150, "src/a sums foo(120) + baz(30)");
}
