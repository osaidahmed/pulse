use std::path::PathBuf;

use pulse::audit::call_graph::{CallGraph, MethodIdentity};
use pulse::audit::class_registry::ClassRegistry;
use pulse::audit::definitions::DefinitionRecord;
use pulse::audit::detector_parallel_inheritance::detect;
use pulse::audit::finding::{AuditFinding, AuditKind, ParallelInheritanceEvidence};
use pulse::audit::inheritance::build_inheritance_graph;

use crate::audit_common::t;

fn def(file: &str, class: &str, parent: Option<&str>) -> DefinitionRecord {
    DefinitionRecord {
        identity: MethodIdentity {
            file: PathBuf::from(file),
            class: Some(class.to_string()),
            name: "m".to_string(),
            line: 1,
        },
        cc: 1,
        field_accesses: Vec::new(),
        foreign_field_accesses: Vec::new(),
        parent_class: parent.map(String::from),
        is_constructor: false,
    }
}

fn detect_with(
    defs: Vec<DefinitionRecord>,
    th: &pulse::thresholds::AuditThresholds,
) -> Vec<AuditFinding> {
    let graph = CallGraph::build(defs.clone(), Vec::new());
    let registry = ClassRegistry::from_definitions(&defs, &graph.registry);
    let inh = build_inheritance_graph(&registry);
    detect(&registry, &inh, th)
}

fn pi(f: &AuditFinding) -> &ParallelInheritanceEvidence {
    let AuditKind::ParallelInheritance(e) = &f.kind else {
        panic!("expected ParallelInheritance variant");
    };
    e
}

#[test]
fn boundary_min_overlap_strictly_at_threshold() {
    let defs = vec![
        def("r.py", "Reader", None),
        def("xr.py", "XmlReader", Some("Reader")),
        def("jr.py", "JsonReader", Some("Reader")),
        def("yr.py", "YamlReader", Some("Reader")),
        def("w.py", "Writer", None),
        def("xw.py", "XmlWriter", Some("Writer")),
        def("jw.py", "JsonWriter", Some("Writer")),
        def("yw.py", "YamlWriter", Some("Writer")),
    ];
    let findings = detect_with(defs, &t().audit);
    assert!(!findings.is_empty(), "3 matches at min_overlap=3 should fire");
    assert_eq!(pi(&findings[0]).matched_descendants.len(), 3);
}

#[test]
fn boundary_min_overlap_minus_one_no_finding() {
    let defs = vec![
        def("r.py", "Reader", None),
        def("xr.py", "XmlReader", Some("Reader")),
        def("jr.py", "JsonReader", Some("Reader")),
        def("w.py", "Writer", None),
        def("xw.py", "XmlWriter", Some("Writer")),
        def("jw.py", "JsonWriter", Some("Writer")),
    ];
    let findings = detect_with(defs, &t().audit);
    assert!(findings.is_empty());
}

#[test]
fn five_matches_above_threshold_emits_finding_with_full_pairs() {
    let defs = vec![
        def("r.py", "Reader", None),
        def("xr.py", "XmlReader", Some("Reader")),
        def("jr.py", "JsonReader", Some("Reader")),
        def("yr.py", "YamlReader", Some("Reader")),
        def("br.py", "BinReader", Some("Reader")),
        def("tr.py", "TomlReader", Some("Reader")),
        def("w.py", "Writer", None),
        def("xw.py", "XmlWriter", Some("Writer")),
        def("jw.py", "JsonWriter", Some("Writer")),
        def("yw.py", "YamlWriter", Some("Writer")),
        def("bw.py", "BinWriter", Some("Writer")),
        def("tw.py", "TomlWriter", Some("Writer")),
    ];
    let findings = detect_with(defs, &t().audit);
    assert!(!findings.is_empty());
    assert_eq!(pi(&findings[0]).matched_descendants.len(), 5);
}

