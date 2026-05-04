use pulse::audit::discovery::{freqt_mine, RawCluster};
use pulse::audit::scoring::apply_idf;
use pulse::audit::walker::{extract_subtrees, SubtreeRecord};
use pulse::audit::{extract_subtrees_for_dir, walk_typed_source_files};
use pulse::parse::{self, Language};
use pulse::thresholds::Thresholds;
use std::path::{Path, PathBuf};
use std::process::Command;

fn t() -> Thresholds {
    Thresholds::default()
}

fn fab_record(fp: u64, file: &str, line: u32, snippet: &str) -> SubtreeRecord {
    SubtreeRecord {
        fingerprint: fp,
        file: PathBuf::from(file),
        line,
        depth: 5,
        named_node_count: 8,
        snippet: snippet.to_string(),
    }
}

#[test]
fn stress_walker_handles_one_thousand_functions_in_one_file() {
    let mut src = String::new();
    for i in 0..1000 {
        src.push_str(&format!("def f{i}(x):\n    if x == {i}:\n        return x\n    return 0\n\n"));
    }
    let tree = parse::parse_only(&src, Language::Python).unwrap();
    let path = Path::new("big.py");
    let records = extract_subtrees(&tree, &src, Language::Python, path, &t().audit);
    assert!(!records.is_empty());
}

#[test]
fn stress_walker_completes_under_10s_on_one_thousand_functions() {
    let mut src = String::new();
    for i in 0..1000 {
        src.push_str(&format!("def f{i}(x):\n    if x == {i}:\n        return x\n    return 0\n\n"));
    }
    let start = std::time::Instant::now();
    let tree = parse::parse_only(&src, Language::Python).unwrap();
    let path = Path::new("big.py");
    let _ = extract_subtrees(&tree, &src, Language::Python, path, &t().audit);
    assert!(start.elapsed().as_secs() < 10);
}

#[test]
fn stress_freqt_mine_handles_ten_thousand_records() {
    let records: Vec<SubtreeRecord> = (0..10_000)
        .map(|i| fab_record(i % 100, &format!("f{}.py", i % 50), 1, "x"))
        .collect();
    let _ = freqt_mine(&records, &t().audit);
}

#[test]
fn stress_freqt_mine_completes_under_5s_on_ten_thousand_records() {
    let records: Vec<SubtreeRecord> = (0..10_000)
        .map(|i| fab_record(i % 100, &format!("f{}.py", i % 50), 1, "x"))
        .collect();
    let start = std::time::Instant::now();
    let _ = freqt_mine(&records, &t().audit);
    assert!(start.elapsed().as_secs() < 5);
}

#[test]
fn stress_apply_idf_handles_one_thousand_clusters() {
    let clusters: Vec<RawCluster> = (0..1000)
        .map(|i| RawCluster {
            fingerprint: i,
            support: 5,
            file_count: 3,
            representative_snippet: "x".to_string(),
            locations: vec![(PathBuf::from("a.py"), 1)],
        })
        .collect();
    let result = apply_idf(clusters, 100, &t().audit);
    assert_eq!(result.len(), t().audit.max_findings_reported);
}

#[test]
fn stress_audit_directory_with_one_hundred_files() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..100 {
        std::fs::write(
            dir.path().join(format!("f{i}.py")),
            "def f(x):\n    if x == 1:\n        return x\n    return 0\n",
        ).unwrap();
    }
    let typed = walk_typed_source_files(dir.path(), true);
    assert_eq!(typed.len(), 100);
}

#[test]
fn stress_audit_completes_under_30s_on_one_hundred_files() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..100 {
        std::fs::write(
            dir.path().join(format!("f{i}.py")),
            "def f(x):\n    if x == 1:\n        return x\n    return 0\n",
        ).unwrap();
    }
    let start = std::time::Instant::now();
    let _ = extract_subtrees_for_dir(dir.path(), Language::Python, &t().audit);
    assert!(start.elapsed().as_secs() < 30);
}

#[test]
fn stress_walker_handles_deeply_nested_python_50() {
    let mut src = String::from("def f():\n");
    for i in 0..50 {
        src.push_str(&"    ".repeat(i + 1));
        src.push_str("if x == 1:\n");
    }
    src.push_str(&"    ".repeat(51));
    src.push_str("return 1\n");
    let _ = parse::parse_only(&src, Language::Python);
}

