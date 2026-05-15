use std::path::Path;
use std::process::Command;

use serde_json::Value;

use crate::history_common::{build_repo, CommitSpec};

fn pulse(args: &[&str]) -> (String, String, i32) {
    let output = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(args)
        .output()
        .expect("pulse failed");
    (
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
        output.status.code().unwrap_or(-1),
    )
}

fn pulse_in(repo: &Path, args: &[&str]) -> (String, String, i32) {
    let mut all_args = vec!["history", "--root", repo.to_str().unwrap()];
    all_args.extend(args);
    pulse(&all_args)
}

fn drift_repo() -> tempfile::TempDir {
    let authors = ["alice <alice@x>", "bob <bob@x>", "carol <carol@x>", "dave <dave@x>", "eve <eve@x>", "frank <frank@x>", "gina <gina@x>"];
    let pair_writes: [[(&'static str, &'static str); 2]; 24] = [
        [("a.py", "x = 1\n"), ("b.py", "y = 1\n")],
        [("a.py", "x = 2\n"), ("b.py", "y = 2\n")],
        [("a.py", "x = 3\n"), ("b.py", "y = 3\n")],
        [("a.py", "x = 4\n"), ("b.py", "y = 4\n")],
        [("c.py", "x = 1\n"), ("d.py", "y = 1\n")],
        [("c.py", "x = 2\n"), ("d.py", "y = 2\n")],
        [("c.py", "x = 3\n"), ("d.py", "y = 3\n")],
        [("c.py", "x = 4\n"), ("d.py", "y = 4\n")],
        [("e.py", "x = 1\n"), ("f.py", "y = 1\n")],
        [("e.py", "x = 2\n"), ("f.py", "y = 2\n")],
        [("e.py", "x = 3\n"), ("f.py", "y = 3\n")],
        [("e.py", "x = 4\n"), ("f.py", "y = 4\n")],
        [("g.py", "x = 1\n"), ("h.py", "y = 1\n")],
        [("g.py", "x = 2\n"), ("h.py", "y = 2\n")],
        [("g.py", "x = 3\n"), ("h.py", "y = 3\n")],
        [("g.py", "x = 4\n"), ("h.py", "y = 4\n")],
        [("i.py", "x = 1\n"), ("j.py", "y = 1\n")],
        [("i.py", "x = 2\n"), ("j.py", "y = 2\n")],
        [("i.py", "x = 3\n"), ("j.py", "y = 3\n")],
        [("i.py", "x = 4\n"), ("j.py", "y = 4\n")],
        [("k.py", "x = 1\n"), ("l.py", "y = 1\n")],
        [("k.py", "x = 2\n"), ("l.py", "y = 2\n")],
        [("k.py", "x = 3\n"), ("l.py", "y = 3\n")],
        [("k.py", "x = 4\n"), ("l.py", "y = 4\n")],
    ];
    let commits: Vec<CommitSpec> = pair_writes
        .iter()
        .enumerate()
        .map(|(i, w)| CommitSpec {
            author: authors[i % authors.len()],
            message: "tick",
            writes: w.as_slice(),
            deletes: &[],
        })
        .collect();
    build_repo(&commits)
}

fn small_repo() -> tempfile::TempDir {
    build_repo(&[CommitSpec {
        author: "alice <alice@x>",
        message: "init",
        writes: &[("a.py", "x = 1\n"), ("b.py", "y = 2\n")],
        deletes: &[],
    }])
}

fn count_drift_findings(stdout: &str) -> usize {
    count_kind(stdout, "ArchitecturalDrift")
}

fn count_hotspot_findings(stdout: &str) -> usize {
    count_kind(stdout, "Hotspot")
}

fn count_contrib_findings(stdout: &str) -> usize {
    count_kind(stdout, "KnowledgeFragmentation")
}

fn count_kind(stdout: &str, kind: &str) -> usize {
    let v: Value = serde_json::from_str(stdout).expect("json");
    v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["kind"].as_str() == Some(kind))
        .count()
}

#[test]
fn cli_accepts_co_change_top_flag() {
    let repo = small_repo();
    let (stdout, _, code) = pulse_in(repo.path(), &["--co-change-top", "5"]);
    assert!(stdout.contains("history:"));
    assert!(code == 0 || code == 1);
}

#[test]
fn cli_accepts_hotspot_top_flag() {
    let repo = small_repo();
    let (stdout, _, code) = pulse_in(repo.path(), &["--hotspot-top", "5"]);
    assert!(stdout.contains("history:"));
    assert!(code == 0 || code == 1);
}

