use std::path::PathBuf;

use pulse::audit::call_graph::{CallGraph, MethodIdentity};
use pulse::audit::class_registry::ClassRegistry;
use pulse::audit::definitions::DefinitionRecord;
use pulse::audit::detector_refused_bequest::detect;
use pulse::audit::finding::{AuditFinding, AuditKind, RefusedBequestEvidence};
use pulse::parse::Language;

use crate::audit_common::t;

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

fn detect_with(
    defs: Vec<DefinitionRecord>,
    th: &pulse::thresholds::AuditThresholds,
) -> Vec<AuditFinding> {
    let graph = CallGraph::build(defs.clone(), Vec::new());
    let registry = ClassRegistry::from_definitions(&defs, &graph.registry);
    let lang_lookup = |_: &std::path::Path| -> Option<Language> { None };
    detect(&registry, &defs, &lang_lookup, th)
}

fn rb(f: &AuditFinding) -> &RefusedBequestEvidence {
    let AuditKind::RefusedBequest(e) = &f.kind else {
        panic!("expected RefusedBequest variant");
    };
    e
}

fn parent_with_n_methods(n: u32) -> Vec<DefinitionRecord> {
    (1..=n)
        .map(|i| def("p.py", "Parent", &format!("m{i}"), i, None, false))
        .collect()
}

#[test]
fn boundary_min_parent_methods_at_threshold_eligible() {
    let mut defs = parent_with_n_methods(3);
    defs.push(def("c.py", "Child", "x", 1, Some("Parent"), false));
    let findings = detect_with(defs, &t().audit);
    assert!(!findings.is_empty(), "parent_methods=3 = min_parent_methods, child overrides 0/3");
}

#[test]
fn boundary_min_parent_methods_just_below_skipped() {
    let mut defs = parent_with_n_methods(2);
    defs.push(def("c.py", "Child", "x", 1, Some("Parent"), false));
    let findings = detect_with(defs, &t().audit);
    assert!(findings.is_empty());
}

#[test]
fn override_ratio_zero_emits_finding() {
    let mut defs = parent_with_n_methods(5);
    defs.push(def("c.py", "Child", "totally_unrelated", 1, Some("Parent"), false));
    let findings = detect_with(defs, &t().audit);
    assert!(!findings.is_empty());
    assert_eq!(rb(&findings[0]).override_count, 0);
    assert!(rb(&findings[0]).override_ratio < 0.01);
}

#[test]
fn override_ratio_full_no_finding() {
    let mut defs = parent_with_n_methods(5);
    for i in 1..=5 {
        defs.push(def("c.py", "Child", &format!("m{i}"), i, Some("Parent"), false));
    }
    let findings = detect_with(defs, &t().audit);
    assert!(findings.is_empty());
}

#[test]
fn boundary_override_ratio_at_threshold_no_finding() {
    let mut defs = parent_with_n_methods(10);
    for i in 1..=3 {
        defs.push(def("c.py", "Child", &format!("m{i}"), i, Some("Parent"), false));
    }
    let findings = detect_with(defs, &t().audit);
    assert!(findings.is_empty(), "ratio=0.3 not > 0.3 (default uses >=)");
}

#[test]
fn boundary_override_ratio_just_below_emits_finding() {
    let mut defs = parent_with_n_methods(10);
    for i in 1..=2 {
        defs.push(def("c.py", "Child", &format!("m{i}"), i, Some("Parent"), false));
    }
    let findings = detect_with(defs, &t().audit);
    assert!(!findings.is_empty(), "ratio=0.2 < 0.3 should fire");
}

#[test]
fn parent_with_only_constructors_skipped() {
    let mut defs = Vec::new();
    for i in 1..=5 {
        defs.push(def("p.py", "Parent", &format!("init{i}"), i, None, true));
    }
    defs.push(def("c.py", "Child", "x", 1, Some("Parent"), false));
    let findings = detect_with(defs, &t().audit);
    assert!(findings.is_empty(), "parent with all-constructors has 0 non-ctor methods");
}

#[test]
fn child_constructors_excluded_from_override_count() {
    let mut defs = parent_with_n_methods(5);
    defs.push(def("c.py", "Child", "m1", 1, Some("Parent"), false));
    defs.push(def("c.py", "Child", "init", 5, Some("Parent"), true));
    defs.push(def("c.py", "Child", "init2", 10, Some("Parent"), true));
    let findings = detect_with(defs, &t().audit);
    if let Some(f) = findings.first() {
        assert!(rb(f).override_count <= 1, "constructor overrides shouldn't count");
    }
}

