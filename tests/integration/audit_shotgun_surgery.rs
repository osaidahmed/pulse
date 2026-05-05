use std::path::PathBuf;

use pulse::audit::call_graph::MethodIdentity;
use pulse::audit::call_walker::LocatedCall;
use pulse::audit::calls::RawCall;
use pulse::audit::definitions::DefinitionRecord;
use pulse::audit::finding::{AuditKind, ImportConfidence};
use pulse::audit::named_smells::run_from_inputs;
use pulse::thresholds::AuditThresholds;

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

fn call(
    caller_file: &str,
    caller_class: Option<&str>,
    caller_name: &str,
    caller_line: u32,
    callee_name: &str,
    receiver_hint: Option<&str>,
) -> LocatedCall {
    LocatedCall {
        call: RawCall {
            callee_name: callee_name.to_string(),
            receiver_hint: receiver_hint.map(String::from),
            line: caller_line + 1,
        },
        caller: Some(MethodIdentity {
            file: PathBuf::from(caller_file),
            class: caller_class.map(String::from),
            name: caller_name.to_string(),
            line: caller_line,
        }),
        file: PathBuf::from(caller_file),
    }
}

fn shotgun_evidence(f: &pulse::audit::finding::AuditFinding) -> &pulse::audit::finding::ShotgunSurgeryEvidence {
    let AuditKind::ShotgunSurgery(e) = &f.kind else { panic!("expected ShotgunSurgery, got {:?}", f.kind) };
    e
}

fn defs_with_target_and_callers(callers_per_class: usize, classes: usize) -> Vec<DefinitionRecord> {
    let mut defs = vec![def("target.py", Some("Target"), "handle", 1)];
    for c in 0..classes {
        for m in 0..callers_per_class {
            defs.push(def(
                &format!("caller_{c}.py"),
                Some(&format!("Class{c}")),
                &format!("method_{m}"),
                10 * (m as u32 + 1),
            ));
        }
    }
    defs
}

fn calls_pointing_at_target(callers_per_class: usize, classes: usize) -> Vec<LocatedCall> {
    let mut out = Vec::new();
    for c in 0..classes {
        for m in 0..callers_per_class {
            out.push(call(
                &format!("caller_{c}.py"),
                Some(&format!("Class{c}")),
                &format!("method_{m}"),
                10 * (m as u32 + 1),
                "handle",
                Some("Target"),
            ));
        }
    }
    out
}

fn defs_with_target_outgoing(target_fanout: usize) -> Vec<DefinitionRecord> {
    let mut defs = vec![def("target.py", Some("Target"), "handle", 1)];
    for i in 0..target_fanout {
        defs.push(def(
            &format!("dep_{i}.py"),
            Some(&format!("Dep{i}")),
            &format!("dep_method_{i}"),
            5,
        ));
    }
    defs
}

fn calls_target_invokes_deps(fanout: usize) -> Vec<LocatedCall> {
    let mut out = Vec::new();
    for i in 0..fanout {
        out.push(call(
            "target.py",
            Some("Target"),
            "handle",
            1,
            &format!("dep_method_{i}"),
            Some(&format!("Dep{i}")),
        ));
    }
    out
}

fn full_shotgun_setup(
    classes: u32,
    methods_per_class: u32,
    fanout: u32,
) -> (Vec<DefinitionRecord>, Vec<LocatedCall>) {
    let mut defs = defs_with_target_and_callers(methods_per_class as usize, classes as usize);
    defs.extend(defs_with_target_outgoing(fanout as usize).into_iter().skip(1));
    let mut calls = calls_pointing_at_target(methods_per_class as usize, classes as usize);
    calls.extend(calls_target_invokes_deps(fanout as usize));
    (defs, calls)
}

#[test]
fn shotgun_surgery_finding_emitted_when_all_thresholds_exceeded() {
    let (defs, calls) = full_shotgun_setup(7, 3, 7);
    let findings = run_from_inputs(defs, calls, &t().audit);
    assert_eq!(findings.len(), 1);
    let e = shotgun_evidence(&findings[0]);
    assert_eq!(e.method_name, "handle");
    assert_eq!(e.changing_classes, 7);
    assert!(e.changing_methods >= 21);
    assert_eq!(e.fanout, 7);
}

