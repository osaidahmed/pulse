use std::collections::HashSet;
use std::path::{Path, PathBuf};

use pulse::history::build_co_change::{is_build_file, rank_build_drift};
use pulse::history::co_change::{mine, revisions_in_scope};
use pulse::history::finding::{HistoryFinding, HistoryKind};
use pulse::history::git::Commit;
use pulse::history::thresholds::HistoryThresholds;
use pulse::history::{run, HistoryOpts};
use pulse::thresholds::Thresholds;

use crate::history_common::{build_repo, CommitSpec};

fn t() -> HistoryThresholds {
    Thresholds::default().history
}

fn commit(hash: &str, author: &str, ts: i64, files: &[&str]) -> Commit {
    Commit { hash: hash.into(), author: author.into(), timestamp: ts, files: files.iter().map(PathBuf::from).collect() }
}

fn typed(paths: &[&str]) -> HashSet<PathBuf> {
    paths.iter().map(PathBuf::from).collect()
}

fn ranked(commits: &[Commit], sources: &HashSet<PathBuf>, th: &HistoryThresholds) -> Vec<HistoryFinding> {
    let pairs = mine(commits, th);
    let scope = revisions_in_scope(commits, th);
    rank_build_drift(&pairs, &scope, sources, th)
}

fn build_evidence(f: &HistoryFinding) -> (&Path, &Path, u32, u32, f64) {
    let HistoryKind::BuildCoChange(e) = &f.kind else { panic!("expected build co-change") };
    (e.build_file.as_path(), e.source_file.as_path(), e.support, e.source_revisions, e.ratio)
}

#[test]
fn is_build_file_recognizes_manifests() {
    assert!(is_build_file(Path::new("Cargo.toml")));
    assert!(is_build_file(Path::new("crate/Cargo.toml")));
    assert!(is_build_file(Path::new("package.json")));
    assert!(is_build_file(Path::new("app.csproj")));
    assert!(is_build_file(Path::new("Makefile")));
}

#[test]
fn is_build_file_rejects_source() {
    assert!(!is_build_file(Path::new("src/lib.rs")));
    assert!(!is_build_file(Path::new("main.py")));
    assert!(!is_build_file(Path::new("README.md")));
}

#[test]
fn flags_source_coupled_to_manifest() {
    let commits = vec![
        commit("h1", "a@x", 1, &["Cargo.toml", "src/lib.rs"]),
        commit("h2", "a@x", 2, &["Cargo.toml", "src/lib.rs"]),
        commit("h3", "a@x", 3, &["Cargo.toml", "src/lib.rs"]),
    ];
    let findings = ranked(&commits, &typed(&["src/lib.rs"]), &t());
    assert_eq!(findings.len(), 1);
    let (build, source, support, source_revs, ratio) = build_evidence(&findings[0]);
    assert_eq!(build, Path::new("Cargo.toml"));
    assert_eq!(source, Path::new("src/lib.rs"));
    assert_eq!(support, 3);
    assert_eq!(source_revs, 3);
    assert!((ratio - 1.0).abs() < 1e-9, "ratio {ratio}");
}

#[test]
fn filters_below_min_support() {
    let commits = vec![
        commit("h1", "a@x", 1, &["Cargo.toml", "src/lib.rs"]),
        commit("h2", "a@x", 2, &["Cargo.toml", "src/lib.rs"]),
    ];
    let findings = ranked(&commits, &typed(&["src/lib.rs"]), &t());
    assert!(findings.is_empty(), "support 2 below the default floor of 3 is filtered");
}

#[test]
fn filters_below_min_ratio() {
    let mut commits = vec![
        commit("h1", "a@x", 1, &["Cargo.toml", "src/lib.rs"]),
        commit("h2", "a@x", 2, &["Cargo.toml", "src/lib.rs"]),
        commit("h3", "a@x", 3, &["Cargo.toml", "src/lib.rs"]),
    ];
    for i in 0..7 {
        commits.push(commit("s", "a@x", 10 + i, &["src/lib.rs"]));
    }
    let findings = ranked(&commits, &typed(&["src/lib.rs"]), &t());
    assert!(findings.is_empty(), "co-change in 3 of 10 source revisions (0.3) is below the 0.5 floor");
}

#[test]
fn requires_source_in_typed_set() {
    let commits = vec![
        commit("h1", "a@x", 1, &["Cargo.toml", "src/lib.rs"]),
        commit("h2", "a@x", 2, &["Cargo.toml", "src/lib.rs"]),
        commit("h3", "a@x", 3, &["Cargo.toml", "src/lib.rs"]),
    ];
    let findings = ranked(&commits, &typed(&[]), &t());
    assert!(findings.is_empty(), "an untyped source path is not reported");
}

