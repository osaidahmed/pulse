use std::path::PathBuf;

use pulse::history::finding::{BlobEvidence, ChangeShotgunEvidence, HistoryFinding, HistoryKind};
use pulse::history::output::{format_findings, format_findings_json};

fn blob(file: &str, multi: u32, total: u32, ratio: f64) -> HistoryFinding {
    HistoryFinding {
        kind: HistoryKind::FileBlob(BlobEvidence {
            file: PathBuf::from(file),
            multi_file_commits: multi,
            total_multi_file_commits: total,
            blob_ratio: ratio,
        }),
        action_label: None,
    }
}

fn shotgun(file: &str, partner_count: u32, packages: &[&str]) -> HistoryFinding {
    HistoryFinding {
        kind: HistoryKind::ChangeShotgun(ChangeShotgunEvidence {
            file: PathBuf::from(file),
            partner_count,
            package_count: packages.len() as u32,
            packages: packages.iter().map(|&p| PathBuf::from(p)).collect(),
        }),
        action_label: None,
    }
}

#[test]
fn blob_finding_renders_multi_file_commit_ratio() {
    let out = format_findings(&[blob("src/util.rs", 30, 40, 0.75)], None);
    assert!(out.contains("src/util.rs"), "got: {out}");
    assert!(out.contains("30 of 40 multi-file commits"), "got: {out}");
    assert!(out.contains("(75%)"), "ratio rendered as percent: {out}");
    assert!(out.contains("evolutionary smells"), "blob lives under evolution: {out}");
}

#[test]
fn shotgun_finding_renders_partner_and_package_counts() {
    let out = format_findings(&[shotgun("src/core.rs", 8, &["pkg/a", "pkg/b", "pkg/c"])], None);
    assert!(out.contains("src/core.rs"), "got: {out}");
    assert!(out.contains("8 confident partners across 3 packages"), "got: {out}");
    assert!(out.contains("evolutionary smells"), "shotgun lives under evolution: {out}");
}

#[test]
fn blob_renders_json_with_evidence_fields() {
    let json_str = format_findings_json(&[blob("src/util.rs", 30, 40, 0.75)], None);
    let v: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    let f = &v["findings"][0];
    assert_eq!(f["kind"], "FileBlob");
    assert_eq!(f["file"], "src/util.rs");
    assert_eq!(f["multi_file_commits"], 30);
    assert_eq!(f["total_multi_file_commits"], 40);
}

#[test]
fn shotgun_renders_json_with_packages_array() {
    let json_str = format_findings_json(&[shotgun("src/core.rs", 8, &["pkg/a", "pkg/b", "pkg/c"])], None);
    let v: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    let f = &v["findings"][0];
    assert_eq!(f["kind"], "ChangeShotgun");
    assert_eq!(f["file"], "src/core.rs");
    assert_eq!(f["partner_count"], 8);
    assert_eq!(f["package_count"], 3);
    let pkgs = f["packages"].as_array().expect("packages is an array");
    assert_eq!(pkgs.len(), 3);
    assert_eq!(pkgs[0], "pkg/a");
}

#[test]
fn shotgun_json_counts_under_evolution_pillar() {
    let json_str = format_findings_json(&[shotgun("src/core.rs", 4, &["pkg/x", "pkg/y"])], None);
    let v: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert_eq!(v["summary"]["by_pillar"]["evolution"], 1);
}
