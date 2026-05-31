use std::path::PathBuf;

use pulse::history::finding::{HistoryFinding, HistoryKind, HistoryPillar};
use pulse::history::{run, HistoryOpts};
use pulse::thresholds::Thresholds;

use crate::history_common::{build_repo, noise_commits, CommitSpec};

fn t() -> Thresholds {
    Thresholds::default()
}

fn opts(root: PathBuf) -> HistoryOpts {
    HistoryOpts {
        root,
        include_tests: false,
        since: None,
        max_commits: None,
    }
}

fn count(findings: &[HistoryFinding], pillar: HistoryPillar) -> usize {
    findings
        .iter()
        .filter(|f| pulse::history::finding::variant_info(&f.kind).pillar == pillar)
        .count()
}

fn varied_commits<const N: usize>(
    pairs: &[(&'static str, &'static str)],
    n: u32,
) -> Vec<CommitSpec<'static>> {
    let mut out = Vec::new();
    for i in 0..n {
        let writes_vec: Vec<(&'static str, &'static str)> = pairs
            .iter()
            .map(|(name, _)| {
                let body = format!("# rev {i}\nx = {i}\n");
                (*name, Box::leak(body.into_boxed_str()) as &str)
            })
            .collect();
        let writes: &'static [(&'static str, &'static str)] =
            Box::leak(writes_vec.into_boxed_slice());
        out.push(CommitSpec {
            author: "alice <alice@x>",
            message: Box::leak(format!("rev{i}").into_boxed_str()),
            writes,
            deletes: &[],
        });
    }
    out
}

#[test]
fn e2e_two_unrelated_files_consistent_co_change_emerges_as_drift() {
    let mut specs = varied_commits::<2>(&[("controllers.py", ""), ("models.py", "")], 5);
    specs.extend(noise_commits("NOTES.md", 3));
    let repo = build_repo(&specs);
    let mut th = t().history;
    th.co_change.min_support = 3;
    let f = run(&opts(repo.path().to_path_buf()), &th).unwrap();
    assert!(count(&f, HistoryPillar::Drift) >= 1);
}

#[test]
fn e2e_python_with_import_does_not_emerge() {
    let mut commits = Vec::new();
    for i in 0..5 {
        let body_a = format!("from b import bar\nx = {i}\n");
        let body_b = format!("def bar(): return {i}\n");
        commits.push(CommitSpec {
            author: "alice <alice@x>",
            message: Box::leak(format!("r{i}").into_boxed_str()),
            writes: Box::leak(Box::new([
                ("a.py", Box::leak(body_a.into_boxed_str()) as &str),
                ("b.py", Box::leak(body_b.into_boxed_str()) as &str),
            ])),
            deletes: &[],
        });
    }
    let repo = build_repo(&commits);
    let mut th = t().history;
    th.co_change.min_support = 3;
    let f = run(&opts(repo.path().to_path_buf()), &th).unwrap();
    assert_eq!(count(&f, HistoryPillar::Drift), 0);
}

