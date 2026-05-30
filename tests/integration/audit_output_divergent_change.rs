use std::path::PathBuf;

use pulse::audit::finding::{
    AuditFinding, AuditKind, DivergentChangeEvidence, ImportConfidence,
};
use pulse::audit::output::{format_findings, format_findings_json};

use crate::audit_common::t;

fn finding_with(e: DivergentChangeEvidence) -> AuditFinding {
    AuditFinding {
        kind: AuditKind::DivergentChange(e),
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

fn sample(confidence: ImportConfidence) -> DivergentChangeEvidence {
    DivergentChangeEvidence {
        class_file: PathBuf::from("svc.py"),
        class_name: "Orchestrator".to_string(),
        changing_classes: 9,
        fanout: 11,
        method_count: 8,
        confidence,
    }
}

#[test]
fn human_includes_class_name() {
    let out = format_findings(&[finding_with(sample(ImportConfidence::High))], None, &t().audit);
    assert!(out.contains("divergent change"));
    assert!(out.contains("Orchestrator"));
}

#[test]
fn human_includes_file_path() {
    let out = format_findings(&[finding_with(sample(ImportConfidence::High))], None, &t().audit);
    assert!(out.contains("svc.py"));
}

#[test]
fn human_shows_all_metrics() {
    let out = format_findings(&[finding_with(sample(ImportConfidence::High))], None, &t().audit);
    assert!(out.contains("CC:"));
    assert!(out.contains("fanout:"));
    assert!(out.contains("method count:"));
}

#[test]
fn human_shows_metric_values() {
    let out = format_findings(&[finding_with(sample(ImportConfidence::High))], None, &t().audit);
    assert!(out.contains('9'));
    assert!(out.contains("11"));
    assert!(out.contains('8'));
}

#[test]
fn human_renders_each_confidence_label() {
    for (conf, expected) in [
        (ImportConfidence::High, "high"),
        (ImportConfidence::Medium, "medium"),
        (ImportConfidence::Low, "low"),
        (ImportConfidence::BestEffort, "best-effort"),
        (ImportConfidence::NaAbstraction, "n/a-abstraction"),
    ] {
        let out = format_findings(&[finding_with(sample(conf))], None, &t().audit);
        assert!(out.contains(expected), "{conf:?} should render as {expected} in: {out}");
    }
}

#[test]
fn human_strips_root_prefix() {
    let mut e = sample(ImportConfidence::Medium);
    e.class_file = PathBuf::from("/tmp/proj/svc.py");
    let out = format_findings(
        &[finding_with(e)],
        Some(std::path::Path::new("/tmp/proj")),
        &t().audit,
    );
    assert!(out.contains("svc.py"));
    assert!(!out.contains("/tmp/proj/svc.py"));
}

#[test]
fn human_separates_consecutive_findings_with_blank_line() {
    let f1 = finding_with(sample(ImportConfidence::High));
    let f2 = finding_with(sample(ImportConfidence::Low));
    let out = format_findings(&[f1, f2], None, &t().audit);
    assert!(out.contains("\n\n"));
}

#[test]
fn human_handles_zero_metrics() {
    let mut e = sample(ImportConfidence::High);
    e.changing_classes = 0;
    e.fanout = 0;
    e.method_count = 0;
    let out = format_findings(&[finding_with(e)], None, &t().audit);
    assert!(out.contains("CC:"));
    assert!(out.contains('0'));
}

#[test]
fn human_handles_unicode_class_name() {
    let mut e = sample(ImportConfidence::Medium);
    e.class_name = "Σερβις".to_string();
    let out = format_findings(&[finding_with(e)], None, &t().audit);
    assert!(out.contains("Σερβις"));
}

#[test]
fn json_parses_as_valid_json() {
    let out = format_findings_json(&[finding_with(sample(ImportConfidence::High))], None);
    let _: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
}

#[test]
fn json_kind_is_divergent_change() {
    let out = format_findings_json(&[finding_with(sample(ImportConfidence::High))], None);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v[0]["kind"], "DivergentChange");
}

#[test]
fn json_includes_class_identity() {
    let out = format_findings_json(&[finding_with(sample(ImportConfidence::High))], None);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v[0]["class_name"], "Orchestrator");
    assert_eq!(v[0]["class_file"], "svc.py");
}

#[test]
fn json_includes_all_metrics_with_correct_types() {
    let out = format_findings_json(&[finding_with(sample(ImportConfidence::High))], None);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v[0]["changing_classes"], 9);
    assert_eq!(v[0]["fanout"], 11);
    assert_eq!(v[0]["method_count"], 8);
    assert!(v[0]["changing_classes"].is_number());
}

#[test]
fn json_emits_confidence_label() {
    let out = format_findings_json(&[finding_with(sample(ImportConfidence::Low))], None);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v[0]["confidence"], "low");
}

#[test]
fn json_strips_root_prefix() {
    let mut e = sample(ImportConfidence::Medium);
    e.class_file = PathBuf::from("/tmp/proj/svc.py");
    let out = format_findings_json(
        &[finding_with(e)],
        Some(std::path::Path::new("/tmp/proj")),
    );
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v[0]["class_file"], "svc.py");
}

#[test]
fn json_array_for_multiple_findings() {
    let f1 = finding_with(sample(ImportConfidence::High));
    let f2 = finding_with(sample(ImportConfidence::Medium));
    let out = format_findings_json(&[f1, f2], None);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v.as_array().unwrap().len(), 2);
}
