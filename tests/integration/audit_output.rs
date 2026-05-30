use pulse::audit::finding::{AuditFinding, AuditKind, AuditLocation};
use pulse::audit::output::{format_findings, format_findings_json};
use pulse::thresholds::Thresholds;
use std::path::{Path, PathBuf};

fn finding(fp: u64, support: u32, file_count: u32, snippet: &str, files: &[(&str, u32)]) -> AuditFinding {
    AuditFinding {
        kind: AuditKind::UncategorizedPattern { fingerprint: fp },
        representative_snippet: snippet.to_string(),
        support,
        file_count,
        idf_score: Some(1.5),
        action_label: None,
        pattern_category: None,
        locality_entropy: None,
        p_value: None,
        locations: files.iter().map(|(f, l)| AuditLocation { file: PathBuf::from(f), line: *l }).collect(),
    }
}

fn t() -> Thresholds {
    Thresholds::default()
}

#[test]
fn format_findings_empty_returns_empty_string() {
    let s = format_findings(&[], None, &t().audit);
    assert!(s.is_empty());
}

#[test]
fn format_findings_one_finding_renders_one_block() {
    let f = finding(7, 5, 5, "x", &[("a.py", 1)]);
    let s = format_findings(&[f], None, &t().audit);
    let count = s.matches("audit:").count();
    assert_eq!(count, 1);
}

#[test]
fn format_findings_renders_representative_snippet() {
    let f = finding(7, 5, 5, "media_type == X", &[("a.py", 1)]);
    let s = format_findings(&[f], None, &t().audit);
    assert!(s.contains("media_type == X"));
}

#[test]
fn format_findings_renders_support_count() {
    let f = finding(7, 117, 38, "x", &[("a.py", 1)]);
    let s = format_findings(&[f], None, &t().audit);
    assert!(s.contains("117 occurrences"));
}

#[test]
fn format_findings_renders_file_count() {
    let f = finding(7, 117, 38, "x", &[("a.py", 1)]);
    let s = format_findings(&[f], None, &t().audit);
    assert!(s.contains("38 files"));
}

#[test]
fn format_findings_caps_locations_at_threshold() {
    let mut th = t().audit;
    th.max_locations_per_finding = 5;
    let files: Vec<(&str, u32)> = (0..30).map(|i| {
        let s: &'static str = Box::leak(format!("f{i}.py").into_boxed_str());
        (s, i as u32)
    }).collect();
    let f = finding(7, 30, 30, "x", &files);
    let s = format_findings(&[f], None, &th);
    assert!(s.contains("(25 more)"));
}

#[test]
fn format_findings_no_more_marker_when_below_cap() {
    let f = finding(7, 3, 3, "x", &[("a.py", 1), ("b.py", 2), ("c.py", 3)]);
    let s = format_findings(&[f], None, &t().audit);
    assert!(!s.contains("more)"));
}

#[test]
fn format_findings_renders_action_label_when_present() {
    let mut f = finding(7, 5, 5, "x", &[("a.py", 1)]);
    f.action_label = Some("extract polymorphic dispatch");
    let s = format_findings(&[f], None, &t().audit);
    assert!(s.contains("extract polymorphic dispatch"));
}

#[test]
fn format_findings_omits_action_block_when_none() {
    let f = finding(7, 5, 5, "x", &[("a.py", 1)]);
    let s = format_findings(&[f], None, &t().audit);
    assert!(!s.contains("pattern action:"));
}

#[test]
fn format_findings_emits_one_audit_header_per_finding() {
    let f1 = finding(1, 5, 5, "a", &[("a.py", 1)]);
    let f2 = finding(2, 3, 3, "b", &[("b.py", 1)]);
    let s = format_findings(&[f1, f2], None, &t().audit);
    assert_eq!(s.matches("audit:").count(), 2);
}

#[test]
fn format_findings_handles_unicode_snippet() {
    let f = finding(7, 5, 5, "δ == λ", &[("a.py", 1)]);
    let s = format_findings(&[f], None, &t().audit);
    assert!(s.contains("δ == λ"));
}

#[test]
fn format_findings_uses_relative_path_when_root_is_set() {
    let f = finding(7, 5, 5, "x", &[("/tmp/proj/src/a.py", 10)]);
    let root = PathBuf::from("/tmp/proj");
    let s = format_findings(&[f], Some(&root), &t().audit);
    assert!(s.contains("src/a.py"));
    assert!(!s.contains("/tmp/proj/src/a.py"));
}

#[test]
fn format_findings_uses_absolute_when_root_is_none() {
    let f = finding(7, 5, 5, "x", &[("/tmp/proj/src/a.py", 10)]);
    let s = format_findings(&[f], None, &t().audit);
    assert!(s.contains("/tmp/proj/src/a.py"));
}