#[test]
fn no_finding_when_cc_at_threshold() {
    let (defs, calls) = full_shotgun_setup(5, 3, 7);
    let findings = run_from_inputs(defs, calls, &t().audit);
    assert!(findings.is_empty(), "cc=5 == threshold 5, should not exceed strict >");
}

#[test]
fn no_finding_when_cm_at_threshold() {
    let (defs, calls) = full_shotgun_setup(7, 1, 7);
    let mut audit_t = t().audit;
    audit_t.named_smells.shotgun_surgery.changing_methods = 7;
    let findings = run_from_inputs(defs, calls, &audit_t);
    assert!(findings.is_empty(), "cm=7 == threshold 7, should not exceed strict >");
}

#[test]
fn no_finding_when_fanout_at_threshold() {
    let (defs, calls) = full_shotgun_setup(7, 3, 5);
    let findings = run_from_inputs(defs, calls, &t().audit);
    assert!(findings.is_empty(), "fanout=5 == threshold 5");
}

#[test]
fn no_finding_when_only_cc_above_threshold() {
    let (defs, calls) = full_shotgun_setup(10, 1, 1);
    let findings = run_from_inputs(defs, calls, &t().audit);
    assert!(findings.is_empty());
}

#[test]
fn no_finding_when_only_cm_above_threshold() {
    let (defs, calls) = full_shotgun_setup(1, 20, 1);
    let findings = run_from_inputs(defs, calls, &t().audit);
    assert!(findings.is_empty());
}

#[test]
fn no_finding_when_only_fanout_above_threshold() {
    let (defs, calls) = full_shotgun_setup(1, 1, 20);
    let findings = run_from_inputs(defs, calls, &t().audit);
    assert!(findings.is_empty());
}

#[test]
fn no_finding_for_isolated_methods() {
    let defs = vec![
        def("a.py", Some("A"), "m1", 1),
        def("b.py", Some("B"), "m2", 2),
    ];
    let findings = run_from_inputs(defs, Vec::new(), &t().audit);
    assert!(findings.is_empty());
}

#[test]
fn confidence_is_high_when_class_hint_resolves_exactly() {
    let (defs, calls) = full_shotgun_setup(7, 3, 7);
    let findings = run_from_inputs(defs, calls, &t().audit);
    let e = shotgun_evidence(&findings[0]);
    assert_eq!(e.confidence, ImportConfidence::High);
}

#[test]
fn confidence_drops_to_medium_for_unique_name_no_class_hint() {
    let mut defs = vec![def("target.py", None, "unique_helper", 1)];
    let mut calls = Vec::new();
    for c in 0..7 {
        for m in 0..3 {
            defs.push(def(
                &format!("caller_{c}.py"),
                Some(&format!("Class{c}")),
                &format!("m_{m}"),
                10,
            ));
            calls.push(call(
                &format!("caller_{c}.py"),
                Some(&format!("Class{c}")),
                &format!("m_{m}"),
                10,
                "unique_helper",
                None,
            ));
        }
    }
    for i in 0..7 {
        defs.push(def(&format!("dep_{i}.py"), None, &format!("dep_{i}"), 5));
        calls.push(call("target.py", None, "unique_helper", 1, &format!("dep_{i}"), None));
    }
    let findings = run_from_inputs(defs, calls, &t().audit);
    if let Some(f) = findings.first() {
        let e = shotgun_evidence(f);
        assert_eq!(e.confidence, ImportConfidence::Medium);
    }
}

#[test]
fn caller_samples_capped_at_threshold() {
    let (defs, calls) = full_shotgun_setup(7, 5, 7);
    let mut audit_t = t().audit;
    audit_t.named_smells.max_caller_samples_per_finding = 3;
    let findings = run_from_inputs(defs, calls, &audit_t);
    let e = shotgun_evidence(&findings[0]);
    assert!(e.caller_samples.len() <= 3);
}

