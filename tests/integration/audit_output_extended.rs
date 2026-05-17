use pulse::audit::finding::{AuditFinding, AuditKind, AuditLocation};
use pulse::audit::output::{format_findings, format_findings_json};
use pulse::thresholds::Thresholds;
use std::path::{Path, PathBuf};

fn t() -> Thresholds { Thresholds::default() }

fn fab(fp: u64, support: u32, files: u32, snippet: &str, locs: &[(&str, u32)]) -> AuditFinding {
    AuditFinding {
        kind: AuditKind::UncategorizedPattern { fingerprint: fp },
        representative_snippet: snippet.to_string(),
        support, file_count: files,
        idf_score: Some(1.5), action_label: None,
        pattern_category: None,
        locations: locs.iter().map(|(p, l)| AuditLocation { file: PathBuf::from(*p), line: *l }).collect(),
    }
}

#[test]
fn output_human_starts_with_audit_prefix() {
    let f = fab(7, 5, 5, "x", &[("a.py", 1)]);
    let s = format_findings(&[f], None, &t().audit);
    assert!(s.starts_with("audit:") || s.starts_with("## "));
}

#[test]
fn output_human_includes_pattern_word() {
    let f = fab(7, 5, 5, "x", &[("a.py", 1)]);
    let s = format_findings(&[f], None, &t().audit);
    assert!(s.contains("pattern"));
}

#[test]
fn output_human_includes_files_word() {
    let f = fab(7, 5, 5, "x", &[("a.py", 1)]);
    let s = format_findings(&[f], None, &t().audit);
    assert!(s.contains("files"));
}

#[test]
fn output_human_includes_occurrences_word() {
    let f = fab(7, 5, 5, "x", &[("a.py", 1)]);
    let s = format_findings(&[f], None, &t().audit);
    assert!(s.contains("occurrences"));
}

#[test]
fn output_human_renders_each_finding_independently() {
    let f1 = fab(1, 5, 5, "a", &[("a.py", 1)]);
    let f2 = fab(2, 4, 4, "b", &[("b.py", 1)]);
    let s = format_findings(&[f1, f2], None, &t().audit);
    assert!(s.contains("5 occurrences"));
    assert!(s.contains("4 occurrences"));
}

#[test]
fn output_human_locations_count_matches_min_of_max_and_total() {
    let mut th = t().audit;
    th.max_locations_per_finding = 3;
    let f = fab(7, 5, 5, "x", &[("a.py", 1), ("b.py", 2), ("c.py", 3), ("d.py", 4), ("e.py", 5)]);
    let s = format_findings(&[f], None, &th);
    let lines: Vec<&str> = s.lines().filter(|l| l.contains(".py:")).collect();
    assert_eq!(lines.len(), 3);
    assert!(s.contains("(2 more)"));
}

#[test]
fn output_human_handles_one_thousand_locations_with_cap() {
    let mut th = t().audit;
    th.max_locations_per_finding = 20;
    let locs: Vec<(String, u32)> = (0..1000).map(|i| (format!("f{i}.py"), i as u32)).collect();
    let loc_refs: Vec<(&str, u32)> = locs.iter().map(|(s, l)| (s.as_str(), *l)).collect();
    let f = fab(7, 1000, 1000, "x", &loc_refs);
    let s = format_findings(&[f], None, &th);
    assert!(s.contains("(980 more)"));
}

#[test]
fn output_json_field_kind_label_correct() {
    let f = fab(7, 5, 5, "x", &[("a.py", 1)]);
    let s = format_findings_json(&[f], None);
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(v[0]["kind"].as_str().unwrap(), "UncategorizedPattern");
}

#[test]
fn output_json_field_fingerprint_round_trips() {
    let f = fab(0xfeed_face, 5, 5, "x", &[("a.py", 1)]);
    let s = format_findings_json(&[f], None);
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(v[0]["fingerprint"].as_u64().unwrap(), 0xfeed_face);
}

#[test]
fn output_json_field_support_round_trips() {
    let f = fab(7, 117, 38, "x", &[("a.py", 1)]);
    let s = format_findings_json(&[f], None);
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(v[0]["support"].as_u64().unwrap(), 117);
}

#[test]
fn output_json_field_file_count_round_trips() {
    let f = fab(7, 117, 38, "x", &[("a.py", 1)]);
    let s = format_findings_json(&[f], None);
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(v[0]["file_count"].as_u64().unwrap(), 38);
}

