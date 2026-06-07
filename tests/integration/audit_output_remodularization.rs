use std::path::PathBuf;

use pulse::audit::finding::{
    AuditFinding, AuditKind, AuditLocation, ImportConfidence, MergeComponentsEvidence, MoveFileEvidence,
};
use pulse::audit::output::{format_findings, format_findings_json};
use pulse::thresholds::Thresholds;

fn t() -> Thresholds {
    Thresholds::default()
}

fn p(s: &str) -> PathBuf {
    PathBuf::from(s)
}

fn move_finding(file: &str, current: &str, target: &str, size: u32, share: f64) -> AuditFinding {
    AuditFinding {
        kind: AuditKind::MoveFile(MoveFileEvidence {
            file: p(file),
            current_dir: p(current),
            target_dir: p(target),
            community_size: size,
            home_share: share,
            confidence: ImportConfidence::Medium,
        }),
        representative_snippet: String::new(),
        support: size,
        file_count: 1,
        idf_score: None,
        action_label: None,
        pattern_category: None,
        locality_entropy: None,
        p_value: None,
        locations: vec![AuditLocation { file: p(file), line: 1 }],
    }
}

fn merge_finding(dirs: &[&str], files: u32) -> AuditFinding {
    let components: Vec<PathBuf> = dirs.iter().map(|d| p(d)).collect();
    AuditFinding {
        kind: AuditKind::MergeComponents(MergeComponentsEvidence {
            components: components.clone(),
            community_files: files,
            confidence: ImportConfidence::Medium,
        }),
        representative_snippet: String::new(),
        support: files,
        file_count: files,
        idf_score: None,
        action_label: None,
        pattern_category: None,
        locality_entropy: None,
        p_value: None,
        locations: components.iter().map(|c| AuditLocation { file: c.clone(), line: 1 }).collect(),
    }
}

#[test]
fn move_file_human_render() {
    let out = format_findings(&[move_finding("util/helper.rs", "util", "core", 5, 0.8)], None, &t().audit);
    assert!(out.contains("file to relocate"), "got: {out}");
    assert!(out.contains("util/helper.rs"), "got: {out}");
    assert!(out.contains("core"), "target dir shown: {out}");
    assert!(out.contains("% in target"), "got: {out}");
    assert!(out.contains("consider relocating"), "advisory action shown: {out}");
}

#[test]
fn move_file_json_render() {
    let out = format_findings_json(&[move_finding("util/helper.rs", "util", "core", 5, 0.8)], None);
    assert!(serde_json::from_str::<serde_json::Value>(&out).is_ok(), "valid JSON: {out}");
    for field in ["\"MoveFile\"", "current_dir", "target_dir", "community_size", "home_share"] {
        assert!(out.contains(field), "missing {field} in {out}");
    }
}

#[test]
fn merge_components_human_render() {
    let out = format_findings(&[merge_finding(&["core", "shared"], 6)], None, &t().audit);
    assert!(out.contains("components to merge"), "got: {out}");
    assert!(out.contains("core"), "got: {out}");
    assert!(out.contains("shared"), "got: {out}");
    assert!(out.contains("shared community"), "got: {out}");
    assert!(out.contains("consider merging"), "advisory action shown: {out}");
}

#[test]
fn merge_components_json_render() {
    let out = format_findings_json(&[merge_finding(&["core", "shared"], 6)], None);
    assert!(serde_json::from_str::<serde_json::Value>(&out).is_ok(), "valid JSON: {out}");
    for field in ["\"MergeComponents\"", "components", "community_files"] {
        assert!(out.contains(field), "missing {field} in {out}");
    }
}