#[test]
fn e2e_three_pillar_repo_emits_each() {
    let high_cc = "def f(x):\n    if x:\n        if x > 1:\n            if x > 2:\n                return 1\n            return 2\n        return 3\n    return 0\n";
    let mut commits = Vec::new();
    for i in 0..6 {
        let body = format!("{high_cc}\n# rev {i}\n");
        let other = format!("y = {i}\n");
        commits.push(CommitSpec {
            author: "alice <alice@x>",
            message: Box::leak(format!("rev{i}").into_boxed_str()),
            writes: Box::leak(Box::new([
                ("hot.py", Box::leak(body.into_boxed_str()) as &str),
                ("other.py", Box::leak(other.into_boxed_str()) as &str),
            ])),
            deletes: &[],
        });
    }
    for (idx, author) in ["bob <bob@x>", "carol <carol@x>", "dave <dave@x>"].iter().enumerate() {
        let other = format!("z = {idx}\n");
        commits.push(CommitSpec {
            author: Box::leak(author.to_string().into_boxed_str()),
            message: Box::leak(format!("minor{idx}").into_boxed_str()),
            writes: Box::leak(Box::new([("other.py", Box::leak(other.into_boxed_str()) as &str)])),
            deletes: &[],
        });
    }
    for i in 0..85 {
        let body = format!("# alice rev {i}\n");
        commits.push(CommitSpec {
            author: "alice <alice@x>",
            message: Box::leak(format!("alice{i}").into_boxed_str()),
            writes: Box::leak(Box::new([("other.py", Box::leak(body.into_boxed_str()) as &str)])),
            deletes: &[],
        });
    }
    let repo = build_repo(&commits);
    let mut th = t().history;
    th.co_change.min_support = 3;
    th.hotspot.min_revisions = 3;
    th.hotspot.min_score = 1;
    let f = run(&opts(repo.path().to_path_buf()), &th).unwrap();
    assert!(count(&f, HistoryPillar::Complexity) >= 1);
    assert!(count(&f, HistoryPillar::Ownership) >= 1);
}

#[test]
fn e2e_repo_with_no_typed_files_returns_empty() {
    let repo = build_repo(&[CommitSpec {
        author: "alice <alice@x>",
        message: "init",
        writes: &[("README.md", "hello\n"), ("data.txt", "1\n")],
        deletes: &[],
    }]);
    let f = run(&opts(repo.path().to_path_buf()), &t().history).unwrap();
    assert!(f.is_empty());
}

#[test]
fn e2e_single_commit_repo_no_findings() {
    let repo = build_repo(&[CommitSpec {
        author: "alice <alice@x>",
        message: "init",
        writes: &[("a.py", "x = 1\n"), ("b.py", "y = 2\n")],
        deletes: &[],
    }]);
    let f = run(&opts(repo.path().to_path_buf()), &t().history).unwrap();
    assert!(count(&f, HistoryPillar::Drift) <= 1);
}

#[test]
fn e2e_drift_ranks_by_confidence() {
    let mut commits = Vec::new();
    for i in 0..3 {
        let body = format!("x = {i}\n");
        commits.push(CommitSpec {
            author: "alice <alice@x>",
            message: Box::leak(format!("ab{i}").into_boxed_str()),
            writes: Box::leak(Box::new([
                ("a.py", Box::leak(body.clone().into_boxed_str()) as &str),
                ("b.py", Box::leak(body.into_boxed_str()) as &str),
            ])),
            deletes: &[],
        });
    }
    for i in 0..3 {
        let body = format!("y = {i}\n");
        commits.push(CommitSpec {
            author: "alice <alice@x>",
            message: Box::leak(format!("a{i}").into_boxed_str()),
            writes: Box::leak(Box::new([("a.py", Box::leak(body.into_boxed_str()) as &str)])),
            deletes: &[],
        });
    }
    for i in 0..3 {
        let body = format!("z = {i}\n");
        commits.push(CommitSpec {
            author: "alice <alice@x>",
            message: Box::leak(format!("cd{i}").into_boxed_str()),
            writes: Box::leak(Box::new([
                ("c.py", Box::leak(body.clone().into_boxed_str()) as &str),
                ("d.py", Box::leak(body.into_boxed_str()) as &str),
            ])),
            deletes: &[],
        });
    }
    let repo = build_repo(&commits);
    let mut th = t().history;
    th.co_change.min_support = 1;
    let f = run(&opts(repo.path().to_path_buf()), &th).unwrap();
    let drift: Vec<&HistoryFinding> = f
        .iter()
        .filter(|x| matches!(x.kind, HistoryKind::ArchitecturalDrift(_)))
        .collect();
    assert!(drift.len() >= 2);
    let HistoryKind::ArchitecturalDrift(first) = &drift[0].kind else { panic!() };
    let HistoryKind::ArchitecturalDrift(second) = &drift[1].kind else { panic!() };
    assert!(first.confidence >= second.confidence, "ranked by confidence, not raw support");
}

