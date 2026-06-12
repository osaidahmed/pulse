use crate::common::t;
use std::process::Command;

struct GitProject {
    dir: tempfile::TempDir,
    baseline_dir: tempfile::TempDir,
}

impl GitProject {
    fn new() -> Self {
        let p = Self { dir: tempfile::tempdir().unwrap(), baseline_dir: tempfile::tempdir().unwrap() };
        p.git(&["init", "-q"]);
        p
    }

    fn plain_dirs() -> Self {
        Self { dir: tempfile::tempdir().unwrap(), baseline_dir: tempfile::tempdir().unwrap() }
    }

    fn git(&self, args: &[&str]) {
        let output = Command::new("git")
            .arg("-C")
            .arg(self.dir.path())
            .args(["-c", "user.email=t@example.com", "-c", "user.name=t", "-c", "commit.gpgsign=false"])
            .args(args)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .output()
            .expect("git available in test environment");
        assert!(output.status.success(), "git {args:?} failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    fn write(&self, name: &str, body: &str) {
        std::fs::write(self.dir.path().join(name), body).unwrap();
    }

    fn commit_all(&self) {
        self.git(&["add", "-A"]);
        self.git(&["commit", "-q", "-m", "x", "--allow-empty"]);
    }

    fn pulse(&self, flag: &str) -> String {
        let output = Command::new(env!("CARGO_BIN_EXE_pulse"))
            .arg(flag)
            .current_dir(self.dir.path())
            .env("PULSE_BASELINE_DIR", self.baseline_dir.path())
            .stdin(std::process::Stdio::null())
            .output()
            .expect("failed to run pulse");
        String::from_utf8(output.stdout).unwrap()
    }
}

fn calm_fn(name: &str) -> String {
    format!("def {name}(flag):\n    if flag:\n        x = 0\n    return flag\n")
}

fn spicy_fn(name: &str) -> String {
    let ifs = t().function.cc_warning;
    let mut s = format!("def {name}(flag):\n");
    for _ in 0..ifs {
        s.push_str("    if flag:\n        x = 0\n");
    }
    s
}

#[test]
fn out_of_band_edit_is_caught_at_stop() {
    let p = GitProject::new();
    p.write("svc.py", &calm_fn("worker"));
    p.commit_all();
    p.pulse("--cleanup");
    p.write("svc.py", &spicy_fn("worker"));
    let out = p.pulse("--stop");
    assert!(out.to_lowercase().contains("complex method"), "edits made outside the hook must surface: {out}");
}

#[test]
fn committed_mid_turn_edit_is_caught_at_stop() {
    let p = GitProject::new();
    p.write("svc.py", &calm_fn("worker"));
    p.commit_all();
    p.pulse("--cleanup");
    p.write("svc.py", &spicy_fn("worker"));
    p.commit_all();
    let out = p.pulse("--stop");
    assert!(out.to_lowercase().contains("complex method"), "committing does not hide a turn regression: {out}");
}

#[test]
fn clean_turn_is_silent() {
    let p = GitProject::new();
    p.write("svc.py", &calm_fn("worker"));
    p.commit_all();
    p.pulse("--cleanup");
    let out = p.pulse("--stop");
    assert!(out.is_empty(), "no changes means no stop output: {out}");
}

#[test]
fn committed_regression_is_not_relitigated_next_turn() {
    let p = GitProject::new();
    p.write("svc.py", &calm_fn("worker"));
    p.commit_all();
    p.pulse("--cleanup");
    p.write("svc.py", &spicy_fn("worker"));
    p.commit_all();
    let first = p.pulse("--stop");
    assert!(!first.is_empty(), "first stop reports: {first}");
    let second = p.pulse("--stop");
    assert!(second.is_empty(), "the next turn starts from the new head: {second}");
}

#[test]
fn untracked_new_smelly_file_is_caught() {
    let p = GitProject::new();
    p.commit_all();
    p.pulse("--cleanup");
    p.write("fresh.py", &spicy_fn("worker"));
    let out = p.pulse("--stop");
    assert!(out.to_lowercase().contains("complex method"), "untracked files are part of the turn: {out}");
}

#[test]
fn preexisting_smell_in_a_touched_file_stays_suppressed() {
    let p = GitProject::new();
    p.write("svc.py", &spicy_fn("legacy"));
    p.commit_all();
    p.pulse("--cleanup");
    let appended = format!("{}\n{}", spicy_fn("legacy"), calm_fn("addition"));
    p.write("svc.py", &appended);
    let out = p.pulse("--stop");
    assert!(
        !out.to_lowercase().contains("complex method"),
        "unchanged pre-existing smells are not turn regressions: {out}"
    );
}

#[test]
fn non_git_directory_is_silent() {
    let p = GitProject::plain_dirs();
    p.write("svc.py", &spicy_fn("worker"));
    p.pulse("--cleanup");
    let out = p.pulse("--stop");
    assert!(out.is_empty(), "no repository means no turn delta: {out}");
}
