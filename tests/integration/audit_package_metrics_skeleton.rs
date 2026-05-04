use std::path::{Path, PathBuf};

use pulse::audit::finding::{AuditKind, ImportConfidence};
use pulse::audit::graph::InputEdge;
use pulse::audit::martin::AbstractnessRecord;
use pulse::audit::package_metrics::{run_from_edges, ModuleProfile};
use pulse::parse::Language;
use pulse::thresholds::Thresholds;

fn t() -> Thresholds {
    Thresholds::default()
}

fn p(s: &str) -> PathBuf {
    PathBuf::from(s)
}

fn rust_edge(src: &str, dst: &str) -> InputEdge {
    InputEdge {
        source: p(src),
        target: p(dst),
        source_lang: Language::Rust,
        target_lang: Language::Rust,
    }
}

fn high_concrete_profile(_path: &Path) -> ModuleProfile {
    ModuleProfile {
        abstractness: AbstractnessRecord {
            abstractness: 0.0,
            confidence: ImportConfidence::High,
        },
        import_confidence: ImportConfidence::High,
    }
}

fn high_abstract_profile(_path: &Path) -> ModuleProfile {
    ModuleProfile {
        abstractness: AbstractnessRecord {
            abstractness: 1.0,
            confidence: ImportConfidence::High,
        },
        import_confidence: ImportConfidence::High,
    }
}

#[test]
fn empty_input_emits_zero_edge_finding() {
    let findings = run_from_edges(&[], high_concrete_profile, &t().audit);
    assert_eq!(findings.len(), 1);
    assert!(matches!(findings[0].kind, AuditKind::ZeroEdgeProject { .. }));
}

#[test]
fn dag_yields_only_distance_findings_no_cycles() {
    let edges = [rust_edge("a.rs", "b.rs"), rust_edge("b.rs", "c.rs")];
    let findings = run_from_edges(&edges, high_concrete_profile, &t().audit);
    let cycle_count = findings
        .iter()
        .filter(|f| matches!(f.kind, AuditKind::ImportCycle(_)))
        .count();
    assert_eq!(cycle_count, 0);
}

#[test]
fn two_node_cycle_yields_one_cycle_finding() {
    let edges = [rust_edge("a.rs", "b.rs"), rust_edge("b.rs", "a.rs")];
    let findings = run_from_edges(&edges, high_concrete_profile, &t().audit);
    let cycle_count = findings
        .iter()
        .filter(|f| matches!(f.kind, AuditKind::ImportCycle(_)))
        .count();
    assert_eq!(cycle_count, 1);
}

#[test]
fn cycle_finding_carries_member_paths() {
    let edges = [rust_edge("a.rs", "b.rs"), rust_edge("b.rs", "a.rs")];
    let findings = run_from_edges(&edges, high_concrete_profile, &t().audit);
    let cycle = findings
        .iter()
        .find(|f| matches!(f.kind, AuditKind::ImportCycle(_)))
        .unwrap();
    let AuditKind::ImportCycle(c) = &cycle.kind else { panic!() };
    assert_eq!(c.members.len(), 2);
}

#[test]
fn cycle_confidence_inherits_minimum_member_confidence() {
    let edges = [rust_edge("a.rs", "b.rs"), rust_edge("b.rs", "a.rs")];
    let findings = run_from_edges(
        &edges,
        |path| {
            let conf = if path.ends_with("b.rs") {
                ImportConfidence::Low
            } else {
                ImportConfidence::High
            };
            ModuleProfile {
                abstractness: AbstractnessRecord {
                    abstractness: 0.0,
                    confidence: ImportConfidence::High,
                },
                import_confidence: conf,
            }
        },
        &t().audit,
    );
    let cycle = findings
        .iter()
        .find(|f| matches!(f.kind, AuditKind::ImportCycle(_)))
        .unwrap();
    let AuditKind::ImportCycle(c) = &cycle.kind else { panic!() };
    assert_eq!(c.confidence, ImportConfidence::Low);
}

#[test]
fn distance_finding_emitted_for_unstable_concrete_module() {
    let edges = [
        rust_edge("hub.rs", "a.rs"),
        rust_edge("hub.rs", "b.rs"),
        rust_edge("hub.rs", "c.rs"),
    ];
    let findings = run_from_edges(&edges, high_concrete_profile, &t().audit);
    let distance_count = findings
        .iter()
        .filter(|f| matches!(f.kind, AuditKind::DistanceFromMainSequence(_)))
        .count();
    assert!(distance_count >= 1, "concrete unstable modules without abstractness should appear");
}

