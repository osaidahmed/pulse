
use crate::common::*;
use std::process::Command;

fn pulse(args: &[&str]) -> (String, String, i32) {
    let output = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(args)
        .output()
        .expect("pulse failed");
    (
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
        output.status.code().unwrap_or(-1),
    )
}

// ===========================================================================
// Debug command
// ===========================================================================

#[test]
fn debug_shows_module_and_function_metrics() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("t.py");
    std::fs::write(&p, "def f(x):\n    if x > 0:\n        return 1\n    return 0\n").unwrap();
    let (_, stderr, _) = pulse(&["debug", p.to_str().unwrap()]);
    assert!(stderr.contains("Module:"), "should show module line, got: {stderr}");
    assert!(stderr.contains("cc="), "should show cc metric");
    assert!(stderr.contains("cogc="), "should show cogc metric");
    assert!(stderr.contains("short_vars="), "should show short_vars metric");
    assert!(stderr.contains("str_match="), "should show str_match metric");
}

#[test]
fn debug_shows_correct_function_name() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("t.py");
    std::fs::write(&p, "def my_function():\n    return 42\n").unwrap();
    let (_, stderr, _) = pulse(&["debug", p.to_str().unwrap()]);
    assert!(stderr.contains("my_function"), "should show function name");
}

// ===========================================================================
// Budget command
// ===========================================================================

#[test]
fn budget_shows_headroom() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("t.py");
    std::fs::write(&p, "def f():\n    return 1\n").unwrap();
    let (_, stderr, _) = pulse(&["budget", p.to_str().unwrap()]);
    assert!(stderr.contains("functions:"), "should show function count");
    assert!(stderr.contains("LOC:"), "should show LOC");
    assert!(stderr.contains("total cc:"), "should show cc");
    assert!(stderr.contains("room:"), "should show remaining room");
}

#[test]
fn budget_new_shows_thresholds() {
    let (_, stderr, _) = pulse(&["budget", "--new"]);
    assert!(stderr.contains("max functions:"), "got: {stderr}");
    assert!(stderr.contains("max LOC:"), "got: {stderr}");
    assert!(stderr.contains("max total cc:"), "got: {stderr}");
    assert!(stderr.contains("per-function:"), "got: {stderr}");
}

#[test]
fn budget_nonexistent_file_shows_error() {
    let (_, stderr, _) = pulse(&["budget", "/no/such/file.rs"]);
    assert!(stderr.contains("unsupported or unreadable"), "got: {stderr}");
}

// ===========================================================================
// Check -a / --all
// ===========================================================================

#[test]
fn check_all_finds_smells_in_directory() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("smelly.py");
    std::fs::write(&p, "def f(a, b, c, d, e, f, g, h):\n    return a\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", "-a"])
        .current_dir(dir.path())
        .output()
        .expect("failed");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Excess Arguments"), "should find smell, got: {stdout}");
}

#[test]
fn check_all_skips_hidden_dirs() {
    let dir = tempfile::tempdir().unwrap();
    let hidden = dir.path().join(".hidden");
    std::fs::create_dir(&hidden).unwrap();
    std::fs::write(hidden.join("smelly.py"), "def f(a, b, c, d, e, f, g, h):\n    return a\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", "-a"])
        .current_dir(dir.path())
        .output()
        .expect("failed");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.is_empty(), "hidden dir should be skipped, got: {stdout}");
}

#[test]
fn check_all_clean_dir_exit_0() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("clean.py"), "def f():\n    return 1\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", "-a"])
        .current_dir(dir.path())
        .output()
        .expect("failed");
    assert!(output.status.success(), "clean dir should exit 0");
}

#[test]
fn check_all_skips_test_files_by_default() {
    let dir = tempfile::tempdir().unwrap();
    let tests_dir = dir.path().join("tests");
    std::fs::create_dir(&tests_dir).unwrap();
    let smelly = "def f(a, b, c, d, e, f, g, h):\n    return a\n";
    std::fs::write(tests_dir.join("test_thing.py"), smelly).unwrap();
    std::fs::write(dir.path().join("test_top.py"), smelly).unwrap();
    std::fs::write(dir.path().join("foo_test.go"), "package x\nfunc F(a, b, c, d, e, f, g, h int) int { return a }\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", "-a"])
        .current_dir(dir.path())
        .output()
        .expect("failed");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.is_empty(), "test files should be skipped, got: {stdout}");
    assert!(output.status.success(), "no findings → exit 0");
}