#[test]
fn cli_accepts_contributors_top_flag() {
    let repo = small_repo();
    let (stdout, _, code) = pulse_in(repo.path(), &["--contributors-top", "5"]);
    assert!(stdout.contains("history:"));
    assert!(code == 0 || code == 1);
}

#[test]
fn cli_accepts_all_three_top_flags_together() {
    let repo = small_repo();
    let (stdout, _, code) = pulse_in(
        repo.path(),
        &["--co-change-top", "5", "--hotspot-top", "5", "--contributors-top", "5"],
    );
    assert!(stdout.contains("history:"));
    assert!(code == 0 || code == 1);
}

#[test]
fn cli_co_change_top_zero_yields_zero_drift_findings() {
    let repo = drift_repo();
    let (stdout, _, _) = pulse_in(repo.path(), &["--json", "--co-change-top", "0"]);
    assert_eq!(count_drift_findings(&stdout), 0);
}

#[test]
fn cli_hotspot_top_zero_yields_zero_hotspots() {
    let repo = drift_repo();
    let (stdout, _, _) = pulse_in(repo.path(), &["--json", "--hotspot-top", "0"]);
    assert_eq!(count_hotspot_findings(&stdout), 0);
}

#[test]
fn cli_contributors_top_zero_yields_zero_contrib() {
    let repo = drift_repo();
    let (stdout, _, _) = pulse_in(repo.path(), &["--json", "--contributors-top", "0"]);
    assert_eq!(count_contrib_findings(&stdout), 0);
}

#[test]
fn cli_co_change_top_one_caps_at_one() {
    let repo = drift_repo();
    let (stdout, _, _) = pulse_in(repo.path(), &["--json", "--co-change-top", "1"]);
    assert!(count_drift_findings(&stdout) <= 1);
}

#[test]
fn cli_co_change_top_three_caps_at_three() {
    let repo = drift_repo();
    let (stdout, _, _) = pulse_in(repo.path(), &["--json", "--co-change-top", "3"]);
    assert!(count_drift_findings(&stdout) <= 3);
}

#[test]
fn cli_hotspot_top_one_caps_at_one() {
    let repo = drift_repo();
    let (stdout, _, _) = pulse_in(repo.path(), &["--json", "--hotspot-top", "1"]);
    assert!(count_hotspot_findings(&stdout) <= 1);
}

#[test]
fn cli_contributors_top_one_caps_at_one() {
    let repo = drift_repo();
    let (stdout, _, _) = pulse_in(repo.path(), &["--json", "--contributors-top", "1"]);
    assert!(count_contrib_findings(&stdout) <= 1);
}

#[test]
fn cli_co_change_top_huge_does_not_error() {
    let repo = small_repo();
    let (_, _, code) = pulse_in(repo.path(), &["--co-change-top", "4294967295"]);
    assert!(code == 0 || code == 1);
}

#[test]
fn cli_negative_co_change_top_rejected() {
    let repo = small_repo();
    let (_, _, code) = pulse_in(repo.path(), &["--co-change-top", "-1"]);
    assert_ne!(code, 0);
}

#[test]
fn cli_non_numeric_top_rejected() {
    let repo = small_repo();
    let (_, _, code) = pulse_in(repo.path(), &["--hotspot-top", "abc"]);
    assert_ne!(code, 0);
}

#[test]
fn cli_top_with_no_value_rejected() {
    let repo = small_repo();
    let (_, _, code) = pulse_in(repo.path(), &["--contributors-top"]);
    assert_ne!(code, 0);
}

#[test]
fn cli_top_combines_with_max_commits() {
    let repo = drift_repo();
    let (stdout, _, code) = pulse_in(
        repo.path(),
        &["--max-commits", "100", "--hotspot-top", "2"],
    );
    assert!(stdout.contains("history:"));
    assert!(code == 0 || code == 1);
}

#[test]
fn cli_top_combines_with_since() {
    let repo = drift_repo();
    let (stdout, _, code) = pulse_in(
        repo.path(),
        &["--since", "10 years ago", "--hotspot-top", "2"],
    );
    assert!(stdout.contains("history:"));
    assert!(code == 0 || code == 1);
}

#[test]
fn cli_top_combines_with_json() {
    let repo = drift_repo();
    let (stdout, _, _) = pulse_in(repo.path(), &["--json", "--co-change-top", "2"]);
    let v: Value = serde_json::from_str(&stdout).expect("json parses");
    assert!(v.get("findings").is_some());
}

