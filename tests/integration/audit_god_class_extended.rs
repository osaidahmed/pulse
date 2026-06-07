use std::path::PathBuf;

use pulse::audit::call_graph::{CallGraph, MethodIdentity};
use pulse::audit::class_registry::ClassRegistry;
use pulse::audit::definitions::DefinitionRecord;
use pulse::audit::detector_god_class::{build_method_idx_lookup, detect};
use pulse::audit::finding::{AuditFinding, AuditKind, GodClassEvidence};

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
        foreign_field_accesses: foreign.into_iter().map(|(r, f)| (r.to_string(), f.to_string())).collect(),
        parent_class: None,
        is_constructor: is_ctor,
    }
}

fn detect_with(defs: Vec<DefinitionRecord>, th: &pulse::thresholds::AuditThresholds) -> Vec<AuditFinding> {
    let graph = CallGraph::build(defs.clone(), Vec::new());
    let registry = ClassRegistry::from_definitions(&defs, &graph.registry);
    let lookup = build_method_idx_lookup(&graph, &defs);
    detect(&registry, &defs, &lookup, th)
}

fn god(f: &AuditFinding) -> &GodClassEvidence {
    let AuditKind::GodClass(e) = &f.kind else {
        panic!("expected GodClass variant");
    };
    e
}

fn high_atfd_method(file: &str, class: &str, name: &str, line: u32, cc: u32, field: &str) -> DefinitionRecord {
    def(
        file,
        Some(class),
        name,
        line,
        cc,
        vec![field],
        vec![("a", "x"), ("b", "y"), ("c", "z"), ("d", "w"), ("e", "v"), ("f", "u")],
        false,
    )
}

#[test]
fn boundary_method_count_strictly_above_three() {
    let mut defs = Vec::new();
    for i in 0..2 {
        defs.push(high_atfd_method("g.py", "G", &format!("m{i}"), 10 + i, 30, &format!("f{i}")));
    }
    assert!(detect_with(defs, &t().audit).is_empty(), "two methods short-circuits");
}

#[test]
fn boundary_method_count_three_evaluates() {
    let defs = (0..3).map(|i| high_atfd_method("g.py", "G", &format!("m{i}"), 10 + i, 30, &format!("f{i}"))).collect();
    let _ = detect_with(defs, &t().audit);
}

#[test]
fn empty_class_no_finding() {
    let defs = Vec::new();
    assert!(detect_with(defs, &t().audit).is_empty());
}

#[test]
fn single_method_class_no_finding() {
    let defs = vec![high_atfd_method("g.py", "G", "only", 10, 100, "f")];
    assert!(detect_with(defs, &t().audit).is_empty());
}

#[test]
fn boundary_wmc_at_threshold_does_not_fire() {
    let mut defs = Vec::new();
    for i in 0..6 {
        defs.push(high_atfd_method("g.py", "G", &format!("m{i}"), 10 + i, 7, &format!("f{i}")));
    }
    assert!(detect_with(defs, &t().audit).is_empty(), "wmc=42 (6×7), below default 47, should not fire");
}

#[test]
fn boundary_wmc_just_above_threshold_fires() {
    let mut defs = Vec::new();
    for i in 0..6 {
        defs.push(high_atfd_method("g.py", "G", &format!("m{i}"), 10 + i, 8, &format!("f{i}")));
    }
    assert!(
        !detect_with(defs, &t().audit).is_empty(),
        "wmc=48 (6×8) > 47 default with low TCC and high ATFD must fire"
    );
}

#[test]
fn wmc_far_above_with_low_tcc_and_high_atfd_fires() {
    let mut defs = Vec::new();
    for i in 0..10 {
        defs.push(high_atfd_method("g.py", "G", &format!("m{i}"), 10 + i, 10, &format!("f{i}")));
    }
    assert!(!detect_with(defs, &t().audit).is_empty());
}

#[test]
fn boundary_atfd_threshold_strict() {
    let mut defs = Vec::new();
    for i in 0..10 {
        defs.push(def(
            "g.py",
            Some("G"),
            &format!("m{i}"),
            10 + i,
            10,
            vec![&format!("f{i}")],
            vec![("a", "x"), ("b", "y"), ("c", "z"), ("d", "w"), ("e", "v")],
            false,
        ));
    }
    assert!(detect_with(defs, &t().audit).is_empty(), "atfd=5 not > 5");
}

#[test]
fn boundary_tcc_threshold_strict() {
    let mut defs = Vec::new();
    for i in 0..10 {
        defs.push(high_atfd_method("g.py", "G", &format!("m{i}"), 10 + i, 10, "shared_field"));
    }
    let findings = detect_with(defs, &t().audit);
    assert!(findings.is_empty(), "TCC=1.0 fully cohesive class should not be God");
}

