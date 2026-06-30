use std::collections::HashSet;
use std::path::PathBuf;

use pulse_history::contributors::{author_counts_per_file, rank};
use pulse_history::finding::{HistoryFinding, HistoryKind};
use pulse_history::git::Commit;
use pulse_thresholds::Thresholds;

fn t() -> Thresholds {
    Thresholds::default()
}

fn commit(author: &str, files: &[&str]) -> Commit {
    Commit { hash: "h".into(), author: author.into(), timestamp: 1, files: files.iter().map(PathBuf::from).collect() }
}

fn typed(paths: &[&str]) -> HashSet<PathBuf> {
    paths.iter().map(PathBuf::from).collect()
}

fn extract_evidence(f: &HistoryFinding) -> &pulse_history::finding::FragmentationEvidence {
    let HistoryKind::KnowledgeFragmentation(e) = &f.kind else { panic!("expected fragmentation finding") };
    e
}

fn build_commits(author_counts: &[(&str, u32)], file: &str) -> Vec<Commit> {
    let mut out = Vec::new();
    for (author, count) in author_counts {
        for _ in 0..*count {
            out.push(commit(author, &[file]));
        }
    }
    out
}

#[test]
fn rank_empty_commits_returns_empty() {
    let findings = rank(&[], &typed(&[]), &t().history);
    assert!(findings.is_empty());
}

#[test]
fn rank_single_author_no_finding() {
    let commits = build_commits(&[("alice@x", 20)], "a.py");
    let findings = rank(&commits, &typed(&["a.py"]), &t().history);
    assert!(findings.is_empty());
}

#[test]
fn rank_two_authors_evenly_split_no_finding() {
    let commits = build_commits(&[("alice@x", 10), ("bob@x", 10)], "a.py");
    let findings = rank(&commits, &typed(&["a.py"]), &t().history);
    assert!(findings.is_empty());
}

#[test]
fn rank_three_minor_authors_flagged() {
    let commits = build_commits(&[("alice@x", 91), ("bob@x", 3), ("carol@x", 3), ("dave@x", 3)], "a.py");
    let findings = rank(&commits, &typed(&["a.py"]), &t().history);
    assert_eq!(findings.len(), 1);
    let e = extract_evidence(&findings[0]);
    assert_eq!(e.minor_contributor_count, 3);
    assert_eq!(e.total_contributors, 4);
}

#[test]
fn rank_two_minor_authors_below_threshold_no_finding() {
    let commits = build_commits(&[("alice@x", 94), ("bob@x", 3), ("carol@x", 3)], "a.py");
    let findings = rank(&commits, &typed(&["a.py"]), &t().history);
    assert!(findings.is_empty(), "below min_minor_authors should not fire");
}

#[test]
fn rank_below_min_total_commits_no_finding() {
    let commits = build_commits(&[("alice@x", 1), ("bob@x", 1), ("carol@x", 1), ("dave@x", 1)], "a.py");
    let findings = rank(&commits, &typed(&["a.py"]), &t().history);
    assert!(findings.is_empty());
}

#[test]
fn rank_at_min_total_commits_can_emit() {
    let commits = build_commits(&[("alice@x", 92), ("bob@x", 3), ("carol@x", 3), ("dave@x", 3)], "a.py");
    let mut th = t().history;
    th.contributors.min_total_commits = 5;
    let findings = rank(&commits, &typed(&["a.py"]), &th);
    assert_eq!(findings.len(), 1);
}

#[test]
fn rank_exactly_5pct_is_major_not_minor() {
    let commits = build_commits(&[("alice@x", 80), ("bob@x", 5), ("carol@x", 5), ("dave@x", 5), ("eve@x", 5)], "a.py");
    let findings = rank(&commits, &typed(&["a.py"]), &t().history);
    assert!(findings.is_empty(), "exactly 5% authors are major, no minors");
}

#[test]
fn rank_just_below_5pct_is_minor() {
    let commits = build_commits(&[("alice@x", 91), ("bob@x", 3), ("carol@x", 3), ("dave@x", 3)], "a.py");
    let findings = rank(&commits, &typed(&["a.py"]), &t().history);
    let e = extract_evidence(&findings[0]);
    assert_eq!(e.minor_contributor_count, 3);
}

#[test]
fn rank_top_minor_authors_capped_at_5() {
    let mut author_counts: Vec<(&str, u32)> = vec![("alice@x", 90)];
    let names = ["a@x", "b@x", "c@x", "d@x", "e@x", "f@x", "g@x"];
    for n in &names {
        author_counts.push((n, 1));
    }
    let commits = build_commits(&author_counts, "a.py");
    let findings = rank(&commits, &typed(&["a.py"]), &t().history);
    let e = extract_evidence(&findings[0]);
    assert_eq!(e.top_minor_authors.len(), 5);
}