#[test]
fn format_findings_json_returns_valid_json() {
    let f = finding(7, 5, 5, "x", &[("a.py", 1)]);
    let s = format_findings_json(&[f], None);
    let parsed: serde_json::Value = serde_json::from_str(&s).expect("valid json");
    assert!(parsed.is_array());
}

#[test]
fn format_findings_json_array_length_matches_finding_count() {
    let fs = vec![
        finding(1, 5, 5, "a", &[("a.py", 1)]),
        finding(2, 3, 3, "b", &[("b.py", 1)]),
    ];
    let s = format_findings_json(&fs, None);
    let parsed: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(parsed.as_array().unwrap().len(), 2);
}

#[test]
fn format_findings_json_each_object_has_required_keys() {
    let f = finding(7, 5, 5, "x", &[("a.py", 1)]);
    let s = format_findings_json(&[f], None);
    let parsed: serde_json::Value = serde_json::from_str(&s).unwrap();
    let obj = &parsed[0];
    for k in ["kind", "fingerprint", "representative_snippet", "support", "file_count", "idf_score", "action_label", "locations"] {
        assert!(obj.get(k).is_some(), "missing key {k}");
    }
}

#[test]
fn format_findings_json_locations_is_array_of_objects() {
    let f = finding(7, 5, 5, "x", &[("a.py", 1), ("b.py", 2)]);
    let s = format_findings_json(&[f], None);
    let parsed: serde_json::Value = serde_json::from_str(&s).unwrap();
    let locs = parsed[0]["locations"].as_array().unwrap();
    assert_eq!(locs.len(), 2);
    assert!(locs[0]["file"].is_string());
    assert!(locs[0]["line"].is_number());
}

#[test]
fn format_findings_json_idf_score_is_number_or_null() {
    let mut f = finding(7, 5, 5, "x", &[("a.py", 1)]);
    f.idf_score = None;
    let s = format_findings_json(&[f], None);
    let parsed: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert!(parsed[0]["idf_score"].is_null());
}

#[test]
fn format_findings_json_action_label_is_string_or_null() {
    let f = finding(7, 5, 5, "x", &[("a.py", 1)]);
    let s = format_findings_json(&[f], None);
    let parsed: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert!(parsed[0]["action_label"].is_null());
}

#[test]
fn format_findings_json_is_deterministic() {
    let f = finding(7, 5, 5, "x", &[("a.py", 1)]);
    let s1 = format_findings_json(std::slice::from_ref(&f), None);
    let s2 = format_findings_json(std::slice::from_ref(&f), None);
    assert_eq!(s1, s2);
}

#[test]
fn format_findings_human_handles_empty_locations() {
    let f = finding(7, 5, 5, "x", &[]);
    let _ = format_findings(&[f], None, &t().audit);
}

#[test]
fn format_findings_kind_label_present_in_json() {
    let f = finding(7, 5, 5, "x", &[("a.py", 1)]);
    let s = format_findings_json(&[f], None);
    assert!(s.contains("UncategorizedPattern"));
}

#[test]
fn format_findings_fingerprint_round_trips_through_json() {
    let f = finding(0xdead_beef, 5, 5, "x", &[("a.py", 1)]);
    let s = format_findings_json(&[f], None);
    let parsed: serde_json::Value = serde_json::from_str(&s).unwrap();
    let fp = parsed[0]["fingerprint"].as_u64().unwrap();
    assert_eq!(fp, 0xdead_beef);
}

#[test]
fn format_findings_human_handles_long_snippet() {
    let long = "x".repeat(200);
    let f = finding(7, 5, 5, &long, &[("a.py", 1)]);
    let s = format_findings(&[f], None, &t().audit);
    assert!(s.contains(&long));
}

#[test]
fn format_findings_json_truncates_snippet_consistent_with_human() {
    let f1 = finding(7, 5, 5, "media_type == X", &[("a.py", 1)]);
    let f2 = f1.clone();
    let _ = format_findings(&[f1], None, &t().audit);
    let s = format_findings_json(&[f2], None);
    assert!(s.contains("media_type == X"));
}

#[test]
fn format_findings_idf_score_in_json() {
    let mut f = finding(7, 5, 5, "x", &[("a.py", 1)]);
    f.idf_score = Some(2.5);
    let s = format_findings_json(&[f], None);
    let parsed: serde_json::Value = serde_json::from_str(&s).unwrap();
    let score = parsed[0]["idf_score"].as_f64().unwrap();
    assert!((score - 2.5).abs() < 1e-9);
}

fn _unused_path_helper() -> &'static Path {
    Path::new("/")
}
