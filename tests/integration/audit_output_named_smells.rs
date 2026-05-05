use std::path::PathBuf;

use pulse::audit::finding::{
    AuditFinding, AuditKind, AuditLocation, ImportConfidence, ShotgunSurgeryEvidence,
};
use pulse::audit::output::{format_findings, format_findings_json};

use crate::audit_common::t;

fn finding_with_evidence(e: ShotgunSurgeryEvidence) -> AuditFinding {
    AuditFinding {
        kind: AuditKind::ShotgunSurgery(e),
        representative_snippet: String::new(),
        support: 0,
        file_count: 0,
        idf_score: None,
        action_label: None,
        pattern_category: None,
        locations: Vec::new(),
    }
}

fn sample_evidence(callers: usize, confidence: ImportConfidence) -> ShotgunSurgeryEvidence {
    let mut samples = Vec::new();
    for i in 0..callers {
        samples.push(AuditLocation {
            file: PathBuf::from(format!("caller_{i}.py")),
            line: i as u32 + 1,
        });
    }
    ShotgunSurgeryEvidence {
        method_file: PathBuf::from("api.py"),
        method_class: Some("Service".to_string()),
        method_name: "handle".to_string(),
        method_line: 42,
        changing_classes: 6,
        changing_methods: 12,
        fanout: 7,
        confidence,
        caller_samples: samples,
        name_collision_count: 0,
        additional_definitions: Vec::new(),
    }
}

#[test]
fn human_renderer_includes_method_name() {
    let f = finding_with_evidence(sample_evidence(2, ImportConfidence::High));
    let out = format_findings(&[f], None, &t().audit);
    assert!(out.contains("Service.handle"));
}

#[test]
fn human_renderer_includes_class_name_qualified() {
    let f = finding_with_evidence(sample_evidence(2, ImportConfidence::High));
    let out = format_findings(&[f], None, &t().audit);
    assert!(out.contains("Service.handle"));
}

#[test]
fn human_renderer_includes_metrics() {
    let f = finding_with_evidence(sample_evidence(2, ImportConfidence::High));
    let out = format_findings(&[f], None, &t().audit);
    assert!(out.contains("CC:"));
    assert!(out.contains("CM:"));
    assert!(out.contains("fanout:"));
}

#[test]
fn human_renderer_shows_metric_values() {
    let f = finding_with_evidence(sample_evidence(2, ImportConfidence::High));
    let out = format_findings(&[f], None, &t().audit);
    assert!(out.contains("6"));
    assert!(out.contains("12"));
    assert!(out.contains("7"));
}

#[test]
fn human_renderer_shows_definition_location() {
    let f = finding_with_evidence(sample_evidence(2, ImportConfidence::High));
    let out = format_findings(&[f], None, &t().audit);
    assert!(out.contains("api.py:42"));
}

#[test]
fn human_renderer_displays_confidence_label() {
    for (conf, expected) in [
        (ImportConfidence::High, "high"),
        (ImportConfidence::Medium, "medium"),
        (ImportConfidence::Low, "low"),
        (ImportConfidence::BestEffort, "best-effort"),
    ] {
        let f = finding_with_evidence(sample_evidence(1, conf));
        let out = format_findings(&[f], None, &t().audit);
        assert!(out.contains(expected), "{conf:?} should render as {expected} in: {out}");
    }
}

#[test]
fn human_renderer_lists_caller_samples() {
    let f = finding_with_evidence(sample_evidence(3, ImportConfidence::High));
    let out = format_findings(&[f], None, &t().audit);
    assert!(out.contains("caller_0.py:1"));
    assert!(out.contains("caller_2.py:3"));
}

#[test]
fn human_renderer_caps_callers_with_overflow_marker() {
    let evidence = sample_evidence(50, ImportConfidence::High);
    let f = finding_with_evidence(evidence);
    let mut audit_t = t().audit;
    audit_t.named_smells.max_caller_samples_per_finding = 5;
    let out = format_findings(&[f], None, &audit_t);
    assert!(out.contains("(45 more)"));
}