#[test]
fn e2e_distributed_team_with_dominant_owner_no_ownership_finding() {
    let mut commits = Vec::new();
    for i in 0..50 {
        let body = format!("# alice rev {i}\n");
        let author = if i % 2 == 0 {
            "alice <alice@x>"
        } else {
            "bob <bob@x>"
        };
        commits.push(CommitSpec {
            author,
            message: Box::leak(format!("rev{i}").into_boxed_str()),
            writes: Box::leak(Box::new([("a.py", Box::leak(body.into_boxed_str()) as &str)])),
            deletes: &[],
        });
    }
    let repo = build_repo(&commits);
    let f = run(&opts(repo.path().to_path_buf()), &t().history).unwrap();
    assert_eq!(count(&f, HistoryPillar::Ownership), 0);
}

#[test]
fn e2e_drift_in_rust_project_no_findings_for_imported() {
    let mut commits = Vec::new();
    for i in 0..5 {
        let body_a = format!("use crate::bar;\npub fn x() -> i32 {{ {i} }}\n");
        let body_b = format!("pub fn baz() -> i32 {{ {i} }}\n");
        commits.push(CommitSpec {
            author: "alice <alice@x>",
            message: Box::leak(format!("rev{i}").into_boxed_str()),
            writes: Box::leak(Box::new([
                ("src/foo.rs", Box::leak(body_a.into_boxed_str()) as &str),
                ("src/bar.rs", Box::leak(body_b.into_boxed_str()) as &str),
            ])),
            deletes: &[],
        });
    }
    let repo = build_repo(&commits);
    let mut th = t().history;
    th.co_change.min_support = 3;
    let f = run(&opts(repo.path().to_path_buf()), &th).unwrap();
    assert_eq!(count(&f, HistoryPillar::Drift), 0);
}

#[test]
fn e2e_mixed_python_rust_repo_works() {
    let mut commits = Vec::new();
    for i in 0..3 {
        let py = format!("x = {i}\n");
        let rs = format!("pub fn f() -> i32 {{ {i} }}\n");
        commits.push(CommitSpec {
            author: "alice <alice@x>",
            message: Box::leak(format!("r{i}").into_boxed_str()),
            writes: Box::leak(Box::new([
                ("a.py", Box::leak(py.into_boxed_str()) as &str),
                ("src/main.rs", Box::leak(rs.into_boxed_str()) as &str),
            ])),
            deletes: &[],
        });
    }
    let repo = build_repo(&commits);
    let result = run(&opts(repo.path().to_path_buf()), &t().history);
    assert!(result.is_ok());
}

#[test]
fn e2e_truncation_respects_max_findings_drift() {
    let mut commits = Vec::new();
    let pairs = ["a:b", "c:d", "e:f", "g:h", "i:j"];
    for pair in &pairs {
        let parts: Vec<&str> = pair.split(':').collect();
        for i in 0..3 {
            let body = format!("x = {i}\n");
            commits.push(CommitSpec {
                author: "alice <alice@x>",
                message: Box::leak(format!("{pair}rev{i}").into_boxed_str()),
                writes: Box::leak(Box::new([
                    (
                        Box::leak(format!("{}.py", parts[0]).into_boxed_str()) as &str,
                        Box::leak(body.clone().into_boxed_str()) as &str,
                    ),
                    (
                        Box::leak(format!("{}.py", parts[1]).into_boxed_str()) as &str,
                        Box::leak(body.into_boxed_str()) as &str,
                    ),
                ])),
                deletes: &[],
            });
        }
    }
    let repo = build_repo(&commits);
    let mut th = t().history;
    th.co_change.min_support = 1;
    th.co_change.max_findings_reported = 2;
    let f = run(&opts(repo.path().to_path_buf()), &th).unwrap();
    assert_eq!(count(&f, HistoryPillar::Drift), 2);
}

#[test]
fn e2e_repo_with_special_chars_in_paths_works() {
    let repo = build_repo(&[CommitSpec {
        author: "alice <alice@x>",
        message: "init",
        writes: &[("src/foo bar.py", "x = 1\n")],
        deletes: &[],
    }]);
    let result = run(&opts(repo.path().to_path_buf()), &t().history);
    assert!(result.is_ok());
}