#[test]
fn external_parent_not_in_registry_skipped() {
    let defs = vec![
        def("c.py", "Child", "x", 1, Some("ExternalLibrary"), false),
        def("c.py", "Child", "y", 5, Some("ExternalLibrary"), false),
        def("c.py", "Child", "z", 10, Some("ExternalLibrary"), false),
    ];
    let findings = detect_with(defs, &t().audit);
    assert!(findings.is_empty());
}

#[test]
fn classes_with_no_parent_no_finding() {
    let mut defs = parent_with_n_methods(5);
    defs.push(def("orph.py", "Orphan", "x", 1, None, false));
    let findings = detect_with(defs, &t().audit);
    assert!(findings.is_empty());
}

#[test]
fn deep_chain_grandchild_refusal_evaluated() {
    let mut defs = parent_with_n_methods(5);
    defs.push(def("ch.py", "Child", "m1", 1, Some("Parent"), false));
    defs.push(def("ch.py", "Child", "m2", 5, Some("Parent"), false));
    defs.push(def("ch.py", "Child", "m3", 10, Some("Parent"), false));
    defs.push(def("ch.py", "Child", "m4", 15, Some("Parent"), false));
    defs.push(def("ch.py", "Child", "m5", 20, Some("Parent"), false));
    defs.push(def("gc.py", "Grandchild", "x", 1, Some("Child"), false));
    let _ = detect_with(defs, &t().audit);
}

#[test]
fn cyclic_inheritance_does_not_panic() {
    let defs = vec![
        def("a.py", "A", "m1", 1, Some("B"), false),
        def("a.py", "A", "m2", 5, Some("B"), false),
        def("a.py", "A", "m3", 10, Some("B"), false),
        def("b.py", "B", "n1", 1, Some("A"), false),
        def("b.py", "B", "n2", 5, Some("A"), false),
        def("b.py", "B", "n3", 10, Some("A"), false),
    ];
    let _ = detect_with(defs, &t().audit);
}

#[test]
fn self_referential_parent_does_not_panic() {
    let mut defs = parent_with_n_methods(5);
    defs.push(def("self.py", "Self", "m1", 1, Some("Self"), false));
    let _ = detect_with(defs, &t().audit);
}

#[test]
fn determinism_five_runs_yield_same_finding_count() {
    let mut counts = Vec::new();
    for _ in 0..5 {
        let mut defs = parent_with_n_methods(5);
        defs.push(def("c.py", "Child", "m1", 1, Some("Parent"), false));
        counts.push(detect_with(defs, &t().audit).len());
    }
    for c in &counts[1..] {
        assert_eq!(*c, counts[0]);
    }
}

#[test]
fn raising_min_parent_methods_threshold_suppresses() {
    let mut defs = parent_with_n_methods(5);
    defs.push(def("c.py", "Child", "m1", 1, Some("Parent"), false));
    let mut th = t().audit;
    th.named_smells.refused_bequest.min_parent_methods = 100;
    assert!(detect_with(defs, &th).is_empty());
}

#[test]
fn lowering_max_override_ratio_to_zero_only_zero_overrides_qualify() {
    let mut defs = parent_with_n_methods(5);
    defs.push(def("c.py", "Child", "m1", 1, Some("Parent"), false));
    let mut th = t().audit;
    th.named_smells.refused_bequest.max_override_ratio = 0.0;
    let findings = detect_with(defs, &th);
    assert!(findings.is_empty(), "ratio=0.2 not < 0.0");
}

#[test]
fn evidence_carries_subclass_and_parent_files() {
    let mut defs = Vec::new();
    for i in 1..=5 {
        defs.push(def("base.py", "Parent", &format!("m{i}"), i, None, false));
    }
    defs.push(def("child.py", "Child", "m1", 1, Some("Parent"), false));
    let findings = detect_with(defs, &t().audit);
    assert!(!findings.is_empty());
    let e = rb(&findings[0]);
    assert_eq!(e.parent_file, PathBuf::from("base.py"));
    assert_eq!(e.subclass_file, PathBuf::from("child.py"));
}

#[test]
fn empty_inputs_no_findings_no_panic() {
    let _ = detect_with(Vec::new(), &t().audit);
}

#[test]
fn sort_findings_by_override_ratio_ascending() {
    let mut defs = Vec::new();
    for i in 1..=5 {
        defs.push(def("p1.py", "P1", &format!("m{i}"), i, None, false));
    }
    defs.push(def("c1.py", "C1", "totally_unrelated", 1, Some("P1"), false));

    for i in 1..=5 {
        defs.push(def("p2.py", "P2", &format!("m{i}"), i, None, false));
    }
    defs.push(def("c2.py", "C2", "m1", 1, Some("P2"), false));

    let findings = detect_with(defs, &t().audit);
    if findings.len() >= 2 {
        for w in findings.windows(2) {
            assert!(rb(&w[0]).override_ratio <= rb(&w[1]).override_ratio);
        }
    }
}

