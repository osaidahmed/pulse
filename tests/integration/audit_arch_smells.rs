use std::path::{Path, PathBuf};

use pulse::audit::finding::{
    AuditKind, CompoundEvidence, GodComponentEvidence, HubLikeEvidence, ImportConfidence, UnstableDepEvidence,
};
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
        loc: 0,
    }
}

fn profile_with_loc(path: &Path, loc_of: impl Fn(&Path) -> u32) -> ModuleProfile {
    ModuleProfile {
        abstractness: AbstractnessRecord { abstractness: 0.0, confidence: ImportConfidence::High },
        import_confidence: ImportConfidence::High,
        loc: loc_of(path),
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

fn god_components(edges: &[InputEdge], loc_of: impl Fn(&Path) -> u32 + Copy) -> Vec<GodComponentEvidence> {
    run_from_edges(edges, |p| profile_with_loc(p, loc_of), &t().audit)
        .into_iter()
        .filter_map(|f| match f.kind {
            AuditKind::GodComponent(e) => Some(e),
            _ => None,
        })
        .collect()
}

fn compounds(edges: &[InputEdge]) -> Vec<CompoundEvidence> {
    run_from_edges(edges, profile, &t().audit)
        .into_iter()
        .filter_map(|f| match f.kind {
            AuditKind::CompoundArchSmell(e) => Some(e),
            _ => None,
        })
        .collect()
}

fn compounds_with_loc(edges: &[InputEdge], loc_of: impl Fn(&Path) -> u32 + Copy) -> Vec<CompoundEvidence> {
    run_from_edges(edges, |p| profile_with_loc(p, loc_of), &t().audit)
        .into_iter()
        .filter_map(|f| match f.kind {
            AuditKind::CompoundArchSmell(e) => Some(e),
            _ => None,
        })
        .collect()
}

fn unstable_hub_edges() -> Vec<InputEdge> {
    vec![
        edge("x/m.rs", "s/m.rs"),
        edge("y/m.rs", "s/m.rs"),
        edge("s/m.rs", "u1/m.rs"),
        edge("s/m.rs", "u2/m.rs"),
        edge("u1/m.rs", "a/m.rs"),
        edge("u1/m.rs", "b/m.rs"),
        edge("u2/m.rs", "c/m.rs"),
        edge("u2/m.rs", "d/m.rs"),
    ]
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
    assert!(e.centrality > 0.0, "centrality is populated from PageRank");
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
    let edges =
        [edge("x/m.rs", "s/m.rs"), edge("s/m.rs", "u1/m.rs"), edge("u1/m.rs", "a/m.rs"), edge("u1/m.rs", "b/m.rs")];
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
    assert!(h.centrality > 0.0, "centrality is populated from PageRank");
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
    let edges =
        [edge("a/m.rs", "b/m.rs"), edge("b/m.rs", "c/m.rs"), edge("c/m.rs", "d/m.rs"), edge("d/m.rs", "a/m.rs")];
    assert!(hub_likes(&edges).is_empty(), "a uniform ring has no hub");
}

#[test]
fn flags_oversized_component_against_the_loc_distribution() {
    let edges =
        [edge("g/m.rs", "a/m.rs"), edge("g/m.rs", "b/m.rs"), edge("g/m.rs", "c/m.rs"), edge("g/m.rs", "d/m.rs")];
    let gods = god_components(&edges, |p| if p.starts_with("g") { 900 } else { 100 });
    assert_eq!(gods.len(), 1, "only the outlier component is a god component");
    let g = &gods[0];
    assert_eq!(g.component.as_path(), Path::new("g"));
    assert_eq!((g.loc, g.file_count), (900, 1));
    assert!((g.density - 900.0).abs() < 1e-9, "density is loc per file");
    assert!(g.centrality > 0.0, "centrality is populated from PageRank");
    assert_eq!(g.confidence, ImportConfidence::Medium);
}

#[test]
fn uniform_component_sizes_have_no_god_component() {
    let edges =
        [edge("g/m.rs", "a/m.rs"), edge("g/m.rs", "b/m.rs"), edge("g/m.rs", "c/m.rs"), edge("g/m.rs", "d/m.rs")];
    assert!(god_components(&edges, |_| 200).is_empty(), "uniformly-sized components yield no outlier");
}

#[test]
fn single_component_is_not_a_god_component() {
    let edges = [edge("src/a.rs", "src/b.rs")];
    assert!(god_components(&edges, |_| 5000).is_empty(), "a single component has no distribution to stand out from");
}

#[test]
fn two_components_lack_a_distribution_for_god_detection() {
    let edges = [edge("small/m.rs", "large/m.rs")];
    assert!(
        god_components(&edges, |p| if p.starts_with("large") { 1000 } else { 100 }).is_empty(),
        "two components are too few to call one an outlier"
    );
}

#[test]
fn component_with_two_co_occurring_smells_is_a_compound() {
    let cs = compounds(&unstable_hub_edges());
    let s = cs.iter().find(|e| e.component.as_path() == Path::new("s")).expect("s is a compound");
    assert!(s.constituent_kinds.contains(&"unstable_dependency"));
    assert!(s.constituent_kinds.contains(&"hub_like_dependency"));
    assert!(s.combined_severity > 0.0);
    assert_eq!(s.confidence, ImportConfidence::Medium);
}

#[test]
fn a_lone_smell_is_not_a_compound() {
    let edges = [
        edge("in1/m.rs", "h/m.rs"),
        edge("in2/m.rs", "h/m.rs"),
        edge("in3/m.rs", "h/m.rs"),
        edge("h/m.rs", "out1/m.rs"),
        edge("h/m.rs", "out2/m.rs"),
        edge("h/m.rs", "out3/m.rs"),
    ];
    assert!(compounds(&edges).is_empty(), "a hub with no other smell is not a compound");
}

#[test]
fn god_component_joins_a_compound() {
    let cs = compounds_with_loc(&unstable_hub_edges(), |p| if p.starts_with("s") { 900 } else { 100 });
    let s = cs.iter().find(|e| e.component.as_path() == Path::new("s")).expect("s is a compound");
    assert!(s.constituent_kinds.contains(&"god_component"));
    assert!(s.constituent_kinds.contains(&"unstable_dependency"));
    assert!(s.constituent_kinds.contains(&"hub_like_dependency"));
    assert!(s.combined_severity > 0.0);
    assert_eq!(s.confidence, ImportConfidence::Medium);
}
