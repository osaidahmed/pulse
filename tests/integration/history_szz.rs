use std::path::PathBuf;

use pulse::history::finding::HistoryKind;
use pulse::history::szz;
use pulse::history::thresholds::SzzThresholds;
use pulse::parse::Language;

use crate::history_common::{build_repo, CommitSpec};

fn typed(repo: &std::path::Path, rel: &str) -> Vec<(PathBuf, Language)> {
    vec![(repo.join(rel), Language::Python)]
}

fn enabled() -> SzzThresholds {
    SzzThresholds { enabled: true, min_inducing: 1, ..SzzThresholds::DEFAULTS }
}

fn defect_prone_files(findings: &[pulse::history::finding::HistoryFinding]) -> Vec<(PathBuf, u32)> {
    findings
        .iter()
        .filter_map(|f| match &f.kind {
            HistoryKind::DefectProneFile(e) => Some((e.file.clone(), e.fix_count)),
            _ => None,
        })
        .collect()
}

const BUGGY: &str = "def parse(data):\n    x = data[0]\n    return x + 1\n";
const FIXED: &str = "def parse(data):\n    x = data[0]\n    return x\n";

#[test]
fn disabled_szz_yields_no_findings() {
    let repo = build_repo(&[
        CommitSpec { author: "a <a@x>", message: "add parser", writes: &[("foo.py", BUGGY)], deletes: &[] },
        CommitSpec { author: "a <a@x>", message: "fix parse crash", writes: &[("foo.py", FIXED)], deletes: &[] },
    ]);
    let findings = szz::rank(repo.path(), &typed(repo.path(), "foo.py"), &SzzThresholds::DEFAULTS);
    assert!(findings.is_empty(), "szz is off by default");
}

#[test]
fn file_whose_bug_was_fixed_is_flagged() {
    let repo = build_repo(&[
        CommitSpec { author: "a <a@x>", message: "add parser", writes: &[("foo.py", BUGGY)], deletes: &[] },
        CommitSpec { author: "a <a@x>", message: "fix parse crash", writes: &[("foo.py", FIXED)], deletes: &[] },
    ]);
    let prone = defect_prone_files(&szz::rank(repo.path(), &typed(repo.path(), "foo.py"), &enabled()));
    assert_eq!(prone.len(), 1, "the bug-fixed file should be flagged: {prone:?}");
    assert!(prone[0].0.ends_with("foo.py"));
    assert!(prone[0].1 >= 1, "fix_count should count the fixing commit");
}

#[test]
fn addition_only_fix_is_not_attributed() {
    let guarded = "def parse(data):\n    assert data\n    x = data[0]\n    return x + 1\n";
    let repo = build_repo(&[
        CommitSpec { author: "a <a@x>", message: "add parser", writes: &[("foo.py", BUGGY)], deletes: &[] },
        CommitSpec {
            author: "a <a@x>",
            message: "fix: guard empty input",
            writes: &[("foo.py", guarded)],
            deletes: &[],
        },
    ]);
    let prone = defect_prone_files(&szz::rank(repo.path(), &typed(repo.path(), "foo.py"), &enabled()));
    assert!(prone.is_empty(), "a fix that only adds lines blames no introducer: {prone:?}");
}

#[test]
fn non_fix_history_yields_no_findings() {
    let repo = build_repo(&[
        CommitSpec { author: "a <a@x>", message: "add parser", writes: &[("foo.py", BUGGY)], deletes: &[] },
        CommitSpec { author: "a <a@x>", message: "refactor parser", writes: &[("foo.py", FIXED)], deletes: &[] },
    ]);
    let prone = defect_prone_files(&szz::rank(repo.path(), &typed(repo.path(), "foo.py"), &enabled()));
    assert!(prone.is_empty(), "no commit message signals a fix");
}
