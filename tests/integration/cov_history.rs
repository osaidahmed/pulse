use std::path::{Path, PathBuf};
use std::process::Command;

use pulse::history::finding::{
    variant_info, BuildCoChangeEvidence, FragmentationEvidence, HistoryFinding, HistoryKind, HistoryPillar,
};
use pulse::history::output::{format_findings, format_findings_json};

use crate::history_common::{build_repo, CommitSpec};

fn build_cochange(source: &str, build: &str, support: u32, source_revisions: u32, ratio: f64) -> HistoryFinding {
    HistoryFinding {
        kind: HistoryKind::BuildCoChange(BuildCoChangeEvidence {
            build_file: PathBuf::from(build),
            source_file: PathBuf::from(source),
            support,
            source_revisions,
            ratio,
            last_seen_unix: 1_700_000_000,
            distinct_authors: 2,
        }),
        action_label: None,
    }
}

fn ownership_no_authors(file: &str, total: u32, minor: u32) -> HistoryFinding {
    HistoryFinding {
        kind: HistoryKind::KnowledgeFragmentation(FragmentationEvidence {
            file: PathBuf::from(file),
            total_contributors: total,
            minor_contributor_count: minor,
            minor_contributor_pct: f64::from(minor) / f64::from(total),
            top_minor_authors: Vec::new(),
        }),
        action_label: None,
    }
}

#[test]
fn build_cochange_renders_source_to_build_arrow_line() {
    let out = format_findings(&[build_cochange("src/lib.rs", "Cargo.toml", 8, 10, 0.8)], None);
    assert!(out.contains("src/lib.rs ↔ Cargo.toml"), "got: {out}");
    assert!(out.contains("8 of 10 revisions"), "got: {out}");
    assert!(out.contains("80% co-change"), "ratio rendered as percent: {out}");
    assert!(out.contains("2 authors"), "got: {out}");
}

#[test]
fn build_cochange_lives_under_evolution_section_in_human_output() {
    let out = format_findings(&[build_cochange("src/lib.rs", "Cargo.toml", 8, 10, 0.8)], None);
    assert!(out.contains("evolutionary smells — 1 findings"), "build co-change is an evolution smell: {out}");
}

#[test]
fn build_cochange_dispatches_to_evolution_pillar() {
    let f = build_cochange("src/lib.rs", "Cargo.toml", 8, 10, 0.8);
    assert_eq!(variant_info(&f.kind).pillar, HistoryPillar::Evolution);
}

#[test]
fn build_cochange_slug_and_label_describe_coupling() {
    let f = build_cochange("src/lib.rs", "Cargo.toml", 8, 10, 0.8);
    let info = variant_info(&f.kind);
    assert_eq!(info.slug, "build_co_change");
    assert_eq!(info.label, "build co-change coupling");
    assert!(!info.action.is_empty());
}

#[test]
fn build_cochange_renders_json_with_build_and_source_fields() {
    let json_str = format_findings_json(&[build_cochange("src/lib.rs", "Cargo.toml", 8, 10, 0.8)], None);
    let v: serde_json::Value = serde_json::from_str(&json_str).expect("json parses");
    let f = &v["findings"][0];
    assert_eq!(f["kind"], "BuildCoChange");
    assert_eq!(f["build_file"], "Cargo.toml");
    assert_eq!(f["source_file"], "src/lib.rs");
    assert_eq!(f["support"], 8);
    assert_eq!(f["source_revisions"], 10);
    assert_eq!(f["distinct_authors"], 2);
}

#[test]
fn build_cochange_json_counts_under_evolution_pillar() {
    let json_str = format_findings_json(&[build_cochange("src/lib.rs", "Cargo.toml", 8, 10, 0.8)], None);
    let v: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert_eq!(v["summary"]["by_pillar"]["evolution"], 1);
}

#[test]
fn build_cochange_json_carries_action_when_set() {
    let mut f = build_cochange("src/lib.rs", "Cargo.toml", 8, 10, 0.8);
    f.action_label = Some("check the manifest");
    let json_str = format_findings_json(&[f], None);
    let v: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert_eq!(v["findings"][0]["action"], "check the manifest");
}

#[test]
fn ownership_with_no_minor_authors_omits_author_suffix() {
    let out = format_findings(&[ownership_no_authors("src/o.rs", 10, 6)], None);
    assert!(out.contains("total=10, minor=6 (60%)"), "got: {out}");
    assert!(!out.contains("(60%) —"), "no author suffix dash when authors empty: {out}");
}

#[test]
fn ownership_with_no_minor_authors_json_has_empty_authors_array() {
    let json_str = format_findings_json(&[ownership_no_authors("src/o.rs", 10, 6)], None);
    let v: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    let authors = v["findings"][0]["top_minor_authors"].as_array().expect("authors is array");
    assert!(authors.is_empty(), "expected empty authors array: {json_str}");
}

#[test]
fn header_root_label_uses_given_path() {
    let out = format_findings(&[build_cochange("src/lib.rs", "Cargo.toml", 8, 10, 0.8)], Some(Path::new("/proj")));
    assert!(out.contains("history: /proj — 1 findings"), "got: {out}");
}

fn pulse_history(args: &[&str]) -> (String, String, i32) {
    let output = Command::new(env!("CARGO_BIN_EXE_pulse")).args(args).output().expect("pulse failed to run");
    (
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
        output.status.code().unwrap_or(-1),
    )
}

