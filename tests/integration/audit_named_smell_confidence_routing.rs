use std::path::PathBuf;

use pulse::audit::finding::{
    AuditFinding, AuditKind, AuditLocation, ClassIdentityRef, DivergentChangeEvidence, FeatureEnvyEvidence,
    GodClassEvidence, ImportConfidence, ParallelInheritanceEvidence, PatternCategory, RefusedBequestEvidence,
    ShotgunSurgeryEvidence,
};
use pulse::audit::output::{format_findings_filtered, format_findings_json_filtered};
use pulse::config::AuditSuppression;

use crate::audit_common::t;

fn wrap(kind: AuditKind) -> AuditFinding {
    AuditFinding {
        kind,
        representative_snippet: String::new(),
        support: 0,
        file_count: 0,
        idf_score: None,
        action_label: None,
        pattern_category: None,
        locality_entropy: None,
        p_value: None,
        locations: Vec::new(),
    }
}

fn shotgun(c: ImportConfidence) -> AuditFinding {
    wrap(AuditKind::ShotgunSurgery(ShotgunSurgeryEvidence {
        method_file: PathBuf::from("svc.py"),
        method_class: Some("Svc".to_string()),
        method_name: "go".to_string(),
        method_line: 1,
        changing_classes: 5,
        changing_methods: 10,
        fanout: 6,
        confidence: c,
        caller_samples: vec![AuditLocation { file: PathBuf::from("a.py"), line: 1 }],
        name_collision_count: 0,
        additional_definitions: Vec::new(),
    }))
}

fn divergent(c: ImportConfidence) -> AuditFinding {
    wrap(AuditKind::DivergentChange(DivergentChangeEvidence {
        class_file: PathBuf::from("dc.py"),
        class_name: "DC".to_string(),
        changing_classes: 4,
        fanout: 4,
        method_count: 8,
        confidence: c,
    }))
}

fn feature_envy(c: ImportConfidence) -> AuditFinding {
    wrap(AuditKind::FeatureEnvy(FeatureEnvyEvidence {
        method_file: PathBuf::from("fe.py"),
        method_class: Some("FE".to_string()),
        method_name: "envy".to_string(),
        method_line: 1,
        atfd: 6,
        foreign_call_count: 8,
        intra_call_count: 1,
        envied_class: Some("Other".to_string()),
        confidence: c,
    }))
}

fn god_class(c: ImportConfidence) -> AuditFinding {
    wrap(AuditKind::GodClass(GodClassEvidence {
        class_file: PathBuf::from("gc.py"),
        class_name: "GC".to_string(),
        wmc: 50,
        tcc: 0.1,
        atfd: 6,
        method_count: 25,
        confidence: c,
    }))
}

fn parallel_inheritance(c: ImportConfidence) -> AuditFinding {
    wrap(AuditKind::ParallelInheritance(ParallelInheritanceEvidence {
        root_a: ClassIdentityRef { file: PathBuf::from("a.py"), name: "A".to_string() },
        root_b: ClassIdentityRef { file: PathBuf::from("b.py"), name: "B".to_string() },
        matched_descendants: vec![("A1".into(), "B1".into())],
        confidence: c,
    }))
}

fn refused_bequest(c: ImportConfidence) -> AuditFinding {
    wrap(AuditKind::RefusedBequest(RefusedBequestEvidence {
        subclass_file: PathBuf::from("sub.py"),
        subclass_name: "Sub".to_string(),
        parent_file: PathBuf::from("parent.py"),
        parent_name: "Parent".to_string(),
        override_count: 0,
        parent_method_count: 5,
        override_ratio: 0.0,
        confidence: c,
    }))
}

fn render_human(findings: Vec<AuditFinding>) -> String {
    let suppression = AuditSuppression::new();
    format_findings_filtered(&findings, None, &t().audit, false, &suppression)
}

fn render_json(findings: Vec<AuditFinding>) -> serde_json::Value {
    let suppression = AuditSuppression::new();
    let s = format_findings_json_filtered(&findings, None, false, &suppression);
    serde_json::from_str::<serde_json::Value>(&s).expect("valid JSON")
}

fn label_for(c: ImportConfidence) -> &'static str {
    match c {
        ImportConfidence::High => "high",
        ImportConfidence::Medium => "medium",
        ImportConfidence::Low => "low",
        ImportConfidence::BestEffort => "best-effort",
        ImportConfidence::NaAbstraction => "na",
    }
}