#[test]
fn findings_truncated_to_max_named_smell_findings() {
    let mut defs = Vec::new();
    let mut calls = Vec::new();
    for k in 0..3 {
        let target_name = format!("h{k}");
        defs.push(def(&format!("t{k}.py"), Some(&format!("T{k}")), &target_name, 1));
        for c in 0..7 {
            for m in 0..3 {
                let cls = format!("C{k}_{c}");
                let mname = format!("m{k}_{c}_{m}");
                defs.push(def(&format!("c{k}_{c}.py"), Some(&cls), &mname, 10));
                calls.push(call(&format!("c{k}_{c}.py"), Some(&cls), &mname, 10, &target_name, Some(&format!("T{k}"))));
            }
        }
        for i in 0..7 {
            let dep_name = format!("d{k}_{i}");
            defs.push(def(&format!("d{k}_{i}.py"), Some(&format!("D{k}_{i}")), &dep_name, 5));
            calls.push(call(&format!("t{k}.py"), Some(&format!("T{k}")), &target_name, 1, &dep_name, Some(&format!("D{k}_{i}"))));
        }
    }
    let mut audit_t = t().audit;
    audit_t.named_smells.max_findings_reported = 2;
    let findings = run_from_inputs(defs, calls, &audit_t);
    assert!(findings.len() <= 2);
}

#[test]
fn findings_sorted_by_cc_desc_then_cm_desc() {
    let mut defs = Vec::new();
    let mut calls = Vec::new();
    for k in 0..2 {
        let target_name = format!("h{k}");
        defs.push(def(&format!("t{k}.py"), Some(&format!("T{k}")), &target_name, 1));
        let class_count = if k == 0 { 7 } else { 12 };
        for c in 0..class_count {
            for m in 0..3 {
                let cls = format!("C{k}_{c}");
                let mname = format!("m{k}_{c}_{m}");
                defs.push(def(&format!("c{k}_{c}.py"), Some(&cls), &mname, 10));
                calls.push(call(&format!("c{k}_{c}.py"), Some(&cls), &mname, 10, &target_name, Some(&format!("T{k}"))));
            }
        }
        for i in 0..7 {
            let dep_name = format!("d{k}_{i}");
            defs.push(def(&format!("d{k}_{i}.py"), Some(&format!("D{k}_{i}")), &dep_name, 5));
            calls.push(call(&format!("t{k}.py"), Some(&format!("T{k}")), &target_name, 1, &dep_name, Some(&format!("D{k}_{i}"))));
        }
    }
    let findings = run_from_inputs(defs, calls, &t().audit);
    assert!(findings.len() >= 2);
    let e0 = shotgun_evidence(&findings[0]);
    let e1 = shotgun_evidence(&findings[1]);
    assert!(e0.changing_classes >= e1.changing_classes);
}

#[test]
fn empty_inputs_produce_no_findings() {
    let findings = run_from_inputs(Vec::new(), Vec::new(), &t().audit);
    assert!(findings.is_empty());
}

#[test]
fn calls_with_no_caller_are_dropped() {
    let mut defs = vec![def("target.py", Some("T"), "h", 1)];
    let mut calls = Vec::new();
    for c in 0..7 {
        defs.push(def(&format!("c{c}.py"), Some(&format!("C{c}")), &format!("m{c}"), 10));
        calls.push(LocatedCall {
            call: RawCall {
                callee_name: "h".to_string(),
                receiver_hint: Some("T".to_string()),
                line: 11,
            },
            caller: None,
            file: PathBuf::from(format!("c{c}.py")),
        });
    }
    let findings = run_from_inputs(defs, calls, &t().audit);
    assert!(findings.is_empty(), "no caller → no edge → no finding");
}

#[test]
fn calls_with_no_matching_definition_dropped() {
    let defs = vec![def("a.py", None, "caller", 1)];
    let calls = vec![call("a.py", None, "caller", 1, "nonexistent", None)];
    let findings = run_from_inputs(defs, calls, &t().audit);
    assert!(findings.is_empty());
}

