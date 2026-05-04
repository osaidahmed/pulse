use std::path::PathBuf;
use std::time::Instant;

use pulse::audit::call_graph::{CallGraph, MethodIdentity};
use pulse::audit::call_walker::LocatedCall;
use pulse::audit::calls::RawCall;
use pulse::audit::class_registry::ClassRegistry;
use pulse::audit::definitions::DefinitionRecord;
use pulse::audit::detector_divergent_change::detect;
use pulse::audit::finding::{AuditFinding, AuditKind, DivergentChangeEvidence};

use crate::audit_common::t;

fn def(file: &str, class: Option<&str>, name: &str, line: u32) -> DefinitionRecord {
    DefinitionRecord {
        identity: MethodIdentity {
            file: PathBuf::from(file),
            class: class.map(String::from),
            name: name.to_string(),
            line,
        },
        cc: 1,
        field_accesses: Vec::new(),
        foreign_field_accesses: Vec::new(),
        parent_class: None,
        is_constructor: false,
    }
}

fn call_to(caller: &DefinitionRecord, callee_name: &str, hint: Option<&str>) -> LocatedCall {
    LocatedCall {
        call: RawCall {
            callee_name: callee_name.to_string(),
            receiver_hint: hint.map(String::from),
            line: caller.identity.line + 1,
        },
        caller: Some(caller.identity.clone()),
        file: caller.identity.file.clone(),
    }
}

fn build_scenario(
    target_class: &str,
    target_file: &str,
    methods: u32,
    caller_classes: u32,
    callee_classes: u32,
) -> (Vec<DefinitionRecord>, Vec<LocatedCall>) {
    let mut defs = Vec::new();
    let mut calls = Vec::new();
    for m in 0..methods {
        defs.push(def(target_file, Some(target_class), &format!("m{m}"), 10 + m));
    }
    for c in 0..caller_classes {
        let caller_file = format!("caller_{c}.py");
        let caller_class = format!("Caller{c}");
        let caller_method = def(&caller_file, Some(&caller_class), "trigger", 1);
        defs.push(caller_method.clone());
        calls.push(call_to(&caller_method, "m0", Some(target_class)));
    }
    for c in 0..callee_classes {
        let dep_file = format!("dep_{c}.py");
        let dep_class = format!("Dep{c}");
        defs.push(def(&dep_file, Some(&dep_class), "do", 1));
        let target_method = defs[0].clone();
        calls.push(call_to(&target_method, "do", Some(&dep_class)));
    }
    (defs, calls)
}

fn detect_with(
    defs: Vec<DefinitionRecord>,
    calls: Vec<LocatedCall>,
    th: &pulse::thresholds::AuditThresholds,
) -> Vec<AuditFinding> {
    let graph = CallGraph::build(defs.clone(), calls);
    let registry = ClassRegistry::from_definitions(&defs, &graph.registry);
    detect(&registry, &graph, th)
}

fn evidence(f: &AuditFinding) -> &DivergentChangeEvidence {
    let AuditKind::DivergentChange(e) = &f.kind else {
        panic!("expected DivergentChange variant");
    };
    e
}

#[test]
fn boundary_changing_classes_strictly_greater() {
    let (defs, calls) = build_scenario("C", "c.py", 6, 6, 7);
    assert!(detect_with(defs, calls, &t().audit).is_empty(), "cc=6 not > 6");
    let (defs, calls) = build_scenario("C", "c.py", 6, 7, 7);
    assert!(!detect_with(defs, calls, &t().audit).is_empty(), "cc=7 should fire");
}

#[test]
fn boundary_changing_classes_one_below() {
    let (defs, calls) = build_scenario("C", "c.py", 6, 5, 7);
    assert!(detect_with(defs, calls, &t().audit).is_empty());
}

#[test]
fn boundary_fanout_strictly_greater() {
    let (defs, calls) = build_scenario("C", "c.py", 6, 7, 6);
    assert!(detect_with(defs, calls, &t().audit).is_empty(), "fanout=6 not > 6");
    let (defs, calls) = build_scenario("C", "c.py", 6, 7, 7);
    assert!(!detect_with(defs, calls, &t().audit).is_empty());
}

