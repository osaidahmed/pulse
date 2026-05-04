use pulse::audit::{walk_typed_source_files, AuditOpts};
use pulse::parse::Language;
use pulse::thresholds::Thresholds;
use std::fs;
use std::path::Path;
use std::process::Command;

fn write(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

fn collect_relative_paths(root: &Path, include_tests: bool) -> Vec<String> {
    let mut out: Vec<String> = walk_typed_source_files(root, include_tests)
        .into_iter()
        .map(|(p, _)| {
            p.strip_prefix(root)
                .unwrap_or(&p)
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    out.sort();
    out
}

#[test]
fn walk_skips_files_in_tests_dir_when_include_tests_false() {
    let dir = tempfile::tempdir().unwrap();
    write(&dir.path().join("src/lib.py"), "def f(): pass\n");
    write(&dir.path().join("tests/test_lib.py"), "def test_f(): pass\n");

    let collected = collect_relative_paths(dir.path(), false);
    assert_eq!(collected, vec!["src/lib.py".to_string()]);
}

#[test]
fn walk_includes_files_in_tests_dir_when_include_tests_true() {
    let dir = tempfile::tempdir().unwrap();
    write(&dir.path().join("src/lib.py"), "def f(): pass\n");
    write(&dir.path().join("tests/test_lib.py"), "def test_f(): pass\n");

    let collected = collect_relative_paths(dir.path(), true);
    assert_eq!(collected, vec!["src/lib.py".to_string(), "tests/test_lib.py".to_string()]);
}

#[test]
fn walk_skips_python_test_prefix_files_outside_tests_dir() {
    let dir = tempfile::tempdir().unwrap();
    write(&dir.path().join("src/test_helpers.py"), "def test_x(): pass\n");
    write(&dir.path().join("src/helpers.py"), "def helper(): pass\n");

    let collected = collect_relative_paths(dir.path(), false);
    assert_eq!(collected, vec!["src/helpers.py".to_string()]);
}

#[test]
fn walk_skips_rust_underscore_test_suffix_files() {
    let dir = tempfile::tempdir().unwrap();
    write(&dir.path().join("src/lib.rs"), "fn main() {}\n");
    write(&dir.path().join("src/lib_test.rs"), "fn test_x() {}\n");

    let collected = collect_relative_paths(dir.path(), false);
    assert_eq!(collected, vec!["src/lib.rs".to_string()]);
}

#[test]
fn walk_skips_typescript_dot_test_files() {
    let dir = tempfile::tempdir().unwrap();
    write(&dir.path().join("src/api.ts"), "export const x = 1;\n");
    write(&dir.path().join("src/api.test.ts"), "test('x', () => {});\n");
    write(&dir.path().join("src/api.spec.ts"), "test('y', () => {});\n");

    let collected = collect_relative_paths(dir.path(), false);
    assert_eq!(collected, vec!["src/api.ts".to_string()]);
}

#[test]
fn walk_skips_specs_dir() {
    let dir = tempfile::tempdir().unwrap();
    write(&dir.path().join("lib/sinatra.rb"), "module Sinatra; end\n");
    write(&dir.path().join("spec/lib_spec.rb"), "describe Sinatra do\nend\n");

    let collected = collect_relative_paths(dir.path(), false);
    assert_eq!(collected, vec!["lib/sinatra.rb".to_string()]);
}

#[test]
fn walk_skips_double_underscore_tests_dir() {
    let dir = tempfile::tempdir().unwrap();
    write(&dir.path().join("src/utils.ts"), "export const f = () => 1;\n");
    write(
        &dir.path().join("src/__tests__/utils.ts"),
        "test('f', () => {});\n",
    );

    let collected = collect_relative_paths(dir.path(), false);
    assert_eq!(collected, vec!["src/utils.ts".to_string()]);
}

#[test]
fn walk_keeps_normal_source_files_unaffected() {
    let dir = tempfile::tempdir().unwrap();
    write(&dir.path().join("src/api.py"), "def api(): pass\n");
    write(&dir.path().join("src/services.py"), "def svc(): pass\n");
    write(&dir.path().join("src/models.py"), "def mdl(): pass\n");

    let with_tests = collect_relative_paths(dir.path(), true);
    let without_tests = collect_relative_paths(dir.path(), false);
    assert_eq!(with_tests, without_tests);
    assert_eq!(with_tests.len(), 3);
}

#[test]
fn audit_run_excludes_tests_by_default() {
    let dir = tempfile::tempdir().unwrap();
    let pattern = "def f(x):\n    if x == 1:\n        return x + 1\n    return x\n";
    for i in 0..5 {
        write(&dir.path().join(format!("src/mod_{i}.py")), pattern);
    }
    for i in 0..7 {
        write(
            &dir.path().join(format!("src/decoy_{i}.py")),
            &format!("def unique_{i}():\n    return {i} * 2\n"),
        );
    }
    write(&dir.path().join("tests/test_mod.py"), pattern);

    let opts = AuditOpts {
        root: dir.path().to_path_buf(),
        layer: None,
        json: false,
        include_tests: false,
    };
    let findings = pulse::audit::run(&opts, &Thresholds::default().audit);

    assert!(!findings.is_empty(), "src duplication should still surface");
    for f in &findings {
        for loc in &f.locations {
            let s = loc.file.to_string_lossy();
            assert!(
                !s.contains("/tests/") && !s.contains("/test/"),
                "test path leaked into findings: {s}"
            );
        }
    }
}

#[test]
fn audit_run_with_include_tests_picks_up_test_files() {
    let dir = tempfile::tempdir().unwrap();
    let pattern = "def g(y):\n    if y == 2:\n        return y + 2\n    return y\n";
    write(&dir.path().join("src/only.py"), pattern);
    for i in 0..5 {
        write(&dir.path().join(format!("tests/test_{i}.py")), pattern);
    }
    for i in 0..7 {
        write(
            &dir.path().join(format!("tests/test_decoy_{i}.py")),
            &format!("def test_unique_{i}():\n    assert {i} * 3 == {}\n", i * 3),
        );
    }

    let opts_default = AuditOpts {
        root: dir.path().to_path_buf(),
        layer: None,
        json: false,
        include_tests: false,
    };
    let findings_default = pulse::audit::run(&opts_default, &Thresholds::default().audit);
    assert!(
        findings_default.is_empty(),
        "single src file shouldn't cluster — tests should be excluded"
    );

    let opts_with = AuditOpts {
        root: dir.path().to_path_buf(),
        layer: None,
        json: false,
        include_tests: true,
    };
    let findings_with = pulse::audit::run(&opts_with, &Thresholds::default().audit);
    assert!(
        !findings_with.is_empty(),
        "with --include-tests, the matching test files should produce a cluster"
    );
}

#[test]
fn audit_extract_subtrees_for_dir_includes_tests() {
    let dir = tempfile::tempdir().unwrap();
    let pattern = "def h(z):\n    if z == 3:\n        return z + 3\n    return z\n";
    write(&dir.path().join("src/a.py"), pattern);
    write(&dir.path().join("tests/test_a.py"), pattern);

    let recs = pulse::audit::extract_subtrees_for_dir(
        dir.path(),
        Language::Python,
        &Thresholds::default().audit,
    );
    let has_test = recs
        .iter()
        .any(|r| r.file.to_string_lossy().contains("/tests/"));
    assert!(
        has_test,
        "extract_subtrees_for_dir is the public API used by callers that already have their own filtering — should NOT exclude tests internally"
    );
}

#[test]
fn cli_audit_default_excludes_tests_dir_in_walk() {
    let dir = tempfile::tempdir().unwrap();
    let pattern = "def f(x):\n    if x == 1:\n        return x + 1\n    return x\n";
    for i in 0..6 {
        write(&dir.path().join(format!("src/mod_{i}.py")), pattern);
    }
    write(&dir.path().join("tests/test_dup.py"), pattern);

    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .arg("audit")
        .arg("--root")
        .arg(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap_or_default();
    assert!(
        !stdout.contains("/tests/") && !stdout.contains("test_dup.py"),
        "default invocation must not surface tests/test_dup.py in output: {stdout}"
    );
}

fn build_dir_for_cli_flag_test(dir: &Path) {
    let pattern = "def g(y):\n    if y == 2:\n        return y + 2\n    return y\n";
    write(&dir.join("src/only.py"), pattern);
    for i in 0..5 {
        write(&dir.join(format!("tests/test_{i}.py")), pattern);
    }
    for i in 0..7 {
        write(
            &dir.join(format!("tests/test_decoy_{i}.py")),
            &format!("def test_unique_{i}():\n    assert {i} * 3 == {}\n", i * 3),
        );
    }
}

#[test]
fn cli_audit_include_tests_flag_short_form_overrides_default() {
    let dir = tempfile::tempdir().unwrap();
    build_dir_for_cli_flag_test(dir.path());

    let default_out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .arg("audit")
        .arg("--root")
        .arg(dir.path())
        .output()
        .unwrap();
    assert_eq!(
        default_out.status.code().unwrap_or(-1),
        0,
        "without -t, single src file produces no findings"
    );

    let with_out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .arg("-t")
        .arg("audit")
        .arg("--root")
        .arg(dir.path())
        .output()
        .unwrap();
    assert_eq!(
        with_out.status.code().unwrap_or(-1),
        1,
        "with -t, the matching test files cluster and trigger exit 1"
    );
}

#[test]
fn cli_audit_include_tests_flag_long_form_works() {
    let dir = tempfile::tempdir().unwrap();
    build_dir_for_cli_flag_test(dir.path());

    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .arg("--include-tests")
        .arg("audit")
        .arg("--root")
        .arg(dir.path())
        .output()
        .unwrap();
    assert_eq!(out.status.code().unwrap_or(-1), 1);
}
