use std::path::PathBuf;

use pulse::audit::{self, walk_typed_source_files_filtered, AuditOpts, IgnoreFilter, PassChoice};
use pulse::config::{AuditSuppression, IgnoreMatcher};

use crate::audit_common::t;

fn write_file(path: &std::path::Path, body: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, body).unwrap();
}

fn build_filter<'a>(
    patterns: &[String],
    matcher_slot: &'a mut Option<IgnoreMatcher>,
    base: &'a std::path::Path,
) -> IgnoreFilter<'a> {
    *matcher_slot = Some(IgnoreMatcher::from_patterns(patterns));
    IgnoreFilter::new(matcher_slot.as_ref().unwrap(), base)
}

fn collect_paths(typed: &[(PathBuf, pulse::parse::Language)]) -> Vec<String> {
    let mut out: Vec<String> = typed
        .iter()
        .map(|(p, _)| p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string())
        .collect();
    out.sort();
    out
}

#[test]
fn ignore_excludes_top_level_glob_match() {
    let dir = tempfile::tempdir().unwrap();
    write_file(&dir.path().join("keep.py"), "x = 1\n");
    write_file(&dir.path().join("target/skip.py"), "y = 2\n");
    let mut slot = None;
    let patterns = vec!["target/**".to_string()];
    let filter = build_filter(&patterns, &mut slot, dir.path());
    let typed = walk_typed_source_files_filtered(dir.path(), true, &filter);
    let names = collect_paths(&typed);
    assert!(names.contains(&"keep.py".to_string()));
    assert!(!names.iter().any(|n| n == "skip.py"));
}

#[test]
fn ignore_excludes_nested_files_under_glob_dir() {
    let dir = tempfile::tempdir().unwrap();
    write_file(&dir.path().join("target/sub/deep.py"), "z = 3\n");
    write_file(&dir.path().join("target/shallow.py"), "z = 4\n");
    write_file(&dir.path().join("keep.py"), "x = 1\n");
    let mut slot = None;
    let patterns = vec!["target/**".to_string()];
    let filter = build_filter(&patterns, &mut slot, dir.path());
    let typed = walk_typed_source_files_filtered(dir.path(), true, &filter);
    let names = collect_paths(&typed);
    assert_eq!(names, vec!["keep.py".to_string()]);
}

#[test]
fn ignore_pattern_with_trailing_slash_is_normalized() {
    let dir = tempfile::tempdir().unwrap();
    write_file(&dir.path().join("build/out.py"), "x = 1\n");
    write_file(&dir.path().join("build/sub/deep.py"), "y = 2\n");
    write_file(&dir.path().join("kept.py"), "z = 3\n");
    let mut slot = None;
    let patterns = vec!["build/".to_string()];
    let filter = build_filter(&patterns, &mut slot, dir.path());
    let typed = walk_typed_source_files_filtered(dir.path(), true, &filter);
    let names = collect_paths(&typed);
    assert_eq!(names, vec!["kept.py".to_string()]);
}

#[test]
fn ignore_or_combines_patterns() {
    let dir = tempfile::tempdir().unwrap();
    write_file(&dir.path().join("target/a.py"), "a = 1\n");
    write_file(&dir.path().join("vendor/b.py"), "b = 2\n");
    write_file(&dir.path().join("kept.py"), "c = 3\n");
    let mut slot = None;
    let patterns = vec!["target/**".to_string(), "vendor/**".to_string()];
    let filter = build_filter(&patterns, &mut slot, dir.path());
    let typed = walk_typed_source_files_filtered(dir.path(), true, &filter);
    let names = collect_paths(&typed);
    assert_eq!(names, vec!["kept.py".to_string()]);
}

#[test]
fn empty_pattern_list_does_not_filter() {
    let dir = tempfile::tempdir().unwrap();
    write_file(&dir.path().join("a.py"), "x = 1\n");
    write_file(&dir.path().join("b.py"), "y = 2\n");
    let mut slot = None;
    let patterns: Vec<String> = vec![];
    let filter = build_filter(&patterns, &mut slot, dir.path());
    let typed = walk_typed_source_files_filtered(dir.path(), true, &filter);
    assert_eq!(typed.len(), 2);
}