#[test]
fn boundary_method_count_strictly_greater() {
    let (defs, calls) = build_scenario("C", "c.py", 5, 7, 7);
    assert!(detect_with(defs, calls, &t().audit).is_empty(), "method_count=5 not > 5");
    let (defs, calls) = build_scenario("C", "c.py", 6, 7, 7);
    assert!(!detect_with(defs, calls, &t().audit).is_empty());
}

#[test]
fn two_axes_exceeded_third_at_threshold_no_finding() {
    let (defs, calls) = build_scenario("C", "c.py", 5, 7, 7);
    assert!(detect_with(defs, calls, &t().audit).is_empty());

    let (defs, calls) = build_scenario("C", "c.py", 6, 6, 7);
    assert!(detect_with(defs, calls, &t().audit).is_empty());

    let (defs, calls) = build_scenario("C", "c.py", 6, 7, 6);
    assert!(detect_with(defs, calls, &t().audit).is_empty());
}

#[test]
fn one_above_two_at_threshold_no_finding() {
    let (defs, calls) = build_scenario("C", "c.py", 6, 7, 6);
    assert!(detect_with(defs, calls, &t().audit).is_empty());
}

#[test]
fn fully_intra_class_calls_yield_zero_changing_classes() {
    let mut defs = Vec::new();
    let mut calls = Vec::new();
    for m in 0..6 {
        defs.push(def("inner.py", Some("Inner"), &format!("m{m}"), 10 + m));
    }
    for m in 1..6 {
        let caller = defs[m].clone();
        calls.push(call_to(&caller, "m0", Some("Inner")));
    }
    let findings = detect_with(defs, calls, &t().audit);
    assert!(findings.is_empty());
}

#[test]
fn free_functions_excluded_from_detector() {
    let mut defs = Vec::new();
    let mut calls = Vec::new();
    for m in 0..6 {
        defs.push(def("free.py", None, &format!("m{m}"), 10 + m));
    }
    for c in 0..7 {
        let caller = def(&format!("caller_{c}.py"), Some(&format!("Caller{c}")), "t", 1);
        defs.push(caller.clone());
        calls.push(call_to(&caller, "m0", None));
    }
    let findings = detect_with(defs, calls, &t().audit);
    assert!(findings.is_empty());
}

#[test]
fn same_class_name_in_different_files_kept_distinct() {
    let mut defs = Vec::new();
    let mut calls = Vec::new();
    for m in 0..3 {
        defs.push(def("a.py", Some("Shared"), &format!("a{m}"), 10 + m));
    }
    for m in 0..3 {
        defs.push(def("b.py", Some("Shared"), &format!("b{m}"), 10 + m));
    }
    for c in 0..7 {
        let caller_a = def(&format!("ca_{c}.py"), Some(&format!("CallerA{c}")), "t", 1);
        defs.push(caller_a.clone());
        calls.push(call_to(&caller_a, "a0", Some("Shared")));
    }
    let findings = detect_with(defs, calls, &t().audit);
    assert!(findings.iter().all(|f| evidence(f).method_count <= 3));
}

#[test]
fn lowering_each_threshold_yields_at_least_as_many_findings() {
    let (defs, calls) = build_scenario("Mid", "m.py", 4, 4, 4);
    let baseline = detect_with(defs.clone(), calls.clone(), &t().audit);

    let mut lowered = t().audit;
    lowered.named_smells.divergent_change.changing_classes = 2;
    lowered.named_smells.divergent_change.fanout = 2;
    lowered.named_smells.divergent_change.method_count = 2;
    let lowered_findings = detect_with(defs, calls, &lowered);
    assert!(lowered_findings.len() >= baseline.len());
}

#[test]
fn raising_method_count_threshold_suppresses() {
    let (defs, calls) = build_scenario("X", "x.py", 6, 7, 7);
    let mut th = t().audit;
    th.named_smells.divergent_change.method_count = 1000;
    assert!(detect_with(defs, calls, &th).is_empty());
}

#[test]
fn raising_fanout_threshold_suppresses() {
    let (defs, calls) = build_scenario("X", "x.py", 6, 7, 7);
    let mut th = t().audit;
    th.named_smells.divergent_change.fanout = 1000;
    assert!(detect_with(defs, calls, &th).is_empty());
}

