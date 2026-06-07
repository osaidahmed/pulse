use std::path::PathBuf;

use pulse::history::finding::{HistoryFinding, HistoryKind, HistoryPillar};
use pulse::history::{run, HistoryError, HistoryOpts};
use pulse::thresholds::Thresholds;

use crate::history_common::{build_repo, noise_commits, CommitSpec};

fn t() -> Thresholds {
    Thresholds::default()
}

fn opts(root: PathBuf) -> HistoryOpts {
    HistoryOpts { root, include_tests: false, since: None, max_commits: None }
}

fn count_pillar(findings: &[HistoryFinding], pillar: HistoryPillar) -> usize {
    findings.iter().filter(|f| pulse::history::finding::variant_info(&f.kind).pillar == pillar).count()
}

#[test]
fn run_returns_not_a_git_repo_for_plain_dir() {
    let dir = tempfile::tempdir().unwrap();
    let result = run(&opts(dir.path().to_path_buf()), &t().history);
    assert!(matches!(result, Err(HistoryError::NotAGitRepo(_))));
}

#[test]
fn run_zero_commit_repo_returns_empty_findings() {
    let dir = tempfile::tempdir().unwrap();
    let _ = std::process::Command::new("git").arg("-C").arg(dir.path()).args(["init", "-q", "-b", "main"]).status();
    let findings = run(&opts(dir.path().to_path_buf()), &t().history).unwrap();
    assert!(findings.is_empty());
}

#[test]
fn run_simple_repo_does_not_panic() {
    let repo = build_repo(&[CommitSpec {
        author: "alice <alice@x>",
        message: "init",
        writes: &[("a.py", "x = 1\n")],
        deletes: &[],
    }]);
    let _ = run(&opts(repo.path().to_path_buf()), &t().history).unwrap();
}

#[test]
fn run_unlinked_co_change_pair_emerges_as_drift() {
    let mut commits = Vec::new();
    for i in 0..5 {
        let msg = format!("rev{i}");
        let body_a = format!("x = {i}\n");
        let body_b = format!("y = {i}\n");
        commits.push(CommitSpec {
            author: "alice <alice@x>",
            message: Box::leak(msg.into_boxed_str()),
            writes: Box::leak(Box::new([
                ("a.py", Box::leak(body_a.into_boxed_str()) as &str),
                ("b.py", Box::leak(body_b.into_boxed_str()) as &str),
            ])),
            deletes: &[],
        });
    }
    commits.extend(noise_commits("NOTES.md", 2));
    let repo = build_repo(&commits);
    let mut th = t().history;
    th.co_change.min_support = 3;
    let findings = run(&opts(repo.path().to_path_buf()), &th).unwrap();
    assert!(count_pillar(&findings, HistoryPillar::Drift) >= 1);
}

#[test]
fn run_imported_pair_does_not_emerge_as_drift() {
    let mut commits = Vec::new();
    for i in 0..5 {
        let msg = format!("rev{i}");
        let body = format!("from b import bar\nx = {i}\n");
        commits.push((msg, body));
    }
    let specs: Vec<CommitSpec> = commits
        .iter()
        .map(|(msg, body)| CommitSpec {
            author: "alice <alice@x>",
            message: Box::leak(msg.clone().into_boxed_str()),
            writes: Box::leak(Box::new([
                ("a.py", Box::leak(body.clone().into_boxed_str()) as &str),
                ("b.py", "def bar(): pass\n"),
            ])),
            deletes: &[],
        })
        .collect();
    let repo = build_repo(&specs);
    let mut th = t().history;
    th.co_change.min_support = 3;
    let findings = run(&opts(repo.path().to_path_buf()), &th).unwrap();
    assert_eq!(count_pillar(&findings, HistoryPillar::Drift), 0);
}

