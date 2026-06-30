use std::path::PathBuf;

use pulse_audit::call_graph::{CallGraph, MethodIdentity};
use pulse_audit::class_registry::ClassRegistry;
use pulse_audit::definitions::DefinitionRecord;
use pulse_audit::detector::refused_bequest::detect;
use pulse_audit::finding::{AuditKind, RefusedBequestEvidence};

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

fn refused_evidence(f: &pulse_audit::finding::AuditFinding) -> &RefusedBequestEvidence {
    let AuditKind::RefusedBequest(e) = &f.kind else {
        panic!("expected RefusedBequest, got {:?}", f.kind);
    };
    e
}

fn detect_with(
    defs: Vec<DefinitionRecord>,
    audit_t: &pulse_thresholds::AuditThresholds,
) -> Vec<pulse_audit::finding::AuditFinding> {
    let graph = CallGraph::build(defs.clone(), Vec::new());
    let registry = ClassRegistry::from_definitions(&defs, &graph.registry);
    let abstractness = |_: &std::path::Path| -> Option<f64> { None };
    detect(&registry, &defs, &graph, &abstractness, audit_t)
}

#[test]
fn subclass_that_calls_inherited_methods_does_not_refuse_bequest() {
    use pulse_audit::call_walker::LocatedCall;
    use pulse_audit::calls::RawCall;
    let mut defs: Vec<DefinitionRecord> =
        (0..4).map(|i| def("base.py", "Base", &format!("m{i}"), 10 + i, None, false)).collect();
    let child = def("child.py", "Child", "extra", 1, Some("Base"), false);
    defs.push(child.clone());
    let call = |name: &str, line: u32| LocatedCall {
        call: RawCall { callee_name: name.to_string(), receiver_hint: Some("Base".to_string()), line },
        caller: Some(child.identity.clone()),
        file: child.identity.file.clone(),
    };
    let calls = vec![call("m0", 2), call("m1", 3)];
    let graph = CallGraph::build(defs.clone(), calls);
    let registry = ClassRegistry::from_definitions(&defs, &graph.registry);
    let abstractness = |_: &std::path::Path| -> Option<f64> { None };
    let findings = detect(&registry, &defs, &graph, &abstractness, &t().audit);
    assert!(findings.is_empty(), "a subclass that calls inherited methods is using its bequest, not refusing it");
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

fn wrapper_evidence() -> RefusedBequestEvidence {
    RefusedBequestEvidence {
        subclass_file: PathBuf::from("wrap.java"),
        subclass_name: "Wrapper".to_string(),
        parent_file: PathBuf::from("base.java"),
        parent_name: "Base".to_string(),
        override_count: 1,
        parent_method_count: 10,
        override_ratio: 0.1,
        confidence: pulse_audit::finding::ImportConfidence::Medium,
    }
}

fn bindings_with_field(field_type: &str) -> pulse_audit::binding::BindingTable {
    let mut fields = std::collections::BTreeMap::new();
    fields.insert("delegate".to_string(), field_type.to_string());
    let mut bindings = pulse_audit::binding::BindingTable::default();
    bindings.insert_class(pulse_audit::binding::ClassBinding {
        file: PathBuf::from("wrap.java"),
        name: "Wrapper".to_string(),
        parents: vec!["Base".to_string()],
        fields,
    });
    bindings
}

#[test]
fn decorator_wrapping_its_parent_is_recognized() {
    use pulse_audit::detector::refused_bequest::is_decorator_wrapper;
    assert!(
        is_decorator_wrapper(&wrapper_evidence(), &bindings_with_field("Base")),
        "a subclass holding a field typed as its parent is a wrapper, not a refusal"
    );
}

#[test]
fn subclass_without_ancestor_typed_field_is_not_a_wrapper() {
    use pulse_audit::detector::refused_bequest::is_decorator_wrapper;
    assert!(!is_decorator_wrapper(&wrapper_evidence(), &bindings_with_field("Logger")));
}

#[test]
fn subclass_with_no_field_bindings_is_not_a_wrapper() {
    use pulse_audit::detector::refused_bequest::is_decorator_wrapper;
    assert!(!is_decorator_wrapper(&wrapper_evidence(), &pulse_audit::binding::BindingTable::default()));
}

#[test]
fn subclass_inheriting_the_wrapped_field_from_a_base_wrapper_is_recognized() {
    use pulse_audit::binding::{BindingTable, ClassBinding};
    use pulse_audit::detector::refused_bequest::is_decorator_wrapper;
    let mut bindings = BindingTable::default();
    let mut wrapped = std::collections::BTreeMap::new();
    wrapped.insert("delegate".to_string(), "Base".to_string());
    bindings.insert_class(ClassBinding {
        file: PathBuf::from("base_wrapper.java"),
        name: "BaseWrapper".to_string(),
        parents: vec!["Base".to_string()],
        fields: wrapped,
    });
    bindings.insert_class(ClassBinding {
        file: PathBuf::from("leaf.java"),
        name: "Leaf".to_string(),
        parents: vec!["BaseWrapper".to_string()],
        fields: std::collections::BTreeMap::new(),
    });
    let e = RefusedBequestEvidence {
        subclass_file: PathBuf::from("leaf.java"),
        subclass_name: "Leaf".to_string(),
        parent_file: PathBuf::from("base_wrapper.java"),
        parent_name: "BaseWrapper".to_string(),
        override_count: 1,
        parent_method_count: 10,
        override_ratio: 0.1,
        confidence: pulse_audit::finding::ImportConfidence::Medium,
    };
    assert!(
        is_decorator_wrapper(&e, &bindings),
        "a leaf that inherits the wrapped field from a base wrapper is still part of the decoration hierarchy"
    );
}