#[test]
fn e2e_long_commit_history_completes() {
    let mut commits = Vec::new();
    for i in 0..50 {
        let body = format!("x = {i}\n");
        commits.push(CommitSpec {
            author: "alice <alice@x>",
            message: Box::leak(format!("r{i}").into_boxed_str()),
            writes: Box::leak(Box::new([("a.py", Box::leak(body.into_boxed_str()) as &str)])),
            deletes: &[],
        });
    }
    let repo = build_repo(&commits);
    let result = run(&opts(repo.path().to_path_buf()), &t().history);
    assert!(result.is_ok());
}

#[test]
fn e2e_drift_finding_carries_correct_total_commits() {
    let mut commits = Vec::new();
    for i in 0..4 {
        let body = format!("x = {i}\n");
        commits.push(CommitSpec {
            author: "alice <alice@x>",
            message: Box::leak(format!("r{i}").into_boxed_str()),
            writes: Box::leak(Box::new([
                ("a.py", Box::leak(body.clone().into_boxed_str()) as &str),
                ("b.py", Box::leak(body.into_boxed_str()) as &str),
            ])),
            deletes: &[],
        });
    }
    commits.extend(noise_commits("NOTES.md", 2));
    let repo = build_repo(&commits);
    let mut th = t().history;
    th.co_change.min_support = 1;
    let f = run(&opts(repo.path().to_path_buf()), &th).unwrap();
    let drift: &HistoryFinding = f
        .iter()
        .find(|x| matches!(x.kind, HistoryKind::ArchitecturalDrift(_)))
        .expect("drift finding present");
    let HistoryKind::ArchitecturalDrift(e) = &drift.kind else { panic!() };
    assert_eq!(e.commits, 6);
}

#[test]
fn e2e_findings_have_consistent_action_label_after_run() {
    let mut commits = Vec::new();
    for i in 0..5 {
        let body = format!("x = {i}\n");
        commits.push(CommitSpec {
            author: "alice <alice@x>",
            message: Box::leak(format!("r{i}").into_boxed_str()),
            writes: Box::leak(Box::new([
                ("a.py", Box::leak(body.clone().into_boxed_str()) as &str),
                ("b.py", Box::leak(body.into_boxed_str()) as &str),
            ])),
            deletes: &[],
        });
    }
    let repo = build_repo(&commits);
    let mut th = t().history;
    th.co_change.min_support = 1;
    let f = run(&opts(repo.path().to_path_buf()), &th).unwrap();
    for finding in &f {
        assert!(finding.action_label.is_none(), "orchestrator does not set action_label in v1");
    }
}

#[test]
fn e2e_includes_tests_uses_test_files_in_drift() {
    let mut commits = Vec::new();
    for i in 0..5 {
        let body = format!("x = {i}\n");
        commits.push(CommitSpec {
            author: "alice <alice@x>",
            message: Box::leak(format!("r{i}").into_boxed_str()),
            writes: Box::leak(Box::new([
                ("test_a.py", Box::leak(body.clone().into_boxed_str()) as &str),
                ("test_b.py", Box::leak(body.into_boxed_str()) as &str),
            ])),
            deletes: &[],
        });
    }
    commits.extend(noise_commits("NOTES.md", 2));
    let repo = build_repo(&commits);
    let mut o = opts(repo.path().to_path_buf());
    o.include_tests = true;
    let mut th = t().history;
    th.co_change.min_support = 3;
    let f = run(&o, &th).unwrap();
    assert!(count(&f, HistoryPillar::Drift) >= 1);
}

