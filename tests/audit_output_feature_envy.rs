use std::path::PathBuf;

use pulse::audit::finding::{
    AuditFinding, AuditKind, FeatureEnvyEvidence, ImportConfidence,
};
use pulse::audit::output::{format_findings, format_findings_json};

mod audit_common;
use audit_common::t;

fn finding_with(e: FeatureEnvyEvidence) -> AuditFinding {
    AuditFinding {
        kind: AuditKind::FeatureEnvy(e),
        representative_snippet: String::new(),
        support: 0,
        file_count: 0,
        idf_score: None,
        action_label: None,
        locations: Vec::new(),
    }
}

fn sample(confidence: ImportConfidence) -> FeatureEnvyEvidence {
    FeatureEnvyEvidence {
        method_file: PathBuf::from("svc.py"),
        method_class: Some("Orchestrator".to_string()),
        method_name: "compute".to_string(),
        method_line: 17,
        atfd: 9,
        foreign_call_count: 7,
        intra_call_count: 2,
        envied_class: Some("Repository".to_string()),
        confidence,
    }
}

#[test]
fn human_includes_qualified_method_name() {
    let out = format_findings(&[finding_with(sample(ImportConfidence::Medium))], None, &t().audit);
    assert!(out.contains("Orchestrator.compute"));
    assert!(out.contains("feature envy"));
}

#[test]
fn human_includes_definition_location() {
    let out = format_findings(&[finding_with(sample(ImportConfidence::Medium))], None, &t().audit);
    assert!(out.contains("svc.py:17"));
}

#[test]
fn human_shows_atfd_and_call_counts() {
    let out = format_findings(&[finding_with(sample(ImportConfidence::Medium))], None, &t().audit);
    assert!(out.contains("ATFD:"));
    assert!(out.contains("foreign calls:"));
    assert!(out.contains("intra calls:"));
}

#[test]
fn human_shows_metric_values() {
    let out = format_findings(&[finding_with(sample(ImportConfidence::Medium))], None, &t().audit);
    assert!(out.contains("9"));
    assert!(out.contains("7"));
    assert!(out.contains("2"));
}

#[test]
fn human_shows_envied_class_when_present() {
    let out = format_findings(&[finding_with(sample(ImportConfidence::Medium))], None, &t().audit);
    assert!(out.contains("envied class:"));
    assert!(out.contains("Repository"));
}

#[test]
fn human_omits_envied_class_when_none() {
    let mut e = sample(ImportConfidence::Medium);
    e.envied_class = None;
    let out = format_findings(&[finding_with(e)], None, &t().audit);
    assert!(!out.contains("envied class:"));
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
fn human_handles_no_method_class_falls_back_to_bare_name() {
    let mut e = sample(ImportConfidence::Medium);
    e.method_class = None;
    e.method_name = "module_fn".to_string();
    let out = format_findings(&[finding_with(e)], None, &t().audit);
    assert!(out.contains("module_fn"));
    assert!(!out.contains("None.module_fn"));
}

#[test]
fn human_strips_root_prefix() {
    let mut e = sample(ImportConfidence::Medium);
    e.method_file = PathBuf::from("/tmp/proj/svc.py");
    let out = format_findings(
        &[finding_with(e)],
        Some(std::path::Path::new("/tmp/proj")),
        &t().audit,
    );
    assert!(out.contains("svc.py:17"));
    assert!(!out.contains("/tmp/proj/svc.py"));
}

#[test]
fn human_separates_findings_with_blank_line() {
    let f1 = finding_with(sample(ImportConfidence::Medium));
    let f2 = finding_with(sample(ImportConfidence::High));
    let out = format_findings(&[f1, f2], None, &t().audit);
    assert!(out.contains("\n\n"));
}

#[test]
fn human_handles_unicode() {
    let mut e = sample(ImportConfidence::Medium);
    e.method_class = Some("Сервис".to_string());
    e.method_name = "обработать".to_string();
    e.envied_class = Some("Хранилище".to_string());
    let out = format_findings(&[finding_with(e)], None, &t().audit);
    assert!(out.contains("Сервис.обработать"));
    assert!(out.contains("Хранилище"));
}

#[test]
fn json_parses_as_valid_json() {
    let out = format_findings_json(&[finding_with(sample(ImportConfidence::Medium))], None);
    let _: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
}

#[test]
fn json_kind_is_feature_envy() {
    let out = format_findings_json(&[finding_with(sample(ImportConfidence::Medium))], None);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v[0]["kind"], "FeatureEnvy");
}

#[test]
fn json_includes_method_identity() {
    let out = format_findings_json(&[finding_with(sample(ImportConfidence::Medium))], None);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v[0]["method_class"], "Orchestrator");
    assert_eq!(v[0]["method_name"], "compute");
    assert_eq!(v[0]["method_line"], 17);
    assert_eq!(v[0]["method_file"], "svc.py");
}

#[test]
fn json_includes_all_call_metrics() {
    let out = format_findings_json(&[finding_with(sample(ImportConfidence::Medium))], None);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v[0]["atfd"], 9);
    assert_eq!(v[0]["foreign_call_count"], 7);
    assert_eq!(v[0]["intra_call_count"], 2);
    assert_eq!(v[0]["envied_class"], "Repository");
}

#[test]
fn json_method_class_null_when_none() {
    let mut e = sample(ImportConfidence::Medium);
    e.method_class = None;
    let out = format_findings_json(&[finding_with(e)], None);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v[0]["method_class"].is_null());
}

#[test]
fn json_envied_class_null_when_none() {
    let mut e = sample(ImportConfidence::Medium);
    e.envied_class = None;
    let out = format_findings_json(&[finding_with(e)], None);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v[0]["envied_class"].is_null());
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
    e.method_file = PathBuf::from("/tmp/proj/svc.py");
    let out = format_findings_json(&[finding_with(e)], Some(std::path::Path::new("/tmp/proj")));
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v[0]["method_file"], "svc.py");
}