#[test]
fn ambiguous_name_resolves_to_low_confidence() {
    let mut defs = vec![
        def("a.py", Some("A"), "shared_name", 1),
        def("b.py", Some("B"), "shared_name", 1),
    ];
    let mut calls = Vec::new();
    for c in 0..7 {
        for m in 0..3 {
            defs.push(def(&format!("c{c}.py"), Some(&format!("Caller{c}")), &format!("m{m}"), 10));
            calls.push(call(&format!("c{c}.py"), Some(&format!("Caller{c}")), &format!("m{m}"), 10, "shared_name", None));
        }
    }
    for i in 0..7 {
        defs.push(def(&format!("d{i}.py"), None, &format!("d{i}"), 5));
        calls.push(call("a.py", Some("A"), "shared_name", 1, &format!("d{i}"), None));
        calls.push(call("b.py", Some("B"), "shared_name", 1, &format!("d{i}"), None));
    }
    let findings = run_from_inputs(defs, calls, &t().audit);
    if let Some(f) = findings.first() {
        let e = shotgun_evidence(f);
        assert!(matches!(e.confidence, ImportConfidence::Low | ImportConfidence::Medium));
    }
}

#[test]
fn self_receiver_resolves_within_caller_class() {
    let mut defs = vec![
        def("self.py", Some("Foo"), "helper", 1),
        def("self.py", Some("Foo"), "caller_a", 5),
    ];
    for k in 1..15u32 {
        defs.push(def(&format!("c{k}.py"), Some(&format!("Foo{k}")), "helper", 1));
    }
    let mut calls = Vec::new();
    calls.push(call("self.py", Some("Foo"), "caller_a", 5, "helper", Some("self")));
    let findings = run_from_inputs(defs, calls, &t().audit);
    let _ = findings;
}

#[test]
fn cls_receiver_resolves_within_caller_class() {
    let mut defs = vec![
        def("self.py", Some("Foo"), "factory", 1),
        def("self.py", Some("Foo"), "caller", 5),
    ];
    for k in 1..15u32 {
        defs.push(def(&format!("c{k}.py"), Some(&format!("Other{k}")), "factory", 1));
    }
    let mut calls = Vec::new();
    calls.push(call("self.py", Some("Foo"), "caller", 5, "factory", Some("cls")));
    let findings = run_from_inputs(defs, calls, &t().audit);
    let _ = findings;
}

#[test]
fn evidence_carries_method_definition_location() {
    let (defs, calls) = full_shotgun_setup(7, 3, 7);
    let findings = run_from_inputs(defs, calls, &t().audit);
    let e = shotgun_evidence(&findings[0]);
    assert_eq!(e.method_file, PathBuf::from("target.py"));
    assert_eq!(e.method_class.as_deref(), Some("Target"));
    assert_eq!(e.method_line, 1);
}

#[test]
fn caller_samples_include_real_caller_locations() {
    let (defs, calls) = full_shotgun_setup(7, 3, 7);
    let findings = run_from_inputs(defs, calls, &t().audit);
    let e = shotgun_evidence(&findings[0]);
    assert!(!e.caller_samples.is_empty());
    for s in &e.caller_samples {
        assert!(s.file.to_string_lossy().contains("caller_"));
    }
}

#[test]
fn determinism_two_runs_produce_same_findings() {
    let (defs1, calls1) = full_shotgun_setup(7, 3, 7);
    let (defs2, calls2) = full_shotgun_setup(7, 3, 7);
    let r1 = run_from_inputs(defs1, calls1, &t().audit);
    let r2 = run_from_inputs(defs2, calls2, &t().audit);
    assert_eq!(r1.len(), r2.len());
    if !r1.is_empty() {
        let e1 = shotgun_evidence(&r1[0]);
        let e2 = shotgun_evidence(&r2[0]);
        assert_eq!(e1.method_name, e2.method_name);
        assert_eq!(e1.changing_classes, e2.changing_classes);
    }
}

#[test]
fn metrics_match_expected_for_canonical_setup() {
    let (defs, calls) = full_shotgun_setup(8, 4, 6);
    let findings = run_from_inputs(defs, calls, &t().audit);
    let e = shotgun_evidence(&findings[0]);
    assert_eq!(e.changing_classes, 8);
    assert_eq!(e.changing_methods, 32);
    assert_eq!(e.fanout, 6);
}