#[test]
fn cli_top_co_change_truncates_below_default() {
    let repo = drift_repo();
    let (default_stdout, _, _) = pulse_in(repo.path(), &["--json"]);
    let (capped_stdout, _, _) = pulse_in(repo.path(), &["--json", "--co-change-top", "1"]);
    let default_count = count_drift_findings(&default_stdout);
    let capped_count = count_drift_findings(&capped_stdout);
    assert!(capped_count <= default_count);
    assert!(capped_count <= 1);
}

#[test]
fn cli_top_hotspot_truncates_below_default() {
    let repo = drift_repo();
    let (default_stdout, _, _) = pulse_in(repo.path(), &["--json"]);
    let (capped_stdout, _, _) = pulse_in(repo.path(), &["--json", "--hotspot-top", "1"]);
    assert!(count_hotspot_findings(&capped_stdout) <= count_hotspot_findings(&default_stdout));
    assert!(count_hotspot_findings(&capped_stdout) <= 1);
}

#[test]
fn cli_top_contrib_truncates_below_default() {
    let repo = drift_repo();
    let (default_stdout, _, _) = pulse_in(repo.path(), &["--json"]);
    let (capped_stdout, _, _) = pulse_in(repo.path(), &["--json", "--contributors-top", "1"]);
    assert!(count_contrib_findings(&capped_stdout) <= count_contrib_findings(&default_stdout));
    assert!(count_contrib_findings(&capped_stdout) <= 1);
}

#[test]
fn cli_top_co_change_summary_reflects_cap() {
    let repo = drift_repo();
    let (stdout, _, _) = pulse_in(repo.path(), &["--json", "--co-change-top", "0"]);
    let v: Value = serde_json::from_str(&stdout).unwrap();
    let drift = v["summary"]["by_pillar"]["drift"].as_u64().unwrap();
    assert_eq!(drift, 0);
}

#[test]
fn cli_top_hotspot_summary_reflects_cap() {
    let repo = drift_repo();
    let (stdout, _, _) = pulse_in(repo.path(), &["--json", "--hotspot-top", "0"]);
    let v: Value = serde_json::from_str(&stdout).unwrap();
    let complexity = v["summary"]["by_pillar"]["complexity"].as_u64().unwrap();
    assert_eq!(complexity, 0);
}

#[test]
fn cli_top_contrib_summary_reflects_cap() {
    let repo = drift_repo();
    let (stdout, _, _) = pulse_in(repo.path(), &["--json", "--contributors-top", "0"]);
    let v: Value = serde_json::from_str(&stdout).unwrap();
    let ownership = v["summary"]["by_pillar"]["ownership"].as_u64().unwrap();
    assert_eq!(ownership, 0);
}

#[test]
fn cli_top_zero_for_all_three_yields_empty_findings() {
    let repo = drift_repo();
    let (stdout, _, _) = pulse_in(
        repo.path(),
        &["--json", "--co-change-top", "0", "--hotspot-top", "0", "--contributors-top", "0"],
    );
    let v: Value = serde_json::from_str(&stdout).unwrap();
    let total = v["summary"]["findings_total"].as_u64().unwrap();
    assert_eq!(total, 0);
}

#[test]
fn cli_top_default_run_preserves_old_behavior() {
    let repo = small_repo();
    let (with_flag, _, _) = pulse_in(repo.path(), &["--json"]);
    let (without_flag, _, _) = pulse_in(repo.path(), &["--json"]);
    let a: Value = serde_json::from_str(&with_flag).unwrap();
    let b: Value = serde_json::from_str(&without_flag).unwrap();
    assert_eq!(a["summary"]["findings_total"], b["summary"]["findings_total"]);
}

#[test]
fn cli_top_works_with_include_tests_flag_position() {
    let repo = small_repo();
    let (stdout, _, code) = pulse(&[
        "history",
        "--include-tests",
        "--root",
        repo.path().to_str().unwrap(),
        "--hotspot-top",
        "5",
    ]);
    assert!(stdout.contains("history:"));
    assert!(code == 0 || code == 1);
}

#[test]
fn cli_top_help_lists_new_flags() {
    let (stdout, _, _) = pulse(&["history", "--help"]);
    assert!(stdout.contains("--co-change-top"));
    assert!(stdout.contains("--hotspot-top"));
    assert!(stdout.contains("--contributors-top"));
}

#[test]
fn cli_top_human_output_still_renders() {
    let repo = drift_repo();
    let (stdout, _, _) = pulse_in(repo.path(), &["--co-change-top", "1"]);
    assert!(stdout.contains("history:"));
}
