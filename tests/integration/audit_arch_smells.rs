use std::path::{Path, PathBuf};

use pulse::audit::finding::{AuditKind, HubLikeEvidence, ImportConfidence, UnstableDepEvidence};
use pulse::audit::graph::InputEdge;
use pulse::audit::martin::AbstractnessRecord;
use pulse::audit::package_metrics::{run_from_edges, ModuleProfile};
use pulse::parse::Language;

use crate::audit_common::*;

fn edge(src: &str, dst: &str) -> InputEdge {
    InputEdge {
        source: PathBuf::from(src),
        target: PathBuf::from(dst),
        source_lang: Language::Rust,
        target_lang: Language::Rust,
    }
}

fn profile(_path: &Path) -> ModuleProfile {
    ModuleProfile {
        abstractness: AbstractnessRecord { abstractness: 0.0, confidence: ImportConfidence::High },
        import_confidence: ImportConfidence::High,
    }
}

fn unstable_deps(edges: &[InputEdge]) -> Vec<UnstableDepEvidence> {
    run_from_edges(edges, profile, &t().audit)
        .into_iter()
        .filter_map(|f| match f.kind {
            AuditKind::UnstableDependency(e) => Some(e),
            _ => None,
        })
        .collect()
}

fn hub_likes(edges: &[InputEdge]) -> Vec<HubLikeEvidence> {
    run_from_edges(edges, profile, &t().audit)
        .into_iter()
        .filter_map(|f| match f.kind {
            AuditKind::HubLikeDependency(e) => Some(e),
            _ => None,
        })
        .collect()
}

#[test]
fn flags_component_depending_on_less_stable_components() {
    let edges = [
        edge("x/m.rs", "s/m.rs"),
        edge("y/m.rs", "s/m.rs"),
        edge("s/m.rs", "u1/m.rs"),
        edge("s/m.rs", "u2/m.rs"),
        edge("u1/m.rs", "a/m.rs"),
        edge("u1/m.rs", "b/m.rs"),
        edge("u2/m.rs", "c/m.rs"),
        edge("u2/m.rs", "d/m.rs"),
    ];
    let uds = unstable_deps(&edges);
    let s = uds.iter().find(|e| e.component.as_path() == Path::new("s"));
    let e = s.expect("component s depends on two less-stable components");
    assert!((e.strength - 1.0).abs() < 1e-9);
    assert_eq!((e.unstable_deps, e.total_deps), (2, 2));
    assert!(e.gap < 0.0, "gap is negative when deps are more unstable");
    assert_eq!(e.confidence, ImportConfidence::Medium);
}

#[test]
fn stable_dependencies_are_not_flagged() {
    let edges = [
        edge("app/m.rs", "u1/m.rs"),
        edge("app/m.rs", "u2/m.rs"),
        edge("x/m.rs", "u1/m.rs"),
        edge("y/m.rs", "u2/m.rs"),
    ];
    assert!(unstable_deps(&edges).is_empty(), "depending on more-stable components is healthy");
}

#[test]
fn single_dependency_is_not_flagged() {
    let edges = [
        edge("x/m.rs", "s/m.rs"),
        edge("s/m.rs", "u1/m.rs"),
        edge("u1/m.rs", "a/m.rs"),
        edge("u1/m.rs", "b/m.rs"),
    ];
    let uds = unstable_deps(&edges);
    assert!(!uds.iter().any(|e| e.component.as_path() == Path::new("s")));
}

#[test]
fn balanced_high_traffic_component_is_a_hub() {
    let edges = [
        edge("in1/m.rs", "h/m.rs"),
        edge("in2/m.rs", "h/m.rs"),
        edge("in3/m.rs", "h/m.rs"),
        edge("h/m.rs", "out1/m.rs"),
        edge("h/m.rs", "out2/m.rs"),
        edge("h/m.rs", "out3/m.rs"),
    ];
    let hubs = hub_likes(&edges);
    let h = hubs.iter().find(|e| e.component.as_path() == Path::new("h")).expect("h is a hub");
    assert_eq!((h.afferent, h.efferent), (3, 3));
    assert_eq!(h.imbalance, 0);
    assert_eq!(h.confidence, ImportConfidence::Medium);
}

#[test]
fn unbalanced_component_is_not_a_hub() {
    let edges = [
        edge("u/m.rs", "a/m.rs"),
        edge("u/m.rs", "b/m.rs"),
        edge("u/m.rs", "c/m.rs"),
        edge("x/m.rs", "a/m.rs"),
        edge("y/m.rs", "b/m.rs"),
    ];
    assert!(!hub_likes(&edges).iter().any(|e| e.component.as_path() == Path::new("u")));
}

#[test]
fn uniform_ring_has_no_hub() {
    let edges = [
        edge("a/m.rs", "b/m.rs"),
        edge("b/m.rs", "c/m.rs"),
        edge("c/m.rs", "d/m.rs"),
        edge("d/m.rs", "a/m.rs"),
    ];
    assert!(hub_likes(&edges).is_empty(), "a uniform ring has no hub");
}