#[test]
fn check_all_includes_tests_with_flag() {
    let dir = tempfile::tempdir().unwrap();
    let tests_dir = dir.path().join("tests");
    std::fs::create_dir(&tests_dir).unwrap();
    std::fs::write(tests_dir.join("test_thing.py"), "def f(a, b, c, d, e, f, g, h):\n    return a\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", "-a", "--include-tests"])
        .current_dir(dir.path())
        .output()
        .expect("failed");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Excess Arguments"), "test should be analyzed with --include-tests, got: {stdout}");
}

#[test]
fn check_all_skips_test_files_does_not_skip_lookalikes() {
    let dir = tempfile::tempdir().unwrap();
    let smelly = "def f(a, b, c, d, e, f, g, h):\n    return a\n";
    // production files with test-like substrings — must still be analyzed
    std::fs::write(dir.path().join("attestation.py"), smelly).unwrap();
    std::fs::write(dir.path().join("contestant.py"), smelly).unwrap();
    std::fs::write(dir.path().join("manifest.py"), smelly).unwrap();
    std::fs::write(dir.path().join("latest.py"), smelly).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", "-a"])
        .current_dir(dir.path())
        .output()
        .expect("failed");
    let stdout = String::from_utf8(output.stdout).unwrap();
    for name in &["attestation.py", "contestant.py", "manifest.py", "latest.py"] {
        assert!(stdout.contains(name), "production file {name} should be analyzed, got: {stdout}");
    }
}

#[test]
fn check_explicit_test_file_still_analyzes() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("test_foo.py");
    std::fs::write(&p, "def f(a, b, c, d, e, f, g, h):\n    return a\n").unwrap();
    let (stdout, _, _) = pulse(&["check", p.to_str().unwrap()]);
    assert!(stdout.contains("Excess Arguments"), "explicit check on test file should analyze, got: {stdout}");
}

// ===========================================================================
// Usage / invalid args
// ===========================================================================

#[test]
fn no_args_shows_usage() {
    let (_, stderr, code) = pulse(&[]);
    assert_eq!(code, 1);
    assert!(stderr.contains("usage:"), "should show usage, got: {stderr}");
}

#[test]
fn unknown_command_shows_usage() {
    let (_, stderr, code) = pulse(&["notacommand"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("usage:"));
}

// ===========================================================================
// Cleanup
// ===========================================================================

#[test]
fn cleanup_removes_baseline_dir() {
    let bl = tempfile::tempdir().unwrap();
    let bl_path = bl.path().to_path_buf();
    std::fs::create_dir_all(&bl_path).unwrap();
    std::fs::write(bl_path.join("test.txt"), "data").unwrap();
    let _ = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["--cleanup"])
        .env("PULSE_BASELINE_DIR", &bl_path)
        .output()
        .expect("failed");
    assert!(!bl_path.join("test.txt").exists(), "cleanup should remove baseline files");
}

// ===========================================================================
// New smells: Large Struct, Short Variable Names, Stringly-Typed Switch
// ===========================================================================

#[test]
fn large_struct_detected_in_rust() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("t.rs");
    let mut code = String::from("struct Big {\n");
    for i in 0..struct_fields_above() { code.push_str(&format!("    field_{i}: i32,\n")); }
    code.push_str("}\n");
    std::fs::write(&p, &code).unwrap();
    let (stdout, _, _) = pulse(&["check", p.to_str().unwrap()]);
    assert!(stdout.contains("Large Struct"), "should detect large struct, got: {stdout}");
    assert!(stdout.contains("15 fields"), "should show field count");
    assert!(stdout.contains(&format!("threshold: {}", t().module.max_struct_fields)), "should show threshold");
}

#[test]
fn small_struct_not_flagged() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("t.rs");
    std::fs::write(&p, "struct Point {\n    x: f64,\n    y: f64,\n}\n").unwrap();
    let (stdout, _, _) = pulse(&["check", p.to_str().unwrap()]);
    assert!(!stdout.contains("Large Struct"), "small struct should not be flagged, got: {stdout}");
}

