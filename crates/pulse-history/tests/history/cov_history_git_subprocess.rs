use std::path::{Path, PathBuf};

use pulse_history::git::{file_at_commit, files_at_commit, repo_toplevel};

use crate::history_common::{build_repo, CommitSpec};

fn single_file_repo() -> tempfile::TempDir {
    build_repo(&[CommitSpec {
        author: "Alice <alice@example.com>",
        message: "init",
        writes: &[("src/a.py", "x = 1\n"), ("src/sub/b.py", "y = 2\n")],
        deletes: &[],
    }])
}

#[test]
fn repo_toplevel_none_for_plain_dir() {
    let dir = tempfile::tempdir().unwrap();
    assert!(repo_toplevel(dir.path()).is_none());
}

#[test]
fn repo_toplevel_some_for_real_repo() {
    let repo = single_file_repo();
    let top = repo_toplevel(repo.path()).expect("toplevel of real repo");
    let canon_top = std::fs::canonicalize(&top).unwrap();
    let canon_root = std::fs::canonicalize(repo.path()).unwrap();
    assert_eq!(canon_top, canon_root);
}

#[test]
fn files_at_commit_lists_tracked_files_at_head() {
    let repo = single_file_repo();
    let mut files = files_at_commit(repo.path(), "HEAD");
    files.sort();
    assert_eq!(files, vec![PathBuf::from("src/a.py"), PathBuf::from("src/sub/b.py")]);
}

#[test]
fn files_at_commit_empty_for_bad_rev() {
    let repo = single_file_repo();
    let files = files_at_commit(repo.path(), "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef");
    assert!(files.is_empty(), "non-existent rev should yield empty file list");
}

#[test]
fn file_at_commit_returns_content_for_tracked_file() {
    let repo = single_file_repo();
    let content = file_at_commit(repo.path(), "HEAD", Path::new("src/a.py")).expect("tracked file content");
    assert_eq!(content, "x = 1\n");
}

#[test]
fn file_at_commit_none_for_missing_path() {
    let repo = single_file_repo();
    let content = file_at_commit(repo.path(), "HEAD", Path::new("src/does_not_exist.py"));
    assert!(content.is_none(), "missing path at HEAD should be None");
}

#[test]
fn file_at_commit_none_for_bad_rev() {
    let repo = single_file_repo();
    let content = file_at_commit(repo.path(), "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef", Path::new("src/a.py"));
    assert!(content.is_none(), "non-existent rev should be None");
}