#[test]
fn output_json_field_representative_snippet_preserves_unicode() {
    let f = fab(7, 5, 5, "δ == λ.value", &[("a.py", 1)]);
    let s = format_findings_json(&[f], None);
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(v[0]["representative_snippet"].as_str().unwrap(), "δ == λ.value");
}

#[test]
fn output_json_field_locations_each_has_file_and_line() {
    let f = fab(7, 5, 5, "x", &[("a.py", 1), ("b.py", 2)]);
    let s = format_findings_json(&[f], None);
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    let locs = v[0]["locations"].as_array().unwrap();
    for loc in locs {
        assert!(loc["file"].is_string());
        assert!(loc["line"].is_number());
    }
}

#[test]
fn output_json_locations_count_matches_input() {
    let f = fab(7, 5, 5, "x", &[("a.py", 1), ("b.py", 2), ("c.py", 3)]);
    let s = format_findings_json(&[f], None);
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(v[0]["locations"].as_array().unwrap().len(), 3);
}

#[test]
fn output_human_relative_path_strips_root() {
    let f = fab(7, 5, 5, "x", &[("/abs/proj/a.py", 1)]);
    let root = PathBuf::from("/abs/proj");
    let s = format_findings(&[f], Some(&root), &t().audit);
    assert!(s.contains("a.py:"));
    assert!(!s.contains("/abs/proj/a.py"));
}

#[test]
fn output_human_path_unchanged_when_root_does_not_match() {
    let f = fab(7, 5, 5, "x", &[("/abs/foo.py", 1)]);
    let root = PathBuf::from("/different/root");
    let s = format_findings(&[f], Some(&root), &t().audit);
    assert!(s.contains("/abs/foo.py"));
}

#[test]
fn output_json_handles_root_for_relative_paths() {
    let f = fab(7, 5, 5, "x", &[("/abs/proj/a.py", 1)]);
    let root = PathBuf::from("/abs/proj");
    let s = format_findings_json(&[f], Some(&root));
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    let path = v[0]["locations"][0]["file"].as_str().unwrap();
    assert_eq!(path, "a.py");
}

#[test]
fn output_human_with_empty_locations_omits_locations_block() {
    let f = fab(7, 5, 5, "x", &[]);
    let s = format_findings(&[f], None, &t().audit);
    assert!(!s.contains("locations:"));
}

#[test]
fn output_human_with_one_location_renders_correctly() {
    let f = fab(7, 5, 5, "x", &[("a.py", 42)]);
    let s = format_findings(&[f], None, &t().audit);
    assert!(s.contains("a.py:42"));
}

#[test]
fn output_human_includes_representative_label() {
    let f = fab(7, 5, 5, "x", &[("a.py", 1)]);
    let s = format_findings(&[f], None, &t().audit);
    assert!(s.contains("representative:"));
}

#[test]
fn output_human_action_label_when_set() {
    let mut f = fab(7, 5, 5, "x", &[("a.py", 1)]);
    f.action_label = Some("introduce polymorphism");
    let s = format_findings(&[f], None, &t().audit);
    assert!(s.contains("pattern action:"));
    assert!(s.contains("introduce polymorphism"));
}

#[test]
fn output_human_no_action_label_omits_block() {
    let mut f = fab(7, 5, 5, "x", &[("a.py", 1)]);
    f.action_label = None;
    let s = format_findings(&[f], None, &t().audit);
    assert!(!s.contains("pattern action:"));
}

#[test]
fn output_json_action_label_string_when_set() {
    let mut f = fab(7, 5, 5, "x", &[("a.py", 1)]);
    f.action_label = Some("extract helper");
    let s = format_findings_json(&[f], None);
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(v[0]["action_label"].as_str().unwrap(), "extract helper");
}

#[test]
fn output_json_idf_score_round_trips() {
    let mut f = fab(7, 5, 5, "x", &[("a.py", 1)]);
    f.idf_score = Some(2.5);
    let s = format_findings_json(&[f], None);
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    let score = v[0]["idf_score"].as_f64().unwrap();
    assert!((score - 2.5).abs() < 1e-9);
}

#[test]
fn output_json_idf_score_null_when_none() {
    let mut f = fab(7, 5, 5, "x", &[("a.py", 1)]);
    f.idf_score = None;
    let s = format_findings_json(&[f], None);
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert!(v[0]["idf_score"].is_null());
}

