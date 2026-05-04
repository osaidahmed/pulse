use std::path::PathBuf;

use pulse::audit::call_graph::{CallGraph, MethodIdentity};
use pulse::audit::call_walker::LocatedCall;
use pulse::audit::calls::RawCall;
use pulse::audit::class_registry::ClassRegistry;
use pulse::audit::definitions::DefinitionRecord;
use pulse::audit::detector_divergent_change::detect;
use pulse::audit::finding::{AuditKind, DivergentChangeEvidence};

use crate::audit_common::t;

fn def(file: &str, class: &str, name: &str, line: u32) -> DefinitionRecord {
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

fn divergent_evidence(f: &pulse::audit::finding::AuditFinding) -> &DivergentChangeEvidence {
    let AuditKind::DivergentChange(e) = &f.kind else {
        panic!("expected DivergentChange, got {:?}", f.kind);
    };
    e
}

fn build_class_with_callers_and_callees(
    target_class: &str,
    target_file: &str,
    methods: u32,
    caller_classes: u32,
    callee_classes: u32,
) -> (Vec<DefinitionRecord>, Vec<LocatedCall>) {
    let mut defs = Vec::new();
    let mut calls = Vec::new();
    for m in 0..methods {
        defs.push(def(target_file, target_class, &format!("m{m}"), 10 + m));
    }
    for c in 0..caller_classes {
        let caller_file = format!("caller_{c}.py");
        let caller_class = format!("Caller{c}");
        let caller_method = def(&caller_file, &caller_class, "trigger", 1);
        defs.push(caller_method.clone());
        calls.push(call_to(&caller_method, "m0", Some(target_class)));
    }
    for c in 0..callee_classes {
        let dep_file = format!("dep_{c}.py");
        let dep_class = format!("Dep{c}");
        defs.push(def(&dep_file, &dep_class, "do", 1));
        let target_method = defs[0].clone();
        calls.push(call_to(&target_method, "do", Some(&dep_class)));
    }
    (defs, calls)
}

fn detect_with(
    defs: Vec<DefinitionRecord>,
    calls: Vec<LocatedCall>,
    t: &pulse::thresholds::AuditThresholds,
) -> Vec<pulse::audit::finding::AuditFinding> {
    let graph = CallGraph::build(defs.clone(), calls);
    let registry = ClassRegistry::from_definitions(&defs, &graph.registry);
    detect(&registry, &graph, t)
}

#[test]
fn divergent_class_with_high_cc_fanout_methods_emits_finding() {
    let (defs, calls) = build_class_with_callers_and_callees("Big", "big.py", 6, 7, 7);
    let findings = detect_with(defs, calls, &t().audit);
    assert!(!findings.is_empty());
    let e = divergent_evidence(&findings[0]);
    assert_eq!(e.class_name, "Big");
    assert!(e.changing_classes > 6);
    assert!(e.fanout > 6);
    assert!(e.method_count > 5);
}

#[test]
fn no_finding_when_method_count_at_threshold() {
    let (defs, calls) = build_class_with_callers_and_callees("Class", "c.py", 5, 7, 7);
    let findings = detect_with(defs, calls, &t().audit);
    assert!(findings.is_empty(), "method_count=5 not strictly greater than 5");
}

#[test]
fn no_finding_when_cc_below_threshold() {
    let (defs, calls) = build_class_with_callers_and_callees("Class", "c.py", 6, 3, 7);
    let findings = detect_with(defs, calls, &t().audit);
    assert!(findings.is_empty());
}

#[test]
fn no_finding_when_fanout_below_threshold() {
    let (defs, calls) = build_class_with_callers_and_callees("Class", "c.py", 6, 7, 3);
    let findings = detect_with(defs, calls, &t().audit);
    assert!(findings.is_empty());
}

#[test]
fn empty_inputs_yield_empty() {
    let findings = detect_with(Vec::new(), Vec::new(), &t().audit);
    assert!(findings.is_empty());
}

#[test]
fn class_with_few_methods_does_not_trigger() {
    let (defs, calls) = build_class_with_callers_and_callees("Tiny", "t.py", 2, 7, 7);
    let findings = detect_with(defs, calls, &t().audit);
    assert!(findings.is_empty());
}

#[test]
fn lowering_thresholds_surfaces_smaller_classes() {
    let (defs, calls) = build_class_with_callers_and_callees("Mid", "m.py", 3, 4, 4);
    let mut audit_t = t().audit;
    audit_t.named_smells.divergent_change.changing_classes = 2;
    audit_t.named_smells.divergent_change.fanout = 2;
    audit_t.named_smells.divergent_change.method_count = 2;
    let findings = detect_with(defs, calls, &audit_t);
    assert!(!findings.is_empty());
}

#[test]
fn raising_threshold_suppresses_finding() {
    let (defs, calls) = build_class_with_callers_and_callees("Big", "b.py", 6, 7, 7);
    let mut audit_t = t().audit;
    audit_t.named_smells.divergent_change.changing_classes = 100;
    let findings = detect_with(defs, calls, &audit_t);
    assert!(findings.is_empty());
}

#[test]
fn determinism_two_runs_same_count() {
    let (defs1, calls1) = build_class_with_callers_and_callees("X", "x.py", 6, 7, 7);
    let (defs2, calls2) = build_class_with_callers_and_callees("X", "x.py", 6, 7, 7);
    let r1 = detect_with(defs1, calls1, &t().audit);
    let r2 = detect_with(defs2, calls2, &t().audit);
    assert_eq!(r1.len(), r2.len());
}

#[test]
fn ordering_by_cc_desc() {
    let mut defs = Vec::new();
    let mut calls = Vec::new();
    for cc_count in [9u32, 7, 8] {
        for m in 0..6 {
            defs.push(def(&format!("c_{cc_count}.py"), &format!("C{cc_count}"), &format!("m{m}"), 10 + m));
        }
        for c in 0..cc_count {
            let caller = def(&format!("call_{cc_count}_{c}.py"), &format!("Caller{cc_count}_{c}"), "t", 1);
            defs.push(caller.clone());
            calls.push(call_to(&caller, "m0", Some(&format!("C{cc_count}"))));
        }
        for c in 0..7u32 {
            defs.push(def(&format!("dep_{cc_count}_{c}.py"), &format!("Dep{cc_count}_{c}"), "do", 1));
            let target = def(&format!("c_{cc_count}.py"), &format!("C{cc_count}"), "m0", 10);
            calls.push(call_to(&target, "do", Some(&format!("Dep{cc_count}_{c}"))));
        }
    }
    let findings = detect_with(defs, calls, &t().audit);
    assert!(findings.len() >= 2);
    let cc_first = divergent_evidence(&findings[0]).changing_classes;
    let cc_second = divergent_evidence(&findings[1]).changing_classes;
    assert!(cc_first >= cc_second);
}