#[test]
fn ignores_build_to_build_pairs() {
    let commits = vec![
        commit("h1", "a@x", 1, &["Cargo.toml", "Cargo.lock"]),
        commit("h2", "a@x", 2, &["Cargo.toml", "Cargo.lock"]),
        commit("h3", "a@x", 3, &["Cargo.toml", "Cargo.lock"]),
    ];
    let findings = ranked(&commits, &typed(&["Cargo.lock"]), &t());
    assert!(findings.is_empty(), "two manifests co-changing is not a source coupling");
}

#[test]
fn ignores_source_to_source_pairs() {
    let mut th = t();
    th.build_co_change.min_support = 1;
    let commits = vec![commit("h1", "a@x", 1, &["src/a.rs", "src/b.rs"])];
    let findings = ranked(&commits, &typed(&["src/a.rs", "src/b.rs"]), &th);
    assert!(findings.is_empty(), "two source files co-changing is plain co-change, not build coupling");
}

#[test]
fn directional_ratio_in_evidence() {
    let commits = vec![
        commit("h1", "a@x", 1, &["Cargo.toml", "src/lib.rs"]),
        commit("h2", "a@x", 2, &["Cargo.toml", "src/lib.rs"]),
        commit("h3", "a@x", 3, &["Cargo.toml", "src/lib.rs"]),
        commit("h4", "a@x", 4, &["src/lib.rs"]),
    ];
    let findings = ranked(&commits, &typed(&["src/lib.rs"]), &t());
    assert_eq!(findings.len(), 1);
    let (_, _, support, source_revs, ratio) = build_evidence(&findings[0]);
    assert_eq!(support, 3);
    assert_eq!(source_revs, 4);
    assert!((ratio - 0.75).abs() < 1e-9, "ratio {ratio}");
}

#[test]
fn sorts_by_ratio_desc() {
    let mut commits = vec![
        commit("a1", "a@x", 1, &["Cargo.toml", "src/a.rs"]),
        commit("a2", "a@x", 2, &["Cargo.toml", "src/a.rs"]),
        commit("a3", "a@x", 3, &["Cargo.toml", "src/a.rs"]),
        commit("b1", "a@x", 4, &["package.json", "web/b.js"]),
        commit("b2", "a@x", 5, &["package.json", "web/b.js"]),
        commit("b3", "a@x", 6, &["package.json", "web/b.js"]),
    ];
    for i in 0..2 {
        commits.push(commit("bx", "a@x", 20 + i, &["web/b.js"]));
    }
    let findings = ranked(&commits, &typed(&["src/a.rs", "web/b.js"]), &t());
    assert_eq!(findings.len(), 2);
    let (_, first_source, _, _, first_ratio) = build_evidence(&findings[0]);
    let (_, _, _, _, second_ratio) = build_evidence(&findings[1]);
    assert_eq!(first_source, Path::new("src/a.rs"), "ratio-1.0 coupling ranks first");
    assert!(first_ratio >= second_ratio);
}

#[test]
fn truncates_at_max_findings() {
    let mut th = t();
    th.build_co_change.max_findings_reported = 1;
    let commits = vec![
        commit("a1", "a@x", 1, &["Cargo.toml", "src/a.rs"]),
        commit("a2", "a@x", 2, &["Cargo.toml", "src/a.rs"]),
        commit("a3", "a@x", 3, &["Cargo.toml", "src/a.rs"]),
        commit("b1", "a@x", 4, &["package.json", "web/b.js"]),
        commit("b2", "a@x", 5, &["package.json", "web/b.js"]),
        commit("b3", "a@x", 6, &["package.json", "web/b.js"]),
    ];
    let findings = ranked(&commits, &typed(&["src/a.rs", "web/b.js"]), &th);
    assert_eq!(findings.len(), 1);
}

fn spec(message: &'static str, body: &'static str) -> CommitSpec<'static> {
    CommitSpec {
        author: "alice <alice@x>",
        message,
        writes: Box::leak(vec![("Cargo.toml", body), ("src/lib.rs", body)].into_boxed_slice()),
        deletes: &[],
    }
}

#[test]
fn end_to_end_reports_build_coupling() {
    let commits = vec![spec("c1", "fn one() {}\n"), spec("c2", "fn two() {}\n"), spec("c3", "fn three() {}\n")];
    let dir = build_repo(&commits);
    let opts = HistoryOpts { root: dir.path().to_path_buf(), include_tests: false, since: None, max_commits: None };
    let findings = run(&opts, &t()).expect("history run");
    let coupling = findings.iter().find_map(|f| match &f.kind {
        HistoryKind::BuildCoChange(e) => Some(e),
        _ => None,
    });
    let e = coupling.expect("a build co-change finding");
    assert!(e.source_file.ends_with("src/lib.rs"), "source {:?}", e.source_file);
    assert!(e.build_file.ends_with("Cargo.toml"), "build {:?}", e.build_file);
    assert_eq!(e.support, 3);
}
