use std::path::PathBuf;

use pulse::audit::call_graph::{CallGraph, MethodIdentity};
use pulse::audit::call_walker::LocatedCall;
use pulse::audit::calls::RawCall;
use pulse::audit::definitions::DefinitionRecord;
use pulse::audit::detector_feature_envy::detect;
use pulse::audit::finding::{AuditKind, FeatureEnvyEvidence};

use crate::audit_common::t;

fn def(file: &str, class: Option<&str>, name: &str, line: u32, foreign: Vec<(&str, &str)>) -> DefinitionRecord {
    DefinitionRecord {
        identity: MethodIdentity {
            file: PathBuf::from(file),
            class: class.map(String::from),
            name: name.to_string(),
            line,
        },
        cc: 1,
        field_accesses: Vec::new(),
        foreign_field_accesses: foreign.into_iter().map(|(r, f)| (r.to_string(), f.to_string())).collect(),
        parent_class: None,
        is_constructor: false,
    }
}

#[allow(clippy::similar_names)]
fn call_to(caller: &DefinitionRecord, callee: &str, hint: Option<&str>) -> LocatedCall {
    LocatedCall {
        call: RawCall {
            callee_name: callee.to_string(),
            receiver_hint: hint.map(String::from),
            line: caller.identity.line + 1,
        },
        caller: Some(caller.identity.clone()),
        file: caller.identity.file.clone(),
    }
}

fn envy_evidence(f: &pulse::audit::finding::AuditFinding) -> &FeatureEnvyEvidence {
    let AuditKind::FeatureEnvy(e) = &f.kind else {
        panic!("expected FeatureEnvy, got {:?}", f.kind);
    };
    e
}

fn detect_with(
    defs: Vec<DefinitionRecord>,
    calls: Vec<LocatedCall>,
    audit_t: &pulse::thresholds::AuditThresholds,
) -> Vec<pulse::audit::finding::AuditFinding> {
    let graph = CallGraph::build(defs.clone(), calls);
    detect(&defs, &graph, audit_t)
}

#[test]
fn method_with_high_atfd_and_foreign_ratio_emits_finding() {
    let envious = def(
        "a.py",
        Some("Foo"),
        "method",
        1,
        vec![("bar", "x"), ("bar", "y"), ("bar", "z"), ("bar", "w"), ("bar", "v"), ("bar", "u")],
    );
    let bar_method1 = def("b.py", Some("Bar"), "do1", 1, vec![]);
    let bar_method2 = def("b.py", Some("Bar"), "do2", 1, vec![]);
    let calls = vec![call_to(&envious, "do1", Some("Bar")), call_to(&envious, "do2", Some("Bar"))];
    let findings = detect_with(vec![envious, bar_method1, bar_method2], calls, &t().audit);
    assert!(!findings.is_empty(), "expected Feature Envy finding");
    let e = envy_evidence(&findings[0]);
    assert_eq!(e.method_name, "method");
    assert!(e.atfd > 5);
}

#[test]
fn method_with_high_atfd_low_foreign_ratio_no_finding() {
    let envious =
        def("a.py", Some("Foo"), "m", 1, vec![("x", "a"), ("x", "b"), ("x", "c"), ("x", "d"), ("x", "e"), ("x", "f")]);
    let intra1 = def("a.py", Some("Foo"), "intra1", 1, vec![]);
    let intra2 = def("a.py", Some("Foo"), "intra2", 1, vec![]);
    let calls = vec![call_to(&envious, "intra1", Some("Foo")), call_to(&envious, "intra2", Some("Foo"))];
    let findings = detect_with(vec![envious, intra1, intra2], calls, &t().audit);
    assert!(findings.is_empty(), "high intra-class ratio suppresses Feature Envy");
}

#[test]
fn method_with_low_atfd_no_finding() {
    let envious = def("a.py", Some("Foo"), "m", 1, vec![("x", "a")]);
    let bar = def("b.py", Some("Bar"), "do", 1, vec![]);
    let calls = vec![call_to(&envious, "do", Some("Bar"))];
    let findings = detect_with(vec![envious, bar], calls, &t().audit);
    assert!(findings.is_empty());
}

