use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

pub struct CommitSpec<'a> {
    pub author: &'a str,
    pub message: &'a str,
    pub writes: &'a [(&'a str, &'a str)],
    pub deletes: &'a [&'a str],
}

const BASE_TIMESTAMP: i64 = 1_700_000_000;

pub fn noise_commits(file: &'static str, n: u32) -> Vec<CommitSpec<'static>> {
    let mut out = Vec::new();
    for i in 0..n {
        let body = format!("noise {i}\n");
        let writes: &'static [(&'static str, &'static str)] =
            Box::leak(vec![(file, Box::leak(body.into_boxed_str()) as &str)].into_boxed_slice());
        out.push(CommitSpec {
            author: "alice <alice@x>",
            message: Box::leak(format!("noise{i}").into_boxed_str()),
            writes,
            deletes: &[],
        });
    }
    out
}

pub fn build_repo(commits: &[CommitSpec]) -> TempDir {
    let dir = tempfile::tempdir().expect("create tempdir");
    let path = dir.path();
    init_repo(path);
    for (idx, spec) in commits.iter().enumerate() {
        apply_writes(path, spec.writes);
        apply_deletes(path, spec.deletes);
        run_git(path, &["add", "-A"]);
        let date = format!("{} +0000", BASE_TIMESTAMP + i64::try_from(idx).unwrap() * 60);
        commit_with_identity(path, spec, &date);
    }
    dir
}

fn init_repo(path: &Path) {
    run_git(path, &["init", "-q", "-b", "main"]);
}

fn apply_writes(root: &Path, writes: &[(&str, &str)]) {
    for (rel, content) in writes {
        let full = root.join(rel);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).expect("create parent dir");
        }
        std::fs::write(&full, content).expect("write fixture file");
    }
}

fn apply_deletes(root: &Path, deletes: &[&str]) {
    for rel in deletes {
        let _ = std::fs::remove_file(root.join(rel));
    }
}

fn commit_with_identity(path: &Path, spec: &CommitSpec, date: &str) {
    let status = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["commit", "--no-gpg-sign", "--allow-empty-message", "--allow-empty", "-q", "-m", spec.message])
        .arg(format!("--author={}", spec.author))
        .env("GIT_AUTHOR_DATE", date)
        .env("GIT_COMMITTER_DATE", date)
        .env("GIT_COMMITTER_NAME", "test")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .status()
        .expect("run git commit");
    assert!(status.success(), "git commit failed for spec {:?}", spec.message);
}

fn run_git(path: &Path, args: &[&str]) {
    let status = Command::new("git").arg("-C").arg(path).args(args).status().expect("run git");
    assert!(status.success(), "git {args:?} failed");
}