#[test]
fn human_renderer_no_overflow_marker_below_cap() {
    let f = finding_with_evidence(sample_evidence(3, ImportConfidence::High));
    let out = format_findings(&[f], None, &t().audit);
    assert!(!out.contains("more)"));
}

#[test]
fn human_renderer_handles_no_class() {
    let mut e = sample_evidence(2, ImportConfidence::High);
    e.method_class = None;
    e.method_name = "free_fn".to_string();
    let f = finding_with_evidence(e);
    let out = format_findings(&[f], None, &t().audit);
    assert!(out.contains("free_fn"));
    assert!(!out.contains("None.free_fn"));
}

#[test]
fn json_renderer_produces_valid_json() {
    let f = finding_with_evidence(sample_evidence(2, ImportConfidence::High));
    let out = format_findings_json(&[f], None);
    let _: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
}

#[test]
fn json_renderer_carries_kind_field() {
    let f = finding_with_evidence(sample_evidence(2, ImportConfidence::High));
    let out = format_findings_json(&[f], None);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v[0]["kind"], "ShotgunSurgery");
}

#[test]
fn json_renderer_includes_metrics() {
    let f = finding_with_evidence(sample_evidence(2, ImportConfidence::High));
    let out = format_findings_json(&[f], None);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v[0]["changing_classes"], 6);
    assert_eq!(v[0]["changing_methods"], 12);
    assert_eq!(v[0]["fanout"], 7);
}

#[test]
fn json_renderer_includes_method_identity() {
    let f = finding_with_evidence(sample_evidence(2, ImportConfidence::High));
    let out = format_findings_json(&[f], None);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v[0]["method_class"], "Service");
    assert_eq!(v[0]["method_name"], "handle");
    assert_eq!(v[0]["method_line"], 42);
}

#[test]
fn json_renderer_serializes_callers_array() {
    let f = finding_with_evidence(sample_evidence(3, ImportConfidence::High));
    let out = format_findings_json(&[f], None);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let callers = v[0]["callers"].as_array().unwrap();
    assert_eq!(callers.len(), 3);
    assert_eq!(callers[0]["line"], 1);
}

#[test]
fn json_renderer_handles_no_class_as_null() {
    let mut e = sample_evidence(0, ImportConfidence::High);
    e.method_class = None;
    let f = finding_with_evidence(e);
    let out = format_findings_json(&[f], None);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v[0]["method_class"].is_null());
}

#[test]
fn json_renderer_emits_confidence_label() {
    let f = finding_with_evidence(sample_evidence(1, ImportConfidence::Low));
    let out = format_findings_json(&[f], None);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v[0]["confidence"], "low");
}

#[test]
fn human_renderer_separates_findings_with_blank_line() {
    let f1 = finding_with_evidence(sample_evidence(1, ImportConfidence::High));
    let f2 = finding_with_evidence(sample_evidence(1, ImportConfidence::Medium));
    let out = format_findings(&[f1, f2], None, &t().audit);
    assert!(out.contains("\n\n"));
}

#[test]
fn empty_findings_renders_empty_human_string() {
    let out = format_findings(&[], None, &t().audit);
    assert!(out.is_empty());
}

#[test]
fn empty_findings_renders_empty_json_array() {
    let out = format_findings_json(&[], None);
    assert_eq!(out, "[]");
}

#[test]
fn root_relative_path_strips_prefix() {
    let mut e = sample_evidence(1, ImportConfidence::High);
    e.method_file = PathBuf::from("/tmp/proj/api.py");
    let f = finding_with_evidence(e);
    let out = format_findings(&[f], Some(std::path::Path::new("/tmp/proj")), &t().audit);
    assert!(out.contains("api.py:42"));
    assert!(!out.contains("/tmp/proj/api.py"));
}

#[test]
fn confidence_naabstraction_renders() {
    let f = finding_with_evidence(sample_evidence(1, ImportConfidence::NaAbstraction));
    let out = format_findings(&[f], None, &t().audit);
    assert!(out.contains("n/a-abstraction"));
}