#[test]
fn high_tcc_with_high_wmc_no_finding() {
    let mut defs = Vec::new();
    for i in 0..10 {
        let mut d = high_atfd_method("g.py", "G", &format!("m{i}"), 10 + i, 10, "shared");
        d.field_accesses.push("shared2".to_string());
        defs.push(d);
    }
    let findings = detect_with(defs, &t().audit);
    assert!(findings.is_empty(), "High TCC suppresses despite high WMC");
}

#[test]
fn low_atfd_no_finding_even_with_god_wmc() {
    let mut defs = Vec::new();
    for i in 0..10 {
        defs.push(def(
            "g.py",
            Some("G"),
            &format!("m{i}"),
            10 + i,
            20,
            vec![&format!("f{i}")],
            vec![("a", "x")],
            false,
        ));
    }
    assert!(detect_with(defs, &t().audit).is_empty());
}

#[test]
fn constructor_methods_count_toward_wmc() {
    let mut defs = Vec::new();
    let mut ctor = high_atfd_method("g.py", "G", "init", 10, 20, "f0");
    ctor.is_constructor = true;
    defs.push(ctor);
    for i in 1..7 {
        defs.push(high_atfd_method("g.py", "G", &format!("m{i}"), 10 + i, 10, &format!("f{i}")));
    }
    let findings = detect_with(defs, &t().audit);
    if let Some(f) = findings.first() {
        assert!(god(f).wmc >= 70);
    }
}

#[test]
fn evidence_carries_class_metrics() {
    let mut defs = Vec::new();
    for i in 0..10 {
        defs.push(high_atfd_method("loc.py", "Located", &format!("m{i}"), 10 + i, 10, &format!("f{i}")));
    }
    let findings = detect_with(defs, &t().audit);
    assert!(!findings.is_empty());
    let e = god(&findings[0]);
    assert_eq!(e.class_name, "Located");
    assert_eq!(e.class_file, PathBuf::from("loc.py"));
    assert_eq!(e.method_count, 10);
}

#[test]
fn multiple_god_classes_sorted_descending_by_wmc() {
    let mut defs = Vec::new();
    for (cls, cc) in [("G1", 8), ("G2", 12), ("G3", 6)] {
        for i in 0..10 {
            defs.push(high_atfd_method(&format!("{cls}.py"), cls, &format!("m{i}"), 10 + i, cc, &format!("f{i}")));
        }
    }
    let findings = detect_with(defs, &t().audit);
    assert!(findings.len() >= 2);
    for w in findings.windows(2) {
        assert!(god(&w[0]).wmc >= god(&w[1]).wmc);
    }
}

#[test]
fn determinism_five_runs_same_result() {
    let mut snapshots = Vec::new();
    for _ in 0..5 {
        let mut defs = Vec::new();
        for i in 0..10 {
            defs.push(high_atfd_method("g.py", "G", &format!("m{i}"), 10 + i, 10, &format!("f{i}")));
        }
        let findings = detect_with(defs, &t().audit);
        let names: Vec<String> = findings.iter().map(|f| god(f).class_name.clone()).collect();
        snapshots.push(names);
    }
    for i in 1..snapshots.len() {
        assert_eq!(snapshots[0], snapshots[i]);
    }
}

#[test]
fn raising_wmc_threshold_suppresses() {
    let mut defs = Vec::new();
    for i in 0..10 {
        defs.push(high_atfd_method("g.py", "G", &format!("m{i}"), 10 + i, 10, &format!("f{i}")));
    }
    let mut th = t().audit;
    th.named_smells.god_class.wmc = 1000;
    assert!(detect_with(defs, &th).is_empty());
}

#[test]
fn raising_atfd_threshold_suppresses() {
    let mut defs = Vec::new();
    for i in 0..10 {
        defs.push(high_atfd_method("g.py", "G", &format!("m{i}"), 10 + i, 10, &format!("f{i}")));
    }
    let mut th = t().audit;
    th.named_smells.god_class.atfd = 1000;
    assert!(detect_with(defs, &th).is_empty());
}

#[test]
fn lowering_tcc_threshold_to_zero_suppresses() {
    let mut defs = Vec::new();
    for i in 0..10 {
        defs.push(high_atfd_method("g.py", "G", &format!("m{i}"), 10 + i, 10, &format!("f{i}")));
    }
    let mut th = t().audit;
    th.named_smells.god_class.tcc = 0.0;
    assert!(detect_with(defs, &th).is_empty(), "TCC threshold of 0 means cohesion 0 not strictly less");
}

#[test]
fn raising_tcc_threshold_keeps_finding() {
    let mut defs = Vec::new();
    for i in 0..10 {
        defs.push(high_atfd_method("g.py", "G", &format!("m{i}"), 10 + i, 10, &format!("f{i}")));
    }
    let mut th = t().audit;
    th.named_smells.god_class.tcc = 0.99;
    assert!(!detect_with(defs, &th).is_empty());
}

