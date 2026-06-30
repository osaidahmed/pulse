use std::collections::HashSet;
use std::path::{Path, PathBuf};

use pulse_audit::graph::{ImportGraph, InputEdge};
use pulse_history::co_change::{mine, rank_drift, revisions_in_scope};
use pulse_history::finding::{HistoryFinding, HistoryKind};
use pulse_history::git::Commit;
use pulse_history::thresholds::HistoryThresholds;
use pulse_syntax::parse::Language;
use pulse_thresholds::Thresholds;

fn t() -> Thresholds {
    Thresholds::default()
}

fn commit(hash: &str, author: &str, ts: i64, files: &[&str]) -> Commit {
    Commit { hash: hash.into(), author: author.into(), timestamp: ts, files: files.iter().map(PathBuf::from).collect() }
}

fn empty_graph() -> ImportGraph {
    ImportGraph::build(&[])
}

fn graph_linking(a: &str, b: &str) -> ImportGraph {
    ImportGraph::build(&[InputEdge {
        source: PathBuf::from(a),
        target: PathBuf::from(b),
        source_lang: Language::Python,
        target_lang: Language::Python,
    }])
}

fn typed(paths: &[&str]) -> HashSet<PathBuf> {
    paths.iter().map(PathBuf::from).collect()
}

fn drift_pair(f: &HistoryFinding) -> (&Path, &Path) {
    let HistoryKind::ArchitecturalDrift(e) = &f.kind else { panic!("expected drift") };
    (e.file_a.as_path(), e.file_b.as_path())
}

#[test]
fn mine_empty_commits_yields_empty_pairs() {
    let pairs = mine(&[], &t().history);
    assert!(pairs.is_empty());
}

#[test]
fn mine_single_commit_two_files_yields_one_pair() {
    let commits = vec![commit("h1", "a@x", 1, &["a.py", "b.py"])];
    let pairs = mine(&commits, &t().history);
    assert_eq!(pairs.len(), 1);
}

#[test]
fn mine_three_files_yields_three_pairs() {
    let commits = vec![commit("h1", "a@x", 1, &["a.py", "b.py", "c.py"])];
    let pairs = mine(&commits, &t().history);
    assert_eq!(pairs.len(), 3);
}

#[test]
fn mine_pairs_use_canonical_lex_order() {
    let commits = vec![commit("h1", "a@x", 1, &["b.py", "a.py"])];
    let pairs = mine(&commits, &t().history);
    let key = pairs.keys().next().unwrap();
    assert_eq!(key.0, PathBuf::from("a.py"));
    assert_eq!(key.1, PathBuf::from("b.py"));
}

#[test]
fn mine_increments_support_across_commits() {
    let commits = vec![
        commit("h1", "a@x", 1, &["a.py", "b.py"]),
        commit("h2", "a@x", 2, &["a.py", "b.py"]),
        commit("h3", "a@x", 3, &["a.py", "b.py"]),
    ];
    let pairs = mine(&commits, &t().history);
    let agg = pairs.values().next().unwrap();
    assert_eq!(agg.support, 3);
}

#[test]
fn mine_tracks_max_last_seen() {
    let commits = vec![
        commit("h1", "a@x", 100, &["a.py", "b.py"]),
        commit("h2", "a@x", 50, &["a.py", "b.py"]),
        commit("h3", "a@x", 200, &["a.py", "b.py"]),
    ];
    let pairs = mine(&commits, &t().history);
    let agg = pairs.values().next().unwrap();
    assert_eq!(agg.last_seen, 200);
}

#[test]
fn mine_tracks_distinct_authors() {
    let commits = vec![
        commit("h1", "alice@x", 1, &["a.py", "b.py"]),
        commit("h2", "bob@x", 2, &["a.py", "b.py"]),
        commit("h3", "alice@x", 3, &["a.py", "b.py"]),
    ];
    let pairs = mine(&commits, &t().history);
    let agg = pairs.values().next().unwrap();
    assert_eq!(agg.authors.len(), 2);
    assert!(agg.authors.contains("alice@x"));
    assert!(agg.authors.contains("bob@x"));
}

#[test]
fn mine_drops_mega_commit() {
    let mut th = t().history;
    th.max_commit_files = 5;
    let files: Vec<&str> = vec!["a.py", "b.py", "c.py", "d.py", "e.py", "f.py"];
    let commits = vec![commit("big", "a@x", 1, &files)];
    let pairs = mine(&commits, &th);
    assert!(pairs.is_empty(), "mega commit should be dropped");
}

