use std::path::PathBuf;

use pulse::audit::finding::{AuditFinding, AuditKind, ClassIdentityRef, ImportConfidence, ParallelInheritanceEvidence};
use pulse::audit::output::{format_findings, format_findings_json};

use crate::audit_common::t;

fn finding_with(e: ParallelInheritanceEvidence) -> AuditFinding {
    AuditFinding {
        kind: AuditKind::ParallelInheritance(e),
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

fn sample_with(pairs: Vec<(String, String)>, confidence: ImportConfidence) -> ParallelInheritanceEvidence {
    ParallelInheritanceEvidence {
        root_a: ClassIdentityRef { file: PathBuf::from("readers.py"), name: "Reader".to_string() },
        root_b: ClassIdentityRef { file: PathBuf::from("writers.py"), name: "Writer".to_string() },
        matched_descendants: pairs,
        confidence,
    }
}

fn three_pairs() -> Vec<(String, String)> {
    vec![
        ("JsonReader".to_string(), "JsonWriter".to_string()),
        ("XmlReader".to_string(), "XmlWriter".to_string()),
        ("YamlReader".to_string(), "YamlWriter".to_string()),
    ]
}

#[test]
fn human_includes_root_names() {
    let out = format_findings(&[finding_with(sample_with(three_pairs(), ImportConfidence::Medium))], None, &t().audit);
    assert!(out.contains("parallel inheritance"));
    assert!(out.contains("Reader"));
    assert!(out.contains("Writer"));
}

#[test]
fn human_includes_both_root_files() {
    let out = format_findings(&[finding_with(sample_with(three_pairs(), ImportConfidence::Medium))], None, &t().audit);
    assert!(out.contains("readers.py"));
    assert!(out.contains("writers.py"));
}

#[test]
fn human_lists_matched_pair_count() {
    let out = format_findings(&[finding_with(sample_with(three_pairs(), ImportConfidence::Medium))], None, &t().audit);
    assert!(out.contains("matched pairs:"));
    assert!(out.contains('3'));
}

#[test]
fn human_renders_each_pair() {
    let out = format_findings(&[finding_with(sample_with(three_pairs(), ImportConfidence::Medium))], None, &t().audit);
    assert!(out.contains("JsonReader"));
    assert!(out.contains("JsonWriter"));
    assert!(out.contains("XmlReader"));
    assert!(out.contains("YamlReader"));
}

#[test]
fn human_caps_pair_list_at_twenty() {
    let pairs: Vec<(String, String)> = (0..30).map(|i| (format!("R{i}"), format!("W{i}"))).collect();
    let out = format_findings(&[finding_with(sample_with(pairs, ImportConfidence::Medium))], None, &t().audit);
    assert!(out.contains("R0"));
    assert!(out.contains("R19"));
    assert!(!out.contains("R20"), "rendered output should be capped at 20 pairs but contained R20: {out}");
    assert!(!out.contains("R29"));
}

#[test]
fn human_handles_empty_pair_list() {
    let out = format_findings(&[finding_with(sample_with(Vec::new(), ImportConfidence::Medium))], None, &t().audit);
    assert!(out.contains("matched pairs:"));
    assert!(out.contains('0'));
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
        let out = format_findings(&[finding_with(sample_with(three_pairs(), conf))], None, &t().audit);
        assert!(out.contains(expected), "{conf:?} should render as {expected} in: {out}");
    }
}

#[test]
fn human_strips_root_prefix() {
    let mut e = sample_with(three_pairs(), ImportConfidence::Medium);
    e.root_a.file = PathBuf::from("/tmp/proj/readers.py");
    e.root_b.file = PathBuf::from("/tmp/proj/writers.py");
    let out = format_findings(&[finding_with(e)], Some(std::path::Path::new("/tmp/proj")), &t().audit);
    assert!(!out.contains("/tmp/proj/"));
    assert!(out.contains("readers.py"));
    assert!(out.contains("writers.py"));
}

#[test]
fn human_separates_findings_with_blank_line() {
    let f1 = finding_with(sample_with(three_pairs(), ImportConfidence::Medium));
    let f2 = finding_with(sample_with(three_pairs(), ImportConfidence::High));
    let out = format_findings(&[f1, f2], None, &t().audit);
    assert!(out.contains("\n\n"));
}

#[test]
fn human_handles_unicode_root_names() {
    let mut e = sample_with(three_pairs(), ImportConfidence::Medium);
    e.root_a.name = "Чтец".to_string();
    e.root_b.name = "Писатель".to_string();
    let out = format_findings(&[finding_with(e)], None, &t().audit);
    assert!(out.contains("Чтец"));
    assert!(out.contains("Писатель"));
}

#[test]
fn json_parses_as_valid_json() {
    let out = format_findings_json(&[finding_with(sample_with(three_pairs(), ImportConfidence::Medium))], None);
    let _: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
}

#[test]
fn json_kind_is_parallel_inheritance() {
    let out = format_findings_json(&[finding_with(sample_with(three_pairs(), ImportConfidence::Medium))], None);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v[0]["kind"], "ParallelInheritance");
}

#[test]
fn json_includes_root_identities() {
    let out = format_findings_json(&[finding_with(sample_with(three_pairs(), ImportConfidence::Medium))], None);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v[0]["root_a_name"], "Reader");
    assert_eq!(v[0]["root_a_file"], "readers.py");
    assert_eq!(v[0]["root_b_name"], "Writer");
    assert_eq!(v[0]["root_b_file"], "writers.py");
}

#[test]
fn json_pairs_serialize_as_two_element_arrays() {
    let out = format_findings_json(&[finding_with(sample_with(three_pairs(), ImportConfidence::Medium))], None);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let pairs = v[0]["matched_descendants"].as_array().unwrap();
    assert_eq!(pairs.len(), 3);
    assert_eq!(pairs[0][0], "JsonReader");
    assert_eq!(pairs[0][1], "JsonWriter");
}

#[test]
fn json_includes_full_pairs_no_truncation() {
    let pairs: Vec<(String, String)> = (0..30).map(|i| (format!("R{i}"), format!("W{i}"))).collect();
    let out = format_findings_json(&[finding_with(sample_with(pairs, ImportConfidence::Medium))], None);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let arr = v[0]["matched_descendants"].as_array().unwrap();
    assert_eq!(arr.len(), 30);
    assert_eq!(arr[29][0], "R29");
}

#[test]
fn json_handles_empty_pair_list() {
    let out = format_findings_json(&[finding_with(sample_with(Vec::new(), ImportConfidence::Medium))], None);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let arr = v[0]["matched_descendants"].as_array().unwrap();
    assert!(arr.is_empty());
}

#[test]
fn json_emits_confidence_label() {
    let out = format_findings_json(&[finding_with(sample_with(three_pairs(), ImportConfidence::Low))], None);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v[0]["confidence"], "low");
}

#[test]
fn json_strips_root_prefix() {
    let mut e = sample_with(three_pairs(), ImportConfidence::Medium);
    e.root_a.file = PathBuf::from("/tmp/proj/readers.py");
    e.root_b.file = PathBuf::from("/tmp/proj/writers.py");
    let out = format_findings_json(&[finding_with(e)], Some(std::path::Path::new("/tmp/proj")));
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v[0]["root_a_file"], "readers.py");
    assert_eq!(v[0]["root_b_file"], "writers.py");
}
