use std::path::Path;
use std::process::Command;

use crate::history_common::{build_repo, CommitSpec};

fn pulse_history(repo: &Path, analytics: &Path, extra: &[&str]) -> (String, String, i32) {
    let mut args = vec!["history", "--root", repo.to_str().unwrap()];
    args.extend_from_slice(extra);
    let output = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(&args)
        .env("PULSE_ANALYTICS_DIR", analytics)
        .output()
        .expect("pulse failed to run");
    (
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
        output.status.code().unwrap_or(-1),
    )
}

fn single_commit_repo() -> tempfile::TempDir {
    build_repo(&[CommitSpec {
        author: "alice <alice@x>",
        message: "init",
        writes: &[("a.py", "def f():\n    return 1\n"), ("b.py", "def g():\n    return 2\n")],
        deletes: &[],
    }])
}

fn break_object_store(repo: &Path) {
    let objects = repo.join(".git").join("objects");
    for entry in std::fs::read_dir(&objects).expect("read .git/objects").filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            std::fs::remove_dir_all(&path).expect("remove object subdir");
        } else {
            std::fs::remove_file(&path).expect("remove object file");
        }
    }
}

#[test]
fn calibrate_without_pulse_config_takes_none_branch_and_succeeds() {
    let repo = single_commit_repo();
    let analytics = tempfile::tempdir().unwrap();
    assert!(
        !repo.path().join(".pulse.toml").exists(),
        "fixture repo must have no config so calibrate hits the None config branch",
    );
    let (stdout, stderr, code) = pulse_history(repo.path(), analytics.path(), &["--jit-calibrate"]);
    assert_eq!(code, 0, "calibrate with no config should succeed via the None branch (stderr: {stderr})");
    assert!(stdout.contains("jit calibration written:"), "expected calibrate success message: {stdout}");
}

#[test]
fn run_on_repo_with_corrupt_objects_reports_git_failed() {
    let repo = single_commit_repo();
    break_object_store(repo.path());
    let analytics = tempfile::tempdir().unwrap();
    let (_, stderr, code) = pulse_history(repo.path(), analytics.path(), &[]);
    assert_eq!(code, 2, "git log failure must dispatch to handle_error and exit 2 (stderr: {stderr})");
    assert!(stderr.contains("git failed"), "expected GitFailed message from handle_error: {stderr}");
}

#[test]
fn run_git_failed_message_includes_exit_code_marker() {
    let repo = single_commit_repo();
    break_object_store(repo.path());
    let analytics = tempfile::tempdir().unwrap();
    let (_, stderr, _) = pulse_history(repo.path(), analytics.path(), &[]);
    assert!(stderr.contains("code"), "GitFailed line formats the git exit code: {stderr}");
}

#[test]
fn calibrate_on_repo_with_corrupt_objects_reports_git_failed() {
    let repo = single_commit_repo();
    break_object_store(repo.path());
    let analytics = tempfile::tempdir().unwrap();
    let (_, stderr, code) = pulse_history(repo.path(), analytics.path(), &["--jit-calibrate"]);
    assert_eq!(code, 2, "calibrate git log failure must dispatch to handle_error and exit 2 (stderr: {stderr})");
    assert!(stderr.contains("git failed"), "expected GitFailed message from calibrate path: {stderr}");
}