#[test]
fn mine_keeps_commit_at_cap() {
    let mut th = t().history;
    th.max_commit_files = 3;
    let commits = vec![commit("ok", "a@x", 1, &["a.py", "b.py", "c.py"])];
    let pairs = mine(&commits, &th);
    assert_eq!(pairs.len(), 3);
}

#[test]
fn mine_one_file_commit_no_pairs() {
    let commits = vec![commit("h1", "a@x", 1, &["only.py"])];
    let pairs = mine(&commits, &t().history);
    assert!(pairs.is_empty());
}

#[test]
fn mine_zero_file_commit_no_pairs() {
    let commits = vec![commit("merge", "a@x", 1, &[])];
    let pairs = mine(&commits, &t().history);
    assert!(pairs.is_empty());
}

#[test]
fn mine_does_not_create_self_pair() {
    let commits = vec![commit("h1", "a@x", 1, &["a.py", "a.py"])];
    let pairs = mine(&commits, &t().history);
    assert!(pairs.is_empty());
}

#[test]
fn mine_independent_of_commit_order() {
    let c1 = commit("h1", "a@x", 1, &["a.py", "b.py"]);
    let c2 = commit("h2", "b@x", 2, &["a.py", "b.py"]);
    let p1 = mine(&[c1.clone(), c2.clone()], &t().history);
    let p2 = mine(&[c2, c1], &t().history);
    assert_eq!(p1.len(), p2.len());
    let v1 = p1.values().next().unwrap();
    let v2 = p2.values().next().unwrap();
    assert_eq!(v1.support, v2.support);
    assert_eq!(v1.authors.len(), v2.authors.len());
}

fn drift_metrics_of(f: &HistoryFinding) -> (f64, f64, f64) {
    let HistoryKind::ArchitecturalDrift(e) = &f.kind else { panic!("expected drift") };
    (e.confidence, e.lift, e.jaccard)
}

fn relax(th: &mut HistoryThresholds) {
    th.co_change.min_confidence = 0.0;
    th.co_change.min_lift = 0.0;
}

fn ranked(commits: &[Commit], typed_paths: &HashSet<PathBuf>, th: &HistoryThresholds) -> Vec<HistoryFinding> {
    let pairs = mine(commits, th);
    let scope = revisions_in_scope(commits, th);
    rank_drift(pairs, &scope, &empty_graph(), typed_paths, th)
}

#[test]
fn rank_drift_filters_below_min_support() {
    let mut th = t().history;
    th.co_change.min_support = 3;
    relax(&mut th);
    let commits = vec![commit("h1", "a@x", 1, &["a.py", "b.py"]), commit("h2", "a@x", 2, &["a.py", "b.py"])];
    let findings = ranked(&commits, &typed(&["a.py", "b.py"]), &th);
    assert!(findings.is_empty());
}

#[test]
fn rank_drift_includes_at_min_support() {
    let mut th = t().history;
    th.co_change.min_support = 3;
    relax(&mut th);
    let commits = vec![
        commit("h1", "a@x", 1, &["a.py", "b.py"]),
        commit("h2", "a@x", 2, &["a.py", "b.py"]),
        commit("h3", "a@x", 3, &["a.py", "b.py"]),
    ];
    let findings = ranked(&commits, &typed(&["a.py", "b.py"]), &th);
    assert_eq!(findings.len(), 1);
}

#[test]
fn rank_drift_suppresses_directly_linked_pair() {
    let mut th = t().history;
    th.co_change.min_support = 1;
    relax(&mut th);
    let commits = vec![commit("h1", "a@x", 1, &["a.py", "b.py"])];
    let pairs = mine(&commits, &th);
    let scope = revisions_in_scope(&commits, &th);
    let typed_paths = typed(&["a.py", "b.py"]);
    let graph = graph_linking("a.py", "b.py");
    let findings = rank_drift(pairs, &scope, &graph, &typed_paths, &th);
    assert!(findings.is_empty(), "linked pair should be suppressed");
}