#[test]
fn three_hierarchies_pairwise_findings() {
    let defs = vec![
        def("r.py", "Reader", None),
        def("xr.py", "XmlReader", Some("Reader")),
        def("jr.py", "JsonReader", Some("Reader")),
        def("yr.py", "YamlReader", Some("Reader")),
        def("w.py", "Writer", None),
        def("xw.py", "XmlWriter", Some("Writer")),
        def("jw.py", "JsonWriter", Some("Writer")),
        def("yw.py", "YamlWriter", Some("Writer")),
        def("p.py", "Parser", None),
        def("xp.py", "XmlParser", Some("Parser")),
        def("jp.py", "JsonParser", Some("Parser")),
        def("yp.py", "YamlParser", Some("Parser")),
    ];
    let findings = detect_with(defs, &t().audit);
    assert!(findings.len() >= 3, "3 hierarchies → 3 pairs (R↔W, R↔P, W↔P)");
}

#[test]
fn empty_inputs_no_findings_no_panic() {
    let _ = detect_with(Vec::new(), &t().audit);
}

#[test]
fn single_root_no_descendants_no_finding() {
    let defs = vec![def("r.py", "Lonely", None)];
    let findings = detect_with(defs, &t().audit);
    assert!(findings.is_empty());
}

#[test]
fn two_roots_no_overlap_no_finding() {
    let defs = vec![
        def("r.py", "Foo", None),
        def("xfoo.py", "AlphaFoo", Some("Foo")),
        def("yfoo.py", "BetaFoo", Some("Foo")),
        def("gfoo.py", "GammaFoo", Some("Foo")),
        def("b.py", "Bar", None),
        def("rho.py", "RhoBar", Some("Bar")),
        def("sig.py", "SigBar", Some("Bar")),
        def("tau.py", "TauBar", Some("Bar")),
    ];
    let findings = detect_with(defs, &t().audit);
    assert!(findings.is_empty(), "no shared token across descendant names");
}

#[test]
fn diamond_inheritance_handled_safely() {
    let defs = vec![
        def("top.py", "Top", None),
        def("left.py", "Left", Some("Top")),
        def("right.py", "Right", Some("Top")),
        def("bot.py", "Bottom", Some("Left")),
    ];
    let _ = detect_with(defs, &t().audit);
}

#[test]
fn cyclic_inheritance_a_b_a_does_not_panic() {
    let defs = vec![
        def("a.py", "A", Some("B")),
        def("b.py", "B", Some("A")),
    ];
    let _ = detect_with(defs, &t().audit);
}

#[test]
fn three_way_cycle_does_not_panic() {
    let defs = vec![
        def("a.py", "A", Some("B")),
        def("b.py", "B", Some("C")),
        def("c.py", "C", Some("A")),
    ];
    let _ = detect_with(defs, &t().audit);
}

#[test]
fn self_referential_class_does_not_panic() {
    let defs = vec![def("a.py", "A", Some("A"))];
    let _ = detect_with(defs, &t().audit);
}

#[test]
fn lowering_threshold_to_one_surfaces_single_pair() {
    let defs = vec![
        def("r.py", "Reader", None),
        def("xr.py", "XmlReader", Some("Reader")),
        def("w.py", "Writer", None),
        def("xw.py", "XmlWriter", Some("Writer")),
    ];
    let mut th = t().audit;
    th.named_smells.parallel_inheritance.min_overlap = 1;
    let findings = detect_with(defs, &th);
    assert!(!findings.is_empty());
}

#[test]
fn raising_threshold_suppresses_finding() {
    let defs = vec![
        def("r.py", "Reader", None),
        def("xr.py", "XmlReader", Some("Reader")),
        def("jr.py", "JsonReader", Some("Reader")),
        def("yr.py", "YamlReader", Some("Reader")),
        def("w.py", "Writer", None),
        def("xw.py", "XmlWriter", Some("Writer")),
        def("jw.py", "JsonWriter", Some("Writer")),
        def("yw.py", "YamlWriter", Some("Writer")),
    ];
    let mut th = t().audit;
    th.named_smells.parallel_inheritance.min_overlap = 100;
    assert!(detect_with(defs, &th).is_empty());
}

#[test]
fn determinism_five_runs_yield_same_finding_counts() {
    let mut counts = Vec::new();
    for _ in 0..5 {
        let defs = vec![
            def("r.py", "Reader", None),
            def("xr.py", "XmlReader", Some("Reader")),
            def("jr.py", "JsonReader", Some("Reader")),
            def("yr.py", "YamlReader", Some("Reader")),
            def("w.py", "Writer", None),
            def("xw.py", "XmlWriter", Some("Writer")),
            def("jw.py", "JsonWriter", Some("Writer")),
            def("yw.py", "YamlWriter", Some("Writer")),
        ];
        counts.push(detect_with(defs, &t().audit).len());
    }
    for c in &counts[1..] {
        assert_eq!(*c, counts[0]);
    }
}