#[test]
fn stress_walker_handles_extremely_wide_tuple_one_thousand() {
    let mut src = String::from("xs = (");
    for i in 0..1000 {
        if i > 0 {
            src.push_str(", ");
        }
        src.push_str(&format!("{i}"));
    }
    src.push_str(")\n");
    let tree = parse::parse_only(&src, Language::Python).unwrap();
    let _ = extract_subtrees(&tree, &src, Language::Python, Path::new("t.py"), &t().audit);
}

#[test]
fn stress_walker_handles_long_attribute_chain() {
    let mut src = String::from("x");
    for i in 0..200 {
        src.push_str(&format!(".a{i}"));
    }
    src.push('\n');
    let _ = parse::parse_only(&src, Language::Python);
}

#[test]
fn stress_walker_handles_long_method_chain() {
    let mut src = String::from("x");
    for _ in 0..100 {
        src.push_str(".m()");
    }
    src.push('\n');
    let _ = parse::parse_only(&src, Language::Python);
}

#[test]
fn stress_walker_handles_giant_string_literal() {
    let big: String = "a".repeat(100_000);
    let src = format!("x = \"{big}\"\n");
    let _ = parse::parse_only(&src, Language::Python);
}

#[test]
fn stress_walker_handles_many_keyword_args() {
    let mut src = String::from("f(");
    for i in 0..200 {
        if i > 0 {
            src.push_str(", ");
        }
        src.push_str(&format!("k{i}=1"));
    }
    src.push_str(")\n");
    let _ = parse::parse_only(&src, Language::Python);
}

#[test]
fn stress_walker_handles_many_class_methods() {
    let mut src = String::from("class C:\n");
    for i in 0..500 {
        src.push_str(&format!("    def m{i}(self):\n        return 1\n"));
    }
    let _ = parse::parse_only(&src, Language::Python);
}

#[test]
fn stress_walker_handles_long_dict_literal() {
    let mut src = String::from("xs = {");
    for i in 0..500 {
        if i > 0 {
            src.push_str(", ");
        }
        src.push_str(&format!("{i}: {i}"));
    }
    src.push_str("}\n");
    let _ = parse::parse_only(&src, Language::Python);
}

#[test]
fn stress_walker_handles_chained_comparisons_long() {
    let mut src = String::from("x = a");
    for _ in 0..100 {
        src.push_str(" < b");
    }
    src.push('\n');
    let _ = parse::parse_only(&src, Language::Python);
}

#[test]
fn stress_walker_handles_long_concatenation() {
    let mut src = String::from("x = a");
    for _ in 0..200 {
        src.push_str(" + b");
    }
    src.push('\n');
    let _ = parse::parse_only(&src, Language::Python);
}

#[test]
fn stress_walker_handles_huge_match_statement() {
    let mut src = String::from("match x:\n");
    for i in 0..200 {
        src.push_str(&format!("    case {i}:\n        pass\n"));
    }
    let _ = parse::parse_only(&src, Language::Python);
}

#[test]
fn stress_pulse_audit_runs_on_pulse_self_under_60s() {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src_dir = manifest.join("src");
    let start = std::time::Instant::now();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["audit", "--root", src_dir.to_str().unwrap()])
        .output().unwrap();
    assert!(start.elapsed().as_secs() < 60, "elapsed: {:?}", start.elapsed());
    let _ = out;
}

#[test]
fn stress_audit_handles_directory_with_500_small_files() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..500 {
        std::fs::write(dir.path().join(format!("f{i}.py")), "x = 1\n").unwrap();
    }
    let start = std::time::Instant::now();
    let _ = walk_typed_source_files(dir.path(), true);
    assert!(start.elapsed().as_secs() < 10);
}

#[test]
fn stress_apply_idf_handles_zero_total_files_no_panic() {
    let clusters: Vec<RawCluster> = (0..100).map(|i| RawCluster {
        fingerprint: i,
        support: 5,
        file_count: 3,
        representative_snippet: "x".to_string(),
        locations: vec![(PathBuf::from("a.py"), 1)],
    }).collect();
    let _ = apply_idf(clusters, 0, &t().audit);
}