#[test]
fn rank_top_minor_authors_sorted_by_commit_desc() {
    let commits = build_commits(&[("alice@x", 88), ("bob@x", 4), ("carol@x", 3), ("dave@x", 2), ("eve@x", 1)], "a.py");
    let findings = rank(&commits, &typed(&["a.py"]), &t().history);
    let e = extract_evidence(&findings[0]);
    assert_eq!(e.top_minor_authors[0], "bob@x");
    assert_eq!(e.top_minor_authors[1], "carol@x");
    assert_eq!(e.top_minor_authors[2], "dave@x");
}

#[test]
fn rank_excludes_paths_outside_typed_set() {
    let commits = build_commits(&[("alice@x", 91), ("bob@x", 3), ("carol@x", 3), ("dave@x", 3)], "a.py");
    let findings = rank(&commits, &typed(&["other.py"]), &t().history);
    assert!(findings.is_empty());
}

#[test]
fn rank_truncates_at_max_findings_reported() {
    let mut commits = Vec::new();
    for f in &["a.py", "b.py", "c.py"] {
        commits.extend(build_commits(&[("alice@x", 91), ("bob@x", 3), ("carol@x", 3), ("dave@x", 3)], f));
    }
    let mut th = t().history;
    th.contributors.max_findings_reported = 2;
    let findings = rank(&commits, &typed(&["a.py", "b.py", "c.py"]), &th);
    assert_eq!(findings.len(), 2);
}

#[test]
fn rank_action_label_unset_initially() {
    let commits = build_commits(&[("alice@x", 91), ("bob@x", 3), ("carol@x", 3), ("dave@x", 3)], "a.py");
    let findings = rank(&commits, &typed(&["a.py"]), &t().history);
    assert!(findings[0].action_label.is_none());
}

#[test]
fn rank_total_contributors_counts_all_authors() {
    let commits = build_commits(&[("alice@x", 91), ("bob@x", 3), ("carol@x", 3), ("dave@x", 3)], "a.py");
    let findings = rank(&commits, &typed(&["a.py"]), &t().history);
    let e = extract_evidence(&findings[0]);
    assert_eq!(e.total_contributors, 4);
}

#[test]
fn rank_lex_tiebreak_for_equal_minor_count() {
    let mut commits = Vec::new();
    for f in &["zzz.py", "aaa.py"] {
        commits.extend(build_commits(&[("alice@x", 91), ("bob@x", 3), ("carol@x", 3), ("dave@x", 3)], f));
    }
    let findings = rank(&commits, &typed(&["aaa.py", "zzz.py"]), &t().history);
    assert_eq!(findings.len(), 2);
    let first_e = extract_evidence(&findings[0]);
    assert_eq!(first_e.file, PathBuf::from("aaa.py"));
}

#[test]
fn author_counts_per_file_basic() {
    let commits = vec![commit("alice@x", &["a.py"]), commit("alice@x", &["a.py"]), commit("bob@x", &["a.py"])];
    let result = author_counts_per_file(&commits, &typed(&["a.py"]));
    let counts = result.get(&PathBuf::from("a.py")).unwrap();
    assert_eq!(counts.get("alice@x"), Some(&2));
    assert_eq!(counts.get("bob@x"), Some(&1));
}

#[test]
fn author_counts_per_file_excludes_outside_typed() {
    let commits = vec![commit("a@x", &["a.py", "b.py"])];
    let result = author_counts_per_file(&commits, &typed(&["a.py"]));
    assert_eq!(result.len(), 1);
    assert!(result.contains_key(&PathBuf::from("a.py")));
}

#[test]
fn rank_returns_observed_pct_in_evidence() {
    let commits = build_commits(&[("alice@x", 91), ("bob@x", 3), ("carol@x", 3), ("dave@x", 3)], "a.py");
    let findings = rank(&commits, &typed(&["a.py"]), &t().history);
    let e = extract_evidence(&findings[0]);
    assert!((e.minor_contributor_pct - 0.75).abs() < 0.01);
}

#[test]
fn rank_orders_by_minor_count_desc() {
    let mut commits = Vec::new();
    commits.extend(build_commits(&[("alice@x", 88), ("b@x", 3), ("c@x", 3), ("d@x", 3), ("e@x", 3)], "high.py"));
    commits.extend(build_commits(&[("alice@x", 91), ("b@x", 3), ("c@x", 3), ("d@x", 3)], "low.py"));
    let findings = rank(&commits, &typed(&["high.py", "low.py"]), &t().history);
    let first_e = extract_evidence(&findings[0]);
    assert_eq!(first_e.file, PathBuf::from("high.py"));
}
