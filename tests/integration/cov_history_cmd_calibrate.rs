use std::path::Path;
use std::process::Command;

use crate::history_common::{build_repo, CommitSpec};

fn pulse_calibrate(repo: &Path, analytics_dir: &Path) -> (String, String, i32) {
    let output = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["history", "--root", repo.to_str().unwrap(), "--jit-calibrate"])
        .env("PULSE_ANALYTICS_DIR", analytics_dir)
        .output()
        .expect("pulse failed to run");
    (
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
        output.status.code().unwrap_or(-1),
    )
}

fn calibrate_in(root: &Path, analytics_dir: &Path) -> (String, String, i32) {
    let output = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["history", "--root", root.to_str().unwrap(), "--jit-calibrate"])
        .env("PULSE_ANALYTICS_DIR", analytics_dir)
        .output()
        .expect("pulse failed to run");
    (
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
        output.status.code().unwrap_or(-1),
    )
}

fn typed_repo() -> tempfile::TempDir {
    build_repo(&[
        CommitSpec {
            author: "alice <alice@x>",
            message: "init",
            writes: &[("a.py", "def f():\n    return 1\n"), ("b.py", "def g():\n    return 2\n")],
            deletes: &[],
        },
        CommitSpec {
            author: "bob <bob@x>",
            message: "edit",
            writes: &[("a.py", "def f():\n    return 11\n")],
            deletes: &[],
        },
    ])
}

#[test]
fn calibrate_success_writes_jit_calibration_and_exits_zero() {
    let repo = typed_repo();
    let analytics = tempfile::tempdir().unwrap();
    let (stdout, stderr, code) = pulse_calibrate(repo.path(), analytics.path());
    assert_eq!(code, 0, "calibration success should exit 0 (stderr: {stderr})");
    assert!(stdout.contains("jit calibration written:"), "expected success message in stdout: {stdout}");
}

#[test]
fn calibrate_success_persists_calibration_file_under_analytics_dir() {
    let repo = typed_repo();
    let analytics = tempfile::tempdir().unwrap();
    let (_, _, code) = pulse_calibrate(repo.path(), analytics.path());
    assert_eq!(code, 0);
    let jit_dir = analytics.path().join("jit");
    assert!(jit_dir.is_dir(), "calibrate should create the jit subdir under the analytics dir");
    let written: Vec<_> = std::fs::read_dir(&jit_dir)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .collect();
    assert!(!written.is_empty(), "expected a calibration json file to be written under {jit_dir:?}");
}

#[test]
fn calibrate_on_non_git_dir_dispatches_to_handle_error() {
    let not_a_repo = tempfile::tempdir().unwrap();
    let analytics = tempfile::tempdir().unwrap();
    let (_, stderr, code) = calibrate_in(not_a_repo.path(), analytics.path());
    assert_eq!(code, 2, "handle_error exits with code 2");
    assert!(stderr.contains("not a git repository"), "expected NotAGitRepo message from handle_error: {stderr}");
}

#[test]
fn calibrate_on_missing_root_errors_nonzero() {
    let analytics = tempfile::tempdir().unwrap();
    let (_, stderr, code) = calibrate_in(Path::new("/nonexistent/path/xyz98765"), analytics.path());
    assert_ne!(code, 0, "missing root must not exit 0");
    assert!(!stderr.is_empty(), "expected an error message on stderr");
}