#[test]
fn distance_finding_suppressed_for_main_sequence_modules() {
    let edges = [rust_edge("a.rs", "abstract_b.rs")];
    let findings = run_from_edges(&edges, high_abstract_profile, &t().audit);
    let on_main_sequence = findings
        .iter()
        .filter(|f| matches!(&f.kind, AuditKind::DistanceFromMainSequence(m) if m.distance < 0.7))
        .count();
    assert_eq!(on_main_sequence, 0);
}

#[test]
fn martin_findings_are_truncated_to_max_reported() {
    let mut th = t().audit;
    th.package_metrics.max_martin_findings_reported = 2;
    let mut edges: Vec<InputEdge> = Vec::new();
    for i in 0..10 {
        edges.push(rust_edge(&format!("dep_{i}.rs"), "core.rs"));
    }
    let findings = run_from_edges(&edges, high_concrete_profile, &th);
    let distance_count = findings
        .iter()
        .filter(|f| matches!(f.kind, AuditKind::DistanceFromMainSequence(_)))
        .count();
    assert!(distance_count <= 2);
}

#[test]
fn cycle_findings_are_truncated_to_max_reported() {
    let mut th = t().audit;
    th.package_metrics.max_cycle_findings_reported = 1;
    let edges = [
        rust_edge("a.rs", "b.rs"),
        rust_edge("b.rs", "a.rs"),
        rust_edge("c.rs", "d.rs"),
        rust_edge("d.rs", "c.rs"),
        rust_edge("e.rs", "f.rs"),
        rust_edge("f.rs", "e.rs"),
    ];
    let findings = run_from_edges(&edges, high_concrete_profile, &th);
    let cycle_count = findings
        .iter()
        .filter(|f| matches!(f.kind, AuditKind::ImportCycle(_)))
        .count();
    assert_eq!(cycle_count, 1);
}

#[test]
fn pipeline_is_deterministic_across_runs() {
    let edges = [
        rust_edge("a.rs", "b.rs"),
        rust_edge("b.rs", "c.rs"),
        rust_edge("c.rs", "a.rs"),
    ];
    let f1 = run_from_edges(&edges, high_concrete_profile, &t().audit);
    let f2 = run_from_edges(&edges, high_concrete_profile, &t().audit);
    assert_eq!(f1.len(), f2.len());
    for (a, b) in f1.iter().zip(f2.iter()) {
        match (&a.kind, &b.kind) {
            (AuditKind::ImportCycle(c1), AuditKind::ImportCycle(c2)) => {
                assert_eq!(c1.members, c2.members);
            }
            (AuditKind::DistanceFromMainSequence(m1), AuditKind::DistanceFromMainSequence(m2)) => {
                assert_eq!(m1.module, m2.module);
            }
            _ => {}
        }
    }
}

#[test]
fn martin_findings_sorted_by_distance_descending() {
    let edges = [
        rust_edge("a.rs", "b.rs"),
        rust_edge("c.rs", "d.rs"),
        rust_edge("e.rs", "f.rs"),
        rust_edge("dep1.rs", "rigid_core.rs"),
        rust_edge("dep2.rs", "rigid_core.rs"),
        rust_edge("dep3.rs", "rigid_core.rs"),
        rust_edge("dep4.rs", "rigid_core.rs"),
    ];
    let findings = run_from_edges(&edges, high_concrete_profile, &t().audit);
    let distances: Vec<f64> = findings
        .iter()
        .filter_map(|f| match &f.kind {
            AuditKind::DistanceFromMainSequence(m) => Some(m.distance),
            _ => None,
        })
        .collect();
    for window in distances.windows(2) {
        assert!(window[0] >= window[1]);
    }
}

#[test]
fn distance_finding_has_correct_locations_block() {
    let edges = [
        rust_edge("dep1.rs", "rigid.rs"),
        rust_edge("dep2.rs", "rigid.rs"),
        rust_edge("dep3.rs", "rigid.rs"),
    ];
    let findings = run_from_edges(&edges, high_concrete_profile, &t().audit);
    let f = findings
        .iter()
        .find(|f| {
            matches!(&f.kind, AuditKind::DistanceFromMainSequence(m) if m.module == p("rigid.rs"))
        })
        .unwrap();
    assert_eq!(f.locations.len(), 1);
    assert_eq!(f.locations[0].file, p("rigid.rs"));
    assert_eq!(f.locations[0].line, 1);
}

#[test]
fn cycle_finding_has_one_location_per_member() {
    let edges = [
        rust_edge("a.rs", "b.rs"),
        rust_edge("b.rs", "c.rs"),
        rust_edge("c.rs", "a.rs"),
    ];
    let findings = run_from_edges(&edges, high_concrete_profile, &t().audit);
    let cycle = findings
        .iter()
        .find(|f| matches!(f.kind, AuditKind::ImportCycle(_)))
        .unwrap();
    assert_eq!(cycle.locations.len(), 3);
}