#[test]
fn pattern_that_matches_nothing_is_noop() {
    let dir = tempfile::tempdir().unwrap();
    write_file(&dir.path().join("a.py"), "x = 1\n");
    write_file(&dir.path().join("b.py"), "y = 2\n");
    let mut slot = None;
    let patterns = vec!["nonexistent/**".to_string()];
    let filter = build_filter(&patterns, &mut slot, dir.path());
    let typed = walk_typed_source_files_filtered(dir.path(), true, &filter);
    assert_eq!(typed.len(), 2);
}

#[test]
fn unfiltered_walker_keeps_existing_behavior() {
    let dir = tempfile::tempdir().unwrap();
    write_file(&dir.path().join("target/a.py"), "x = 1\n");
    write_file(&dir.path().join("kept.py"), "y = 2\n");
    let typed = pulse::audit::walk_typed_source_files(dir.path(), true);
    let names = collect_paths(&typed);
    assert!(names.contains(&"a.py".to_string()) || names.contains(&"kept.py".to_string()));
    assert!(typed.iter().any(|(p, _)| p.to_string_lossy().contains("kept.py")));
}

#[test]
fn dotfile_dirs_still_skipped_with_no_patterns() {
    let dir = tempfile::tempdir().unwrap();
    write_file(&dir.path().join(".git/x.py"), "secret = 1\n");
    write_file(&dir.path().join("kept.py"), "y = 2\n");
    let mut slot = None;
    let patterns: Vec<String> = vec![];
    let filter = build_filter(&patterns, &mut slot, dir.path());
    let typed = walk_typed_source_files_filtered(dir.path(), true, &filter);
    let names = collect_paths(&typed);
    assert_eq!(names, vec!["kept.py".to_string()]);
}

#[test]
fn ignore_composes_with_include_tests_false() {
    let dir = tempfile::tempdir().unwrap();
    write_file(&dir.path().join("target/test_x.py"), "x = 1\n");
    write_file(&dir.path().join("test_keep.py"), "y = 2\n");
    write_file(&dir.path().join("prod.py"), "z = 3\n");
    let mut slot = None;
    let patterns = vec!["target/**".to_string()];
    let filter = build_filter(&patterns, &mut slot, dir.path());
    let typed = walk_typed_source_files_filtered(dir.path(), false, &filter);
    let names = collect_paths(&typed);
    assert_eq!(names, vec!["prod.py".to_string()]);
}

#[test]
fn ignore_composes_with_include_tests_true() {
    let dir = tempfile::tempdir().unwrap();
    write_file(&dir.path().join("target/test_x.py"), "x = 1\n");
    write_file(&dir.path().join("test_keep.py"), "y = 2\n");
    write_file(&dir.path().join("prod.py"), "z = 3\n");
    let mut slot = None;
    let patterns = vec!["target/**".to_string()];
    let filter = build_filter(&patterns, &mut slot, dir.path());
    let typed = walk_typed_source_files_filtered(dir.path(), true, &filter);
    let names = collect_paths(&typed);
    assert!(names.contains(&"prod.py".to_string()));
    assert!(names.contains(&"test_keep.py".to_string()));
    assert!(!names.iter().any(|n| n == "test_x.py"));
}

#[test]
fn ignore_skips_directory_descent_not_just_post_collection() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..50 {
        write_file(&dir.path().join(format!("vendor/lib_{i}.py")), "x = 1\n");
    }
    write_file(&dir.path().join("kept.py"), "y = 2\n");
    let mut slot = None;
    let patterns = vec!["vendor/**".to_string()];
    let filter = build_filter(&patterns, &mut slot, dir.path());
    let typed = walk_typed_source_files_filtered(dir.path(), true, &filter);
    assert_eq!(typed.len(), 1);
}

fn copy_shotgun_fixture_under(root: &std::path::Path, sub: &str) {
    let src = crate::audit_common::scenario_path("shotgun_media_type", "python");
    let dst = root.join(sub);
    std::fs::create_dir_all(&dst).unwrap();
    for entry in std::fs::read_dir(&src).unwrap().flatten() {
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_file() {
            std::fs::copy(&from, &to).unwrap();
        }
    }
}

#[test]
fn audit_run_with_filter_excludes_findings_from_ignored_paths() {
    let dir = tempfile::tempdir().unwrap();
    copy_shotgun_fixture_under(dir.path(), "ignored");
    let mut slot = None;
    let patterns = vec!["ignored/**".to_string()];
    let filter = build_filter(&patterns, &mut slot, dir.path());
    let opts = AuditOpts {
        root: dir.path().to_path_buf(),
        pass: Some(PassChoice::PatternMining),
        json: false,
        include_tests: true, show_noise: false,
        suppression: AuditSuppression::new(),
    };
    let findings = audit::run_with_filter(&opts, &t().audit, &filter);
    assert!(findings.is_empty(), "expected empty findings, got {} findings", findings.len());
}

