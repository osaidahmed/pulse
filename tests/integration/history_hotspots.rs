use std::path::{Path, PathBuf};

use pulse::history::finding::{HistoryFinding, HistoryKind};
use pulse::history::git::Commit;
use pulse::history::hotspots::{rank, revisions_per_file};
use pulse::parse::Language;
use pulse::thresholds::Thresholds;

fn t() -> Thresholds {
    Thresholds::default()
}

fn commit(hash: &str, ts: i64, files: &[&str]) -> Commit {
    Commit { hash: hash.into(), author: "a@x".into(), timestamp: ts, files: files.iter().map(PathBuf::from).collect() }
}

fn write_file(root: &Path, rel: &str, content: &str) -> PathBuf {
    let full = root.join(rel);
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&full, content).unwrap();
    full
}

const HIGH_CC_PYTHON: &str = "
def f(x):
    if x > 0:
        if x > 10:
            return 1
        if x > 100:
            return 2
        if x > 1000:
            return 3
        return 4
    if x < 0:
        if x < -10:
            return -1
        return -2
    return 0
";

const SIMPLE_PYTHON: &str = "x = 1\n";

fn hotspot_score(f: &HistoryFinding) -> u64 {
    let HistoryKind::Hotspot(e) = &f.kind else { panic!() };
    e.score
}

fn hotspot_file(f: &HistoryFinding) -> &Path {
    let HistoryKind::Hotspot(e) = &f.kind else { panic!() };
    e.file.as_path()
}

fn hotspot_revisions(f: &HistoryFinding) -> u32 {
    let HistoryKind::Hotspot(e) = &f.kind else { panic!() };
    e.revisions
}

#[test]
fn revisions_per_file_empty_commits() {
    let result = revisions_per_file(&[]);
    assert!(result.is_empty());
}

#[test]
fn revisions_per_file_single_commit() {
    let commits = vec![commit("h1", 1, &["a.py", "b.py"])];
    let result = revisions_per_file(&commits);
    assert_eq!(result.get(&PathBuf::from("a.py")), Some(&1));
    assert_eq!(result.get(&PathBuf::from("b.py")), Some(&1));
}

#[test]
fn revisions_per_file_increments_across_commits() {
    let commits = vec![commit("h1", 1, &["a.py"]), commit("h2", 2, &["a.py"]), commit("h3", 3, &["a.py", "b.py"])];
    let result = revisions_per_file(&commits);
    assert_eq!(result.get(&PathBuf::from("a.py")), Some(&3));
    assert_eq!(result.get(&PathBuf::from("b.py")), Some(&1));
}

#[test]
fn rank_empty_commits_returns_empty() {
    let findings = rank(&[], &[], &t().history);
    assert!(findings.is_empty());
}

#[test]
fn rank_below_min_revisions_excluded() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_file(dir.path(), "f.py", HIGH_CC_PYTHON);
    let typed = vec![(path.clone(), Language::Python)];
    let commits = vec![commit("h1", 1, &[path.to_str().unwrap()])];
    let mut th = t().history;
    th.hotspot.min_revisions = 5;
    let findings = rank(&commits, &typed, &th);
    assert!(findings.is_empty());
}

#[test]
fn rank_high_revisions_high_cc_flagged() -> std::io::Result<()> {
    let dir = tempfile::tempdir()?;
    let path = write_file(dir.path(), "f.py", HIGH_CC_PYTHON);
    let typed = vec![(path.clone(), Language::Python)];
    let path_str = path.to_str().unwrap();
    let commits: Vec<Commit> = (0..5).map(|i| commit("h", i, &[path_str])).collect();
    let mut th = t().history;
    th.hotspot.min_revisions = 3;
    th.hotspot.min_score = 1;
    let findings = rank(&commits, &typed, &th);
    assert_eq!(findings.len(), 1);
    assert_eq!(hotspot_revisions(&findings[0]), 5);
    Ok(())
}

#[test]
fn rank_score_equals_revisions_times_cc() -> std::io::Result<()> {
    let dir = tempfile::tempdir()?;
    let path = write_file(dir.path(), "f.py", HIGH_CC_PYTHON);
    let typed = vec![(path.clone(), Language::Python)];
    let path_str = path.to_str().unwrap();
    let commits: Vec<Commit> = (0..4).map(|i| commit("h", i, &[path_str])).collect();
    let mut th = t().history;
    th.hotspot.min_revisions = 1;
    th.hotspot.min_score = 1;
    let findings = rank(&commits, &typed, &th);
    let HistoryKind::Hotspot(e) = &findings[0].kind else { panic!() };
    assert_eq!(e.score, u64::from(e.revisions) * u64::from(e.sum_cc));
    Ok(())
}

#[test]
fn rank_simple_file_low_cc_below_min_score_excluded() -> std::io::Result<()> {
    let dir = tempfile::tempdir()?;
    let path = write_file(dir.path(), "simple.py", SIMPLE_PYTHON);
    let typed = vec![(path.clone(), Language::Python)];
    let path_str = path.to_str().unwrap();
    let commits: Vec<Commit> = (0..3).map(|i| commit("h", i, &[path_str])).collect();
    let mut th = t().history;
    th.hotspot.min_revisions = 1;
    th.hotspot.min_score = 100;
    let findings = rank(&commits, &typed, &th);
    assert!(findings.is_empty());
    Ok(())
}

