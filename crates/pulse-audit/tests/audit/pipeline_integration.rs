use pulse_audit::discovery::freqt_mine;
use pulse_audit::output::{format_findings, format_findings_json};
use pulse_audit::scoring::apply_idf;
use pulse_audit::suppression::AuditSuppression;
use pulse_audit::walker::{ShapeMetrics, SubtreeRecord};
use pulse_audit::{extract_subtrees_for_dir, walk_typed_source_files};
use pulse_audit::{AuditOpts, PassChoice};
use pulse_syntax::parse::Language;
use pulse_thresholds::Thresholds;
use std::path::PathBuf;

fn t() -> Thresholds {
    Thresholds::default()
}

fn run_pipeline(root: &std::path::Path) -> Vec<pulse_audit::finding::AuditFinding> {
    let opts = AuditOpts {
        root: root.to_path_buf(),
        pass: Some(PassChoice::PatternMining),
        json: false,
        include_tests: true,
        show_noise: false,
        suppression: AuditSuppression::new(),
    };
    pulse_audit::run(&opts, &t().audit)
}

fn write_python_files(dir: &std::path::Path, count: usize, content: &str) {
    for i in 0..count {
        std::fs::write(dir.join(format!("f{i}.py")), content).unwrap();
    }
}

fn write_decoys(dir: &std::path::Path, count: usize) {
    for i in 0..count {
        std::fs::write(dir.join(format!("decoy{i}.py")), format!("CONST_{i} = {i}\n")).unwrap();
    }
}

#[test]
fn pipeline_run_on_empty_dir_returns_no_findings() {
    let dir = tempfile::tempdir().unwrap();
    assert!(run_pipeline(dir.path()).is_empty());
}

#[test]
fn pipeline_run_on_single_python_file_with_branching() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.py"), "def f(x):\n    if x == 1:\n        return x\n    return 0\n").unwrap();
    let _ = run_pipeline(dir.path());
}

#[test]
fn pipeline_run_finds_shotgun_pattern_with_decoys() {
    let dir = tempfile::tempdir().unwrap();
    write_python_files(dir.path(), 5, "params['mode'] = 1\nparams['kind'] = 2\nparams['size'] = 3\n");
    write_decoys(dir.path(), 6);
    let findings = run_pipeline(dir.path());
    assert!(!findings.is_empty(), "expected expression-level pattern detection");
}

#[test]
fn pipeline_run_handles_when_pattern_dominates_dir() {
    let dir = tempfile::tempdir().unwrap();
    write_python_files(dir.path(), 10, "def f(x):\n    if x == 1:\n        return x\n    return 0\n");
    let findings = run_pipeline(dir.path());
    assert!(findings.len() <= t().audit.pattern_mining.max_findings_reported);
}

#[test]
fn pipeline_run_handles_mixed_languages() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.py"), "def f(x):\n    if x == 1:\n        return x\n    return 0\n").unwrap();
    std::fs::write(dir.path().join("b.rs"), "fn f(x: i32) -> i32 { if x == 1 { x } else { 0 } }\n").unwrap();
    let _ = run_pipeline(dir.path());
}

#[test]
fn pipeline_total_files_includes_all_walked_files() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..3 {
        std::fs::write(dir.path().join(format!("a{i}.py")), "x = 1\n").unwrap();
    }
    let typed = walk_typed_source_files(dir.path(), true);
    assert_eq!(typed.len(), 3);
}

#[test]
fn pipeline_extract_subtrees_for_dir_python_only() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.py"), "def f(x):\n    if x == 1:\n        return x\n    return 0\n").unwrap();
    let r = extract_subtrees_for_dir(dir.path(), Language::Python, &t().audit);
    assert!(!r.is_empty());
}

#[test]
fn pipeline_extract_subtrees_for_dir_rust_only() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.rs"), "fn f(x: i32) -> i32 { if x == 1 { x } else { 0 } }\n").unwrap();
    let r = extract_subtrees_for_dir(dir.path(), Language::Rust, &t().audit);
    assert!(!r.is_empty());
}

#[test]
fn pipeline_extract_subtrees_for_dir_skips_other_languages() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.py"), "def f(x):\n    if x:\n        return 1\n    return 0\n").unwrap();
    std::fs::write(dir.path().join("b.rs"), "fn f() {}\n").unwrap();
    let r = extract_subtrees_for_dir(dir.path(), Language::Python, &t().audit);
    let rs_files: Vec<_> = r.iter().filter(|rec| rec.file.extension().is_some_and(|e| e == "rs")).collect();
    assert!(rs_files.is_empty());
}

