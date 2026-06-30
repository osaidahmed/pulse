use std::path::PathBuf;

use pulse_audit::call_graph::{CallGraph, MethodIdentity};
use pulse_audit::call_walker::LocatedCall;
use pulse_audit::calls::RawCall;
use pulse_audit::definitions::DefinitionRecord;
use pulse_audit::detector_feature_envy::detect;
use pulse_audit::finding::{AuditFinding, AuditKind, FeatureEnvyEvidence, ImportConfidence};

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

fn detect_with(
    defs: Vec<DefinitionRecord>,
    calls: Vec<LocatedCall>,
    th: &pulse_thresholds::AuditThresholds,
) -> Vec<AuditFinding> {
    let graph = CallGraph::build(defs.clone(), calls);
    detect(&defs, &graph, th)
}

fn envy(f: &AuditFinding) -> &FeatureEnvyEvidence {
    let AuditKind::FeatureEnvy(e) = &f.kind else {
        panic!("expected FeatureEnvy variant");
    };
    e
}

#[test]
fn boundary_atfd_strictly_greater() {
    let foreign = vec![("b", "f1"), ("b", "f2"), ("b", "f3"), ("b", "f4"), ("b", "f5")];
    let m = def("a.py", Some("A"), "m", 1, foreign);
    let other = def("b.py", Some("B"), "do", 1, vec![]);
    let calls = vec![call_to(&m, "do", Some("B"))];
    assert!(detect_with(vec![m, other], calls, &t().audit).is_empty(), "atfd=5 not > 5");

    let foreign = vec![("b", "f1"), ("b", "f2"), ("b", "f3"), ("b", "f4"), ("b", "f5"), ("b", "f6")];
    let m = def("a.py", Some("A"), "m", 1, foreign);
    let other = def("b.py", Some("B"), "do", 1, vec![]);
    let calls = vec![call_to(&m, "do", Some("B"))];
    assert!(!detect_with(vec![m, other], calls, &t().audit).is_empty());
}

#[test]
fn boundary_foreign_ratio_strictly_greater() {
    let foreign = vec![("b", "f1"), ("b", "f2"), ("b", "f3"), ("b", "f4"), ("b", "f5"), ("b", "f6")];
    let m = def("a.py", Some("A"), "m", 1, foreign.clone());
    let other = def("b.py", Some("B"), "do", 1, vec![]);
    let intra = def("a.py", Some("A"), "intra", 5, vec![]);
    let calls = vec![
        call_to(&m, "do", Some("B")),
        call_to(&m, "do", Some("B")),
        call_to(&m, "do", Some("B")),
        call_to(&m, "intra", Some("A")),
        call_to(&m, "intra", Some("A")),
    ];
    let findings = detect_with(vec![m, other, intra], calls, &t().audit);
    assert!(findings.is_empty(), "ratio 3/5=0.6 not > 0.6");
}

#[test]
fn just_above_ratio_emits_finding() {
    let foreign = vec![("b", "f1"), ("b", "f2"), ("b", "f3"), ("b", "f4"), ("b", "f5"), ("b", "f6")];
    let m = def("a.py", Some("A"), "m", 1, foreign.clone());
    let other = def("b.py", Some("B"), "do", 1, vec![]);
    let intra = def("a.py", Some("A"), "intra", 5, vec![]);
    let calls = vec![
        call_to(&m, "do", Some("B")),
        call_to(&m, "do", Some("B")),
        call_to(&m, "do", Some("B")),
        call_to(&m, "do", Some("B")),
        call_to(&m, "intra", Some("A")),
    ];
    let findings = detect_with(vec![m, other, intra], calls, &t().audit);
    assert!(!findings.is_empty(), "ratio 4/5=0.8 should fire");
}

#[test]
fn atfd_at_threshold_with_high_ratio_no_finding() {
    let foreign = vec![("b", "f1"), ("b", "f2"), ("b", "f3"), ("b", "f4"), ("b", "f5")];
    let m = def("a.py", Some("A"), "m", 1, foreign);
    let other = def("b.py", Some("B"), "do", 1, vec![]);
    let calls = vec![
        call_to(&m, "do", Some("B")),
        call_to(&m, "do", Some("B")),
        call_to(&m, "do", Some("B")),
        call_to(&m, "do", Some("B")),
    ];
    assert!(detect_with(vec![m, other], calls, &t().audit).is_empty());
}

#[test]
fn zero_total_calls_no_finding_even_with_high_atfd() {
    let foreign = vec![("b", "f1"), ("b", "f2"), ("b", "f3"), ("b", "f4"), ("b", "f5"), ("b", "f6")];
    let m = def("a.py", Some("A"), "m", 1, foreign);
    let findings = detect_with(vec![m], Vec::new(), &t().audit);
    assert!(findings.is_empty());
}