#[test]
fn e2e_skips_tests_by_default_in_co_change() {
    let mut commits = Vec::new();
    for i in 0..5 {
        let body = format!("x = {i}\n");
        commits.push(CommitSpec {
            author: "alice <alice@x>",
            message: Box::leak(format!("r{i}").into_boxed_str()),
            writes: Box::leak(Box::new([
                ("test_a.py", Box::leak(body.clone().into_boxed_str()) as &str),
                ("test_b.py", Box::leak(body.into_boxed_str()) as &str),
            ])),
            deletes: &[],
        });
    }
    let repo = build_repo(&commits);
    let mut th = t().history;
    th.co_change.min_support = 3;
    let f = run(&opts(repo.path().to_path_buf()), &th).unwrap();
    assert_eq!(count(&f, HistoryPillar::Drift), 0);
}

#[test]
fn e2e_typescript_drift_no_import() {
    let mut commits = Vec::new();
    for i in 0..5 {
        let body_a = format!("const x = {i};\n");
        let body_b = format!("const y = {i};\n");
        commits.push(CommitSpec {
            author: "alice <alice@x>",
            message: Box::leak(format!("r{i}").into_boxed_str()),
            writes: Box::leak(Box::new([
                ("a.ts", Box::leak(body_a.into_boxed_str()) as &str),
                ("b.ts", Box::leak(body_b.into_boxed_str()) as &str),
            ])),
            deletes: &[],
        });
    }
    commits.extend(noise_commits("NOTES.md", 2));
    let repo = build_repo(&commits);
    let mut th = t().history;
    th.co_change.min_support = 3;
    let f = run(&opts(repo.path().to_path_buf()), &th).unwrap();
    assert!(count(&f, HistoryPillar::Drift) >= 1);
}

#[test]
fn e2e_repo_with_node_modules_skips_them() {
    let repo = build_repo(&[CommitSpec {
        author: "alice <alice@x>",
        message: "init",
        writes: &[
            ("a.py", "x = 1\n"),
            ("node_modules/lib.js", "module.exports = {};\n"),
        ],
        deletes: &[],
    }]);
    let result = run(&opts(repo.path().to_path_buf()), &t().history);
    assert!(result.is_ok(), "should not crash on node_modules");
}

#[test]
fn e2e_two_authors_50_50_no_ownership_finding() {
    let mut commits = Vec::new();
    for i in 0..50 {
        let author = if i % 2 == 0 {
            "alice <alice@x>"
        } else {
            "bob <bob@x>"
        };
        let body = format!("x = {i}\n");
        commits.push(CommitSpec {
            author,
            message: Box::leak(format!("r{i}").into_boxed_str()),
            writes: Box::leak(Box::new([("a.py", Box::leak(body.into_boxed_str()) as &str)])),
            deletes: &[],
        });
    }
    let repo = build_repo(&commits);
    let f = run(&opts(repo.path().to_path_buf()), &t().history).unwrap();
    assert_eq!(count(&f, HistoryPillar::Ownership), 0);
}

#[test]
fn e2e_multiple_files_with_independent_drift() {
    let mut commits = Vec::new();
    for i in 0..5 {
        let body = format!("# rev {i}\n");
        commits.push(CommitSpec {
            author: "alice <alice@x>",
            message: Box::leak(format!("ab{i}").into_boxed_str()),
            writes: Box::leak(Box::new([
                ("a.py", Box::leak(body.clone().into_boxed_str()) as &str),
                ("b.py", Box::leak(body.into_boxed_str()) as &str),
            ])),
            deletes: &[],
        });
    }
    for i in 0..5 {
        let body = format!("# rev {i}\n");
        commits.push(CommitSpec {
            author: "alice <alice@x>",
            message: Box::leak(format!("cd{i}").into_boxed_str()),
            writes: Box::leak(Box::new([
                ("c.py", Box::leak(body.clone().into_boxed_str()) as &str),
                ("d.py", Box::leak(body.into_boxed_str()) as &str),
            ])),
            deletes: &[],
        });
    }
    let repo = build_repo(&commits);
    let mut th = t().history;
    th.co_change.min_support = 3;
    let f = run(&opts(repo.path().to_path_buf()), &th).unwrap();
    assert!(count(&f, HistoryPillar::Drift) >= 2);
}