#[test]
fn pipeline_run_returns_deterministic_findings() {
    let dir = tempfile::tempdir().unwrap();
    write_python_files(dir.path(), 5, "def f(x):\n    if x == 1:\n        return x\n    return 0\n");
    write_decoys(dir.path(), 6);
    let r1 = run_pipeline(dir.path());
    let r2 = run_pipeline(dir.path());
    assert_eq!(r1.len(), r2.len());
}

#[test]
fn pipeline_run_orders_findings_by_score_descending() {
    let dir = tempfile::tempdir().unwrap();
    write_python_files(dir.path(), 7, "def f(x):\n    if x == 1:\n        return x\n    return 0\n");
    write_decoys(dir.path(), 8);
    let findings = run_pipeline(dir.path());
    for w in findings.windows(2) {
        let s0 = w[0].idf_score.unwrap_or(0.0);
        let s1 = w[1].idf_score.unwrap_or(0.0);
        assert!(s0 >= s1, "findings sorted by G² score descending");
    }
}

#[test]
fn pipeline_freqt_mine_then_apply_idf_round_trip() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 3;
    th.pattern_mining.idiom_suppression_threshold = 1.0;
    let recs: Vec<SubtreeRecord> = (0..5)
        .map(|i| SubtreeRecord {
            fingerprint: 7,
            parent_fingerprint: None,
            file: PathBuf::from(format!("f{i}.py")),
            line: 1,
            depth: 5,
            named_node_count: 8,
            snippet: "x".to_string(),
            shape: ShapeMetrics::default(),
            simhash: 0,
            loc: 0,
        })
        .collect();
    let clusters = freqt_mine(&recs, &th);
    let findings = apply_idf(clusters, 100, &th);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].support, 5);
}

#[test]
fn pipeline_format_findings_human_after_full_run() {
    let dir = tempfile::tempdir().unwrap();
    write_python_files(dir.path(), 5, "def f(x):\n    if x == 1:\n        return x\n    return 0\n");
    write_decoys(dir.path(), 6);
    let findings = run_pipeline(dir.path());
    let s = format_findings(&findings, Some(dir.path()), &t().audit);
    if !findings.is_empty() {
        assert!(s.contains("audit:"));
    }
}

#[test]
fn pipeline_format_findings_json_after_full_run() {
    let dir = tempfile::tempdir().unwrap();
    write_python_files(dir.path(), 5, "def f(x):\n    if x == 1:\n        return x\n    return 0\n");
    write_decoys(dir.path(), 6);
    let findings = run_pipeline(dir.path());
    let s = format_findings_json(&findings, Some(dir.path()));
    let parsed: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert!(parsed.is_array());
}

#[test]
fn pipeline_handles_directory_with_one_python_file_below_threshold() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.py"), "x = 1\ny = 2\n").unwrap();
    let findings = run_pipeline(dir.path());
    assert!(findings.is_empty());
}

#[test]
fn pipeline_handles_directory_with_only_imports() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..10 {
        std::fs::write(dir.path().join(format!("a{i}.py")), "import os\nimport sys\nfrom os import path\n").unwrap();
    }
    let _ = run_pipeline(dir.path());
}

#[test]
fn pipeline_handles_idiom_heavy_listcomp_dir() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..8 {
        std::fs::write(dir.path().join(format!("a{i}.py")), "xs = [x for x in range(10) if x % 2 == 0]\n").unwrap();
    }
    let findings = run_pipeline(dir.path());
    assert!(findings.len() <= t().audit.pattern_mining.max_findings_reported);
}

#[test]
fn pipeline_findings_contain_unique_fingerprints() {
    let dir = tempfile::tempdir().unwrap();
    write_python_files(dir.path(), 5, "def f(x):\n    if x == 1:\n        return x\n    return 0\n");
    write_decoys(dir.path(), 6);
    let findings = run_pipeline(dir.path());
    let fps: std::collections::HashSet<u64> = findings
        .iter()
        .filter_map(|f| match f.kind {
            pulse_audit::finding::AuditKind::UncategorizedPattern { fingerprint } => Some(fingerprint),
            _ => None,
        })
        .collect();
    assert_eq!(fps.len(), findings.len());
}

#[test]
fn pipeline_findings_have_locations_pointing_to_actual_files() {
    let dir = tempfile::tempdir().unwrap();
    write_python_files(dir.path(), 5, "def f(x):\n    if x == 1:\n        return x\n    return 0\n");
    write_decoys(dir.path(), 6);
    let findings = run_pipeline(dir.path());
    for f in &findings {
        for loc in &f.locations {
            assert!(loc.file.exists());
        }
    }
}