#[test]
fn stress_freqt_mine_handles_uniform_fingerprints() {
    let records: Vec<SubtreeRecord> = (0..5000)
        .map(|i| fab_record(42, &format!("f{i}.py"), 1, "x"))
        .collect();
    let clusters = freqt_mine(&records, &t().audit);
    assert_eq!(clusters.len(), 1);
    assert_eq!(clusters[0].support, 5000);
}

#[test]
fn stress_freqt_mine_handles_all_unique_fingerprints() {
    let mut th = t().audit;
    th.freqt_min_support = 2;
    let records: Vec<SubtreeRecord> = (0..5000)
        .map(|i| fab_record(i, "a.py", i as u32, "x"))
        .collect();
    let clusters = freqt_mine(&records, &th);
    assert!(clusters.is_empty(), "all unique → no clusters at min_support=2");
}

#[test]
fn stress_walker_with_subtree_min_depth_one() {
    let mut th = t().audit;
    th.subtree_min_depth = 1;
    th.subtree_min_nodes = 1;
    let src = "x = 1\n";
    let tree = parse::parse_only(src, Language::Python).unwrap();
    let records = extract_subtrees(&tree, src, Language::Python, Path::new("t.py"), &th);
    assert!(!records.is_empty(), "min thresholds should accept tiny trees");
}

#[test]
fn stress_walker_with_max_depth_threshold_yields_nothing() {
    let mut th = t().audit;
    th.subtree_min_depth = usize::MAX;
    let src = "def f():\n    return 1\n";
    let tree = parse::parse_only(src, Language::Python).unwrap();
    let records = extract_subtrees(&tree, src, Language::Python, Path::new("t.py"), &th);
    assert!(records.is_empty());
}

#[test]
fn stress_apply_idf_handles_max_findings_one() {
    let mut th = t().audit;
    th.max_findings_reported = 1;
    th.idiom_suppression_threshold = 1.0;
    let clusters: Vec<RawCluster> = (0..100).map(|i| RawCluster {
        fingerprint: i,
        support: 10,
        file_count: 3,
        representative_snippet: "x".to_string(),
        locations: vec![],
    }).collect();
    let result = apply_idf(clusters, 100, &th);
    assert_eq!(result.len(), 1);
}

#[test]
fn stress_apply_idf_handles_max_locations_one() {
    use pulse::audit::finding::AuditFinding;
    use pulse::audit::output::format_findings;
    let mut th = t().audit;
    th.max_locations_per_finding = 1;
    let mut f = AuditFinding {
        kind: pulse::audit::finding::AuditKind::UncategorizedPattern { fingerprint: 7 },
        representative_snippet: "x".to_string(),
        support: 100,
        file_count: 100,
        idf_score: Some(1.0),
        action_label: None,
        locations: (0..100).map(|i| pulse::audit::finding::AuditLocation {
            file: PathBuf::from(format!("f{i}.py")),
            line: 1,
        }).collect(),
    };
    let _ = f.locations.len();
    let s = format_findings(std::slice::from_ref(&f), None, &th);
    assert!(s.contains("(99 more)"));
    f.locations.clear();
}

#[test]
fn stress_walker_handles_empty_function_bodies_repeated() {
    let mut src = String::new();
    for i in 0..500 {
        src.push_str(&format!("def f{i}(): pass\n"));
    }
    let _ = parse::parse_only(&src, Language::Python);
}

#[test]
fn stress_walker_handles_decorators_stacked_high() {
    let mut src = String::new();
    for i in 0..50 {
        src.push_str(&format!("@d{i}\n"));
    }
    src.push_str("def f():\n    pass\n");
    let _ = parse::parse_only(&src, Language::Python);
}

#[test]
fn stress_walker_handles_long_string_concatenation_with_plus() {
    let mut src = String::from("x = \"a\"");
    for _ in 0..200 {
        src.push_str(" + \"b\"");
    }
    src.push('\n');
    let _ = parse::parse_only(&src, Language::Python);
}

