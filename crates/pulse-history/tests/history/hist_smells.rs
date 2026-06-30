use std::collections::HashSet;
use std::path::{Path, PathBuf};

use pulse_history::co_change;
use pulse_history::finding::{BlobEvidence, ChangeShotgunEvidence, HistoryFinding, HistoryKind};
use pulse_history::git::Commit;
use pulse_history::hist_smells::rank;
use pulse_history::thresholds::HistoryThresholds;
use pulse_thresholds::Thresholds;

fn hist_t() -> HistoryThresholds {
    let mut h = Thresholds::default().history;
    h.hist.enabled = true;
    h
}

fn commit(files: &[&str]) -> Commit {
    Commit { hash: "h".into(), author: "a@x".into(), timestamp: 0, files: files.iter().map(PathBuf::from).collect() }
}

fn typed(files: &[&str]) -> HashSet<PathBuf> {
    files.iter().map(PathBuf::from).collect()
}

fn run_hist(commits: &[Commit], typed_paths: &HashSet<PathBuf>, t: &HistoryThresholds) -> Vec<HistoryFinding> {
    let pairs = co_change::mine(commits, t);
    let scope = co_change::revisions_in_scope(commits, t);
    rank(commits, &pairs, &scope, typed_paths, t)
}

fn blobs(commits: &[Commit], typed_paths: &HashSet<PathBuf>) -> Vec<BlobEvidence> {
    run_hist(commits, typed_paths, &hist_t())
        .into_iter()
        .filter_map(|f| match f.kind {
            HistoryKind::FileBlob(e) => Some(e),
            _ => None,
        })
        .collect()
}

fn shotguns(commits: &[Commit], typed_paths: &HashSet<PathBuf>) -> Vec<ChangeShotgunEvidence> {
    run_hist(commits, typed_paths, &hist_t())
        .into_iter()
        .filter_map(|f| match f.kind {
            HistoryKind::ChangeShotgun(e) => Some(e),
            _ => None,
        })
        .collect()
}

#[test]
fn file_in_nearly_every_multi_file_commit_is_a_blob() {
    let mut commits = Vec::new();
    let mut paths: Vec<String> = vec!["god.rs".into()];
    for i in 0..20 {
        let m = format!("mod{i}.rs");
        commits.push(commit(&["god.rs", m.as_str()]));
        paths.push(m);
    }
    let typed_paths: HashSet<PathBuf> = paths.iter().map(PathBuf::from).collect();
    let found = blobs(&commits, &typed_paths);
    assert_eq!(found.len(), 1, "only the file present in nearly every multi-file commit is a blob");
    let g = &found[0];
    assert_eq!(g.file.as_path(), Path::new("god.rs"));
    assert_eq!((g.multi_file_commits, g.total_multi_file_commits), (20, 20));
    assert!((g.blob_ratio - 1.0).abs() < 1e-9);
}

#[test]
fn hist_smells_are_off_by_default() {
    let mut commits = Vec::new();
    for i in 0..20 {
        let m = format!("mod{i}.rs");
        commits.push(commit(&["god.rs", m.as_str()]));
    }
    let typed_paths = typed(&["god.rs"]);
    let default = run_hist(&commits, &typed_paths, &Thresholds::default().history);
    assert!(default.is_empty(), "HIST smells are opt-in via --hist; off by default");
}

#[test]
fn single_file_commits_never_make_a_blob() {
    let commits = vec![commit(&["solo.rs"]), commit(&["solo.rs"]), commit(&["solo.rs"])];
    assert!(
        blobs(&commits, &typed(&["solo.rs"])).is_empty(),
        "a file only ever changed alone has no multi-file footprint"
    );
}

#[test]
fn evenly_distributed_changes_have_no_blob() {
    let mut commits = Vec::new();
    let mut paths = Vec::new();
    for i in 0..20 {
        let a = format!("a{i}.rs");
        let b = format!("b{i}.rs");
        commits.push(commit(&[a.as_str(), b.as_str()]));
        paths.push(a);
        paths.push(b);
    }
    let typed_paths: HashSet<PathBuf> = paths.iter().map(PathBuf::from).collect();
    assert!(blobs(&commits, &typed_paths).is_empty(), "no file dominates the change history");
}

#[test]
fn an_untyped_blob_file_is_not_reported() {
    let mut commits = Vec::new();
    for i in 0..20 {
        let m = format!("mod{i}.rs");
        commits.push(commit(&["generated.bin", m.as_str()]));
    }
    let typed_paths = typed(&[]);
    assert!(blobs(&commits, &typed_paths).is_empty(), "a non-source file is never surfaced as a blob");
}

#[test]
fn file_co_changing_across_many_packages_is_a_change_shotgun() {
    let files = ["core/hub.rs", "pkg1/a.rs", "pkg2/b.rs", "pkg3/c.rs", "pkg4/d.rs"];
    let commits: Vec<Commit> = (0..5).map(|_| commit(&files)).collect();
    let typed_paths = typed(&files);
    let found = shotguns(&commits, &typed_paths);
    let hub = found
        .iter()
        .find(|e| e.file.as_path() == Path::new("core/hub.rs"))
        .expect("hub's changes ripple across four packages");
    assert_eq!(hub.package_count, 4);
    assert_eq!(hub.partner_count, 4);
}

#[test]
fn co_changing_within_few_packages_is_not_a_shotgun() {
    let files = ["core/hub.rs", "pkg1/a.rs", "pkg2/b.rs", "pkg3/c.rs"];
    let commits: Vec<Commit> = (0..5).map(|_| commit(&files)).collect();
    let typed_paths = typed(&files);
    assert!(
        shotguns(&commits, &typed_paths).is_empty(),
        "three packages does not exceed the distinct-package threshold of three"
    );
}

#[test]
fn partners_below_the_confidence_floor_are_not_counted() {
    let mut commits: Vec<Commit> = (0..5).map(|_| commit(&["core/hub.rs", "pkg1/a.rs", "pkg2/b.rs"])).collect();
    commits.extend((0..5).map(|_| commit(&["core/hub.rs", "pkg3/c.rs", "pkg4/d.rs"])));
    let typed_paths = typed(&["core/hub.rs", "pkg1/a.rs", "pkg2/b.rs", "pkg3/c.rs", "pkg4/d.rs"]);
    assert!(
        shotguns(&commits, &typed_paths).is_empty(),
        "partners present in only half of a file's changes fall below the 0.70 confidence floor"
    );
}