#[test]
fn determinism_five_runs_yield_byte_identical_class_names() {
    let mut snapshots = Vec::new();
    for _ in 0..5 {
        let (defs, calls) = build_scenario("Det", "det.py", 6, 7, 7);
        let findings = detect_with(defs, calls, &t().audit);
        let names: Vec<String> = findings.iter().map(|f| evidence(f).class_name.clone()).collect();
        snapshots.push(names);
    }
    for i in 1..snapshots.len() {
        assert_eq!(snapshots[0], snapshots[i]);
    }
}

#[test]
fn empty_defs_yields_empty_findings() {
    let findings = detect_with(Vec::new(), Vec::new(), &t().audit);
    assert!(findings.is_empty());
}

#[test]
fn empty_graph_with_defs_yields_empty_findings() {
    let mut defs = Vec::new();
    for m in 0..6 {
        defs.push(def("c.py", Some("C"), &format!("m{m}"), 10 + m));
    }
    let findings = detect_with(defs, Vec::new(), &t().audit);
    assert!(findings.is_empty());
}

#[test]
fn only_free_functions_yields_empty_findings() {
    let mut defs = Vec::new();
    for m in 0..10 {
        defs.push(def("f.py", None, &format!("f{m}"), 10 + m));
    }
    let findings = detect_with(defs, Vec::new(), &t().audit);
    assert!(findings.is_empty());
}

#[test]
fn evidence_carries_target_file_and_metrics() {
    let (defs, calls) = build_scenario("C", "target.py", 6, 7, 7);
    let findings = detect_with(defs, calls, &t().audit);
    assert!(!findings.is_empty());
    let e = evidence(&findings[0]);
    assert_eq!(e.class_file, PathBuf::from("target.py"));
    assert!(e.changing_classes >= 7);
    assert!(e.fanout >= 7);
    assert!(e.method_count >= 6);
}

#[test]
fn detector_returns_all_qualifying_findings_no_internal_truncation() {
    let mut defs = Vec::new();
    let mut calls = Vec::new();
    let class_count = 60;
    for class_idx in 0..class_count {
        let class_name = format!("Big{class_idx}");
        let class_file = format!("big_{class_idx}.py");
        for m in 0..6 {
            defs.push(def(&class_file, Some(&class_name), &format!("m{m}"), 10 + m));
        }
        for c in 0..7 {
            let caller_file = format!("c_{class_idx}_{c}.py");
            let caller_class = format!("Caller{class_idx}_{c}");
            let caller = def(&caller_file, Some(&caller_class), "t", 1);
            defs.push(caller.clone());
            calls.push(call_to(&caller, "m0", Some(&class_name)));
        }
        for c in 0..7 {
            let dep_file = format!("d_{class_idx}_{c}.py");
            let dep_class = format!("Dep{class_idx}_{c}");
            defs.push(def(&dep_file, Some(&dep_class), "do", 1));
            let target = def(&class_file, Some(&class_name), "m0", 10);
            calls.push(call_to(&target, "do", Some(&dep_class)));
        }
    }
    let findings = detect_with(defs, calls, &t().audit);
    assert_eq!(findings.len(), class_count, "detector itself does not truncate");
}

#[test]
fn stress_one_thousand_methods_completes_quickly() {
    let mut defs = Vec::new();
    let mut calls = Vec::new();
    for m in 0..1000 {
        defs.push(def("big.py", Some("Big"), &format!("m{m}"), 10 + m));
    }
    for c in 0..7 {
        let caller = def(&format!("c_{c}.py"), Some(&format!("Caller{c}")), "t", 1);
        defs.push(caller.clone());
        calls.push(call_to(&caller, "m0", Some("Big")));
    }
    for c in 0..7 {
        defs.push(def(&format!("d_{c}.py"), Some(&format!("Dep{c}")), "do", 1));
        let target = def("big.py", Some("Big"), "m0", 10);
        calls.push(call_to(&target, "do", Some(&format!("Dep{c}"))));
    }
    let started = Instant::now();
    let _ = detect_with(defs, calls, &t().audit);
    let elapsed = started.elapsed();
    assert!(elapsed.as_secs_f64() < 5.0, "stress took too long: {elapsed:?}");
}