#[test]
fn rank_drift_keeps_unlinked_pair() {
    let mut th = t().history;
    th.co_change.min_support = 1;
    relax(&mut th);
    let commits = vec![commit("h1", "a@x", 1, &["a.py", "b.py"])];
    let findings = ranked(&commits, &typed(&["a.py", "b.py"]), &th);
    assert_eq!(findings.len(), 1);
}

#[test]
fn rank_drift_filters_pair_with_path_outside_typed_set() {
    let mut th = t().history;
    th.co_change.min_support = 1;
    relax(&mut th);
    let commits = vec![commit("h1", "a@x", 1, &["a.py", "b.py"])];
    let findings = ranked(&commits, &typed(&["a.py"]), &th);
    assert!(findings.is_empty());
}

#[test]
fn rank_drift_sorts_by_confidence_desc() {
    let mut th = t().history;
    th.co_change.min_support = 1;
    let commits = vec![
        commit("h1", "a@x", 1, &["a.py", "b.py"]),
        commit("h2", "a@x", 2, &["a.py", "b.py"]),
        commit("h3", "a@x", 3, &["a.py"]),
        commit("h4", "a@x", 4, &["a.py"]),
        commit("h5", "a@x", 5, &["c.py", "d.py"]),
        commit("h6", "a@x", 6, &["c.py", "d.py"]),
        commit("h7", "a@x", 7, &["c.py", "d.py"]),
    ];
    let typed_paths = typed(&["a.py", "b.py", "c.py", "d.py"]);
    let findings = ranked(&commits, &typed_paths, &th);
    assert_eq!(findings.len(), 2);
    let (first_a, _) = drift_pair(&findings[0]);
    assert_eq!(first_a, Path::new("c.py"), "higher-confidence pair ranks first");
    let (c0, _, _) = drift_metrics_of(&findings[0]);
    let (c1, _, _) = drift_metrics_of(&findings[1]);
    assert!(c0 >= c1);
}

#[test]
fn rank_drift_lex_tiebreak_for_equal_confidence() {
    let mut th = t().history;
    th.co_change.min_support = 1;
    let commits = vec![commit("h1", "a@x", 1, &["x.py", "y.py"]), commit("h2", "a@x", 2, &["a.py", "b.py"])];
    let typed_paths = typed(&["a.py", "b.py", "x.py", "y.py"]);
    let findings = ranked(&commits, &typed_paths, &th);
    assert_eq!(findings.len(), 2);
    let (first_a, _) = drift_pair(&findings[0]);
    assert_eq!(first_a, Path::new("a.py"));
}

#[test]
fn rank_drift_truncates_at_max_findings() {
    let mut th = t().history;
    th.co_change.min_support = 1;
    th.co_change.max_findings_reported = 2;
    let commits = vec![
        commit("h1", "a@x", 1, &["a.py", "b.py"]),
        commit("h2", "a@x", 2, &["c.py", "d.py"]),
        commit("h3", "a@x", 3, &["e.py", "f.py"]),
    ];
    let typed_paths = typed(&["a.py", "b.py", "c.py", "d.py", "e.py", "f.py"]);
    let findings = ranked(&commits, &typed_paths, &th);
    assert_eq!(findings.len(), 2);
}

#[test]
fn rank_drift_empty_pairs_returns_empty() {
    let scope = revisions_in_scope(&[], &t().history);
    let findings = rank_drift(std::collections::HashMap::new(), &scope, &empty_graph(), &typed(&[]), &t().history);
    assert!(findings.is_empty());
}

#[test]
fn rank_drift_reports_effective_commit_count() {
    let mut th = t().history;
    th.co_change.min_support = 1;
    let commits = vec![
        commit("h1", "a@x", 1, &["a.py", "b.py"]),
        commit("h2", "a@x", 2, &["z.py"]),
        commit("h3", "a@x", 3, &["z.py"]),
    ];
    let findings = ranked(&commits, &typed(&["a.py", "b.py"]), &th);
    let HistoryKind::ArchitecturalDrift(e) = &findings[0].kind else { panic!() };
    assert_eq!(e.commits, 3);
}

#[test]
fn rank_drift_distinct_authors_count_correct() {
    let mut th = t().history;
    th.co_change.min_support = 1;
    relax(&mut th);
    let commits = vec![
        commit("h1", "alice@x", 1, &["a.py", "b.py"]),
        commit("h2", "bob@x", 2, &["a.py", "b.py"]),
        commit("h3", "carol@x", 3, &["a.py", "b.py"]),
    ];
    let findings = ranked(&commits, &typed(&["a.py", "b.py"]), &th);
    let HistoryKind::ArchitecturalDrift(e) = &findings[0].kind else { panic!() };
    assert_eq!(e.distinct_authors, 3);
}