#[test]
fn audit_run_without_filter_emits_findings() {
    let dir = tempfile::tempdir().unwrap();
    copy_shotgun_fixture_under(dir.path(), ".");
    let opts = AuditOpts {
        root: dir.path().to_path_buf(),
        pass: Some(PassChoice::PatternMining),
        json: false,
        include_tests: true, show_noise: false,
        suppression: AuditSuppression::new(),
    };
    let findings = audit::run(&opts, &t().audit);
    assert!(!findings.is_empty(), "shotgun fixture should produce findings without filter");
}

#[test]
fn ignore_works_with_pattern_mining_layer() {
    let dir = tempfile::tempdir().unwrap();
    copy_shotgun_fixture_under(dir.path(), "noisy");
    let mut slot = None;
    let patterns = vec!["noisy/**".to_string()];
    let filter = build_filter(&patterns, &mut slot, dir.path());
    let opts = AuditOpts {
        root: dir.path().to_path_buf(),
        pass: Some(PassChoice::PatternMining),
        json: false,
        include_tests: true, show_noise: false,
        suppression: AuditSuppression::new(),
    };
    let findings = audit::run_with_filter(&opts, &t().audit, &filter);
    assert!(findings.is_empty());
}

#[test]
fn ignore_works_with_package_metrics_layer() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..7 {
        write_file(
            &dir.path().join(format!("ignored/m{i}.rs")),
            "pub fn foo() {}\n",
        );
    }
    let mut slot = None;
    let patterns = vec!["ignored/**".to_string()];
    let filter = build_filter(&patterns, &mut slot, dir.path());
    let opts = AuditOpts {
        root: dir.path().to_path_buf(),
        pass: Some(PassChoice::PackageMetrics),
        json: false,
        include_tests: true, show_noise: false,
        suppression: AuditSuppression::new(),
    };
    let findings = audit::run_with_filter(&opts, &t().audit, &filter);
    assert!(findings.is_empty());
}

#[test]
fn glob_pattern_with_double_star_only_matches_under_dir() {
    let dir = tempfile::tempdir().unwrap();
    write_file(&dir.path().join("targetkeep.py"), "x = 1\n");
    write_file(&dir.path().join("target/skip.py"), "y = 2\n");
    let mut slot = None;
    let patterns = vec!["target/**".to_string()];
    let filter = build_filter(&patterns, &mut slot, dir.path());
    let typed = walk_typed_source_files_filtered(dir.path(), true, &filter);
    let names = collect_paths(&typed);
    assert!(names.contains(&"targetkeep.py".to_string()));
    assert!(!names.iter().any(|n| n == "skip.py"));
}

#[test]
fn ignore_filter_handles_root_outside_filesystem_gracefully() {
    let bogus = std::path::Path::new("/nonexistent/path/does/not/exist");
    let mut slot = None;
    let patterns: Vec<String> = vec![];
    let filter = build_filter(&patterns, &mut slot, bogus);
    let typed = walk_typed_source_files_filtered(bogus, true, &filter);
    assert!(typed.is_empty());
}

#[test]
fn ignore_filter_with_specific_filename_pattern() {
    let dir = tempfile::tempdir().unwrap();
    write_file(&dir.path().join("a.py"), "x = 1\n");
    write_file(&dir.path().join("b.py"), "y = 2\n");
    write_file(&dir.path().join("sub/a.py"), "z = 3\n");
    let mut slot = None;
    let patterns = vec!["a.py".to_string()];
    let filter = build_filter(&patterns, &mut slot, dir.path());
    let typed = walk_typed_source_files_filtered(dir.path(), true, &filter);
    let names: Vec<String> = typed
        .iter()
        .map(|(p, _)| p.strip_prefix(dir.path()).unwrap().to_string_lossy().to_string())
        .collect();
    assert!(names.iter().any(|n| n.ends_with("b.py")));
    assert!(names.iter().any(|n| n.contains("sub") && n.ends_with("a.py")));
    assert!(!names.iter().any(|n| n == "a.py"));
}