#[test]
fn free_function_excluded_from_detection() {
    let foreign = vec![("b", "f1"), ("b", "f2"), ("b", "f3"), ("b", "f4"), ("b", "f5"), ("b", "f6")];
    let m = def("free.py", None, "f", 1, foreign);
    let other = def("b.py", Some("B"), "do", 1, vec![]);
    let calls = vec![call_to(&m, "do", Some("B"))];
    let findings = detect_with(vec![m, other], calls, &t().audit);
    assert!(findings.is_empty());
}

#[test]
fn dominant_envied_class_picks_max_count() {
    let foreign =
        vec![("bar", "x"), ("bar", "y"), ("bar", "z"), ("bar", "w"), ("bar", "v"), ("bar", "u"), ("baz", "p")];
    let m = def("a.py", Some("A"), "m", 1, foreign);
    let bar = def("b.py", Some("Bar"), "do", 1, vec![]);
    let calls = vec![call_to(&m, "do", Some("Bar")), call_to(&m, "do", Some("Bar")), call_to(&m, "do", Some("Bar"))];
    let findings = detect_with(vec![m, bar], calls, &t().audit);
    assert!(!findings.is_empty());
    let envied = envy(&findings[0]).envied_class.clone().unwrap();
    assert_eq!(envied, "bar");
}

#[test]
fn confidence_label_is_medium_by_default() {
    let foreign = vec![("b", "f1"), ("b", "f2"), ("b", "f3"), ("b", "f4"), ("b", "f5"), ("b", "f6")];
    let m = def("a.py", Some("A"), "m", 1, foreign);
    let other = def("b.py", Some("B"), "do", 1, vec![]);
    let calls = vec![call_to(&m, "do", Some("B")); 4];
    let findings = detect_with(vec![m, other], calls, &t().audit);
    assert!(!findings.is_empty());
    assert_eq!(envy(&findings[0]).confidence, ImportConfidence::Medium);
}

#[test]
fn evidence_includes_method_identity() {
    let foreign = vec![("b", "f1"), ("b", "f2"), ("b", "f3"), ("b", "f4"), ("b", "f5"), ("b", "f6")];
    let m = def("location.py", Some("Cls"), "the_method", 42, foreign);
    let other = def("b.py", Some("B"), "do", 1, vec![]);
    let calls = vec![call_to(&m, "do", Some("B")); 4];
    let findings = detect_with(vec![m, other], calls, &t().audit);
    assert!(!findings.is_empty());
    let e = envy(&findings[0]);
    assert_eq!(e.method_class.as_deref(), Some("Cls"));
    assert_eq!(e.method_name, "the_method");
    assert_eq!(e.method_line, 42);
    assert_eq!(e.method_file, PathBuf::from("location.py"));
}

#[test]
fn sort_by_atfd_descending() {
    let mut defs = Vec::new();
    let mut calls = Vec::new();
    for atfd in [6u32, 9, 7, 12] {
        let foreign: Vec<(&str, &str)> = (0..atfd as usize)
            .map(|i| match i {
                0 => ("b", "f0"),
                1 => ("b", "f1"),
                2 => ("b", "f2"),
                3 => ("b", "f3"),
                4 => ("b", "f4"),
                5 => ("b", "f5"),
                6 => ("b", "f6"),
                7 => ("b", "f7"),
                8 => ("b", "f8"),
                9 => ("b", "f9"),
                10 => ("b", "f10"),
                11 => ("b", "f11"),
                _ => ("b", "fN"),
            })
            .collect();
        let m_name = format!("m_atfd_{atfd}");
        let m = def(&format!("{m_name}.py"), Some(&m_name), "go", 1, foreign);
        defs.push(m.clone());
        defs.push(def("b.py", Some("B"), "do", 1, vec![]));
        for _ in 0..atfd {
            calls.push(call_to(&m, "do", Some("B")));
        }
    }
    let findings = detect_with(defs, calls, &t().audit);
    assert!(findings.len() >= 3);
    for w in findings.windows(2) {
        assert!(envy(&w[0]).atfd >= envy(&w[1]).atfd);
    }
}

#[test]
fn empty_inputs_no_findings_no_panic() {
    let _ = detect_with(Vec::new(), Vec::new(), &t().audit);
}

