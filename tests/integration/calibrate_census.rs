use std::fmt::Write as _;
use std::path::Path;

use pulse::audit::IgnoreFilter;
use pulse::calibrate::{self, Census};
use pulse::config::IgnoreMatcher;

use crate::common::t;

fn write_file(path: &Path, body: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, body).unwrap();
}

fn census(root: &Path) -> Census {
    let empty = IgnoreMatcher::from_patterns(&[]);
    let filter = IgnoreFilter::new(&empty, root);
    calibrate::collect(root, &t(), &filter)
}

fn names(files: &[calibrate::FileCensus]) -> Vec<String> {
    files.iter().map(|f| f.path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string()).collect()
}

const APP_PY: &str = "def alpha(x):\n    if x > 1:\n        return x\n    return 0\n";
const TEST_PY: &str = "def test_alpha():\n    assert True\n";

#[test]
fn census_partitions_main_and_test_files() {
    let dir = tempfile::tempdir().unwrap();
    write_file(&dir.path().join("src/app.py"), APP_PY);
    write_file(&dir.path().join("tests/test_app.py"), TEST_PY);
    let census = census(dir.path());
    assert_eq!(names(&census.main), vec!["app.py"]);
    assert_eq!(names(&census.tests), vec!["test_app.py"]);
}

#[test]
fn fixture_paths_are_excluded_entirely() {
    let dir = tempfile::tempdir().unwrap();
    write_file(&dir.path().join("src/app.py"), APP_PY);
    write_file(&dir.path().join("fixtures/sample.py"), APP_PY);
    let census = census(dir.path());
    assert_eq!(names(&census.main), vec!["app.py"]);
    assert!(census.tests.is_empty());
}

#[test]
fn ignore_globs_are_respected() {
    let dir = tempfile::tempdir().unwrap();
    write_file(&dir.path().join("src/app.py"), APP_PY);
    write_file(&dir.path().join("generated/gen.py"), APP_PY);
    let matcher = IgnoreMatcher::from_patterns(&["generated/**".to_string()]);
    let filter = IgnoreFilter::new(&matcher, dir.path());
    let census = calibrate::collect(dir.path(), &t(), &filter);
    assert_eq!(names(&census.main), vec!["app.py"]);
}

#[test]
fn function_and_module_metrics_flow_through() {
    let dir = tempfile::tempdir().unwrap();
    write_file(&dir.path().join("src/app.py"), APP_PY);
    let census = census(dir.path());
    let file = &census.main[0];
    assert_eq!(file.module.total_functions, 1);
    let alpha = file.functions.iter().find(|f| f.name == "alpha").expect("alpha measured");
    assert_eq!(alpha.cc, 2);
    assert!(alpha.loc >= 4);
}

#[test]
fn cpg_enabled_state_is_recorded() {
    let dir = tempfile::tempdir().unwrap();
    write_file(&dir.path().join("src/app.py"), APP_PY);
    let mut enabled = t();
    enabled.cpg.enabled = true;
    let matcher = IgnoreMatcher::from_patterns(&[]);
    let filter = IgnoreFilter::new(&matcher, dir.path());
    assert!(!census(dir.path()).cpg_enabled);
    assert!(calibrate::collect(dir.path(), &enabled, &filter).cpg_enabled);
}

#[test]
fn vendored_outlier_is_excluded_and_recorded() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..8u32 {
        let mut body = String::new();
        let pad = "x".repeat(6 + i as usize);
        for j in 0..(4 + i) {
            let _ = writeln!(body, "def process_{pad}_{i}_{j}(incoming_payload_value):");
            let _ = writeln!(body, "    computed_result_{pad} = incoming_payload_value + {j} + {i}");
            let _ = writeln!(body, "    label_{pad} = \"{}\"", "m".repeat(30 + (i * 7) as usize));
            let _ = writeln!(body, "    return computed_result_{pad} + len(label_{pad})");
        }
        write_file(&dir.path().join(format!("src/module_{i}.py")), &body);
    }
    let mut minified = String::from("var ");
    for i in 0..4000u32 {
        let _ = write!(minified, "a{i}=f(a{}),", i.saturating_sub(1));
    }
    minified.push_str("z=0;");
    write_file(&dir.path().join("src/bundle.min.js"), &minified);
    let census = census(dir.path());
    let vendored: Vec<String> = census
        .vendored_excluded
        .iter()
        .map(|p| p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string())
        .collect();
    assert_eq!(vendored, vec!["bundle.min.js"], "{vendored:?}");
    assert!(!names(&census.main).contains(&"bundle.min.js".to_string()));
}

#[test]
fn census_ordering_is_deterministic() {
    let dir = tempfile::tempdir().unwrap();
    for name in ["zeta", "alpha", "mid"] {
        write_file(&dir.path().join(format!("src/{name}.py")), APP_PY);
    }
    let first = names(&census(dir.path()).main);
    let second = names(&census(dir.path()).main);
    assert_eq!(first, second);
    assert_eq!(first, vec!["alpha.py", "mid.py", "zeta.py"]);
}
