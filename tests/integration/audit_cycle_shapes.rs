use std::path::{Path, PathBuf};

use pulse::audit::finding::{AuditKind, CycleShape, ImportConfidence};
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

fn cycle_shape(edges: &[InputEdge]) -> Option<CycleShape> {
    let findings = run_from_edges(edges, profile, &t().audit);
    findings.iter().find_map(|f| match &f.kind {
        AuditKind::ImportCycle(c) => Some(c.shape),
        _ => None,
    })
}

#[test]
fn two_node_cycle_is_tiny() {
    let shape = cycle_shape(&[edge("a.rs", "b.rs"), edge("b.rs", "a.rs")]);
    assert_eq!(shape, Some(CycleShape::Tiny));
}

#[test]
fn three_node_ring_is_circle() {
    let shape = cycle_shape(&[edge("a.rs", "b.rs"), edge("b.rs", "c.rs"), edge("c.rs", "a.rs")]);
    assert_eq!(shape, Some(CycleShape::Circle));
}

#[test]
fn complete_triad_is_clique() {
    let edges = [
        edge("a.rs", "b.rs"),
        edge("b.rs", "a.rs"),
        edge("a.rs", "c.rs"),
        edge("c.rs", "a.rs"),
        edge("b.rs", "c.rs"),
        edge("c.rs", "b.rs"),
    ];
    assert_eq!(cycle_shape(&edges), Some(CycleShape::Clique));
}

#[test]
fn hub_with_leaves_is_star() {
    let edges = [
        edge("c.rs", "l1.rs"),
        edge("l1.rs", "c.rs"),
        edge("c.rs", "l2.rs"),
        edge("l2.rs", "c.rs"),
        edge("c.rs", "l3.rs"),
        edge("l3.rs", "c.rs"),
    ];
    assert_eq!(cycle_shape(&edges), Some(CycleShape::Star));
}

#[test]
fn ring_with_chords_is_chain() {
    let edges = [
        edge("a.rs", "b.rs"),
        edge("b.rs", "c.rs"),
        edge("c.rs", "d.rs"),
        edge("d.rs", "e.rs"),
        edge("e.rs", "a.rs"),
        edge("a.rs", "c.rs"),
        edge("c.rs", "e.rs"),
    ];
    assert_eq!(cycle_shape(&edges), Some(CycleShape::Chain));
}