#[test]
fn raising_atfd_threshold_suppresses() {
    let foreign = vec![("b", "f1"), ("b", "f2"), ("b", "f3"), ("b", "f4"), ("b", "f5"), ("b", "f6")];
    let m = def("a.py", Some("A"), "m", 1, foreign);
    let other = def("b.py", Some("B"), "do", 1, vec![]);
    let calls = vec![call_to(&m, "do", Some("B")); 4];
    let mut th = t().audit;
    th.named_smells.feature_envy.atfd = 1000;
    assert!(detect_with(vec![m, other], calls, &th).is_empty());
}

#[test]
fn raising_foreign_ratio_threshold_suppresses() {
    let foreign = vec![("b", "f1"), ("b", "f2"), ("b", "f3"), ("b", "f4"), ("b", "f5"), ("b", "f6")];
    let m = def("a.py", Some("A"), "m", 1, foreign);
    let other = def("b.py", Some("B"), "do", 1, vec![]);
    let calls = vec![call_to(&m, "do", Some("B")); 4];
    let mut th = t().audit;
    th.named_smells.feature_envy.foreign_ratio = 1.0;
    assert!(detect_with(vec![m, other], calls, &th).is_empty());
}

#[test]
fn lowering_thresholds_yields_at_least_as_many_findings() {
    let foreign = vec![("b", "x"), ("b", "y"), ("b", "z")];
    let m = def("a.py", Some("A"), "m", 1, foreign);
    let other = def("b.py", Some("B"), "do", 1, vec![]);
    let calls = vec![call_to(&m, "do", Some("B"))];
    let baseline = detect_with(vec![m.clone(), other.clone()], calls.clone(), &t().audit);

    let mut lowered = t().audit;
    lowered.named_smells.feature_envy.atfd = 1;
    lowered.named_smells.feature_envy.foreign_ratio = 0.0;
    let lowered_findings = detect_with(vec![m, other], calls, &lowered);
    assert!(lowered_findings.len() >= baseline.len());
}

#[test]
fn determinism_five_runs_yield_same_finding_count() {
    let mut counts = Vec::new();
    for _ in 0..5 {
        let foreign = vec![("b", "f1"), ("b", "f2"), ("b", "f3"), ("b", "f4"), ("b", "f5"), ("b", "f6")];
        let m = def("a.py", Some("A"), "m", 1, foreign);
        let other = def("b.py", Some("B"), "do", 1, vec![]);
        let calls = vec![call_to(&m, "do", Some("B")); 4];
        let findings = detect_with(vec![m, other], calls, &t().audit);
        counts.push(findings.len());
    }
    for c in &counts[1..] {
        assert_eq!(*c, counts[0]);
    }
}

#[test]
fn intra_calls_do_not_count_as_foreign() {
    let foreign = vec![("b", "f1"), ("b", "f2"), ("b", "f3"), ("b", "f4"), ("b", "f5"), ("b", "f6")];
    let m = def("a.py", Some("A"), "m", 1, foreign);
    let intra1 = def("a.py", Some("A"), "intra1", 2, vec![]);
    let intra2 = def("a.py", Some("A"), "intra2", 3, vec![]);
    let calls = vec![call_to(&m, "intra1", Some("A")), call_to(&m, "intra2", Some("A"))];
    let findings = detect_with(vec![m, intra1, intra2], calls, &t().audit);
    assert!(findings.is_empty(), "all-intra calls should give 0 foreign ratio");
}

#[test]
fn envied_class_unset_when_foreign_list_empty() {
    let m = def("a.py", Some("A"), "m", 1, Vec::new());
    let other = def("b.py", Some("B"), "do", 1, vec![]);
    let calls = vec![call_to(&m, "do", Some("B"))];
    let findings = detect_with(vec![m, other], calls, &t().audit);
    assert!(findings.is_empty(), "atfd=0 cannot exceed threshold");
}

#[test]
fn constructor_with_foreign_accesses_eligible() {
    let foreign = vec![("b", "f1"), ("b", "f2"), ("b", "f3"), ("b", "f4"), ("b", "f5"), ("b", "f6")];
    let mut m = def("a.py", Some("A"), "__init__", 1, foreign);
    m.is_constructor = true;
    let other = def("b.py", Some("B"), "do", 1, vec![]);
    let calls = vec![call_to(&m, "do", Some("B")); 4];
    let findings = detect_with(vec![m, other], calls, &t().audit);
    assert!(!findings.is_empty(), "constructor methods are still subject to feature-envy detection");
}

