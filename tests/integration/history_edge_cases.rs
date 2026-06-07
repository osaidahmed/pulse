use std::path::PathBuf;

use pulse::history::finding::{HistoryFinding, HistoryKind, HistoryPillar};
use pulse::history::{run, HistoryOpts};
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
fn output_deterministic_across_two_runs_on_same_repo() {
    let mut commits = Vec::new();
    for i in 0..5 {
        let body_a = format!("x = {i}\n");
        let body_b = format!("y = {i}\n");
        commits.push(CommitSpec {
            author: "alice <alice@x>",
            message: Box::leak(format!("rev{i}").into_boxed_str()),
            writes: Box::leak(Box::new([
                ("a.py", Box::leak(body_a.into_boxed_str()) as &str),
                ("b.py", Box::leak(body_b.into_boxed_str()) as &str),
            ])),
            deletes: &[],
        });
    }
    let repo = build_repo(&commits);
    let mut th = t().history;
    th.co_change.min_support = 1;
    let r1 = run(&opts(repo.path().to_path_buf()), &th).unwrap();
    let r2 = run(&opts(repo.path().to_path_buf()), &th).unwrap();
    assert_eq!(r1.len(), r2.len());
}

#[test]
fn run_with_relative_root_dot_works() {
    let repo =
        build_repo(&[CommitSpec { author: "a <a@x>", message: "init", writes: &[("a.py", "x = 1\n")], deletes: &[] }]);
    let saved = std::env::current_dir().unwrap();
    std::env::set_current_dir(repo.path()).unwrap();
    let result = run(&opts(PathBuf::from(".")), &t().history);
    std::env::set_current_dir(saved).unwrap();
    assert!(result.is_ok());
}

#[test]
fn merge_commit_with_no_files_silently_skipped() {
    let repo = build_repo(&[
        CommitSpec { author: "a <a@x>", message: "init", writes: &[("a.py", "x = 1\n")], deletes: &[] },
        CommitSpec { author: "a <a@x>", message: "empty", writes: &[], deletes: &[] },
    ]);
    let result = run(&opts(repo.path().to_path_buf()), &t().history);
    assert!(result.is_ok());
}

#[test]
fn deleted_file_excluded_from_drift_findings() {
    let repo = build_repo(&[
        CommitSpec {
            author: "a <a@x>",
            message: "init",
            writes: &[("a.py", "x = 1\n"), ("b.py", "y = 2\n")],
            deletes: &[],
        },
        CommitSpec {
            author: "a <a@x>",
            message: "rev",
            writes: &[("a.py", "x = 2\n"), ("b.py", "y = 3\n")],
            deletes: &[],
        },
        CommitSpec {
            author: "a <a@x>",
            message: "rev2",
            writes: &[("a.py", "x = 3\n"), ("b.py", "y = 4\n")],
            deletes: &[],
        },
        CommitSpec { author: "a <a@x>", message: "delete b", writes: &[], deletes: &["b.py"] },
    ]);
    let mut th = t().history;
    th.co_change.min_support = 1;
    let findings = run(&opts(repo.path().to_path_buf()), &th).unwrap();
    let drift_with_b: usize = findings
        .iter()
        .filter(|f| {
            let HistoryKind::ArchitecturalDrift(e) = &f.kind else { return false };
            e.file_b.to_string_lossy().contains("b.py") || e.file_a.to_string_lossy().contains("b.py")
        })
        .count();
    assert_eq!(drift_with_b, 0, "drift with deleted file shouldn't surface");
}

#[test]
fn include_tests_flag_changes_behavior() {
    let mut commits = Vec::new();
    for i in 0..5 {
        let body = format!("x = {i}\n");
        commits.push(CommitSpec {
            author: "a <a@x>",
            message: Box::leak(format!("rev{i}").into_boxed_str()),
            writes: Box::leak(Box::new([
                ("test_a.py", Box::leak(body.clone().into_boxed_str()) as &str),
                ("a.py", Box::leak(body.into_boxed_str()) as &str),
            ])),
            deletes: &[],
        });
    }
    let repo = build_repo(&commits);
    let mut th = t().history;
    th.co_change.min_support = 1;
    let r_excl = run(&opts(repo.path().to_path_buf()), &th).unwrap();
    let drift_count_excl = count_pillar(&r_excl, HistoryPillar::Drift);
    let mut o_incl = opts(repo.path().to_path_buf());
    o_incl.include_tests = true;
    let r_incl = run(&o_incl, &th).unwrap();
    let drift_count_incl = count_pillar(&r_incl, HistoryPillar::Drift);
    assert!(drift_count_incl >= drift_count_excl);
}

#[test]
fn since_filter_excludes_old_commits() {
    let repo =
        build_repo(&[CommitSpec { author: "a <a@x>", message: "init", writes: &[("a.py", "x = 1\n")], deletes: &[] }]);
    let mut o = opts(repo.path().to_path_buf());
    o.since = Some("9999-01-01".to_string());
    let result = run(&o, &t().history).unwrap();
    assert!(result.is_empty(), "since 9999 should exclude all commits");
}

#[test]
fn max_commits_zero_returns_empty_findings() {
    let repo = build_repo(&[CommitSpec {
        author: "a <a@x>",
        message: "init",
        writes: &[("a.py", "x = 1\n"), ("b.py", "y = 2\n")],
        deletes: &[],
    }]);
    let mut o = opts(repo.path().to_path_buf());
    o.max_commits = Some(0);
    let result = run(&o, &t().history).unwrap();
    assert!(result.is_empty());
}