#[test]
fn stress_walker_handles_nested_list_comprehensions() {
    let mut src = String::from("xs = ");
    for _ in 0..20 {
        src.push_str("[y for y in ");
    }
    src.push('z');
    for _ in 0..20 {
        src.push(']');
    }
    src.push('\n');
    let _ = parse::parse_only(&src, Language::Python);
}

#[test]
fn stress_audit_e2e_pulse_self_no_panic() {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["audit", "--root", manifest.to_str().unwrap()])
        .output()
        .unwrap();
    let code = out.status.code().unwrap_or(-1);
    assert!(code == 0 || code == 1);
}

#[test]
fn stress_freqt_mine_handles_max_support_threshold() {
    let mut th = t().audit;
    th.freqt_min_support = usize::MAX;
    let records: Vec<SubtreeRecord> = (0..10).map(|_| fab_record(7, "a.py", 1, "x")).collect();
    let clusters = freqt_mine(&records, &th);
    assert!(clusters.is_empty());
}

#[test]
fn stress_apply_idf_reproducible_under_random_input_ordering() {
    let cs1: Vec<RawCluster> = (0..50).map(|i| RawCluster {
        fingerprint: i,
        support: 5,
        file_count: 2,
        representative_snippet: format!("snippet_{i}"),
        locations: vec![],
    }).collect();
    let mut cs2 = cs1.clone();
    cs2.reverse();
    let r1 = apply_idf(cs1, 100, &t().audit);
    let r2 = apply_idf(cs2, 100, &t().audit);
    assert_eq!(r1.len(), r2.len());
    for (a, b) in r1.iter().zip(r2.iter()) {
        assert_eq!(a.support, b.support);
        assert_eq!(a.file_count, b.file_count);
    }
}

#[test]
fn stress_walker_handles_very_long_line() {
    let line = "x = ".to_string() + &"1 + ".repeat(1000) + "1";
    let src = format!("{line}\n");
    let _ = parse::parse_only(&src, Language::Python);
}

#[test]
fn stress_walker_handles_million_character_file() {
    let big = "x = 1\n".repeat(170_000);
    let _ = parse::parse_only(&big, Language::Python);
}

#[test]
fn stress_audit_walks_directory_tree_eight_levels_deep() {
    let dir = tempfile::tempdir().unwrap();
    let mut path = dir.path().to_path_buf();
    for level in 0..8 {
        path = path.join(format!("level{level}"));
        std::fs::create_dir(&path).unwrap();
        std::fs::write(path.join("f.py"), "x = 1\n").unwrap();
    }
    let typed = walk_typed_source_files(dir.path(), true);
    assert_eq!(typed.len(), 8, "should find one file per level");
}

#[test]
fn stress_freqt_mine_thousand_distinct_minor_clusters() {
    let mut th = t().audit;
    th.freqt_min_support = 5;
    let mut records = Vec::new();
    for i in 0..1000 {
        for j in 0..5 {
            records.push(fab_record(i, &format!("f{i}_{j}.py"), j, "x"));
        }
    }
    let clusters = freqt_mine(&records, &th);
    assert_eq!(clusters.len(), 1000);
}

#[test]
fn stress_apply_idf_thousand_clusters_with_truncation() {
    let mut th = t().audit;
    th.max_findings_reported = 50;
    th.idiom_suppression_threshold = 1.0;
    let clusters: Vec<RawCluster> = (0..1000).map(|i| RawCluster {
        fingerprint: i,
        support: 10,
        file_count: 5,
        representative_snippet: "x".to_string(),
        locations: vec![],
    }).collect();
    let result = apply_idf(clusters, 100, &th);
    assert_eq!(result.len(), 50);
}

#[test]
fn stress_walker_subtree_count_matches_input_size() {
    let small = "def f():\n    if x:\n        return 1\n    return 0\n";
    let large = small.repeat(50);
    let tree_s = parse::parse_only(small, Language::Python).unwrap();
    let tree_l = parse::parse_only(&large, Language::Python).unwrap();
    let s = extract_subtrees(&tree_s, small, Language::Python, Path::new("t.py"), &t().audit);
    let l = extract_subtrees(&tree_l, &large, Language::Python, Path::new("t.py"), &t().audit);
    assert!(l.len() >= s.len() * 30, "scaling should be linear-ish: {} vs {}", l.len(), s.len());
}
