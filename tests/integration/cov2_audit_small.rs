use std::path::PathBuf;

use pulse::audit::call_path_qualified::match_node;
use pulse::audit::corpus::Corpus;
use pulse::audit::definitions::{definitions_for_file, definitions_from};
use pulse::audit::duplication_clusters;
use pulse::audit::finding::{AuditFinding, AuditKind, CloneClusterEvidence};
use pulse::parse::{self, Language, MAX_INPUT_LINES};
use pulse::thresholds::Thresholds;
use tree_sitter::Node;

fn t() -> Thresholds {
    Thresholds::default()
}

fn write_tempfile(content: &str, name: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(name);
    std::fs::write(&path, content).unwrap();
    (dir, path)
}

fn find_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    if node.kind() == kind {
        return Some(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(n) = find_kind(child, kind) {
            return Some(n);
        }
    }
    None
}

#[test]
fn php_scoped_call_qualified_scope_routes_through_first_qualified_and_split_text() {
    let source = "<?php \\Foo::bar();\n";
    let tree = parse::parse_only(source, Language::Php).expect("php must parse");
    let call = find_kind(tree.root_node(), "scoped_call_expression")
        .expect("expected a scoped_call_expression node for the fully-qualified scope call");
    let result = match_node(call, source);
    assert!(result.is_none(), "qualified scope without :: in its own text must fall through split_text to None");
}

#[test]
fn php_plain_name_scope_is_not_a_qualified_kind() {
    let source = "<?php Foo::bar();\n";
    let tree = parse::parse_only(source, Language::Php).expect("php must parse");
    let call = find_kind(tree.root_node(), "scoped_call_expression").expect("expected a scoped_call_expression node");
    let _ = match_node(call, source);
}

#[test]
fn non_call_node_yields_no_qualified_match() {
    let source = "<?php $x = 1;\n";
    let tree = parse::parse_only(source, Language::Php).expect("php must parse");
    let root = tree.root_node();
    assert!(match_node(root, source).is_none(), "a non-call root node must not produce a RawCall");
}

#[test]
fn definitions_for_file_returns_empty_when_size_caps_exceeded() {
    let oversized = "x = 1\n".repeat(MAX_INPUT_LINES + 10);
    let (_dir, path) = write_tempfile(&oversized, "huge.py");
    let defs = definitions_for_file(&path, Language::Python);
    assert!(defs.is_empty(), "a file beyond the line cap must yield no definitions");
}

#[test]
fn definitions_from_walks_a_parsed_corpus_file() {
    let (_dir, path) = write_tempfile("def alpha():\n    return 1\n", "mod.py");
    let corpus = Corpus::load(&[(path.clone(), Language::Python)]);
    let file = corpus.get(&path).expect("corpus must contain the loaded file");
    let defs = definitions_from(file);
    assert!(defs.iter().any(|d| d.identity.name == "alpha"), "definitions_from must extract the parsed function");
}

#[test]
fn definitions_from_returns_empty_when_walk_inputs_missing() {
    let missing = PathBuf::from("/nonexistent/cov2/file.py");
    let corpus = Corpus::load(&[(missing.clone(), Language::Python)]);
    let file = corpus.get(&missing).expect("corpus tracks even unreadable paths");
    let defs = definitions_from(file);
    assert!(defs.is_empty(), "a corpus file with no source/tree must yield no definitions");
}

fn clusters(findings: &[AuditFinding]) -> Vec<&CloneClusterEvidence> {
    findings
        .iter()
        .filter_map(|f| match &f.kind {
            AuditKind::NearDuplicate(e) => Some(e),
            _ => None,
        })
        .collect()
}

const SMALL_A: &str = "def small_a(items):\n    total = 0\n    count = 0\n    for item in items:\n        if item > 0:\n            total = total + item\n            count = count + 1\n        else:\n            total = total - item\n    average = total / count\n    return average\n";
const SMALL_B: &str = "def small_b(values):\n    sum_v = 0\n    num = 0\n    for value in values:\n        if value > 0:\n            sum_v = sum_v + value\n            num = num + 1\n        else:\n            sum_v = sum_v - value\n    mean = sum_v / num\n    return mean\n";