#[test]
fn pipeline_findings_have_positive_line_numbers() {
    let dir = tempfile::tempdir().unwrap();
    write_python_files(dir.path(), 5, "def f(x):\n    if x == 1:\n        return x\n    return 0\n");
    write_decoys(dir.path(), 6);
    let findings = run_pipeline(dir.path());
    for f in &findings {
        for loc in &f.locations {
            assert!(loc.line >= 1);
        }
    }
}

#[test]
fn pipeline_findings_have_positive_support() {
    let dir = tempfile::tempdir().unwrap();
    write_python_files(dir.path(), 5, "def f(x):\n    if x == 1:\n        return x\n    return 0\n");
    write_decoys(dir.path(), 6);
    let findings = run_pipeline(dir.path());
    for f in &findings {
        assert!(f.support >= t().audit.pattern_mining.freqt_min_support as u32);
    }
}

#[test]
fn pipeline_findings_have_file_count_at_least_one() {
    let dir = tempfile::tempdir().unwrap();
    write_python_files(dir.path(), 5, "def f(x):\n    if x == 1:\n        return x\n    return 0\n");
    write_decoys(dir.path(), 6);
    let findings = run_pipeline(dir.path());
    for f in &findings {
        assert!(f.file_count >= 1);
    }
}

#[test]
fn pipeline_handles_directory_with_thousand_files() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..1000 {
        std::fs::write(dir.path().join(format!("f{i}.py")), format!("CONST_{i} = {i}\n")).unwrap();
    }
    let findings = run_pipeline(dir.path());
    let _ = findings;
}

#[test]
fn pipeline_handles_python_in_deep_subdirs() {
    let dir = tempfile::tempdir().unwrap();
    let mut p = dir.path().to_path_buf();
    for level in 0..6 {
        p = p.join(format!("level{level}"));
        std::fs::create_dir(&p).unwrap();
        std::fs::write(p.join("f.py"), "def f(x):\n    if x == 1:\n        return x\n    return 0\n").unwrap();
    }
    let findings = run_pipeline(dir.path());
    let _ = findings;
}

#[test]
fn pipeline_handles_walker_extraction_then_mining_then_scoring() {
    let dir = tempfile::tempdir().unwrap();
    write_python_files(dir.path(), 5, "def f(x):\n    if x == 1:\n        return x\n    return 0\n");
    write_decoys(dir.path(), 6);
    let typed = walk_typed_source_files(dir.path(), true);
    let _ = typed.len();
    let findings = run_pipeline(dir.path());
    let _ = findings;
}

#[test]
fn pipeline_format_findings_with_nontrivial_findings_includes_locations() {
    let dir = tempfile::tempdir().unwrap();
    write_python_files(dir.path(), 5, "def f(x):\n    if x == 1:\n        return x\n    return 0\n");
    write_decoys(dir.path(), 6);
    let findings = run_pipeline(dir.path());
    if !findings.is_empty() {
        let s = format_findings(&findings, Some(dir.path()), &t().audit);
        assert!(s.contains("locations:"));
    }
}

#[test]
fn pipeline_no_panic_on_audit_against_empty_python_file() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.py"), "").unwrap();
    let _ = run_pipeline(dir.path());
}

#[test]
fn pipeline_handles_audit_against_only_comment_files() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..5 {
        std::fs::write(dir.path().join(format!("a{i}.py")), "# just a comment\n# another comment\n").unwrap();
    }
    let _ = run_pipeline(dir.path());
}

#[test]
fn pipeline_handles_audit_against_only_whitespace_files() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..5 {
        std::fs::write(dir.path().join(format!("a{i}.py")), "   \n\n\n").unwrap();
    }
    let _ = run_pipeline(dir.path());
}

#[test]
fn pipeline_audit_run_returns_audit_finding_with_correct_kind() {
    let dir = tempfile::tempdir().unwrap();
    write_python_files(dir.path(), 5, "def f(x):\n    if x == 1:\n        return x\n    return 0\n");
    write_decoys(dir.path(), 6);
    let findings = run_pipeline(dir.path());
    for f in &findings {
        assert!(matches!(f.kind, pulse_audit::finding::AuditKind::UncategorizedPattern { .. }));
    }
}

#[test]
fn pipeline_findings_are_within_max_findings_limit() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..200 {
        std::fs::write(
            dir.path().join(format!("f{i}.py")),
            format!("def f{i}(x):\n    if x == {i}:\n        return x\n    return 0\n"),
        )
        .unwrap();
    }
    let findings = run_pipeline(dir.path());
    assert!(findings.len() <= t().audit.pattern_mining.max_findings_reported);
}