#[test]
fn parent_class_method_with_foreign_field_eligible() {
    let foreign = vec![("b", "f1"), ("b", "f2"), ("b", "f3"), ("b", "f4"), ("b", "f5"), ("b", "f6")];
    let mut m = def("a.py", Some("A"), "m", 1, foreign);
    m.parent_class = Some("ParentBase".to_string());
    let other = def("b.py", Some("B"), "do", 1, vec![]);
    let calls = vec![call_to(&m, "do", Some("B")); 4];
    let findings = detect_with(vec![m, other], calls, &t().audit);
    assert!(!findings.is_empty());
}

#[test]
fn stress_500_methods_all_potential_envy_completes_quickly() {
    use std::time::Instant;
    let mut defs = Vec::new();
    let mut calls = Vec::new();
    for i in 0..500 {
        let foreign = vec![("ext", "f1"), ("ext", "f2"), ("ext", "f3"), ("ext", "f4"), ("ext", "f5"), ("ext", "f6")];
        let m = def(&format!("c_{i}.py"), Some(&format!("Cls{i}")), "m", 1, foreign);
        defs.push(m.clone());
        for _ in 0..3 {
            calls.push(call_to(&m, "do", Some("Other")));
        }
    }
    defs.push(def("o.py", Some("Other"), "do", 1, vec![]));
    let started = Instant::now();
    let _ = detect_with(defs, calls, &t().audit);
    assert!(started.elapsed().as_secs_f64() < 5.0);
}

#[test]
fn multiple_foreign_classes_dominant_picked_correctly() {
    let foreign = vec![("alpha", "x"), ("alpha", "y"), ("beta", "p"), ("beta", "q"), ("beta", "r"), ("beta", "s")];
    let m = def("a.py", Some("A"), "m", 1, foreign);
    let other = def("b.py", Some("B"), "do", 1, vec![]);
    let calls = vec![call_to(&m, "do", Some("B")); 4];
    let findings = detect_with(vec![m, other], calls, &t().audit);
    if let Some(f) = findings.first() {
        assert_eq!(envy(f).envied_class.as_deref(), Some("beta"));
    }
}

#[test]
fn evidence_carries_intra_and_foreign_counts() {
    let foreign = vec![("b", "f1"), ("b", "f2"), ("b", "f3"), ("b", "f4"), ("b", "f5"), ("b", "f6")];
    let m = def("a.py", Some("A"), "m", 1, foreign);
    let other = def("b.py", Some("B"), "do", 1, vec![]);
    let intra = def("a.py", Some("A"), "intra", 5, vec![]);
    let calls = vec![
        call_to(&m, "do", Some("B")),
        call_to(&m, "do", Some("B")),
        call_to(&m, "do", Some("B")),
        call_to(&m, "do", Some("B")),
        call_to(&m, "intra", Some("A")),
    ];
    let findings = detect_with(vec![m, other, intra], calls, &t().audit);
    assert!(!findings.is_empty());
    let e = envy(&findings[0]);
    assert_eq!(e.foreign_call_count, 4);
    assert_eq!(e.intra_call_count, 1);
}

#[test]
fn no_calls_to_existing_targets_no_finding() {
    let foreign = vec![("b", "f1"), ("b", "f2"), ("b", "f3"), ("b", "f4"), ("b", "f5"), ("b", "f6")];
    let m = def("a.py", Some("A"), "m", 1, foreign);
    let other = def("b.py", Some("B"), "do", 1, vec![]);
    let unrelated_call = LocatedCall {
        call: RawCall { callee_name: "ghost".to_string(), receiver_hint: Some("Phantom".to_string()), line: 5 },
        caller: Some(m.identity.clone()),
        file: m.identity.file.clone(),
    };
    let findings = detect_with(vec![m, other], vec![unrelated_call], &t().audit);
    assert!(findings.is_empty());
}

#[test]
fn three_methods_one_envious_only_envious_reported() {
    let envious_foreign = vec![("b", "f1"), ("b", "f2"), ("b", "f3"), ("b", "f4"), ("b", "f5"), ("b", "f6")];
    let envious = def("a.py", Some("A"), "envious", 1, envious_foreign);
    let normal = def("a.py", Some("A"), "normal", 5, vec![("b", "x")]);
    let isolated = def("a.py", Some("A"), "isolated", 10, vec![]);
    let other = def("b.py", Some("B"), "do", 1, vec![]);
    let calls = vec![
        call_to(&envious, "do", Some("B")),
        call_to(&envious, "do", Some("B")),
        call_to(&envious, "do", Some("B")),
        call_to(&envious, "do", Some("B")),
    ];
    let findings = detect_with(vec![envious, normal, isolated, other], calls, &t().audit);
    assert_eq!(findings.len(), 1);
    assert_eq!(envy(&findings[0]).method_name, "envious");
}