#[test]
fn short_vars_detected_in_python() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("t.py");
    let mut code = String::from("def long_func():\n");
    for c in ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'] {
        code.push_str(&format!("    {c} = 1\n"));
    }
    for line in 0..10 { code.push_str(&format!("    x_{line} = {line}\n")); }
    code.push_str("    return 0\n");
    std::fs::write(&p, &code).unwrap();
    let (stdout, _, _) = pulse(&["check", p.to_str().unwrap()]);
    assert!(stdout.contains("Short Variable Names"), "should detect short vars, got: {stdout}");
    assert!(stdout.contains(&format!("threshold: {}", t().analysis.short_var_max_count)), "should show threshold");
}

#[test]
fn short_vars_exempt_loop_counters() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("t.py");
    let mut code = String::from("def func():\n");
    // Only exempt vars: i, j, k, _
    for c in ['i', 'j', 'k'] { code.push_str(&format!("    {c} = 1\n")); }
    for line in 0..t().analysis.short_var_min_fn_loc as usize { code.push_str(&format!("    var_{line} = {line}\n")); }
    code.push_str("    return 0\n");
    std::fs::write(&p, &code).unwrap();
    let (stdout, _, _) = pulse(&["check", p.to_str().unwrap()]);
    assert!(!stdout.contains("Short Variable Names"), "exempt vars should not trigger, got: {stdout}");
}

#[test]
fn short_vars_not_flagged_in_short_function() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("t.py");
    std::fs::write(&p, "def f():\n    a = 1\n    b = 2\n    c = 3\n    d = 4\n    return a\n").unwrap();
    let (stdout, _, _) = pulse(&["check", p.to_str().unwrap()]);
    assert!(!stdout.contains("Short Variable Names"), "short function should not flag, got: {stdout}");
}

#[test]
fn stringly_typed_detected_in_rust() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("t.rs");
    let code = r#"fn dispatch(cmd: &str) -> i32 {
    match cmd {
        "a" => 1, "b" => 2, "c" => 3,
        "d" => 4, "e" => 5, "f" => 6,
        _ => 0,
    }
}"#;
    std::fs::write(&p, code).unwrap();
    let (stdout, _, _) = pulse(&["check", p.to_str().unwrap()]);
    assert!(stdout.contains("Stringly-Typed Switch"), "should detect string match, got: {stdout}");
    assert!(stdout.contains(&format!("threshold: {}", t().analysis.max_string_match_arms)), "should show threshold");
}

#[test]
fn few_string_match_arms_not_flagged() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("t.rs");
    let code = r#"fn small(s: &str) -> i32 {
    match s {
        "a" => 1,
        "b" => 2,
        _ => 0,
    }
}"#;
    std::fs::write(&p, code).unwrap();
    let (stdout, _, _) = pulse(&["check", p.to_str().unwrap()]);
    assert!(!stdout.contains("Stringly-Typed"), "few arms should not flag, got: {stdout}");
}

#[test]
fn threshold_values_shown_in_complex_method() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("t.py");
    let mut code = String::from("def f(x):\n");
    for i in 0..10 { code.push_str(&format!("    if x > {i}:\n        pass\n")); }
    code.push_str("    return x\n");
    std::fs::write(&p, &code).unwrap();
    let (stdout, _, _) = pulse(&["check", p.to_str().unwrap()]);
    assert!(stdout.contains("threshold:"), "complex method should show threshold, got: {stdout}");
}

#[test]
fn threshold_values_shown_in_excess_args() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("t.py");
    std::fs::write(&p, "def f(a, b, c, d, e, f, g, h):\n    return a\n").unwrap();
    let (stdout, _, _) = pulse(&["check", p.to_str().unwrap()]);
    assert!(stdout.contains(&format!("threshold: {}", t().function.arg_max)), "excess args should show threshold, got: {stdout}");
}

#[test]
fn version_flag_prints_version() {
    let expected = format!("pulse {}", env!("CARGO_PKG_VERSION"));
    let (stdout_long, _, code_long) = pulse(&["--version"]);
    assert_eq!(code_long, 0, "--version should exit 0");
    assert_eq!(stdout_long.trim(), expected);

    let (stdout_short, _, code_short) = pulse(&["-V"]);
    assert_eq!(code_short, 0, "-V should exit 0");
    assert_eq!(stdout_short.trim(), expected);
}