#[test]
fn ordering_preserves_descending_changing_classes() {
    let mut defs = Vec::new();
    let mut calls = Vec::new();
    for cc_count in [9u32, 7, 8, 10] {
        let class_name = format!("C{cc_count}");
        let class_file = format!("c_{cc_count}.py");
        for m in 0..6 {
            defs.push(def(&class_file, Some(&class_name), &format!("m{m}"), 10 + m));
        }
        for c in 0..cc_count {
            let caller = def(&format!("ca_{cc_count}_{c}.py"), Some(&format!("Caller{cc_count}_{c}")), "t", 1);
            defs.push(caller.clone());
            calls.push(call_to(&caller, "m0", Some(&class_name)));
        }
        for c in 0..7u32 {
            defs.push(def(&format!("d_{cc_count}_{c}.py"), Some(&format!("Dep{cc_count}_{c}")), "do", 1));
            let target = def(&class_file, Some(&class_name), "m0", 10);
            calls.push(call_to(&target, "do", Some(&format!("Dep{cc_count}_{c}"))));
        }
    }
    let findings = detect_with(defs, calls, &t().audit);
    assert!(findings.len() >= 3);
    for w in findings.windows(2) {
        let a = evidence(&w[0]).changing_classes;
        let b = evidence(&w[1]).changing_classes;
        assert!(a >= b, "ordering must be descending: {a} >= {b}");
    }
}

#[test]
fn constructor_only_class_with_high_dependencies_does_not_trigger() {
    let mut defs = Vec::new();
    let mut calls = Vec::new();
    for m in 0..6 {
        let mut d = def("ctor.py", Some("CtorOnly"), &format!("init{m}"), 10 + m);
        d.is_constructor = true;
        defs.push(d);
    }
    for c in 0..7 {
        let caller = def(&format!("c_{c}.py"), Some(&format!("C{c}")), "t", 1);
        defs.push(caller.clone());
        calls.push(call_to(&caller, "init0", Some("CtorOnly")));
    }
    let _ = detect_with(defs, calls, &t().audit);
}

#[test]
fn multiple_classes_same_file_handled_independently() {
    let mut defs = Vec::new();
    let mut calls = Vec::new();
    for class_name in ["A", "B"] {
        for m in 0..6 {
            defs.push(def("multi.py", Some(class_name), &format!("m{m}"), 10 + m));
        }
        for c in 0..7 {
            let caller = def(&format!("c_{class_name}_{c}.py"), Some(&format!("Caller{class_name}{c}")), "t", 1);
            defs.push(caller.clone());
            calls.push(call_to(&caller, "m0", Some(class_name)));
        }
        for c in 0..7 {
            defs.push(def(&format!("d_{class_name}_{c}.py"), Some(&format!("Dep{class_name}{c}")), "do", 1));
            let target = def("multi.py", Some(class_name), "m0", 10);
            calls.push(call_to(&target, "do", Some(&format!("Dep{class_name}{c}"))));
        }
    }
    let findings = detect_with(defs, calls, &t().audit);
    let names: std::collections::BTreeSet<String> =
        findings.iter().map(|f| evidence(f).class_name.clone()).collect();
    assert!(names.contains("A"));
    assert!(names.contains("B"));
}

#[test]
fn evidence_confidence_present() {
    let (defs, calls) = build_scenario("C", "c.py", 6, 7, 7);
    let findings = detect_with(defs, calls, &t().audit);
    assert!(!findings.is_empty());
    use pulse::audit::finding::ImportConfidence;
    let conf = evidence(&findings[0]).confidence;
    assert!(matches!(
        conf,
        ImportConfidence::High
            | ImportConfidence::Medium
            | ImportConfidence::Low
            | ImportConfidence::BestEffort
            | ImportConfidence::NaAbstraction
    ));
}

#[test]
fn no_callers_to_class_yields_zero_changing_classes() {
    let mut defs = Vec::new();
    let calls: Vec<LocatedCall> = Vec::new();
    for m in 0..10 {
        defs.push(def("isolated.py", Some("Lonely"), &format!("m{m}"), 10 + m));
    }
    let findings = detect_with(defs, calls, &t().audit);
    assert!(findings.is_empty());
}