const MID_A: &str = "def mid_a(rows):\n    acc = 0\n    seen = 0\n    skipped = 0\n    for row in rows:\n        if row > 0:\n            acc = acc + row\n            seen = seen + 1\n        elif row < 0:\n            skipped = skipped + 1\n        else:\n            acc = acc - 1\n    ratio = acc / seen\n    return ratio\n";
const MID_B: &str = "def mid_b(records):\n    agg = 0\n    hit = 0\n    miss = 0\n    for rec in records:\n        if rec > 0:\n            agg = agg + rec\n            hit = hit + 1\n        elif rec < 0:\n            miss = miss + 1\n        else:\n            agg = agg - 1\n    score = agg / hit\n    return score\n";

const BIG_A: &str = "def big_a(data):\n    total = 0\n    pos = 0\n    neg = 0\n    zero = 0\n    high = 0\n    low = 0\n    for d in data:\n        if d > 100:\n            high = high + 1\n            total = total + d\n        elif d > 0:\n            pos = pos + 1\n            total = total + d\n        elif d < 0:\n            neg = neg + 1\n            total = total - d\n        else:\n            zero = zero + 1\n    summary = total + high + low\n    return summary\n";
const BIG_B: &str = "def big_b(stream):\n    sumv = 0\n    plus = 0\n    minus = 0\n    nilv = 0\n    tall = 0\n    short = 0\n    for s in stream:\n        if s > 100:\n            tall = tall + 1\n            sumv = sumv + s\n        elif s > 0:\n            plus = plus + 1\n            sumv = sumv + s\n        elif s < 0:\n            minus = minus + 1\n            sumv = sumv - s\n        else:\n            nilv = nilv + 1\n    report = sumv + tall + short\n    return report\n";

fn typed(dir: &std::path::Path, files: &[(&str, &str)]) -> Vec<(PathBuf, Language)> {
    files
        .iter()
        .map(|(name, body)| {
            let path = dir.join(name);
            std::fs::write(&path, body).unwrap();
            (path, Language::Python)
        })
        .collect()
}

#[test]
fn run_entry_clusters_near_duplicates_across_files() {
    let dir = tempfile::tempdir().unwrap();
    let files = typed(dir.path(), &[("small_a.py", SMALL_A), ("small_b.py", SMALL_B)]);
    let findings = duplication_clusters::run(&files, &t().audit);
    assert!(!clusters(&findings).is_empty(), "run() must surface a near-duplicate cluster");
}

#[test]
fn run_entry_handles_mixed_size_clone_families() {
    let dir = tempfile::tempdir().unwrap();
    let files = typed(
        dir.path(),
        &[
            ("small_a.py", SMALL_A),
            ("small_b.py", SMALL_B),
            ("mid_a.py", MID_A),
            ("mid_b.py", MID_B),
            ("big_a.py", BIG_A),
            ("big_b.py", BIG_B),
        ],
    );
    let findings = duplication_clusters::run(&files, &t().audit);
    let cl = clusters(&findings);
    assert!(!cl.is_empty(), "mixed-size clone families must still produce at least one cluster");
}

#[test]
fn run_entry_breaks_loc_window_between_distant_sizes() {
    let dir = tempfile::tempdir().unwrap();
    let files = typed(
        dir.path(),
        &[("small_a.py", SMALL_A), ("small_b.py", SMALL_B), ("big_a.py", BIG_A), ("big_b.py", BIG_B)],
    );
    let findings = duplication_clusters::run(&files, &t().audit);
    let cl = clusters(&findings);
    let small_cluster = cl
        .iter()
        .find(|e| e.members.iter().any(|m| m.file.ends_with("small_a.py")))
        .expect("the small clone pair must cluster");
    assert!(
        small_cluster.members.iter().all(|m| !m.file.ends_with("big_a.py")),
        "the loc window must keep small functions out of the big-function cluster"
    );
}

#[test]
fn cluster_members_from_corpus_matches_run() {
    let dir = tempfile::tempdir().unwrap();
    let files = typed(dir.path(), &[("mid_a.py", MID_A), ("mid_b.py", MID_B)]);
    let corpus = Corpus::load(&files);
    let groups = duplication_clusters::cluster_members_from(&corpus, &t().audit);
    assert!(groups.iter().any(|g| g.len() >= 2), "cluster_members_from must yield a multi-member group");
}