#[test]
fn rank_drift_action_label_unset_initially() {
    let mut th = t().history;
    th.co_change.min_support = 1;
    relax(&mut th);
    let commits = vec![commit("h1", "a@x", 1, &["a.py", "b.py"])];
    let findings = ranked(&commits, &typed(&["a.py", "b.py"]), &th);
    assert!(findings[0].action_label.is_none());
}

#[test]
fn rank_drift_last_seen_in_evidence() {
    let mut th = t().history;
    th.co_change.min_support = 1;
    relax(&mut th);
    let commits = vec![commit("h1", "a@x", 100, &["a.py", "b.py"]), commit("h2", "a@x", 500, &["a.py", "b.py"])];
    let findings = ranked(&commits, &typed(&["a.py", "b.py"]), &th);
    let HistoryKind::ArchitecturalDrift(e) = &findings[0].kind else { panic!() };
    assert_eq!(e.last_seen_unix, 500);
}

#[test]
fn rank_drift_filters_lift_at_or_below_one() {
    let mut th = t().history;
    th.co_change.min_support = 1;
    let commits = vec![
        commit("h1", "a@x", 1, &["a.py", "b.py"]),
        commit("h2", "a@x", 2, &["a.py", "b.py"]),
        commit("h3", "a@x", 3, &["a.py", "b.py"]),
    ];
    let findings = ranked(&commits, &typed(&["a.py", "b.py"]), &th);
    assert!(findings.is_empty(), "lift of 1.0 (no association beyond chance) is filtered");
}

#[test]
fn rank_drift_keeps_lift_above_one() {
    let mut th = t().history;
    th.co_change.min_support = 1;
    let commits = vec![
        commit("h1", "a@x", 1, &["a.py", "b.py"]),
        commit("h2", "a@x", 2, &["a.py", "b.py"]),
        commit("h3", "a@x", 3, &["a.py", "b.py"]),
        commit("h4", "a@x", 4, &["z.py"]),
        commit("h5", "a@x", 5, &["z.py"]),
    ];
    let findings = ranked(&commits, &typed(&["a.py", "b.py"]), &th);
    assert_eq!(findings.len(), 1);
    let (_, lift, _) = drift_metrics_of(&findings[0]);
    assert!(lift > 1.0, "got lift {lift}");
}

#[test]
fn rank_drift_filters_below_min_confidence() {
    let mut th = t().history;
    th.co_change.min_support = 1;
    let commits = vec![
        commit("h1", "a@x", 1, &["a.py", "b.py"]),
        commit("h2", "a@x", 2, &["a.py", "b.py"]),
        commit("h3", "a@x", 3, &["a.py"]),
        commit("h4", "a@x", 4, &["a.py"]),
        commit("h5", "a@x", 5, &["a.py"]),
        commit("h6", "a@x", 6, &["z.py"]),
        commit("h7", "a@x", 7, &["z.py"]),
    ];
    let findings = ranked(&commits, &typed(&["a.py", "b.py"]), &th);
    assert!(findings.is_empty(), "confidence 0.4 below the 0.5 floor is filtered");
}

#[test]
fn rank_drift_metrics_match_expected() {
    let mut th = t().history;
    th.co_change.min_support = 1;
    let commits = vec![
        commit("h1", "a@x", 1, &["a.py", "b.py"]),
        commit("h2", "a@x", 2, &["a.py", "b.py"]),
        commit("h3", "a@x", 3, &["a.py"]),
        commit("h4", "a@x", 4, &["a.py"]),
        commit("h5", "a@x", 5, &["z.py"]),
        commit("h6", "a@x", 6, &["z.py"]),
    ];
    let findings = ranked(&commits, &typed(&["a.py", "b.py"]), &th);
    assert_eq!(findings.len(), 1);
    let (conf, lift, jaccard) = drift_metrics_of(&findings[0]);
    assert!((conf - 0.5).abs() < 1e-9, "confidence {conf}");
    assert!((lift - 1.5).abs() < 1e-9, "lift {lift}");
    assert!((jaccard - 0.5).abs() < 1e-9, "jaccard {jaccard}");
}