fn pulse_history_with_analytics(args: &[&str], analytics_dir: &Path) -> (String, String, i32) {
    let output = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(args)
        .env("PULSE_ANALYTICS_DIR", analytics_dir)
        .output()
        .expect("pulse failed to run");
    (
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
        output.status.code().unwrap_or(-1),
    )
}

fn drift_repo() -> tempfile::TempDir {
    let mut commits = Vec::new();
    for i in 0..5 {
        let body = format!("v = {i}\n");
        commits.push(CommitSpec {
            author: "alice <alice@x>",
            message: Box::leak(format!("r{i}").into_boxed_str()),
            writes: Box::leak(Box::new([
                ("a.py", Box::leak(body.clone().into_boxed_str()) as &str),
                ("b.py", Box::leak(body.into_boxed_str()) as &str),
            ])),
            deletes: &[],
        });
    }
    for i in 0..2 {
        let body = format!("noise {i}\n");
        commits.push(CommitSpec {
            author: "alice <alice@x>",
            message: Box::leak(format!("n{i}").into_boxed_str()),
            writes: Box::leak(Box::new([("NOTES.md", Box::leak(body.into_boxed_str()) as &str)])),
            deletes: &[],
        });
    }
    build_repo(&commits)
}

#[test]
fn run_with_findings_prints_human_output_and_exits_one() {
    let repo = drift_repo();
    let (stdout, _, code) = pulse_history(&["history", "--root", repo.path().to_str().unwrap()]);
    assert_eq!(code, 1, "findings present should exit 1");
    assert!(stdout.contains("architectural drift — 1 findings"), "got: {stdout}");
    assert!(stdout.contains("a.py ↔"), "drift pair rendered: {stdout}");
}

#[test]
fn run_with_findings_json_emits_finding_and_exits_one() {
    let repo = drift_repo();
    let (stdout, _, code) = pulse_history(&["history", "--root", repo.path().to_str().unwrap(), "--json"]);
    assert_eq!(code, 1, "findings present should exit 1");
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("json parses");
    assert_eq!(v["summary"]["findings_total"], 1);
    assert_eq!(v["findings"][0]["kind"], "ArchitecturalDrift");
}

#[test]
fn run_with_top_overrides_clamps_and_still_runs() {
    let repo = drift_repo();
    let (stdout, _, code) = pulse_history(&[
        "history",
        "--root",
        repo.path().to_str().unwrap(),
        "--co-change-top",
        "1",
        "--hotspot-top",
        "1",
        "--contributors-top",
        "1",
    ]);
    assert!(code == 0 || code == 1, "override run exits 0 or 1, got {code}");
    assert!(stdout.contains("history:"), "header present with overrides: {stdout}");
}

#[test]
fn run_with_hist_and_arch_trend_flags_runs() {
    let repo = drift_repo();
    let (stdout, _, code) =
        pulse_history(&["history", "--root", repo.path().to_str().unwrap(), "--hist", "--arch-trend"]);
    assert!(code == 0 || code == 1, "opt-in flags exit 0 or 1, got {code}");
    assert!(stdout.contains("history:"), "header present with opt-in flags: {stdout}");
}

#[test]
fn run_with_include_tests_global_flag_runs() {
    let repo = drift_repo();
    let (stdout, _, code) = pulse_history(&["-t", "history", "--root", repo.path().to_str().unwrap()]);
    assert!(code == 0 || code == 1, "include-tests exits 0 or 1, got {code}");
    assert!(stdout.contains("history:"), "header present with include-tests: {stdout}");
}

#[test]
fn run_with_since_flag_runs() {
    let repo = drift_repo();
    let (stdout, _, code) =
        pulse_history(&["history", "--root", repo.path().to_str().unwrap(), "--since", "30 years ago"]);
    assert!(code == 0 || code == 1, "since flag exits 0 or 1, got {code}");
    assert!(stdout.contains("history:"), "header present with since: {stdout}");
}

#[test]
fn run_with_max_commits_flag_runs() {
    let repo = drift_repo();
    let (stdout, _, code) = pulse_history(&["history", "--root", repo.path().to_str().unwrap(), "--max-commits", "3"]);
    assert!(code == 0 || code == 1, "max-commits exits 0 or 1, got {code}");
    assert!(stdout.contains("history:"), "header present with max-commits: {stdout}");
}

#[test]
fn run_on_non_git_directory_dispatches_to_handle_error_not_a_repo() {
    let dir = tempfile::tempdir().unwrap();
    let (_, stderr, code) = pulse_history(&["history", "--root", dir.path().to_str().unwrap()]);
    assert_eq!(code, 2, "handle_error exits 2");
    assert!(stderr.contains("not a git repository"), "expected NotAGitRepo message: {stderr}");
}

#[test]
fn calibrate_write_failure_reports_error_and_exits_two() {
    let repo = drift_repo();
    let analytics = tempfile::tempdir().unwrap();
    std::fs::write(analytics.path().join("jit"), "occupied").expect("write blocker file");
    let (_, stderr, code) = pulse_history_with_analytics(
        &["history", "--root", repo.path().to_str().unwrap(), "--jit-calibrate"],
        analytics.path(),
    );
    assert_eq!(code, 2, "calibration write failure should exit 2");
    assert!(stderr.contains("failed to write jit calibration"), "expected write-failure message: {stderr}");
}