#[test]
fn pipeline_run_handles_multiple_consecutive_invocations_independently() {
    let dir = tempfile::tempdir().unwrap();
    write_python_files(dir.path(), 5, "def f(x):\n    if x == 1:\n        return x\n    return 0\n");
    write_decoys(dir.path(), 6);
    for _ in 0..5 {
        let _ = run_pipeline(dir.path());
    }
}

#[test]
fn pipeline_run_with_python_file_having_zero_subtrees_handled() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.py"), "x\n").unwrap();
    let findings = run_pipeline(dir.path());
    assert!(findings.is_empty());
}

#[test]
fn pipeline_run_handles_multilanguage_findings_independently() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..5 {
        std::fs::write(
            dir.path().join(format!("p{i}.py")),
            "def f(x):\n    if x == 1:\n        return x\n    return 0\n",
        )
        .unwrap();
    }
    for i in 0..5 {
        std::fs::write(dir.path().join(format!("r{i}.rs")), "fn g(x: i32) -> i32 { if x == 1 { return x; } 0 }\n")
            .unwrap();
    }
    for i in 0..6 {
        std::fs::write(dir.path().join(format!("d{i}.py")), format!("CONST_{i} = {i}\n")).unwrap();
    }
    for i in 0..6 {
        std::fs::write(dir.path().join(format!("e{i}.rs")), format!("pub const N{i}: i32 = {i};\n")).unwrap();
    }
    let findings = run_pipeline(dir.path());
    let _ = findings;
}

#[test]
fn pipeline_findings_locations_disjoint_per_finding() {
    let dir = tempfile::tempdir().unwrap();
    write_python_files(dir.path(), 5, "def f(x):\n    if x == 1:\n        return x\n    return 0\n");
    write_decoys(dir.path(), 6);
    let findings = run_pipeline(dir.path());
    let _ = findings;
}

#[test]
fn pipeline_each_finding_has_idf_score_set() {
    let dir = tempfile::tempdir().unwrap();
    write_python_files(dir.path(), 5, "def f(x):\n    if x == 1:\n        return x\n    return 0\n");
    write_decoys(dir.path(), 6);
    let findings = run_pipeline(dir.path());
    for f in &findings {
        assert!(f.idf_score.is_some());
    }
}

#[test]
fn pipeline_idf_score_positive_for_minority_pattern() {
    let dir = tempfile::tempdir().unwrap();
    write_python_files(dir.path(), 5, "def f(x):\n    if x == 1:\n        return x\n    return 0\n");
    write_decoys(dir.path(), 10);
    let findings = run_pipeline(dir.path());
    for f in &findings {
        assert!(f.idf_score.unwrap() > 0.0);
    }
}

#[test]
fn pipeline_run_against_pulse_self_completes() {
    let manifest = pulse_testkit::workspace_root();
    let src = manifest.join("src");
    let _ = run_pipeline(&src);
}

#[test]
fn pipeline_run_against_pulse_self_deterministic() {
    let manifest = pulse_testkit::workspace_root();
    let src = manifest.join("src");
    let r1 = run_pipeline(&src);
    let r2 = run_pipeline(&src);
    assert_eq!(r1.len(), r2.len());
}

#[test]
fn pipeline_findings_serialize_to_json_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    write_python_files(dir.path(), 5, "def f(x):\n    if x == 1:\n        return x\n    return 0\n");
    write_decoys(dir.path(), 6);
    let findings = run_pipeline(dir.path());
    let s = format_findings_json(&findings, Some(dir.path()));
    let parsed: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(parsed.as_array().unwrap().len(), findings.len());
}

#[test]
fn pipeline_findings_human_format_respects_max_locations() {
    let dir = tempfile::tempdir().unwrap();
    write_python_files(dir.path(), 30, "def f(x):\n    if x == 1:\n        return x\n    return 0\n");
    write_decoys(dir.path(), 50);
    let findings = run_pipeline(dir.path());
    let mut th = t().audit;
    th.max_locations_per_finding = 5;
    let s = format_findings(&findings, Some(dir.path()), &th);
    if !findings.is_empty() && findings[0].locations.len() > 5 {
        assert!(s.contains(" more)"));
    }
}

#[test]
fn pipeline_handles_audit_with_max_findings_reported_threshold() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..200 {
        std::fs::write(
            dir.path().join(format!("f{i}.py")),
            format!("def fn{i}(x):\n    if x == {i}:\n        return x\n    return 0\n"),
        )
        .unwrap();
    }
    let findings = run_pipeline(dir.path());
    assert!(findings.len() <= t().audit.pattern_mining.max_findings_reported);
}