#[test]
fn run_high_churn_high_cc_file_produces_hotspot() {
    let high_cc = "def f(x):\n    if x > 0:\n        if x > 10:\n            return 1\n        if x > 100:\n            return 2\n        return 3\n    if x < 0:\n        return -1\n    return 0\n";
    let mut commits = Vec::new();
    for i in 0..6 {
        let msg = format!("rev{i}");
        let body = format!("{high_cc}\n# rev{i}\n");
        commits.push((msg, body));
    }
    let specs: Vec<CommitSpec> = commits
        .iter()
        .map(|(msg, body)| CommitSpec {
            author: "alice <alice@x>",
            message: Box::leak(msg.clone().into_boxed_str()),
            writes: Box::leak(Box::new([("hot.py", Box::leak(body.clone().into_boxed_str()) as &str)])),
            deletes: &[],
        })
        .collect();
    let repo = build_repo(&specs);
    let mut th = t().history;
    th.hotspot.min_revisions = 3;
    th.hotspot.min_score = 1;
    let findings = run(&opts(repo.path().to_path_buf()), &th).unwrap();
    assert!(count_pillar(&findings, HistoryPillar::Complexity) >= 1);
}

#[test]
fn run_minor_contributors_produce_ownership_finding() {
    let mut specs = Vec::new();
    for i in 0..91 {
        let msg = format!("rev{i}");
        let body = format!("x = {i}\n");
        specs.push((msg, "alice <alice@x>".to_string(), body));
    }
    for (idx, author) in ["bob <bob@x>", "carol <carol@x>", "dave <dave@x>"].iter().enumerate() {
        let msg = format!("minor{idx}");
        let body = format!("y = {}\n", 1000 + idx);
        specs.push((msg, author.to_string(), body));
    }
    let commit_specs: Vec<CommitSpec> = specs
        .iter()
        .map(|(msg, author, body)| CommitSpec {
            author: Box::leak(author.clone().into_boxed_str()),
            message: Box::leak(msg.clone().into_boxed_str()),
            writes: Box::leak(Box::new([("o.py", Box::leak(body.clone().into_boxed_str()) as &str)])),
            deletes: &[],
        })
        .collect();
    let repo = build_repo(&commit_specs);
    let findings = run(&opts(repo.path().to_path_buf()), &t().history).unwrap();
    assert!(count_pillar(&findings, HistoryPillar::Ownership) >= 1);
}

#[test]
fn run_max_commits_caps_input() {
    let mut specs = Vec::new();
    for i in 0..10 {
        let msg = format!("rev{i}");
        specs.push((msg, format!("x = {i}\n")));
    }
    let commit_specs: Vec<CommitSpec> = specs
        .iter()
        .map(|(msg, body)| CommitSpec {
            author: "alice <alice@x>",
            message: Box::leak(msg.clone().into_boxed_str()),
            writes: Box::leak(Box::new([("a.py", Box::leak(body.clone().into_boxed_str()) as &str)])),
            deletes: &[],
        })
        .collect();
    let repo = build_repo(&commit_specs);
    let mut o = opts(repo.path().to_path_buf());
    o.max_commits = Some(3);
    let _ = run(&o, &t().history).unwrap();
}

#[test]
fn run_respects_include_tests_false_by_default() {
    let high_cc = "def f(x):\n    if x > 0:\n        return 1\n    return 0\n";
    let specs: Vec<CommitSpec> = (0..5)
        .map(|i| {
            let msg = format!("rev{i}");
            CommitSpec {
                author: "alice <alice@x>",
                message: Box::leak(msg.into_boxed_str()),
                writes: Box::leak(Box::new([("test_helper.py", high_cc)])),
                deletes: &[],
            }
        })
        .collect();
    let repo = build_repo(&specs);
    let mut th = t().history;
    th.hotspot.min_revisions = 1;
    th.hotspot.min_score = 1;
    let findings = run(&opts(repo.path().to_path_buf()), &th).unwrap();
    let test_files_in_findings: Vec<&HistoryFinding> = findings
        .iter()
        .filter(|f| {
            let HistoryKind::Hotspot(e) = &f.kind else { return false };
            e.file.to_string_lossy().contains("test_")
        })
        .collect();
    assert!(test_files_in_findings.is_empty(), "test files should be excluded by default");
}