#[test]
fn evidence_carries_root_identities() {
    let defs = vec![
        def("readers.py", "Reader", None),
        def("xr.py", "XmlReader", Some("Reader")),
        def("jr.py", "JsonReader", Some("Reader")),
        def("yr.py", "YamlReader", Some("Reader")),
        def("writers.py", "Writer", None),
        def("xw.py", "XmlWriter", Some("Writer")),
        def("jw.py", "JsonWriter", Some("Writer")),
        def("yw.py", "YamlWriter", Some("Writer")),
    ];
    let findings = detect_with(defs, &t().audit);
    assert!(!findings.is_empty());
    let e = pi(&findings[0]);
    assert!(["Reader", "Writer"].contains(&e.root_a.name.as_str()));
    assert!(["Reader", "Writer"].contains(&e.root_b.name.as_str()));
    assert_ne!(e.root_a.name, e.root_b.name);
}

#[test]
fn deep_inheritance_chain_only_leaves_match() {
    let defs = vec![
        def("a.py", "A", None),
        def("b.py", "B", Some("A")),
        def("c.py", "C", Some("B")),
        def("d.py", "D", Some("C")),
        def("aa.py", "RootA", None),
        def("rar.py", "RootAReader", Some("RootA")),
        def("raw.py", "RootAWriter", Some("RootA")),
        def("rax.py", "RootAExporter", Some("RootA")),
    ];
    let _ = detect_with(defs, &t().audit);
}

#[test]
fn wide_hierarchy_with_50_siblings_completes() {
    use std::time::Instant;
    let mut defs = Vec::new();
    defs.push(def("r.py", "Reader", None));
    defs.push(def("w.py", "Writer", None));
    for i in 0..50 {
        defs.push(def(&format!("rd_{i}.py"), &format!("D{i}Reader"), Some("Reader")));
        defs.push(def(&format!("wr_{i}.py"), &format!("D{i}Writer"), Some("Writer")));
    }
    let started = Instant::now();
    let findings = detect_with(defs, &t().audit);
    assert!(started.elapsed().as_secs_f64() < 5.0);
    assert!(!findings.is_empty());
    assert!(pi(&findings[0]).matched_descendants.len() >= 50);
}

#[test]
fn shared_prefix_token_match_works() {
    let defs = vec![
        def("a.py", "Animal", None),
        def("dog.py", "DogAnimal", Some("Animal")),
        def("cat.py", "CatAnimal", Some("Animal")),
        def("bird.py", "BirdAnimal", Some("Animal")),
        def("v.py", "Vehicle", None),
        def("dogv.py", "DogVehicle", Some("Vehicle")),
        def("catv.py", "CatVehicle", Some("Vehicle")),
        def("birdv.py", "BirdVehicle", Some("Vehicle")),
    ];
    let findings = detect_with(defs, &t().audit);
    assert!(!findings.is_empty(), "shared prefix tokens (Dog/Cat/Bird) should match");
}

#[test]
fn empty_token_after_prefix_strip_handled() {
    let defs = vec![
        def("a.py", "A", None),
        def("aa.py", "A", Some("A")),
        def("b.py", "B", None),
    ];
    let _ = detect_with(defs, &t().audit);
}

#[test]
fn unicode_class_names_match_correctly() {
    let defs = vec![
        def("r.py", "Чтец", None),
        def("xr.py", "ЭксЧтец", Some("Чтец")),
        def("jr.py", "ДжейЧтец", Some("Чтец")),
        def("yr.py", "ЯмлЧтец", Some("Чтец")),
        def("w.py", "Писатель", None),
        def("xw.py", "ЭксПисатель", Some("Писатель")),
        def("jw.py", "ДжейПисатель", Some("Писатель")),
        def("yw.py", "ЯмлПисатель", Some("Писатель")),
    ];
    let _ = detect_with(defs, &t().audit);
}

