use std::path::PathBuf;

use pulse::audit::call_graph::{CallGraph, MethodIdentity};
use pulse::audit::class_registry::ClassRegistry;
use pulse::audit::definitions::DefinitionRecord;
use pulse::audit::detector_parallel_inheritance::detect;
use pulse::audit::finding::{AuditKind, ParallelInheritanceEvidence};
use pulse::audit::inheritance::build_inheritance_graph;

mod audit_common;
use audit_common::t;

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

fn parallel_evidence(f: &pulse::audit::finding::AuditFinding) -> &ParallelInheritanceEvidence {
    let AuditKind::ParallelInheritance(e) = &f.kind else {
        panic!("expected ParallelInheritance, got {:?}", f.kind);
    };
    e
}

fn detect_with(defs: Vec<DefinitionRecord>, audit_t: &pulse::thresholds::AuditThresholds) -> Vec<pulse::audit::finding::AuditFinding> {
    let graph = CallGraph::build(defs.clone(), Vec::new());
    let registry = ClassRegistry::from_definitions(&defs, &graph.registry);
    let inh = build_inheritance_graph(&registry);
    detect(&registry, &inh, audit_t)
}

#[test]
fn two_hierarchies_with_three_matched_descendants_emits_finding() {
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
    assert!(!findings.is_empty(), "Reader/Writer hierarchies should pair");
    let e = parallel_evidence(&findings[0]);
    assert!(e.matched_descendants.len() >= 3);
}

#[test]
fn single_hierarchy_no_finding() {
    let defs = vec![
        def("r.py", "Reader", None),
        def("xr.py", "XmlReader", Some("Reader")),
        def("jr.py", "JsonReader", Some("Reader")),
        def("yr.py", "YamlReader", Some("Reader")),
    ];
    let findings = detect_with(defs, &t().audit);
    assert!(findings.is_empty(), "single hierarchy has no pair");
}

#[test]
fn two_matched_below_threshold_no_finding() {
    let defs = vec![
        def("r.py", "Reader", None),
        def("xr.py", "XmlReader", Some("Reader")),
        def("jr.py", "JsonReader", Some("Reader")),
        def("w.py", "Writer", None),
        def("xw.py", "XmlWriter", Some("Writer")),
        def("jw.py", "JsonWriter", Some("Writer")),
    ];
    let findings = detect_with(defs, &t().audit);
    assert!(findings.is_empty(), "min_overlap=3, only 2 matched");
}

#[test]
fn lowering_overlap_threshold_surfaces_smaller_pairs() {
    let defs = vec![
        def("r.py", "Reader", None),
        def("xr.py", "XmlReader", Some("Reader")),
        def("jr.py", "JsonReader", Some("Reader")),
        def("w.py", "Writer", None),
        def("xw.py", "XmlWriter", Some("Writer")),
        def("jw.py", "JsonWriter", Some("Writer")),
    ];
    let mut audit_t = t().audit;
    audit_t.named_smells.parallel_inheritance.min_overlap = 2;
    let findings = detect_with(defs, &audit_t);
    assert!(!findings.is_empty());
}

#[test]
fn no_shared_token_no_finding() {
    let defs = vec![
        def("r.py", "Reader", None),
        def("xr.py", "XmlReader", Some("Reader")),
        def("jr.py", "JsonReader", Some("Reader")),
        def("yr.py", "YamlReader", Some("Reader")),
        def("w.py", "Writer", None),
        def("aw.py", "ProtoWriter", Some("Writer")),
        def("bw.py", "BinaryWriter", Some("Writer")),
        def("cw.py", "TextWriter", Some("Writer")),
    ];
    let findings = detect_with(defs, &t().audit);
    assert!(findings.is_empty(), "no shared subclass tokens");
}

#[test]
fn three_way_parallel_emits_pairwise_findings() {
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
    assert!(findings.len() >= 3, "3-way parallel should produce 3 pairwise findings");
}

#[test]
fn empty_registry_no_findings() {
    let findings = detect_with(Vec::new(), &t().audit);
    assert!(findings.is_empty());
}

#[test]
fn single_class_no_findings() {
    let defs = vec![def("a.py", "Foo", None)];
    let findings = detect_with(defs, &t().audit);
    assert!(findings.is_empty());
}

#[test]
fn cyclic_inheritance_does_not_panic() {
    let defs = vec![
        def("a.py", "A", Some("B")),
        def("b.py", "B", Some("A")),
    ];
    let _findings = detect_with(defs, &t().audit);
}

#[test]
fn determinism_two_runs_equal() {
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
    let r1 = detect_with(defs.clone(), &t().audit);
    let r2 = detect_with(defs, &t().audit);
    assert_eq!(r1.len(), r2.len());
}
