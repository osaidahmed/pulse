use std::path::PathBuf;

use pulse::audit::finding::{AuditFinding, AuditKind, ImportConfidence, RefusedBequestEvidence};
use pulse::audit::output::{format_findings, format_findings_json};

use crate::audit_common::t;

fn finding_with(e: RefusedBequestEvidence) -> AuditFinding {
    AuditFinding {
        kind: AuditKind::RefusedBequest(e),
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

fn sample(confidence: ImportConfidence) -> RefusedBequestEvidence {
    RefusedBequestEvidence {
        subclass_file: PathBuf::from("child.py"),
        subclass_name: "ConcreteChild".to_string(),
        parent_file: PathBuf::from("base.py"),
        parent_name: "BaseClass".to_string(),
        override_count: 1,
        parent_method_count: 8,
        override_ratio: 0.125,
        confidence,
    }
}

#[test]
fn human_includes_subclass_and_parent_names() {
    let out = format_findings(&[finding_with(sample(ImportConfidence::Medium))], None, &t().audit);
    assert!(out.contains("refused bequest"));
    assert!(out.contains("ConcreteChild"));
    assert!(out.contains("BaseClass"));
    assert!(out.contains("extends"));
}

#[test]
fn human_includes_both_files() {
    let out = format_findings(&[finding_with(sample(ImportConfidence::Medium))], None, &t().audit);
    assert!(out.contains("child.py"));
    assert!(out.contains("base.py"));
}

#[test]
fn human_shows_override_count_and_total() {
    let out = format_findings(&[finding_with(sample(ImportConfidence::Medium))], None, &t().audit);
    assert!(out.contains("overrides:"));
    assert!(out.contains('1'));
    assert!(out.contains('8'));
}

#[test]
fn human_shows_ratio_with_three_decimals() {
    let out = format_findings(&[finding_with(sample(ImportConfidence::Medium))], None, &t().audit);
    assert!(out.contains("ratio:"));
    assert!(out.contains("0.125"));
}

#[test]
fn human_handles_ratio_zero() {
    let mut e = sample(ImportConfidence::Medium);
    e.override_count = 0;
    e.override_ratio = 0.0;
    let out = format_findings(&[finding_with(e)], None, &t().audit);
    assert!(out.contains("0.000"));
}

#[test]
fn human_handles_ratio_one() {
    let mut e = sample(ImportConfidence::Medium);
    e.override_count = 8;
    e.override_ratio = 1.0;
    let out = format_findings(&[finding_with(e)], None, &t().audit);
    assert!(out.contains("1.000"));
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
    e.subclass_file = PathBuf::from("/tmp/proj/child.py");
    e.parent_file = PathBuf::from("/tmp/proj/base.py");
    let out = format_findings(&[finding_with(e)], Some(std::path::Path::new("/tmp/proj")), &t().audit);
    assert!(!out.contains("/tmp/proj/"));
    assert!(out.contains("child.py"));
    assert!(out.contains("base.py"));
}

#[test]
fn human_separates_findings_with_blank_line() {
    let f1 = finding_with(sample(ImportConfidence::Medium));
    let f2 = finding_with(sample(ImportConfidence::High));
    let out = format_findings(&[f1, f2], None, &t().audit);
    assert!(out.contains("\n\n"));
}

#[test]
fn human_handles_unicode_class_names() {
    let mut e = sample(ImportConfidence::Medium);
    e.subclass_name = "子クラス".to_string();
    e.parent_name = "親クラス".to_string();
    let out = format_findings(&[finding_with(e)], None, &t().audit);
    assert!(out.contains("子クラス"));
    assert!(out.contains("親クラス"));
}

#[test]
fn json_parses_as_valid_json() {
    let out = format_findings_json(&[finding_with(sample(ImportConfidence::Medium))], None);
    let _: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
}

#[test]
fn json_kind_is_refused_bequest() {
    let out = format_findings_json(&[finding_with(sample(ImportConfidence::Medium))], None);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v[0]["kind"], "RefusedBequest");
}

#[test]
fn json_includes_subclass_and_parent_identities() {
    let out = format_findings_json(&[finding_with(sample(ImportConfidence::Medium))], None);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v[0]["subclass_name"], "ConcreteChild");
    assert_eq!(v[0]["subclass_file"], "child.py");
    assert_eq!(v[0]["parent_name"], "BaseClass");
    assert_eq!(v[0]["parent_file"], "base.py");
}

#[test]
fn json_includes_override_metrics() {
    let out = format_findings_json(&[finding_with(sample(ImportConfidence::Medium))], None);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v[0]["override_count"], 1);
    assert_eq!(v[0]["parent_method_count"], 8);
    let ratio = v[0]["override_ratio"].as_f64().unwrap();
    assert!((ratio - 0.125).abs() < 1e-9);
}

#[test]
fn json_ratio_is_number_not_string() {
    let out = format_findings_json(&[finding_with(sample(ImportConfidence::Medium))], None);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v[0]["override_ratio"].is_number());
}

#[test]
fn json_handles_ratio_extremes() {
    for (count, ratio) in [(0u32, 0.0), (8u32, 1.0)] {
        let mut e = sample(ImportConfidence::Medium);
        e.override_count = count;
        e.override_ratio = ratio;
        let out = format_findings_json(&[finding_with(e)], None);
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v[0]["override_count"], count);
        assert!((v[0]["override_ratio"].as_f64().unwrap() - ratio).abs() < 1e-6);
    }
}

#[test]
fn json_emits_confidence_label() {
    let out = format_findings_json(&[finding_with(sample(ImportConfidence::Low))], None);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v[0]["confidence"], "low");
}

#[test]
fn json_strips_root_prefix_for_both_files() {
    let mut e = sample(ImportConfidence::Medium);
    e.subclass_file = PathBuf::from("/tmp/proj/child.py");
    e.parent_file = PathBuf::from("/tmp/proj/base.py");
    let out = format_findings_json(&[finding_with(e)], Some(std::path::Path::new("/tmp/proj")));
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v[0]["subclass_file"], "child.py");
    assert_eq!(v[0]["parent_file"], "base.py");
}