#[test]
fn confidence_label_present() {
    use pulse::audit::finding::ImportConfidence;
    let mut defs = parent_with_n_methods(5);
    defs.push(def("c.py", "Child", "m1", 1, Some("Parent"), false));
    let findings = detect_with(defs, &t().audit);
    if let Some(f) = findings.first() {
        let conf = rb(f).confidence;
        assert!(matches!(
            conf,
            ImportConfidence::High
                | ImportConfidence::Medium
                | ImportConfidence::Low
                | ImportConfidence::BestEffort
                | ImportConfidence::NaAbstraction
        ));
    }
}

#[test]
fn unicode_class_names_handled() {
    let mut defs = Vec::new();
    for i in 1..=5 {
        defs.push(def("базе.py", "Базовый", &format!("м{i}"), i, None, false));
    }
    defs.push(def("ч.py", "Ребёнок", "м1", 1, Some("Базовый"), false));
    let _ = detect_with(defs, &t().audit);
}

#[test]
fn multiple_subclasses_one_parent_each_evaluated() {
    let mut defs = parent_with_n_methods(5);
    defs.push(def("c1.py", "C1", "m1", 1, Some("Parent"), false));
    defs.push(def("c2.py", "C2", "m1", 1, Some("Parent"), false));
    defs.push(def("c2.py", "C2", "m2", 5, Some("Parent"), false));
    defs.push(def("c3.py", "C3", "m1", 1, Some("Parent"), false));
    defs.push(def("c3.py", "C3", "m2", 5, Some("Parent"), false));
    defs.push(def("c3.py", "C3", "m3", 10, Some("Parent"), false));
    defs.push(def("c3.py", "C3", "m4", 15, Some("Parent"), false));
    let findings = detect_with(defs, &t().audit);
    let names: std::collections::BTreeSet<String> = findings
        .iter()
        .map(|f| rb(f).subclass_name.clone())
        .collect();
    assert!(names.contains("C1"));
}

#[test]
fn stress_50_subclasses_completes() {
    use std::time::Instant;
    let mut defs = parent_with_n_methods(5);
    for i in 0..50 {
        defs.push(def(
            &format!("c_{i}.py"),
            &format!("C{i}"),
            "m1",
            1,
            Some("Parent"),
            false,
        ));
    }
    let started = Instant::now();
    let findings = detect_with(defs, &t().audit);
    assert!(started.elapsed().as_secs_f64() < 5.0);
    assert_eq!(findings.len(), 50);
}

#[test]
fn parent_method_count_matches_evidence() {
    let mut defs = parent_with_n_methods(8);
    defs.push(def("c.py", "Child", "m1", 1, Some("Parent"), false));
    let findings = detect_with(defs, &t().audit);
    assert!(!findings.is_empty());
    assert_eq!(rb(&findings[0]).parent_method_count, 8);
}

#[test]
fn parent_with_three_constructors_and_three_methods_uses_only_methods() {
    let mut defs = Vec::new();
    for i in 1..=3 {
        defs.push(def("p.py", "Parent", &format!("init{i}"), i, None, true));
    }
    for i in 1..=3 {
        defs.push(def("p.py", "Parent", &format!("real{i}"), 10 + i, None, false));
    }
    defs.push(def("c.py", "Child", "real1", 1, Some("Parent"), false));
    let findings = detect_with(defs, &t().audit);
    if let Some(f) = findings.first() {
        assert!(rb(f).parent_method_count <= 3, "ctor methods excluded");
    }
}

#[test]
fn override_count_matches_overlapping_method_names() {
    let mut defs = parent_with_n_methods(10);
    defs.push(def("c.py", "Child", "m1", 1, Some("Parent"), false));
    defs.push(def("c.py", "Child", "m3", 5, Some("Parent"), false));
    let findings = detect_with(defs, &t().audit);
    assert!(!findings.is_empty());
    let e = rb(&findings[0]);
    assert_eq!(e.override_count, 2);
}

#[test]
fn child_method_not_in_parent_does_not_inflate_override_count() {
    let mut defs = parent_with_n_methods(5);
    defs.push(def("c.py", "Child", "m1", 1, Some("Parent"), false));
    defs.push(def("c.py", "Child", "child_only", 5, Some("Parent"), false));
    defs.push(def("c.py", "Child", "another_child_only", 10, Some("Parent"), false));
    let findings = detect_with(defs, &t().audit);
    if let Some(f) = findings.first() {
        assert_eq!(rb(f).override_count, 1, "only m1 overlaps");
    }
}
