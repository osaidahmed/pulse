use pulse::history::arch_trend::edges_at_commit;
use pulse::history::finding::HistoryKind;
use pulse::history::thresholds::HistoryThresholds;
use pulse::history::HistoryOpts;

use crate::history_common::{build_repo, CommitSpec};

fn cyclic_repo() -> tempfile::TempDir {
    build_repo(&[
        CommitSpec {
            author: "alice <alice@x>",
            message: "no cycle",
            writes: &[("pkg/a.py", "x = 1\n"), ("pkg/b.py", "y = 2\n"), ("pkg/c.py", "z = 3\n")],
            deletes: &[],
        },
        CommitSpec {
            author: "alice <alice@x>",
            message: "introduce a->b->c->a cycle",
            writes: &[
                ("pkg/a.py", "import pkg.b\n"),
                ("pkg/b.py", "import pkg.c\n"),
                ("pkg/c.py", "import pkg.a\n"),
            ],
            deletes: &[],
        },
    ])
}

fn opts_for(repo: &tempfile::TempDir) -> HistoryOpts {
    HistoryOpts {
        root: repo.path().to_path_buf(),
        include_tests: true,
        since: None,
        max_commits: None,
    }
}

fn has_catalyst(findings: &[pulse::history::finding::HistoryFinding]) -> bool {
    findings.iter().any(|f| matches!(f.kind, HistoryKind::CatalystWarning(_)))
}

#[test]
fn catalyst_flags_a_newly_introduced_cycle() {
    let repo = cyclic_repo();
    let mut t = HistoryThresholds::default();
    t.arch_trend = true;
    let findings = pulse::history::run(&opts_for(&repo), &t).expect("history run");
    assert!(
        has_catalyst(&findings),
        "expected a catalyst warning for the freshly-introduced cycle"
    );
}

#[test]
fn catalyst_is_silent_without_the_flag() {
    let repo = cyclic_repo();
    let t = HistoryThresholds::default();
    let findings = pulse::history::run(&opts_for(&repo), &t).expect("history run");
    assert!(
        !has_catalyst(&findings),
        "catalyst must stay opt-in behind --arch-trend"
    );
}

#[test]
fn reconstructs_import_edges_at_a_commit() {
    let repo = build_repo(&[CommitSpec {
        author: "alice <alice@x>",
        message: "init",
        writes: &[("pkg/a.py", "import pkg.b\n"), ("pkg/b.py", "x = 1\n")],
        deletes: &[],
    }]);

    let edges = edges_at_commit(repo.path(), "HEAD");
    assert!(
        edges
            .iter()
            .any(|e| e.source.ends_with("pkg/a.py") && e.target.ends_with("pkg/b.py")),
        "expected a reconstructed pkg/a.py -> pkg/b.py edge, got: {edges:?}"
    );
}

#[test]
fn reflects_historical_state_not_the_worktree() {
    let repo = build_repo(&[
        CommitSpec {
            author: "alice <alice@x>",
            message: "no edge",
            writes: &[("pkg/a.py", "x = 1\n"), ("pkg/b.py", "y = 2\n")],
            deletes: &[],
        },
        CommitSpec {
            author: "alice <alice@x>",
            message: "introduce edge",
            writes: &[("pkg/a.py", "import pkg.b\n")],
            deletes: &[],
        },
    ]);

    let head = edges_at_commit(repo.path(), "HEAD");
    assert!(
        head.iter().any(|e| e.source.ends_with("pkg/a.py") && e.target.ends_with("pkg/b.py")),
        "HEAD has the edge"
    );
    let parent = edges_at_commit(repo.path(), "HEAD~1");
    assert!(
        !parent.iter().any(|e| e.source.ends_with("pkg/a.py")),
        "the parent commit predates the import edge"
    );
}

#[test]
fn missing_commit_yields_no_edges() {
    let repo = build_repo(&[CommitSpec {
        author: "alice <alice@x>",
        message: "init",
        writes: &[("pkg/a.py", "x = 1\n")],
        deletes: &[],
    }]);
    assert!(edges_at_commit(repo.path(), "deadbeef").is_empty());
}