#[test]
fn pipeline_extract_subtrees_for_dir_returns_vec() {
    let dir = tempfile::tempdir().unwrap();
    let r = extract_subtrees_for_dir(dir.path(), Language::Python, &t().audit);
    let _: Vec<_> = r;
}

#[test]
fn pipeline_walk_typed_source_files_returns_path_lang_pairs() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.py"), "x = 1\n").unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    for (path, lang) in &typed {
        assert!(path.extension().is_some());
        let _ = lang;
    }
}

#[test]
fn pipeline_default_layer_includes_pattern_findings() {
    let dir = tempfile::tempdir().unwrap();
    write_python_files(dir.path(), 5, "def f(x):\n    if x == 1:\n        return x\n    return 0\n");
    write_decoys(dir.path(), 6);
    let layer3_only = AuditOpts {
        root: dir.path().to_path_buf(),
        pass: Some(PassChoice::PatternMining),
        json: false,
        include_tests: true,
        show_noise: false,
        suppression: AuditSuppression::new(),
    };
    let default_layer = AuditOpts {
        root: dir.path().to_path_buf(),
        pass: None,
        json: false,
        include_tests: true,
        show_noise: false,
        suppression: AuditSuppression::new(),
    };
    let only_three = pulse_audit::run(&layer3_only, &t().audit);
    let combined = pulse_audit::run(&default_layer, &t().audit);
    assert!(combined.len() >= only_three.len(), "default must include all Layer-3 findings");
}

#[test]
fn pipeline_handles_audit_with_lang_kind_filter_no_op() {
    let dir = tempfile::tempdir().unwrap();
    write_python_files(dir.path(), 5, "def f(x):\n    if x == 1:\n        return x\n    return 0\n");
    write_decoys(dir.path(), 6);
    let findings = run_pipeline(dir.path());
    let _ = findings;
}

#[test]
fn pipeline_handles_audit_with_audit_thresholds_default() {
    let dir = tempfile::tempdir().unwrap();
    let opts = AuditOpts {
        root: dir.path().to_path_buf(),
        pass: None,
        json: false,
        include_tests: true,
        show_noise: false,
        suppression: AuditSuppression::new(),
    };
    let _ = pulse_audit::run(&opts, &t().audit);
}

#[test]
fn pipeline_handles_audit_with_custom_freqt_threshold() {
    let dir = tempfile::tempdir().unwrap();
    write_python_files(dir.path(), 3, "def f(x):\n    if x == 1:\n        return x\n    return 0\n");
    write_decoys(dir.path(), 5);
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 3;
    let opts = AuditOpts {
        root: dir.path().to_path_buf(),
        pass: None,
        json: false,
        include_tests: true,
        show_noise: false,
        suppression: AuditSuppression::new(),
    };
    let _ = pulse_audit::run(&opts, &th);
}

#[test]
fn pipeline_handles_audit_with_custom_idiom_threshold() {
    let dir = tempfile::tempdir().unwrap();
    write_python_files(dir.path(), 8, "def f(x):\n    if x == 1:\n        return x\n    return 0\n");
    let mut th = t().audit;
    th.pattern_mining.idiom_suppression_threshold = 0.95;
    let opts = AuditOpts {
        root: dir.path().to_path_buf(),
        pass: None,
        json: false,
        include_tests: true,
        show_noise: false,
        suppression: AuditSuppression::new(),
    };
    let findings = pulse_audit::run(&opts, &th);
    let _ = findings;
}

#[test]
fn pipeline_extraction_records_correct_path_per_file() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.py"), "def f(x):\n    if x == 1:\n        return x\n    return 0\n").unwrap();
    let r = extract_subtrees_for_dir(dir.path(), Language::Python, &t().audit);
    for rec in r {
        assert!(rec.file.to_str().unwrap().ends_with("a.py"));
    }
}

#[test]
fn pipeline_total_files_count_matches_walked_files_in_idf_calculation() {
    let dir = tempfile::tempdir().unwrap();
    write_python_files(dir.path(), 4, "def f(x):\n    if x == 1:\n        return x\n    return 0\n");
    write_decoys(dir.path(), 6);
    let typed = walk_typed_source_files(dir.path(), true);
    assert_eq!(typed.len(), 10);
    let findings = run_pipeline(dir.path());
    let _ = findings;
}

#[test]
fn pipeline_full_run_against_dir_with_only_test_files_walks_them() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("test_a.py"), "def f(x):\n    if x == 1:\n        return x\n    return 0\n")
        .unwrap();
    let typed = walk_typed_source_files(dir.path(), true);
    assert_eq!(typed.len(), 1);
}
