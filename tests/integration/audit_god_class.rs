use std::path::PathBuf;

use pulse::audit::call_graph::{CallGraph, MethodIdentity};
use pulse::audit::class_registry::ClassRegistry;
use pulse::audit::definitions::DefinitionRecord;
use pulse::audit::detector_god_class::{build_method_idx_lookup, detect};
use pulse::audit::finding::{AuditKind, GodClassEvidence};

use crate::audit_common::t;

#[allow(clippy::too_many_arguments)]
fn def(
    file: &str,
    class: Option<&str>,
    name: &str,
    line: u32,
    cc: u32,
    fields: Vec<&str>,
    foreign: Vec<(&str, &str)>,
    is_ctor: bool,
) -> DefinitionRecord {
    DefinitionRecord {
        identity: MethodIdentity {
            file: PathBuf::from(file),
            class: class.map(String::from),
            name: name.to_string(),
            line,
        },
        cc,
        field_accesses: fields.into_iter().map(String::from).collect(),
        foreign_field_accesses: foreign
            .into_iter()
            .map(|(r, f)| (r.to_string(), f.to_string()))
            .collect(),
        parent_class: None,
        is_constructor: is_ctor,
    }
}

fn god_evidence(f: &pulse::audit::finding::AuditFinding) -> &GodClassEvidence {
    let AuditKind::GodClass(e) = &f.kind else {
        panic!("expected GodClass, got {:?}", f.kind);
    };
    e
}

fn detect_with(defs: Vec<DefinitionRecord>, audit_t: &pulse::thresholds::AuditThresholds) -> Vec<pulse::audit::finding::AuditFinding> {
    let graph = CallGraph::build(defs.clone(), Vec::new());
    let registry = ClassRegistry::from_definitions(&defs, &graph.registry);
    let lookup = build_method_idx_lookup(&graph, &defs);
    detect(&registry, &defs, &lookup, audit_t)
}

#[test]
fn god_class_with_high_wmc_low_tcc_high_atfd_emits_finding() {
    let mut defs = Vec::new();
    for i in 0..10 {
        defs.push(def(
            "god.py",
            Some("God"),
            &format!("m{i}"),
            10 + i,
            10,
            vec![&format!("field_{i}")],
            vec![("a", "x"), ("b", "y"), ("c", "z"), ("d", "w"), ("e", "v"), ("f", "u")],
            false,
        ));
    }
    let findings = detect_with(defs, &t().audit);
    assert!(!findings.is_empty(), "expected God Class finding");
    let e = god_evidence(&findings[0]);
    assert_eq!(e.class_name, "God");
    assert!(e.wmc > 47);
    assert!(e.tcc < 0.33);
    assert!(e.atfd > 5);
}

#[test]
fn class_with_high_wmc_high_tcc_no_finding() {
    let mut defs = Vec::new();
    for i in 0..10 {
        defs.push(def(
            "c.py",
            Some("Cohesive"),
            &format!("m{i}"),
            10 + i,
            10,
            vec!["shared_field"],
            vec![("a", "x"), ("b", "y"), ("c", "z"), ("d", "w"), ("e", "v"), ("f", "u")],
            false,
        ));
    }
    let findings = detect_with(defs, &t().audit);
    assert!(findings.is_empty(), "high cohesion suppresses God Class");
}

#[test]
fn class_with_low_wmc_no_finding() {
    let mut defs = Vec::new();
    for i in 0..3 {
        defs.push(def(
            "small.py",
            Some("Small"),
            &format!("m{i}"),
            10 + i,
            1,
            vec![&format!("f{i}")],
            vec![("a", "x"), ("b", "y"), ("c", "z"), ("d", "w"), ("e", "v"), ("f", "u")],
            false,
        ));
    }
    let findings = detect_with(defs, &t().audit);
    assert!(findings.is_empty());
}

#[test]
fn class_with_low_atfd_no_finding() {
    let mut defs = Vec::new();
    for i in 0..10 {
        defs.push(def(
            "n.py",
            Some("NoForeign"),
            &format!("m{i}"),
            10 + i,
            10,
            vec![&format!("f{i}")],
            vec![],
            false,
        ));
    }
    let findings = detect_with(defs, &t().audit);
    assert!(findings.is_empty());
}

#[test]
fn empty_inputs_yield_no_findings() {
    let findings = detect_with(Vec::new(), &t().audit);
    assert!(findings.is_empty());
}

#[test]
fn single_method_class_no_finding_due_to_tcc() {
    let defs = vec![def("a.py", Some("Lone"), "m", 1, 50, vec!["f"], vec![("a", "x"), ("b", "y"), ("c", "z"), ("d", "w"), ("e", "v"), ("f", "u")], false)];
    let findings = detect_with(defs, &t().audit);
    assert!(findings.is_empty());
}

#[test]
fn lowering_thresholds_surfaces_smaller_classes() {
    let mut defs = Vec::new();
    for i in 0..3 {
        defs.push(def(
            "m.py",
            Some("Mid"),
            &format!("m{i}"),
            10 + i,
            5,
            vec![&format!("f{i}")],
            vec![("a", "x"), ("b", "y")],
            false,
        ));
    }
    let mut audit_t = t().audit;
    audit_t.named_smells.god_class.wmc = 10;
    audit_t.named_smells.god_class.tcc = 0.5;
    audit_t.named_smells.god_class.atfd = 1;
    let findings = detect_with(defs, &audit_t);
    assert!(!findings.is_empty());
}

#[test]
fn raising_wmc_threshold_suppresses_finding() {
    let mut defs = Vec::new();
    for i in 0..10 {
        defs.push(def(
            "g.py",
            Some("G"),
            &format!("m{i}"),
            10 + i,
            10,
            vec![&format!("f{i}")],
            vec![("a", "x"), ("b", "y"), ("c", "z"), ("d", "w"), ("e", "v"), ("f", "u")],
            false,
        ));
    }
    let mut audit_t = t().audit;
    audit_t.named_smells.god_class.wmc = 1000;
    let findings = detect_with(defs, &audit_t);
    assert!(findings.is_empty());
}

#[test]
fn determinism_two_runs_equal() {
    let mut defs = Vec::new();
    for i in 0..10 {
        defs.push(def(
            "g.py",
            Some("G"),
            &format!("m{i}"),
            10 + i,
            10,
            vec![&format!("f{i}")],
            vec![("a", "x"), ("b", "y"), ("c", "z"), ("d", "w"), ("e", "v"), ("f", "u")],
            false,
        ));
    }
    let r1 = detect_with(defs.clone(), &t().audit);
    let r2 = detect_with(defs, &t().audit);
    assert_eq!(r1.len(), r2.len());
}

#[test]
fn evidence_carries_class_metrics() {
    let mut defs = Vec::new();
    for i in 0..10 {
        defs.push(def(
            "g.py",
            Some("G"),
            &format!("m{i}"),
            10 + i,
            10,
            vec![&format!("f{i}")],
            vec![("a", "x"), ("b", "y"), ("c", "z"), ("d", "w"), ("e", "v"), ("f", "u")],
            false,
        ));
    }
    let findings = detect_with(defs, &t().audit);
    let e = god_evidence(&findings[0]);
    assert_eq!(e.method_count, 10);
    assert_eq!(e.wmc, 100);
}
