
use crate::audit_common::*;
use pulse::audit::walker::{extract_subtrees, SubtreeRecord};
use pulse::audit::extract_subtrees_for_dir;
use pulse::parse::{self, Language};
use std::path::Path;

fn extract_from_source(src: &str) -> Vec<SubtreeRecord> {
    let tree = parse::parse_only(src, Language::Python).unwrap();
    let path = Path::new("test.py");
    extract_subtrees(&tree, src, Language::Python, path, &t().audit)
}

#[test]
fn walker_extracts_zero_subtrees_from_empty_python_file() {
    let records = extract_from_source("");
    assert!(records.is_empty());
}

#[test]
fn walker_extracts_zero_subtrees_from_bare_pass() {
    let records = extract_from_source("pass\n");
    assert!(records.is_empty(), "bare pass below floor; got {records:?}");
}

#[test]
fn walker_emits_records_when_subtree_clears_floor() {
    let src = "def f(x):\n    if x == 1:\n        return x\n    return 0\n";
    let records = extract_from_source(src);
    assert!(!records.is_empty(), "function with branching should clear floor");
}

#[test]
fn walker_records_correct_file_path() {
    let src = "def f(x):\n    if x == 1:\n        return x\n    return 0\n";
    let tree = parse::parse_only(src, Language::Python).unwrap();
    let path = Path::new("/tmp/foo/bar.py");
    let records = extract_subtrees(&tree, src, Language::Python, path, &t().audit);
    assert!(records.iter().all(|r| r.file == path));
}

#[test]
fn walker_records_line_numbers_one_based() {
    let src = "x = 1\nif y == 1:\n    z = 2\n    if w == 3:\n        pass\n";
    let records = extract_from_source(src);
    assert!(records.iter().all(|r| r.line >= 1));
}

#[test]
fn walker_is_deterministic_within_file() {
    let src = "def f(x):\n    if x == 1:\n        return x\n    return 0\n";
    let a = extract_from_source(src);
    let b = extract_from_source(src);
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(b.iter()) {
        assert_eq!(x.fingerprint, y.fingerprint);
        assert_eq!(x.line, y.line);
    }
}

#[test]
fn walker_handles_syntax_error_without_panic() {
    let src = "def f(:\n    x = \nif y\n";
    let _records = extract_from_source(src);
}

#[test]
fn walker_records_named_node_count_above_threshold() {
    let src = "def f(x):\n    if x == 1:\n        return x\n    return 0\n";
    let records = extract_from_source(src);
    let min_nodes = t().audit.pattern_mining.subtree_min_nodes as u32;
    assert!(records.iter().all(|r| r.named_node_count >= min_nodes));
}

#[test]
fn walker_records_depth_above_threshold() {
    let src = "def f(x):\n    if x == 1:\n        return x\n    return 0\n";
    let records = extract_from_source(src);
    let min_depth = t().audit.pattern_mining.subtree_min_depth as u32;
    assert!(records.iter().all(|r| r.depth >= min_depth));
}

#[test]
fn walker_records_snippet_first_line_only() {
    let src = "def f(x):\n    if x == 1:\n        return x\n    return 0\n";
    let records = extract_from_source(src);
    assert!(records.iter().all(|r| !r.snippet.contains('\n')));
}

#[test]
fn walker_records_snippet_truncated_at_eighty_chars() {
    let long_name = "x".repeat(200);
    let src = format!("def f({long_name}):\n    if {long_name} == 1:\n        return 1\n    return 0\n");
    let records = extract_from_source(&src);
    assert!(records.iter().all(|r| r.snippet.chars().count() <= 80));
}

#[test]
fn walker_emits_record_for_each_qualifying_subtree() {
    let src = "def f(x):\n    if x == 1:\n        return x\n    if x == 2:\n        return 2\n    return 0\n";
    let records = extract_from_source(src);
    assert!(records.len() >= 2, "two if-statements expected; got {}", records.len());
}

#[test]
fn walker_floor_filters_smaller_than_min_nodes() {
    let src = "x\n";
    let records = extract_from_source(src);
    assert!(records.is_empty(), "single identifier is below floor");
}

#[test]
fn walker_floor_filters_smaller_than_min_depth() {
    let src = "x = a + b\n";
    let records = extract_from_source(src);
    let min_depth = t().audit.pattern_mining.subtree_min_depth;
    for r in &records {
        assert!(r.depth as usize >= min_depth);
    }
}

#[test]
fn walker_records_pre_order_traversal_order() {
    let src = "def f(x):\n    if x == 1:\n        return x\n    return 0\n";
    let records = extract_from_source(src);
    for w in records.windows(2) {
        let a = &w[0];
        let b = &w[1];
        assert!(a.line <= b.line || (a.line == b.line && a.depth >= b.depth));
    }
}

#[test]
fn walker_handles_unicode_in_source() {
    let src = "def λ(δ):\n    if δ == 1:\n        return δ\n    return 0\n";
    let records = extract_from_source(src);
    assert!(!records.is_empty());
}

#[test]
fn walker_handles_extremely_nested_source() {
    let mut src = String::from("xs = ");
    for _ in 0..30 {
        src.push('[');
    }
    src.push('1');
    for _ in 0..30 {
        src.push(']');
    }
    src.push('\n');
    let _ = extract_from_source(&src);
}