#[test]
fn free_function_not_eligible() {
    let f =
        def("a.py", None, "free_fn", 1, vec![("x", "a"), ("x", "b"), ("x", "c"), ("x", "d"), ("x", "e"), ("x", "f")]);
    let bar = def("b.py", Some("Bar"), "do", 1, vec![]);
    let calls = vec![call_to(&f, "do", Some("Bar"))];
    let findings = detect_with(vec![f, bar], calls, &t().audit);
    assert!(findings.is_empty(), "Feature Envy is class-relative");
}

#[test]
fn method_with_no_calls_no_finding() {
    let m =
        def("a.py", Some("Foo"), "m", 1, vec![("x", "a"), ("x", "b"), ("x", "c"), ("x", "d"), ("x", "e"), ("x", "f")]);
    let findings = detect_with(vec![m], Vec::new(), &t().audit);
    assert!(findings.is_empty(), "no calls = no ratio computation = no finding");
}

#[test]
fn dominant_envied_class_extracted() {
    let envious = def(
        "a.py",
        Some("Foo"),
        "m",
        1,
        vec![("bar", "x"), ("bar", "y"), ("bar", "z"), ("bar", "w"), ("bar", "v"), ("bar", "u"), ("baz", "p")],
    );
    let bar_m = def("b.py", Some("Bar"), "do", 1, vec![]);
    let calls = vec![call_to(&envious, "do", Some("Bar"))];
    let findings = detect_with(vec![envious, bar_m], calls, &t().audit);
    if let Some(f) = findings.first() {
        let e = envy_evidence(f);
        assert_eq!(e.envied_class.as_deref(), Some("bar"));
    }
}

#[test]
fn empty_inputs_produce_no_findings() {
    let findings = detect_with(Vec::new(), Vec::new(), &t().audit);
    assert!(findings.is_empty());
}

#[test]
fn lowering_atfd_threshold_surfaces_more() {
    let m = def("a.py", Some("Foo"), "m", 1, vec![("x", "a"), ("x", "b")]);
    let bar = def("b.py", Some("Bar"), "do", 1, vec![]);
    let calls = vec![call_to(&m, "do", Some("Bar"))];
    let mut audit_t = t().audit;
    audit_t.named_smells.feature_envy.atfd = 1;
    audit_t.named_smells.feature_envy.foreign_ratio = 0.5;
    let findings = detect_with(vec![m, bar], calls, &audit_t);
    assert!(!findings.is_empty());
}

#[test]
fn raising_atfd_threshold_suppresses_finding() {
    let envious = def(
        "a.py",
        Some("Foo"),
        "m",
        1,
        vec![("bar", "x"), ("bar", "y"), ("bar", "z"), ("bar", "w"), ("bar", "v"), ("bar", "u")],
    );
    let bar_m = def("b.py", Some("Bar"), "do", 1, vec![]);
    let calls = vec![call_to(&envious, "do", Some("Bar"))];
    let mut audit_t = t().audit;
    audit_t.named_smells.feature_envy.atfd = 100;
    let findings = detect_with(vec![envious, bar_m], calls, &audit_t);
    assert!(findings.is_empty());
}

#[test]
fn determinism_two_runs_equal() {
    let envious = def(
        "a.py",
        Some("Foo"),
        "m",
        1,
        vec![("bar", "x"), ("bar", "y"), ("bar", "z"), ("bar", "w"), ("bar", "v"), ("bar", "u")],
    );
    let bar_m = def("b.py", Some("Bar"), "do", 1, vec![]);
    let calls1 = vec![call_to(&envious, "do", Some("Bar"))];
    let calls2 = calls1.clone();
    let r1 = detect_with(vec![envious.clone(), bar_m.clone()], calls1, &t().audit);
    let r2 = detect_with(vec![envious, bar_m], calls2, &t().audit);
    assert_eq!(r1.len(), r2.len());
}