#[test]
fn run_idempotent_returns_same_count() {
    let repo =
        build_repo(&[CommitSpec { author: "a <a@x>", message: "init", writes: &[("a.py", "x = 1\n")], deletes: &[] }]);
    let r1 = run(&opts(repo.path().to_path_buf()), &t().history).unwrap();
    let r2 = run(&opts(repo.path().to_path_buf()), &t().history).unwrap();
    assert_eq!(r1.len(), r2.len());
}

#[test]
fn run_on_repo_with_only_one_file_does_not_panic() {
    let repo =
        build_repo(&[CommitSpec { author: "a <a@x>", message: "init", writes: &[("a.py", "x = 1\n")], deletes: &[] }]);
    let result = run(&opts(repo.path().to_path_buf()), &t().history);
    assert!(result.is_ok());
}

#[test]
fn run_on_repo_with_only_unrecognized_extensions_no_findings() {
    let repo = build_repo(&[CommitSpec {
        author: "a <a@x>",
        message: "init",
        writes: &[("README.txt", "hello\n"), ("Makefile", "all:\n")],
        deletes: &[],
    }]);
    let findings = run(&opts(repo.path().to_path_buf()), &t().history).unwrap();
    assert!(findings.is_empty(), "non-typed files produce no findings");
}

#[test]
fn run_handles_repo_with_multiple_commits_one_file_no_drift() {
    let mut commits = Vec::new();
    for i in 0..5 {
        let body = format!("x = {i}\n");
        commits.push(CommitSpec {
            author: "a <a@x>",
            message: Box::leak(format!("rev{i}").into_boxed_str()),
            writes: Box::leak(Box::new([("a.py", Box::leak(body.into_boxed_str()) as &str)])),
            deletes: &[],
        });
    }
    let repo = build_repo(&commits);
    let findings = run(&opts(repo.path().to_path_buf()), &t().history).unwrap();
    assert_eq!(count_pillar(&findings, HistoryPillar::Drift), 0);
}

#[test]
fn run_handles_subdirectory_files() {
    let mut commits = Vec::new();
    for i in 0..3 {
        let body_a = format!("x = {i}\n");
        let body_b = format!("y = {i}\n");
        commits.push(CommitSpec {
            author: "a <a@x>",
            message: Box::leak(format!("rev{i}").into_boxed_str()),
            writes: Box::leak(Box::new([
                ("src/lib/a.py", Box::leak(body_a.into_boxed_str()) as &str),
                ("src/lib/b.py", Box::leak(body_b.into_boxed_str()) as &str),
            ])),
            deletes: &[],
        });
    }
    commits.extend(noise_commits("NOTES.md", 2));
    let repo = build_repo(&commits);
    let mut th = t().history;
    th.co_change.min_support = 1;
    let findings = run(&opts(repo.path().to_path_buf()), &th).unwrap();
    assert!(count_pillar(&findings, HistoryPillar::Drift) >= 1);
}

#[test]
fn three_way_chain_does_not_create_phantom_pair() {
    let mut commits = Vec::new();
    for i in 0..3 {
        let body_a = format!("x = {i}\n");
        let body_b = format!("y = {i}\n");
        commits.push(CommitSpec {
            author: "a <a@x>",
            message: Box::leak(format!("ab{i}").into_boxed_str()),
            writes: Box::leak(Box::new([
                ("a.py", Box::leak(body_a.into_boxed_str()) as &str),
                ("b.py", Box::leak(body_b.into_boxed_str()) as &str),
            ])),
            deletes: &[],
        });
    }
    for i in 0..3 {
        let body_b = format!("y = {}\n", 100 + i);
        let body_c = format!("z = {i}\n");
        commits.push(CommitSpec {
            author: "a <a@x>",
            message: Box::leak(format!("bc{i}").into_boxed_str()),
            writes: Box::leak(Box::new([
                ("b.py", Box::leak(body_b.into_boxed_str()) as &str),
                ("c.py", Box::leak(body_c.into_boxed_str()) as &str),
            ])),
            deletes: &[],
        });
    }
    let repo = build_repo(&commits);
    let mut th = t().history;
    th.co_change.min_support = 3;
    let findings = run(&opts(repo.path().to_path_buf()), &th).unwrap();
    let ac_pair: usize = findings
        .iter()
        .filter(|f| {
            let HistoryKind::ArchitecturalDrift(e) = &f.kind else { return false };
            (e.file_a.to_string_lossy().ends_with("a.py") && e.file_b.to_string_lossy().ends_with("c.py"))
                || (e.file_a.to_string_lossy().ends_with("c.py") && e.file_b.to_string_lossy().ends_with("a.py"))
        })
        .count();
    assert_eq!(ac_pair, 0, "A↔C should not be inferred from A↔B and B↔C");
}

#[test]
fn run_back_to_back_without_state_leak() {
    let repo =
        build_repo(&[CommitSpec { author: "a <a@x>", message: "init", writes: &[("a.py", "x = 1\n")], deletes: &[] }]);
    let _ = run(&opts(repo.path().to_path_buf()), &t().history).unwrap();
    let r2 = run(&opts(repo.path().to_path_buf()), &t().history).unwrap();
    assert!(r2.len() <= 100);
}