#[test]
fn extract_subtrees_for_dir_aggregates_across_python_files() {
    let dir = scenario_path("shotgun_media_type", "python");
    let records = extract_subtrees_for_dir(&dir, Language::Python, &t().audit);
    assert!(!records.is_empty(), "scenario should produce records");
    let unique_files: std::collections::HashSet<_> = records.iter().map(|r| r.file.clone()).collect();
    assert!(unique_files.len() >= 5, "all 5 fixture files should contribute records");
}

#[test]
fn extract_subtrees_for_dir_skips_non_python_files() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.py"), "def f(x):\n    if x == 1:\n        return 1\n    return 0\n").unwrap();
    std::fs::write(dir.path().join("b.txt"), "not python").unwrap();
    std::fs::write(dir.path().join("c.json"), "{}").unwrap();
    let records = extract_subtrees_for_dir(dir.path(), Language::Python, &t().audit);
    let unique_files: std::collections::HashSet<_> = records.iter().map(|r| r.file.clone()).collect();
    assert_eq!(unique_files.len(), 1, "only .py file should contribute");
}

#[test]
fn extract_subtrees_for_dir_returns_empty_for_nonexistent_dir() {
    let records = extract_subtrees_for_dir(Path::new("/no/such/path/xyz"), Language::Python, &t().audit);
    assert!(records.is_empty());
}

#[test]
fn extract_subtrees_for_dir_returns_empty_for_dir_with_no_python() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), "hello").unwrap();
    let records = extract_subtrees_for_dir(dir.path(), Language::Python, &t().audit);
    assert!(records.is_empty());
}

#[test]
fn extract_subtrees_for_dir_recurses_into_subdirs() {
    let dir = tempfile::tempdir().unwrap();
    let sub = dir.path().join("sub");
    std::fs::create_dir(&sub).unwrap();
    std::fs::write(sub.join("a.py"), "def f(x):\n    if x == 1:\n        return 1\n    return 0\n").unwrap();
    let records = extract_subtrees_for_dir(dir.path(), Language::Python, &t().audit);
    assert!(!records.is_empty(), "subdir contents should be walked");
}

#[test]
fn extract_subtrees_for_dir_skips_hidden_dirs() {
    let dir = tempfile::tempdir().unwrap();
    let hidden = dir.path().join(".hidden");
    std::fs::create_dir(&hidden).unwrap();
    std::fs::write(hidden.join("a.py"), "def f(x):\n    if x == 1:\n        return 1\n    return 0\n").unwrap();
    let records = extract_subtrees_for_dir(dir.path(), Language::Python, &t().audit);
    assert!(records.is_empty(), "hidden dir contents should be skipped");
}

#[test]
fn extract_subtrees_for_dir_skips_node_modules() {
    let dir = tempfile::tempdir().unwrap();
    let nm = dir.path().join("node_modules");
    std::fs::create_dir(&nm).unwrap();
    std::fs::write(nm.join("a.py"), "def f(x):\n    if x == 1:\n        return 1\n    return 0\n").unwrap();
    let records = extract_subtrees_for_dir(dir.path(), Language::Python, &t().audit);
    assert!(records.is_empty());
}

#[test]
fn extract_subtrees_for_dir_skips_target() {
    let dir = tempfile::tempdir().unwrap();
    let tgt = dir.path().join("target");
    std::fs::create_dir(&tgt).unwrap();
    std::fs::write(tgt.join("a.py"), "def f(x):\n    if x == 1:\n        return 1\n    return 0\n").unwrap();
    let records = extract_subtrees_for_dir(dir.path(), Language::Python, &t().audit);
    assert!(records.is_empty());
}

#[test]
fn shotgun_media_type_walker_yields_media_type_subtrees() {
    let dir = scenario_path("shotgun_media_type", "python");
    let records = extract_subtrees_for_dir(&dir, Language::Python, &t().audit);
    let with_media = records.iter().filter(|r| r.snippet.contains("media_type")).count();
    assert!(with_media >= 5, "expected 5+ media_type-bearing subtrees, got {with_media}");
}

#[test]
fn audit_thresholds_default_freqt_min_support_is_five() {
    assert_eq!(t().audit.pattern_mining.freqt_min_support, 5);
}

#[test]
fn audit_thresholds_default_subtree_min_depth_is_three() {
    assert_eq!(t().audit.pattern_mining.subtree_min_depth, 3);
}

#[test]
fn audit_thresholds_default_subtree_min_nodes_is_five() {
    assert_eq!(t().audit.pattern_mining.subtree_min_nodes, 5);
}

#[test]
fn shotgun_cache_pattern_walker_yields_cache_call_subtrees() {
    let dir = scenario_path("shotgun_cache_pattern", "python");
    let records = extract_subtrees_for_dir(&dir, Language::Python, &t().audit);
    let with_cache = records.iter().filter(|r| r.snippet.contains("cache")).count();
    assert!(with_cache >= 5);
}

#[test]
fn walker_subtrees_have_consistent_fingerprints_for_same_shape() {
    let src = "def f(x):\n    if x == 1:\n        return x\n    return 0\n\ndef g(x):\n    if x == 1:\n        return x\n    return 0\n";
    let records = extract_from_source(src);
    let mut counts: std::collections::HashMap<u64, usize> = std::collections::HashMap::new();
    for r in &records {
        *counts.entry(r.fingerprint).or_default() += 1;
    }
    let max_count = counts.values().copied().max().unwrap_or(0);
    assert!(max_count >= 2, "two identical functions should have at least one shared fingerprint; got max {max_count}");
}