#[test]
fn output_human_handles_one_finding_with_thousand_locations_capped() {
    let locs: Vec<(String, u32)> = (0..1000).map(|i| (format!("f{i}.py"), 1)).collect();
    let loc_refs: Vec<(&str, u32)> = locs.iter().map(|(s, l)| (s.as_str(), *l)).collect();
    let mut th = t().audit;
    th.max_locations_per_finding = 5;
    let f = fab(7, 1000, 1000, "x", &loc_refs);
    let s = format_findings(&[f], None, &th);
    assert!(s.contains("(995 more)"));
}

#[test]
fn output_human_handles_zero_findings() {
    let s = format_findings(&[], None, &t().audit);
    assert!(s.is_empty());
}

#[test]
fn output_json_handles_zero_findings() {
    let s = format_findings_json(&[], None);
    assert_eq!(s, "[]");
}

#[test]
fn output_json_for_two_findings_same_kind() {
    let f1 = fab(1, 5, 5, "a", &[("a.py", 1)]);
    let f2 = fab(2, 4, 4, "b", &[("b.py", 1)]);
    let s = format_findings_json(&[f1, f2], None);
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(v.as_array().unwrap().len(), 2);
}

#[test]
fn output_json_idf_score_zero_renders_correctly() {
    let mut f = fab(7, 5, 5, "x", &[("a.py", 1)]);
    f.idf_score = Some(0.0);
    let s = format_findings_json(&[f], None);
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert!(v[0]["idf_score"].as_f64().unwrap().abs() < 1e-6);
}

#[test]
fn output_human_locations_block_indented_consistently() {
    let f = fab(7, 5, 5, "x", &[("a.py", 1), ("b.py", 2)]);
    let s = format_findings(&[f], None, &t().audit);
    let location_lines: Vec<&str> = s.lines().filter(|l| l.contains(".py:")).collect();
    for line in location_lines {
        assert!(line.starts_with("  ") || line.starts_with("    ") || line.starts_with("                "));
    }
}

#[test]
fn output_human_two_findings_separated_visually() {
    let f1 = fab(1, 5, 5, "a", &[("a.py", 1)]);
    let f2 = fab(2, 4, 4, "b", &[("b.py", 1)]);
    let s = format_findings(&[f1, f2], None, &t().audit);
    assert!(s.matches("audit:").count() == 2);
}

#[test]
fn output_handles_extreme_numerical_idf_score() {
    let mut f = fab(7, 5, 5, "x", &[("a.py", 1)]);
    f.idf_score = Some(1e308);
    let s = format_findings_json(&[f], None);
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    let score = v[0]["idf_score"].as_f64().unwrap();
    assert!((score - 1e308).abs() / 1e308 < 1e-9);
}

#[test]
fn output_handles_very_long_snippet() {
    let long = "x".repeat(500);
    let f = fab(7, 5, 5, &long, &[("a.py", 1)]);
    let s = format_findings(&[f], None, &t().audit);
    assert!(s.contains(&long));
}

#[test]
fn output_handles_empty_string_snippet() {
    let f = fab(7, 5, 5, "", &[("a.py", 1)]);
    let s = format_findings(&[f], None, &t().audit);
    assert!(s.contains("audit:"));
}

#[test]
fn output_human_handles_empty_string_path_in_location() {
    let f = fab(7, 5, 5, "x", &[("", 1)]);
    let s = format_findings(&[f], None, &t().audit);
    assert!(s.contains(":1"));
}

#[test]
fn output_json_handles_empty_string_path() {
    let f = fab(7, 5, 5, "x", &[("", 1)]);
    let s = format_findings_json(&[f], None);
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert!(v[0]["locations"][0]["file"].is_string());
}

#[test]
fn output_handles_path_with_unicode_in_filename() {
    let f = fab(7, 5, 5, "x", &[("δ.py", 1)]);
    let s = format_findings(&[f], None, &t().audit);
    assert!(s.contains("δ.py"));
}

#[test]
fn output_handles_path_with_space_in_filename() {
    let f = fab(7, 5, 5, "x", &[("hello world.py", 1)]);
    let s = format_findings(&[f], None, &t().audit);
    assert!(s.contains("hello world.py"));
}

#[test]
fn output_handles_path_with_newline_in_filename_unsupported_but_no_panic() {
    let f = fab(7, 5, 5, "x", &[("foo\nbar.py", 1)]);
    let _ = format_findings(&[f], None, &t().audit);
}

#[test]
fn output_human_prints_nothing_extra_for_zero_action_label() {
    let f = fab(7, 5, 5, "x", &[("a.py", 1)]);
    let s = format_findings(&[f], None, &t().audit);
    let action_count = s.matches("pattern action:").count();
    assert_eq!(action_count, 0);
}

