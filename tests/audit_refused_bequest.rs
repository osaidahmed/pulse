use std::path::PathBuf;

use pulse::audit::call_graph::{CallGraph, MethodIdentity};
use pulse::audit::class_registry::ClassRegistry;
use pulse::audit::definitions::DefinitionRecord;
use pulse::audit::detector_refused_bequest::detect;
use pulse::audit::finding::{AuditKind, RefusedBequestEvidence};
use pulse::parse::Language;

mod audit_common;
use audit_common::t;

fn def(file: &str, class: &str, name: &str, line: u32, parent: Option<&str>, is_ctor: bool) -> DefinitionRecord {
    DefinitionRecord {
        identity: MethodIdentity {
            file: PathBuf::from(file),
            class: Some(class.to_string()),
            name: name.to_string(),
            line,
        },
        cc: 1,
        field_accesses: Vec::new(),
        foreign_field_accesses: Vec::new(),
        parent_class: parent.map(String::from),
        is_constructor: is_ctor,
    }
}

fn refused_evidence(f: &pulse::audit::finding::AuditFinding) -> &RefusedBequestEvidence {
    let AuditKind::RefusedBequest(e) = &f.kind else {
        panic!("expected RefusedBequest, got {:?}", f.kind);
    };
    e
}

fn detect_with(
    defs: Vec<DefinitionRecord>,
    audit_t: &pulse::thresholds::AuditThresholds,
) -> Vec<pulse::audit::finding::AuditFinding> {
    let graph = CallGraph::build(defs.clone(), Vec::new());
    let registry = ClassRegistry::from_definitions(&defs, &graph.registry);
    let lang_lookup = |_: &std::path::Path| -> Option<Language> { None };
    detect(&registry, &defs, &lang_lookup, audit_t)
}

#[test]
fn subclass_refusing_most_of_parent_emits_finding() {
    let defs = vec![
        def("p.py", "Parent", "m1", 1, None, false),
        def("p.py", "Parent", "m2", 5, None, false),
        def("p.py", "Parent", "m3", 10, None, false),
        def("p.py", "Parent", "m4", 15, None, false),
        def("p.py", "Parent", "m5", 20, None, false),
        def("c.py", "Child", "m1", 1, Some("Parent"), false),
    ];
    let findings = detect_with(defs, &t().audit);
    assert!(!findings.is_empty(), "subclass overriding 1/5 should trigger");
    let e = refused_evidence(&findings[0]);
    assert_eq!(e.subclass_name, "Child");
    assert_eq!(e.parent_name, "Parent");
    assert_eq!(e.override_count, 1);
    assert_eq!(e.parent_method_count, 5);
}

#[test]
fn subclass_overriding_most_no_finding() {
    let defs = vec![
        def("p.py", "Parent", "m1", 1, None, false),
        def("p.py", "Parent", "m2", 5, None, false),
        def("p.py", "Parent", "m3", 10, None, false),
        def("c.py", "Child", "m1", 1, Some("Parent"), false),
        def("c.py", "Child", "m2", 5, Some("Parent"), false),
        def("c.py", "Child", "m3", 10, Some("Parent"), false),
    ];
    let findings = detect_with(defs, &t().audit);
    assert!(findings.is_empty(), "3/3 overrides = full implementation, no refusal");
}

#[test]
fn parent_with_few_methods_skipped() {
    let defs = vec![
        def("p.py", "Parent", "m1", 1, None, false),
        def("p.py", "Parent", "m2", 5, None, false),
        def("c.py", "Child", "x", 1, Some("Parent"), false),
    ];
    let findings = detect_with(defs, &t().audit);
    assert!(findings.is_empty(), "parent_method_count<3 → skipped");
}

#[test]
fn class_with_no_parent_not_eligible() {
    let defs = vec![
        def("p.py", "Parent", "m1", 1, None, false),
        def("p.py", "Parent", "m2", 5, None, false),
        def("p.py", "Parent", "m3", 10, None, false),
    ];
    let findings = detect_with(defs, &t().audit);
    assert!(findings.is_empty());
}

#[test]
fn external_parent_not_in_registry_skipped() {
    let defs = vec![def("c.py", "Child", "x", 1, Some("ExternalLib"), false)];
    let findings = detect_with(defs, &t().audit);
    assert!(findings.is_empty());
}

#[test]
fn lowering_threshold_surfaces_more() {
    let defs = vec![
        def("p.py", "Parent", "m1", 1, None, false),
        def("p.py", "Parent", "m2", 5, None, false),
        def("p.py", "Parent", "m3", 10, None, false),
        def("p.py", "Parent", "m4", 15, None, false),
        def("c.py", "Child", "m1", 1, Some("Parent"), false),
        def("c.py", "Child", "m2", 5, Some("Parent"), false),
        def("c.py", "Child", "m3", 10, Some("Parent"), false),
    ];
    let mut audit_t = t().audit;
    audit_t.named_smells.refused_bequest.max_override_ratio = 0.9;
    let findings = detect_with(defs, &audit_t);
    assert!(!findings.is_empty());
}

#[test]
fn raising_min_parent_methods_suppresses() {
    let defs = vec![
        def("p.py", "Parent", "m1", 1, None, false),
        def("p.py", "Parent", "m2", 5, None, false),
        def("p.py", "Parent", "m3", 10, None, false),
        def("p.py", "Parent", "m4", 15, None, false),
        def("p.py", "Parent", "m5", 20, None, false),
        def("c.py", "Child", "m1", 1, Some("Parent"), false),
    ];
    let mut audit_t = t().audit;
    audit_t.named_smells.refused_bequest.min_parent_methods = 100;
    let findings = detect_with(defs, &audit_t);
    assert!(findings.is_empty());
}

#[test]
fn empty_defs_no_findings() {
    let findings = detect_with(Vec::new(), &t().audit);
    assert!(findings.is_empty());
}

#[test]
fn determinism_two_runs_equal() {
    let defs = vec![
        def("p.py", "Parent", "m1", 1, None, false),
        def("p.py", "Parent", "m2", 5, None, false),
        def("p.py", "Parent", "m3", 10, None, false),
        def("p.py", "Parent", "m4", 15, None, false),
        def("p.py", "Parent", "m5", 20, None, false),
        def("c.py", "Child", "m1", 1, Some("Parent"), false),
    ];
    let r1 = detect_with(defs.clone(), &t().audit);
    let r2 = detect_with(defs, &t().audit);
    assert_eq!(r1.len(), r2.len());
}

#[test]
fn constructors_excluded_from_method_count() {
    let defs = vec![
        def("p.py", "Parent", "__init__", 1, None, true),
        def("p.py", "Parent", "m1", 5, None, false),
        def("p.py", "Parent", "m2", 10, None, false),
        def("p.py", "Parent", "m3", 15, None, false),
        def("c.py", "Child", "__init__", 1, Some("Parent"), true),
    ];
    let findings = detect_with(defs, &t().audit);
    if let Some(f) = findings.first() {
        let e = refused_evidence(f);
        assert_eq!(e.parent_method_count, 3, "ctor excluded");
    }
}
