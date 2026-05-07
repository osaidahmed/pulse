use std::path::PathBuf;

use pulse::history::git::{collect_commits, is_git_repo, parse_log_output, GitOpts};
use pulse::history::HistoryError;
use pulse::thresholds::Thresholds;

use crate::history_common::{build_repo, CommitSpec};

fn t() -> Thresholds {
    Thresholds::default()
}

fn cap() -> u32 {
    t().history.max_commit_files
}

#[test]
fn parse_single_commit() {
    let stdout = "__COMMIT__\nabc123\nalice@example.com\n1700000000\n\nsrc/foo.rs\nsrc/bar.rs\n";
    let commits = parse_log_output(stdout, cap());
    assert_eq!(commits.len(), 1);
    assert_eq!(commits[0].hash, "abc123");
    assert_eq!(commits[0].author, "alice@example.com");
    assert_eq!(commits[0].timestamp, 1_700_000_000);
    assert_eq!(commits[0].files, vec![PathBuf::from("src/foo.rs"), PathBuf::from("src/bar.rs")]);
}

#[test]
fn parse_multiple_commits() {
    let stdout = concat!(
        "__COMMIT__\nabc\nalice@x\n1700000000\n\nsrc/a.rs\n",
        "__COMMIT__\ndef\nbob@x\n1700000060\n\nsrc/a.rs\nsrc/b.rs\n",
    );
    let commits = parse_log_output(stdout, cap());
    assert_eq!(commits.len(), 2);
    assert_eq!(commits[0].author, "alice@x");
    assert_eq!(commits[1].author, "bob@x");
    assert_eq!(commits[1].files.len(), 2);
}

#[test]
fn parse_skips_merge_only_commits() {
    let stdout = "__COMMIT__\nmerge\nbot@x\n1700000000\n";
    let commits = parse_log_output(stdout, cap());
    assert!(commits.is_empty());
}

#[test]
fn parse_drops_mega_commits() {
    let mut stdout = String::from("__COMMIT__\nbig\nalice@x\n1700000000\n\n");
    let mega = cap() as usize + 5;
    for i in 0..mega {
        stdout.push_str(&format!("file_{i}.rs\n"));
    }
    let commits = parse_log_output(&stdout, cap());
    assert!(commits.is_empty(), "mega-commit should be dropped");
}

#[test]
fn parse_keeps_commit_at_cap() {
    let mut stdout = String::from("__COMMIT__\nedge\nalice@x\n1700000000\n\n");
    for i in 0..cap() {
        stdout.push_str(&format!("file_{i}.rs\n"));
    }
    let commits = parse_log_output(&stdout, cap());
    assert_eq!(commits.len(), 1);
    assert_eq!(commits[0].files.len(), cap() as usize);
}

#[test]
fn parse_handles_invalid_timestamp() {
    let stdout = "__COMMIT__\nabc\nalice@x\nnot_a_number\n\nsrc/a.rs\n";
    let commits = parse_log_output(stdout, cap());
    assert!(commits.is_empty());
}

#[test]
fn parse_empty_input() {
    let commits = parse_log_output("", cap());
    assert!(commits.is_empty());
}

#[test]
fn is_git_repo_true_for_real_repo() {
    let repo = build_repo(&[CommitSpec {
        author: "Alice <alice@x>",
        message: "init",
        writes: &[("a.txt", "hello\n")],
        deletes: &[],
    }]);
    assert!(is_git_repo(repo.path()));
}

#[test]
fn is_git_repo_false_for_plain_dir() {
    let dir = tempfile::tempdir().unwrap();
    assert!(!is_git_repo(dir.path()));
}

#[test]
fn collect_commits_returns_not_a_repo_error() {
    let dir = tempfile::tempdir().unwrap();
    let opts = GitOpts {
        root: dir.path(),
        since: None,
        max_commits: None,
        max_commit_files: cap(),
    };
    let err = collect_commits(&opts).unwrap_err();
    assert!(matches!(err, HistoryError::NotAGitRepo(_)));
}

#[test]
fn collect_commits_zero_commits_returns_empty() {
    let dir = tempfile::tempdir().unwrap();
    let _ = std::process::Command::new("git")
        .arg("-C").arg(dir.path())
        .args(["init", "-q", "-b", "main"])
        .status();
    let opts = GitOpts {
        root: dir.path(),
        since: None,
        max_commits: None,
        max_commit_files: cap(),
    };
    let commits = collect_commits(&opts).unwrap();
    assert!(commits.is_empty());
}

#[test]
fn collect_commits_single_commit() {
    let repo = build_repo(&[CommitSpec {
        author: "Alice <alice@example.com>",
        message: "init",
        writes: &[("src/a.py", "x = 1\n"), ("src/b.py", "y = 2\n")],
        deletes: &[],
    }]);
    let opts = GitOpts {
        root: repo.path(),
        since: None,
        max_commits: None,
        max_commit_files: cap(),
    };
    let commits = collect_commits(&opts).unwrap();
    assert_eq!(commits.len(), 1);
    assert_eq!(commits[0].author, "alice@example.com");
    assert_eq!(commits[0].timestamp, 1_700_000_000);
    let mut files = commits[0].files.clone();
    files.sort();
    assert_eq!(files, vec![PathBuf::from("src/a.py"), PathBuf::from("src/b.py")]);
}

#[test]
fn collect_commits_multi_commit_in_reverse_chronological_order() {
    let repo = build_repo(&[
        CommitSpec {
            author: "Alice <alice@x>",
            message: "first",
            writes: &[("a.py", "1\n")],
            deletes: &[],
        },
        CommitSpec {
            author: "Bob <bob@x>",
            message: "second",
            writes: &[("b.py", "2\n")],
            deletes: &[],
        },
    ]);
    let opts = GitOpts {
        root: repo.path(),
        since: None,
        max_commits: None,
        max_commit_files: cap(),
    };
    let commits = collect_commits(&opts).unwrap();
    assert_eq!(commits.len(), 2);
    assert_eq!(commits[0].author, "bob@x");
    assert_eq!(commits[1].author, "alice@x");
}

#[test]
fn collect_commits_max_commits_caps_result() {
    let repo = build_repo(&[
        CommitSpec { author: "A <a@x>", message: "1", writes: &[("a.py", "1\n")], deletes: &[] },
        CommitSpec { author: "A <a@x>", message: "2", writes: &[("a.py", "2\n")], deletes: &[] },
        CommitSpec { author: "A <a@x>", message: "3", writes: &[("a.py", "3\n")], deletes: &[] },
    ]);
    let opts = GitOpts {
        root: repo.path(),
        since: None,
        max_commits: Some(2),
        max_commit_files: cap(),
    };
    let commits = collect_commits(&opts).unwrap();
    assert_eq!(commits.len(), 2);
}