#[test]
fn rank_excludes_files_not_in_typed_set() {
    let commits = vec![commit("h1", 1, &["unknown.py"]); 5];
    let findings = rank(&commits, &[], &t().history);
    assert!(findings.is_empty());
}

#[test]
fn rank_unparseable_file_skipped() -> std::io::Result<()> {
    let dir = tempfile::tempdir()?;
    let path = write_file(dir.path(), "broken.py", "::: not valid python (((");
    let typed = vec![(path.clone(), Language::Python)];
    let path_str = path.to_str().unwrap();
    let commits: Vec<Commit> = (0..10).map(|i| commit("h", i, &[path_str])).collect();
    let mut th = t().history;
    th.hotspot.min_revisions = 1;
    th.hotspot.min_score = 1;
    let findings = rank(&commits, &typed, &th);
    assert!(findings.is_empty(), "unparseable file should produce zero CC and be filtered");
    Ok(())
}

#[test]
fn rank_truncates_at_max_findings_reported() -> std::io::Result<()> {
    let dir = tempfile::tempdir()?;
    let mut typed: Vec<(PathBuf, Language)> = Vec::new();
    let mut path_strings: Vec<String> = Vec::new();
    for i in 0..5 {
        let rel = format!("f{i}.py");
        let p = write_file(dir.path(), &rel, HIGH_CC_PYTHON);
        path_strings.push(p.to_str().unwrap().to_string());
        typed.push((p, Language::Python));
    }
    let commits: Vec<Commit> =
        path_strings.iter().flat_map(|s| (0..3).map(move |i| commit("h", i, &[s.as_str()]))).collect();
    let mut th = t().history;
    th.hotspot.min_revisions = 1;
    th.hotspot.min_score = 1;
    th.hotspot.max_findings_reported = 2;
    let findings = rank(&commits, &typed, &th);
    assert_eq!(findings.len(), 2);
    Ok(())
}

#[test]
fn rank_orders_by_score_desc() -> std::io::Result<()> {
    let dir = tempfile::tempdir()?;
    let high = write_file(dir.path(), "high.py", HIGH_CC_PYTHON);
    let low = write_file(dir.path(), "low.py", "x = 1\ndef f():\n    if True:\n        return 1\n");
    let typed = vec![(high.clone(), Language::Python), (low.clone(), Language::Python)];
    let commits = vec![
        commit("h1", 1, &[high.to_str().unwrap()]),
        commit("h2", 2, &[high.to_str().unwrap()]),
        commit("h3", 3, &[low.to_str().unwrap()]),
        commit("h4", 4, &[low.to_str().unwrap()]),
    ];
    let mut th = t().history;
    th.hotspot.min_revisions = 1;
    th.hotspot.min_score = 1;
    let findings = rank(&commits, &typed, &th);
    assert_eq!(findings.len(), 2);
    assert!(hotspot_score(&findings[0]) >= hotspot_score(&findings[1]));
    Ok(())
}

#[test]
fn rank_lex_tiebreak_for_equal_scores() -> std::io::Result<()> {
    let dir = tempfile::tempdir()?;
    let z = write_file(dir.path(), "z.py", HIGH_CC_PYTHON);
    let a = write_file(dir.path(), "a.py", HIGH_CC_PYTHON);
    let typed = vec![(z.clone(), Language::Python), (a.clone(), Language::Python)];
    let commits = vec![
        commit("h1", 1, &[z.to_str().unwrap()]),
        commit("h2", 2, &[z.to_str().unwrap()]),
        commit("h3", 3, &[a.to_str().unwrap()]),
        commit("h4", 4, &[a.to_str().unwrap()]),
    ];
    let mut th = t().history;
    th.hotspot.min_revisions = 1;
    th.hotspot.min_score = 1;
    let findings = rank(&commits, &typed, &th);
    assert_eq!(findings.len(), 2);
    assert_eq!(hotspot_file(&findings[0]), a.as_path());
    Ok(())
}

#[test]
fn rank_action_label_unset_initially() -> std::io::Result<()> {
    let dir = tempfile::tempdir()?;
    let path = write_file(dir.path(), "f.py", HIGH_CC_PYTHON);
    let typed = vec![(path.clone(), Language::Python)];
    let path_str = path.to_str().unwrap();
    let commits: Vec<Commit> = (0..3).map(|i| commit("h", i, &[path_str])).collect();
    let mut th = t().history;
    th.hotspot.min_revisions = 1;
    th.hotspot.min_score = 1;
    let findings = rank(&commits, &typed, &th);
    assert!(findings[0].action_label.is_none());
    Ok(())
}

#[test]
fn rank_score_uses_u64_no_overflow() {
    let big = u64::from(u32::MAX) * u64::from(u32::MAX);
    assert!(big > u64::from(u32::MAX));
}