fn check_routing(name: &str, build: impl Fn(ImportConfidence) -> AuditFinding) {
    for (conf, label) in
        [(ImportConfidence::High, "high"), (ImportConfidence::Medium, "medium"), (ImportConfidence::Low, "low")]
    {
        let human = render_human(vec![build(conf)]);
        assert!(
            human.contains(label),
            "{name}/{conf:?}: human output should contain confidence label `{label}`, got:\n{human}"
        );

        let v = render_json(vec![build(conf)]);
        let by_conf = &v["summary"]["by_confidence"];
        assert_eq!(
            by_conf[label].as_u64(),
            Some(1),
            "{name}/{conf:?}: JSON summary.by_confidence.{label} should be 1, got: {by_conf}"
        );

        let findings_arr = v["findings"].as_array().expect("findings array");
        assert_eq!(findings_arr.len(), 1);
        assert_eq!(
            findings_arr[0]["confidence"].as_str(),
            Some(label),
            "{name}/{conf:?}: per-finding confidence should be `{label}`"
        );
    }
}

#[test]
fn shotgun_surgery_confidence_routing() {
    check_routing("ShotgunSurgery", shotgun);
}

#[test]
fn divergent_change_confidence_routing() {
    check_routing("DivergentChange", divergent);
}

#[test]
fn feature_envy_confidence_routing() {
    check_routing("FeatureEnvy", feature_envy);
}

#[test]
fn god_class_confidence_routing() {
    check_routing("GodClass", god_class);
}

#[test]
fn parallel_inheritance_confidence_routing() {
    check_routing("ParallelInheritance", parallel_inheritance);
}

#[test]
fn refused_bequest_confidence_routing() {
    check_routing("RefusedBequest", refused_bequest);
}

fn pattern_finding(support: u32) -> AuditFinding {
    AuditFinding {
        kind: AuditKind::UncategorizedPattern { fingerprint: 1 },
        representative_snippet: String::from("x"),
        support,
        file_count: 1,
        idf_score: None,
        action_label: None,
        pattern_category: Some(PatternCategory::PrimitiveObsession),
        locality_entropy: None,
        p_value: None,
        locations: vec![AuditLocation { file: PathBuf::from("p.py"), line: 1 }],
    }
}

fn confidence_label_for_support(support: u32) -> String {
    let v = render_json(vec![pattern_finding(support)]);
    let arr = v["findings"].as_array().expect("findings");
    arr[0]["confidence"].as_str().expect("confidence str").to_string()
}

#[test]
fn pattern_support_50_routes_high() {
    assert_eq!(confidence_label_for_support(50), "high");
    assert_eq!(confidence_label_for_support(99), "high");
}

#[test]
fn pattern_support_49_routes_medium() {
    assert_eq!(confidence_label_for_support(49), "medium");
}

#[test]
fn pattern_support_10_routes_medium() {
    assert_eq!(confidence_label_for_support(10), "medium");
    assert_eq!(confidence_label_for_support(25), "medium");
}

#[test]
fn pattern_support_9_routes_low() {
    assert_eq!(confidence_label_for_support(9), "low");
    assert_eq!(confidence_label_for_support(1), "low");
}

#[test]
fn header_aggregates_confidence_across_variants() {
    let findings = vec![
        shotgun(ImportConfidence::High),
        feature_envy(ImportConfidence::High),
        god_class(ImportConfidence::Medium),
        refused_bequest(ImportConfidence::Low),
        parallel_inheritance(ImportConfidence::Medium),
        divergent(ImportConfidence::Low),
    ];
    let v = render_json(findings);
    let by_conf = &v["summary"]["by_confidence"];
    assert_eq!(by_conf["high"].as_u64(), Some(2));
    assert_eq!(by_conf["medium"].as_u64(), Some(2));
    assert_eq!(by_conf["low"].as_u64(), Some(2));
}

#[test]
fn high_confidence_does_not_silently_become_low() {
    for (name, build) in [
        ("ShotgunSurgery", shotgun as fn(_) -> _),
        ("DivergentChange", divergent),
        ("FeatureEnvy", feature_envy),
        ("GodClass", god_class),
        ("ParallelInheritance", parallel_inheritance),
        ("RefusedBequest", refused_bequest),
    ] {
        let v = render_json(vec![build(ImportConfidence::High)]);
        let by_conf = &v["summary"]["by_confidence"];
        assert_eq!(
            by_conf["low"].as_u64().unwrap_or(0),
            0,
            "{name} with High should produce 0 in low bucket, got: {by_conf}"
        );
        assert_eq!(
            by_conf["high"].as_u64(),
            Some(1),
            "{name} with High should produce 1 in high bucket, got: {by_conf}"
        );
    }
    let _ = label_for(ImportConfidence::High);
}