#[test]
fn output_relative_path_works_with_trailing_slash_root() {
    let f = fab(7, 5, 5, "x", &[("/proj/a.py", 1)]);
    let root = PathBuf::from("/proj");
    let s = format_findings(&[f], Some(&root), &t().audit);
    assert!(s.contains("a.py:1"));
}

#[test]
fn output_handles_max_path_components_in_root_strip() {
    let p = "/a/b/c/d/e/f/g/h/i/file.py";
    let root = Path::new("/a/b/c/d/e/f/g/h/i");
    let f = fab(7, 5, 5, "x", &[(p, 10)]);
    let s = format_findings(&[f], Some(root), &t().audit);
    assert!(s.contains("file.py:10"));
}

#[test]
fn output_round_trip_human_to_json_count_matches() {
    let f1 = fab(1, 5, 5, "a", &[("a.py", 1)]);
    let f2 = fab(2, 3, 3, "b", &[("b.py", 1)]);
    let f3 = fab(3, 2, 2, "c", &[("c.py", 1)]);
    let h = format_findings(&[f1.clone(), f2.clone(), f3.clone()], None, &t().audit);
    let j = format_findings_json(&[f1, f2, f3], None);
    let v: serde_json::Value = serde_json::from_str(&j).unwrap();
    let h_count = h.matches("audit:").count();
    assert_eq!(v.as_array().unwrap().len(), h_count);
}

#[test]
fn output_locations_threshold_one_works() {
    let mut th = t().audit;
    th.max_locations_per_finding = 1;
    let f = fab(7, 3, 3, "x", &[("a.py", 1), ("b.py", 2), ("c.py", 3)]);
    let s = format_findings(&[f], None, &th);
    assert!(s.contains("(2 more)"));
}

#[test]
fn output_locations_threshold_exact_match_no_more_marker() {
    let mut th = t().audit;
    th.max_locations_per_finding = 3;
    let f = fab(7, 3, 3, "x", &[("a.py", 1), ("b.py", 2), ("c.py", 3)]);
    let s = format_findings(&[f], None, &th);
    assert!(!s.contains("more)"));
}

#[test]
fn output_with_locations_capped_renders_only_capped_count() {
    let mut th = t().audit;
    th.max_locations_per_finding = 2;
    let f = fab(7, 5, 5, "x", &[("a.py", 1), ("b.py", 2), ("c.py", 3), ("d.py", 4), ("e.py", 5)]);
    let s = format_findings(&[f], None, &th);
    assert_eq!(s.matches(".py:").count(), 2);
}

#[test]
fn output_renders_full_path_when_relative_strip_fails() {
    let f = fab(7, 5, 5, "x", &[("/random/abs/file.py", 1)]);
    let root = Path::new("/different/path");
    let s = format_findings(&[f], Some(root), &t().audit);
    assert!(s.contains("/random/abs/file.py"));
}

#[test]
fn output_json_array_contains_object_per_finding() {
    let fs = vec![
        fab(1, 5, 5, "a", &[("a.py", 1)]),
        fab(2, 4, 4, "b", &[("b.py", 1)]),
        fab(3, 3, 3, "c", &[("c.py", 1)]),
    ];
    let s = format_findings_json(&fs, None);
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(v.as_array().unwrap().len(), 3);
    for f in v.as_array().unwrap() {
        assert!(f.is_object());
    }
}

#[test]
fn output_idf_score_negative_supported() {
    let mut f = fab(7, 5, 5, "x", &[("a.py", 1)]);
    f.idf_score = Some(-1.0);
    let s = format_findings_json(&[f], None);
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert!((v[0]["idf_score"].as_f64().unwrap() + 1.0).abs() < 1e-6);
}

#[test]
fn output_json_kind_field_uppercase_camelcase() {
    let f = fab(7, 5, 5, "x", &[("a.py", 1)]);
    let s = format_findings_json(&[f], None);
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(v[0]["kind"].as_str().unwrap(), "UncategorizedPattern");
}

#[test]
fn output_human_includes_a_blank_line_between_findings() {
    let f1 = fab(1, 5, 5, "a", &[("a.py", 1)]);
    let f2 = fab(2, 3, 3, "b", &[("b.py", 1)]);
    let s = format_findings(&[f1, f2], None, &t().audit);
    assert!(s.contains("\n\n"));
}

#[test]
fn output_does_not_emit_extra_unicode_artifacts() {
    let f = fab(7, 5, 5, "ascii_only", &[("a.py", 1)]);
    let s = format_findings(&[f], None, &t().audit);
    assert!(s.is_ascii());
}