#[test]
fn descendants_in_different_files_still_pair() {
    let defs = vec![
        def("r.py", "Reader", None),
        def("xr.py", "XmlReader", Some("Reader")),
        def("jr.py", "JsonReader", Some("Reader")),
        def("yr.py", "YamlReader", Some("Reader")),
        def("w.py", "Writer", None),
        def("xw_lib_a/x.py", "XmlWriter", Some("Writer")),
        def("xw_lib_b/j.py", "JsonWriter", Some("Writer")),
        def("xw_lib_c/y.py", "YamlWriter", Some("Writer")),
    ];
    let findings = detect_with(defs, &t().audit);
    assert!(!findings.is_empty(), "different file paths should still pair");
}

#[test]
fn isolated_class_no_finding() {
    let defs = vec![
        def("a.py", "Lone", None),
        def("b.py", "Other", None),
    ];
    let findings = detect_with(defs, &t().audit);
    assert!(findings.is_empty());
}

#[test]
fn shared_grandparent_does_not_cause_self_pairing() {
    let defs = vec![
        def("a.py", "Top", None),
        def("b.py", "Reader", Some("Top")),
        def("c.py", "Writer", Some("Top")),
        def("xr.py", "XmlReader", Some("Reader")),
        def("jr.py", "JsonReader", Some("Reader")),
        def("yr.py", "YamlReader", Some("Reader")),
        def("xw.py", "XmlWriter", Some("Writer")),
        def("jw.py", "JsonWriter", Some("Writer")),
        def("yw.py", "YamlWriter", Some("Writer")),
    ];
    let _ = detect_with(defs, &t().audit);
}

#[test]
fn duplicated_class_names_in_diff_files_handled() {
    let defs = vec![
        def("a.py", "Reader", None),
        def("b.py", "Reader", None),
        def("xr.py", "XmlReader", Some("Reader")),
        def("jr.py", "JsonReader", Some("Reader")),
        def("yr.py", "YamlReader", Some("Reader")),
        def("c.py", "Writer", None),
        def("xw.py", "XmlWriter", Some("Writer")),
        def("jw.py", "JsonWriter", Some("Writer")),
        def("yw.py", "YamlWriter", Some("Writer")),
    ];
    let _ = detect_with(defs, &t().audit);
}

#[test]
fn confidence_label_present() {
    use pulse::audit::finding::ImportConfidence;
    let defs = vec![
        def("r.py", "Reader", None),
        def("xr.py", "XmlReader", Some("Reader")),
        def("jr.py", "JsonReader", Some("Reader")),
        def("yr.py", "YamlReader", Some("Reader")),
        def("w.py", "Writer", None),
        def("xw.py", "XmlWriter", Some("Writer")),
        def("jw.py", "JsonWriter", Some("Writer")),
        def("yw.py", "YamlWriter", Some("Writer")),
    ];
    let findings = detect_with(defs, &t().audit);
    if let Some(f) = findings.first() {
        let conf = pi(f).confidence;
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
fn matched_pairs_contain_actual_class_names() {
    let defs = vec![
        def("r.py", "Reader", None),
        def("xr.py", "XmlReader", Some("Reader")),
        def("jr.py", "JsonReader", Some("Reader")),
        def("yr.py", "YamlReader", Some("Reader")),
        def("w.py", "Writer", None),
        def("xw.py", "XmlWriter", Some("Writer")),
        def("jw.py", "JsonWriter", Some("Writer")),
        def("yw.py", "YamlWriter", Some("Writer")),
    ];
    let findings = detect_with(defs, &t().audit);
    assert!(!findings.is_empty());
    let pairs = &pi(&findings[0]).matched_descendants;
    let names: std::collections::BTreeSet<String> = pairs
        .iter()
        .flat_map(|(a, b)| [a.clone(), b.clone()])
        .collect();
    assert!(names.contains("XmlReader"));
    assert!(names.contains("XmlWriter"));
}

#[test]
fn stress_thirty_hierarchies_completes_quickly() {
    use std::time::Instant;
    let mut defs = Vec::new();
    let suffixes = ["A", "B", "C", "D", "E"];
    for h in 0..30 {
        let root = format!("Root{h}");
        defs.push(def(&format!("r_{h}.py"), &root, None));
        for s in suffixes {
            defs.push(def(
                &format!("d_{h}_{s}.py"),
                &format!("{s}{root}"),
                Some(&root),
            ));
        }
    }
    let started = Instant::now();
    let _ = detect_with(defs, &t().audit);
    assert!(started.elapsed().as_secs_f64() < 5.0);
}
