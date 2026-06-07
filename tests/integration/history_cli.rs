use std::path::Path;
use std::process::Command;

use crate::history_common::{build_repo, CommitSpec};

fn pulse(args: &[&str]) -> (String, String, i32) {
    let output = Command::new(env!("CARGO_BIN_EXE_pulse")).args(args).output().expect("pulse failed");
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

fn simple_repo() -> tempfile::TempDir {
    build_repo(&[CommitSpec {
        author: "alice <alice@x>",
        message: "init",
        writes: &[("a.py", "x = 1\n"), ("b.py", "y = 2\n")],
        deletes: &[],
    }])
}

#[test]
fn cli_history_runs_on_simple_repo() {
    let repo = simple_repo();
    let (stdout, _, code) = pulse_in(repo.path(), &[]);
    assert!(stdout.contains("history:"), "expected header in stdout: {stdout}");
    assert!(code == 0 || code == 1, "should exit 0 or 1, got {code}");
}

#[test]
fn cli_history_human_output_has_header_and_pillars() {
    let repo = simple_repo();
    let (stdout, _, _) = pulse_in(repo.path(), &[]);
    assert!(stdout.contains("history:"));
    assert!(stdout.contains("architectural drift"));
    assert!(stdout.contains("hotspots"));
    assert!(stdout.contains("knowledge fragmentation"));
}

#[test]
fn cli_history_json_outputs_parseable_envelope() {
    let repo = simple_repo();
    let (stdout, _, _) = pulse_in(repo.path(), &["--json"]);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("json parses");
    assert!(v.get("summary").is_some());
    assert!(v.get("findings").is_some());
}

#[test]
fn cli_history_not_a_git_repo_exits_nonzero_with_stderr_message() {
    let dir = tempfile::tempdir().unwrap();
    let (_, stderr, code) = pulse(&["history", "--root", dir.path().to_str().unwrap()]);
    assert_ne!(code, 0);
    assert!(stderr.contains("not a git repository"), "expected error msg: {stderr}");
}

#[test]
fn cli_history_zero_commits_repo_clean_exit() {
    let dir = tempfile::tempdir().unwrap();
    let _ = Command::new("git").arg("-C").arg(dir.path()).args(["init", "-q", "-b", "main"]).status();
    let (stdout, _, code) = pulse(&["history", "--root", dir.path().to_str().unwrap()]);
    assert_eq!(code, 0, "empty repo should exit 0");
    assert!(stdout.contains("history:"));
}

#[test]
fn cli_history_with_max_commits_flag() {
    let repo = simple_repo();
    let (stdout, _, _) = pulse_in(repo.path(), &["--max-commits", "1"]);
    assert!(stdout.contains("history:"));
}

#[test]
fn cli_history_with_since_flag() {
    let repo = simple_repo();
    let (stdout, _, _) = pulse_in(repo.path(), &["--since", "10 years ago"]);
    assert!(stdout.contains("history:"));
}

#[test]
fn cli_history_unknown_flag_returns_error_code() {
    let repo = simple_repo();
    let (_, _, code) = pulse_in(repo.path(), &["--bogus-flag"]);
    assert_ne!(code, 0);
}

#[test]
fn cli_history_json_summary_root_field_present() {
    let repo = simple_repo();
    let (stdout, _, _) = pulse_in(repo.path(), &["--json"]);
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(v["summary"]["root"].is_string());
}

#[test]
fn cli_history_json_summary_has_findings_total() {
    let repo = simple_repo();
    let (stdout, _, _) = pulse_in(repo.path(), &["--json"]);
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(v["summary"]["findings_total"].is_number());
}

#[test]
fn cli_history_json_summary_has_by_pillar() {
    let repo = simple_repo();
    let (stdout, _, _) = pulse_in(repo.path(), &["--json"]);
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(v["summary"]["by_pillar"]["drift"].is_number());
    assert!(v["summary"]["by_pillar"]["complexity"].is_number());
    assert!(v["summary"]["by_pillar"]["ownership"].is_number());
}

#[test]
fn cli_history_root_does_not_exist_errors_out() {
    let (_, stderr, code) = pulse(&["history", "--root", "/nonexistent/path/xyz12345"]);
    assert_ne!(code, 0);
    assert!(!stderr.is_empty());
}