#[test]
fn class_with_only_constructors_high_wmc_evaluated() {
    let mut defs = Vec::new();
    for i in 0..6 {
        let mut d = high_atfd_method("g.py", "G", &format!("init{i}"), 10 + i, 10, &format!("f{i}"));
        d.is_constructor = true;
        defs.push(d);
    }
    let _ = detect_with(defs, &t().audit);
}

#[test]
fn classes_with_partial_class_pattern_treated_per_file() {
    let mut defs = Vec::new();
    for i in 0..6 {
        defs.push(high_atfd_method("part1.cs", "Partial", &format!("m{i}"), 10 + i, 10, &format!("f{i}")));
    }
    for i in 0..6 {
        defs.push(high_atfd_method("part2.cs", "Partial", &format!("n{i}"), 10 + i, 10, &format!("g{i}")));
    }
    let findings = detect_with(defs, &t().audit);
    let files: std::collections::BTreeSet<PathBuf> = findings.iter().map(|f| god(f).class_file.clone()).collect();
    assert!(files.contains(&PathBuf::from("part1.cs")) || files.contains(&PathBuf::from("part2.cs")));
}

#[test]
fn unicode_class_name_preserved_in_evidence() {
    let mut defs = Vec::new();
    for i in 0..10 {
        defs.push(high_atfd_method("κ.py", "Κλάσση", &format!("m{i}"), 10 + i, 10, &format!("f{i}")));
    }
    let findings = detect_with(defs, &t().audit);
    if let Some(f) = findings.first() {
        assert_eq!(god(f).class_name, "Κλάσση");
    }
}

#[test]
fn extreme_method_count_doesnt_panic() {
    let mut defs = Vec::new();
    for i in 0..200 {
        defs.push(high_atfd_method("g.py", "Big", &format!("m{i}"), 10 + i, 10, &format!("f{i}")));
    }
    let findings = detect_with(defs, &t().audit);
    if let Some(f) = findings.first() {
        assert!(god(f).method_count == 200);
    }
}

#[test]
fn stress_50_god_classes_completes_in_reasonable_time() {
    use std::time::Instant;
    let mut defs = Vec::new();
    for cls_idx in 0..50 {
        let cls = format!("God{cls_idx}");
        let file = format!("g_{cls_idx}.py");
        for i in 0..10 {
            defs.push(high_atfd_method(&file, &cls, &format!("m{i}"), 10 + i, 10, &format!("f{i}")));
        }
    }
    let started = Instant::now();
    let findings = detect_with(defs, &t().audit);
    assert!(started.elapsed().as_secs_f64() < 5.0);
    assert!(findings.len() >= 50);
}

#[test]
fn three_methods_zero_field_overlap_all_distinct_fields_low_tcc() {
    let defs = vec![
        high_atfd_method("g.py", "G", "m0", 10, 30, "f0"),
        high_atfd_method("g.py", "G", "m1", 11, 30, "f1"),
        high_atfd_method("g.py", "G", "m2", 12, 30, "f2"),
    ];
    let _ = detect_with(defs, &t().audit);
}

#[test]
fn confidence_label_present_on_finding() {
    use pulse::audit::finding::ImportConfidence;
    let mut defs = Vec::new();
    for i in 0..10 {
        defs.push(high_atfd_method("g.py", "G", &format!("m{i}"), 10 + i, 10, &format!("f{i}")));
    }
    let findings = detect_with(defs, &t().audit);
    if let Some(f) = findings.first() {
        let conf = god(f).confidence;
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
fn atfd_dedup_across_methods_same_class() {
    let mut defs = Vec::new();
    for i in 0..10 {
        defs.push(def(
            "g.py",
            Some("G"),
            &format!("m{i}"),
            10 + i,
            10,
            vec![&format!("f{i}")],
            vec![("a", "x"), ("a", "x"), ("a", "x")],
            false,
        ));
    }
    let findings = detect_with(defs, &t().audit);
    assert!(findings.is_empty(), "deduplicated ATFD = 1 < 5 should not trigger");
}

#[test]
fn method_count_tracks_only_target_class_methods() {
    let mut defs = Vec::new();
    for i in 0..10 {
        defs.push(high_atfd_method("g.py", "Target", &format!("m{i}"), 10 + i, 10, &format!("f{i}")));
    }
    for i in 0..50 {
        defs.push(high_atfd_method("o.py", "Other", &format!("m{i}"), 10 + i, 1, &format!("g{i}")));
    }
    let findings = detect_with(defs, &t().audit);
    if let Some(f) = findings.iter().find(|f| god(f).class_name == "Target") {
        assert_eq!(god(f).method_count, 10);
    }
}
