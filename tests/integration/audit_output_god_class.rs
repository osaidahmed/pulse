use std::path::PathBuf;

use pulse::audit::finding::{
    AuditFinding, AuditKind, GodClassEvidence, ImportConfidence,
};
use pulse::audit::output::{format_findings, format_findings_json};

use crate::audit_common::t;

fn finding_with(e: GodClassEvidence) -> AuditFinding {
    AuditFinding {
        kind: AuditKind::GodClass(e),
        representative_snippet: String::new(),
        support: 0,
        file_count: 0,
        idf_score: None,
        action_label: None,
        pattern_category: None,
        locations: Vec::new(),
    }
}

fn sample(confidence: ImportConfidence) -> GodClassEvidence {
    GodClassEvidence {
        class_file: PathBuf::from("god.py"),
        class_name: "Manager".to_string(),
        wmc: 73,
        tcc: 0.125,
        atfd: 11,
        method_count: 25,
        confidence,
    }
}

#[test]
fn human_includes_class_name_and_label() {
    let out = format_findings(&[finding_with(sample(ImportConfidence::Medium))], None, &t().audit);
    assert!(out.contains("god class"));
    assert!(out.contains("Manager"));
}

#[test]
fn human_includes_file_path() {
    let out = format_findings(&[finding_with(sample(ImportConfidence::Medium))], None, &t().audit);
    assert!(out.contains("god.py"));
}

#[test]
fn human_shows_all_metrics() {
    let out = format_findings(&[finding_with(sample(ImportConfidence::Medium))], None, &t().audit);
    assert!(out.contains("WMC:"));
    assert!(out.contains("TCC:"));
    assert!(out.contains("ATFD:"));
    assert!(out.contains("method count:"));
}

#[test]
fn human_shows_metric_values() {
    let out = format_findings(&[finding_with(sample(ImportConfidence::Medium))], None, &t().audit);
    assert!(out.contains("73"));
    assert!(out.contains("11"));
    assert!(out.contains("25"));
}

#[test]
fn human_formats_tcc_three_decimal_places() {
    let out = format_findings(&[finding_with(sample(ImportConfidence::Medium))], None, &t().audit);
    assert!(out.contains("0.125"));
}

#[test]
fn human_handles_tcc_zero() {
    let mut e = sample(ImportConfidence::Medium);
    e.tcc = 0.0;
    let out = format_findings(&[finding_with(e)], None, &t().audit);
    assert!(out.contains("0.000"));
}

#[test]
fn human_handles_tcc_one() {
    let mut e = sample(ImportConfidence::Medium);
    e.tcc = 1.0;
    let out = format_findings(&[finding_with(e)], None, &t().audit);
    assert!(out.contains("1.000"));
}

#[test]
fn human_handles_tcc_very_small_nonzero() {
    let mut e = sample(ImportConfidence::Medium);
    e.tcc = 0.0001;
    let out = format_findings(&[finding_with(e)], None, &t().audit);
    assert!(out.contains("0.000"));
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
    e.class_file = PathBuf::from("/tmp/proj/god.py");
    let out = format_findings(
        &[finding_with(e)],
        Some(std::path::Path::new("/tmp/proj")),
        &t().audit,
    );
    assert!(out.contains("god.py"));
    assert!(!out.contains("/tmp/proj/god.py"));
}

#[test]
fn human_separates_findings_with_blank_line() {
    let f1 = finding_with(sample(ImportConfidence::Medium));
    let f2 = finding_with(sample(ImportConfidence::High));
    let out = format_findings(&[f1, f2], None, &t().audit);
    assert!(out.contains("\n\n"));
}

#[test]
fn human_handles_unicode_class_name() {
    let mut e = sample(ImportConfidence::Medium);
    e.class_name = "クラス".to_string();
    let out = format_findings(&[finding_with(e)], None, &t().audit);
    assert!(out.contains("クラス"));
}

#[test]
fn json_parses_as_valid_json() {
    let out = format_findings_json(&[finding_with(sample(ImportConfidence::Medium))], None);
    let _: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
}

#[test]
fn json_kind_is_god_class() {
    let out = format_findings_json(&[finding_with(sample(ImportConfidence::Medium))], None);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v[0]["kind"], "GodClass");
}

#[test]
fn json_includes_class_identity() {
    let out = format_findings_json(&[finding_with(sample(ImportConfidence::Medium))], None);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v[0]["class_name"], "Manager");
    assert_eq!(v[0]["class_file"], "god.py");
}

#[test]
fn json_includes_all_metrics() {
    let out = format_findings_json(&[finding_with(sample(ImportConfidence::Medium))], None);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v[0]["wmc"], 73);
    assert_eq!(v[0]["atfd"], 11);
    assert_eq!(v[0]["method_count"], 25);
    let tcc = v[0]["tcc"].as_f64().unwrap();
    assert!((tcc - 0.125).abs() < 1e-9);
}

#[test]
fn json_tcc_is_number_not_string() {
    let out = format_findings_json(&[finding_with(sample(ImportConfidence::Medium))], None);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v[0]["tcc"].is_number());
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
    e.class_file = PathBuf::from("/tmp/proj/god.py");
    let out = format_findings_json(&[finding_with(e)], Some(std::path::Path::new("/tmp/proj")));
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v[0]["class_file"], "god.py");
}

#[test]
fn json_handles_tcc_zero_and_one() {
    for value in [0.0, 1.0] {
        let mut e = sample(ImportConfidence::Medium);
        e.tcc = value;
        let out = format_findings_json(&[finding_with(e)], None);
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!((v[0]["tcc"].as_f64().unwrap() - value).abs() < 1e-6);
    }
}