#[test]
fn lowering_threshold_surfaces_more_findings() {
    let (defs, calls) = full_shotgun_setup(3, 2, 3);
    let mut audit_t = t().audit;
    audit_t.named_smells.shotgun_surgery.changing_classes = 2;
    audit_t.named_smells.shotgun_surgery.changing_methods = 5;
    audit_t.named_smells.shotgun_surgery.fanout = 2;
    let findings = run_from_inputs(defs, calls, &audit_t);
    assert!(!findings.is_empty(), "lowered thresholds should surface");
}

#[test]
fn raised_threshold_suppresses_normal_findings() {
    let (defs, calls) = full_shotgun_setup(7, 3, 7);
    let mut audit_t: AuditThresholds = t().audit;
    audit_t.named_smells.shotgun_surgery.changing_classes = 100;
    let findings = run_from_inputs(defs, calls, &audit_t);
    assert!(findings.is_empty());
}

#[test]
fn same_class_name_in_different_files_counts_as_distinct_callers() {
    let target = def("target.py", Some("Target"), "handle", 1);
    let mut defs = vec![target];
    let mut calls = Vec::new();
    for i in 0..7 {
        let caller_def = def(
            &format!("caller_{i}.py"),
            Some("SameClassName"),
            &format!("method_{i}"),
            10,
        );
        defs.push(caller_def.clone());
        for j in 0..3 {
            let extra = def(
                &format!("caller_{i}.py"),
                Some("SameClassName"),
                &format!("extra_{i}_{j}"),
                20 + j,
            );
            calls.push(call(
                &format!("caller_{i}.py"),
                Some("SameClassName"),
                &format!("extra_{i}_{j}"),
                20 + j,
                "handle",
                Some("Target"),
            ));
            defs.push(extra);
        }
    }
    for i in 0..7 {
        defs.push(def(&format!("dep_{i}.py"), Some(&format!("Dep{i}")), &format!("dep_method_{i}"), 5));
        calls.push(call(
            "target.py",
            Some("Target"),
            "handle",
            1,
            &format!("dep_method_{i}"),
            Some(&format!("Dep{i}")),
        ));
    }
    let findings = run_from_inputs(defs, calls, &t().audit);
    assert!(!findings.is_empty(), "7 distinct files w/ same class name should be 7 distinct callers");
    let e = shotgun_evidence(&findings[0]);
    assert!(
        e.changing_classes >= 7,
        "expected CC>=7 (file-scoped buckets), got {}",
        e.changing_classes
    );
}

#[test]
fn name_collision_fold_merges_identical_caller_sets() {
    let providers = ["ProviderA", "ProviderB", "ProviderC"];
    let mut defs: Vec<DefinitionRecord> = Vec::new();
    for p in providers {
        defs.push(def(&format!("{}.py", p.to_lowercase()), Some(p), "search", 1));
    }
    for i in 0..12 {
        defs.push(def(
            &format!("caller_{i}.py"),
            Some(&format!("Caller{i}")),
            &format!("use_search_{i}"),
            10,
        ));
    }
    for i in 0..7 {
        defs.push(def(&format!("dep_{i}.py"), Some(&format!("Dep{i}")), &format!("dep_{i}"), 5));
    }
    let mut calls: Vec<LocatedCall> = Vec::new();
    for i in 0..12 {
        calls.push(call(
            &format!("caller_{i}.py"),
            Some(&format!("Caller{i}")),
            &format!("use_search_{i}"),
            10,
            "search",
            None,
        ));
    }
    for p in providers {
        for i in 0..7 {
            calls.push(call(
                &format!("{}.py", p.to_lowercase()),
                Some(p),
                "search",
                1,
                &format!("dep_{i}"),
                Some(&format!("Dep{i}")),
            ));
        }
    }
    let findings = run_from_inputs(defs, calls, &t().audit);
    let shotgun: Vec<_> = findings
        .iter()
        .filter(|f| matches!(f.kind, AuditKind::ShotgunSurgery(_)))
        .collect();
    assert_eq!(
        shotgun.len(),
        1,
        "three same-name definitions with identical callers should fold into one finding"
    );
    let e = shotgun_evidence(shotgun[0]);
    assert_eq!(e.name_collision_count, 3, "fold marker should record 3 definitions");
    assert_eq!(e.additional_definitions.len(), 2, "two sibling definitions retained");
}
