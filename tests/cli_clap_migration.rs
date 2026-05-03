use std::process::Command;

fn pulse(args: &[&str]) -> (String, String, i32) {
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(args)
        .output()
        .expect("pulse failed to spawn");
    (
        String::from_utf8(out.stdout).unwrap_or_default(),
        String::from_utf8(out.stderr).unwrap_or_default(),
        out.status.code().unwrap_or(-1),
    )
}

fn pulse_in_dir(args: &[&str], cwd: &std::path::Path) -> (String, String, i32) {
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("pulse failed to spawn");
    (
        String::from_utf8(out.stdout).unwrap_or_default(),
        String::from_utf8(out.stderr).unwrap_or_default(),
        out.status.code().unwrap_or(-1),
    )
}

#[test]
fn version_long_flag_exits_zero_with_correct_version() {
    let (stdout, _stderr, code) = pulse(&["--version"]);
    assert_eq!(code, 0);
    assert!(
        stdout.contains(env!("CARGO_PKG_VERSION")),
        "stdout should contain version, got: {stdout}",
    );
    assert!(stdout.starts_with("pulse "), "got: {stdout}");
}

#[test]
fn version_short_flag_exits_zero_with_correct_version() {
    let (stdout, _stderr, code) = pulse(&["-V"]);
    assert_eq!(code, 0);
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn help_long_flag_exits_zero() {
    let (stdout, _stderr, code) = pulse(&["--help"]);
    assert_eq!(code, 0, "got code {code}");
    let combined = stdout.to_lowercase();
    assert!(combined.contains("usage") || combined.contains("commands"),
        "help should describe usage, got: {stdout}");
}

#[test]
fn help_short_flag_exits_zero() {
    let (stdout, _stderr, code) = pulse(&["-h"]);
    assert_eq!(code, 0);
    assert!(!stdout.is_empty());
}

#[test]
fn audit_subcommand_help_exits_zero() {
    let (stdout, _stderr, code) = pulse(&["audit", "--help"]);
    assert_eq!(code, 0);
    let lower = stdout.to_lowercase();
    assert!(lower.contains("audit") || lower.contains("usage"),
        "audit help should mention audit/usage, got: {stdout}");
}

#[test]
fn check_subcommand_help_exits_zero() {
    let (_stdout, _stderr, code) = pulse(&["check", "--help"]);
    assert_eq!(code, 0);
}

#[test]
fn budget_subcommand_help_exits_zero() {
    let (_stdout, _stderr, code) = pulse(&["budget", "--help"]);
    assert_eq!(code, 0);
}

#[test]
fn debug_subcommand_help_exits_zero() {
    let (_stdout, _stderr, code) = pulse(&["debug", "--help"]);
    assert_eq!(code, 0);
}

#[test]
fn audit_subcommand_exits_with_not_implemented_marker() {
    let (_stdout, stderr, code) = pulse(&["audit"]);
    assert_eq!(code, 2, "stub must exit with code 2, got {code}");
    assert!(
        stderr.contains("not yet implemented") || stderr.contains("not implemented"),
        "stub stderr should declare it is not implemented, got: {stderr}",
    );
}

#[test]
fn audit_subcommand_with_json_flag_still_stubs() {
    let (_stdout, stderr, code) = pulse(&["audit", "--json"]);
    assert_eq!(code, 2);
    assert!(stderr.to_lowercase().contains("not"), "got: {stderr}");
}

#[test]
fn audit_subcommand_with_layer_flag_still_stubs() {
    let (_stdout, _stderr, code) = pulse(&["audit", "--layer", "3"]);
    assert_eq!(code, 2);
}

#[test]
fn audit_subcommand_with_root_flag_still_stubs() {
    let dir = tempfile::tempdir().unwrap();
    let (_stdout, _stderr, code) = pulse(&["audit", "--root", dir.path().to_str().unwrap()]);
    assert_eq!(code, 2);
}

#[test]
fn audit_subcommand_with_all_three_flags_still_stubs() {
    let dir = tempfile::tempdir().unwrap();
    let (_stdout, _stderr, code) = pulse(&[
        "audit",
        "--json",
        "--layer",
        "3",
        "--root",
        dir.path().to_str().unwrap(),
    ]);
    assert_eq!(code, 2);
}

#[test]
fn audit_subcommand_layer_accepts_valid_integer() {
    let (_stdout, _stderr, code) = pulse(&["audit", "--layer", "3"]);
    assert_eq!(code, 2, "stub valid layer should still stub-exit 2");
}

#[test]
fn audit_subcommand_layer_rejects_non_integer() {
    let (_stdout, stderr, code) = pulse(&["audit", "--layer", "abc"]);
    assert_ne!(code, 0);
    assert!(
        stderr.contains("invalid value") || stderr.contains("invalid digit"),
        "clap should reject non-integer layer with parse error, got: {stderr}",
    );
    assert!(!stderr.contains("not yet implemented"), "clap parse error must not reach stub");
}

#[test]
fn hook_flag_with_no_stdin_exits_zero() {
    use std::io::Write;
    use std::process::Stdio;
    let mut child = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["--hook"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"").unwrap();
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(0), "hook with empty stdin must exit 0");
}

#[test]
fn cleanup_flag_exits_zero() {
    let dir = tempfile::tempdir().unwrap();
    std::env::set_var("PULSE_BASELINE_DIR", dir.path());
    let (_stdout, _stderr, code) = pulse(&["--cleanup"]);
    std::env::remove_var("PULSE_BASELINE_DIR");
    assert_eq!(code, 0);
}

#[test]
fn stop_flag_with_empty_baseline_exits_zero() {
    let dir = tempfile::tempdir().unwrap();
    std::env::set_var("PULSE_BASELINE_DIR", dir.path());
    let (_stdout, _stderr, code) = pulse(&["--stop"]);
    std::env::remove_var("PULSE_BASELINE_DIR");
    assert_eq!(code, 0);
}

#[test]
fn check_with_clean_file_exits_zero_silently() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("clean.py");
    std::fs::write(&p, "def f():\n    return 1\n").unwrap();
    let (stdout, _stderr, code) = pulse(&["check", p.to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(stdout.is_empty(), "clean file produces no output");
}

#[test]
fn check_with_smelly_file_exits_zero_with_findings_on_stdout() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("smelly.py");
    std::fs::write(&p, "def f(a, b, c, d, e, f, g, h):\n    return a\n").unwrap();
    let (stdout, _stderr, code) = pulse(&["check", p.to_str().unwrap()]);
    assert_eq!(code, 0, "check exits 0 even with findings (per existing behavior)");
    assert!(stdout.contains("Excess Arguments"), "got: {stdout}");
}

#[test]
fn check_missing_file_exits_zero_silently() {
    let (stdout, _stderr, code) = pulse(&["check", "/no/such/file.py"]);
    assert_eq!(code, 0);
    assert!(stdout.is_empty());
}

#[test]
fn check_dash_a_finds_smells_in_dir() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("s.py"),
        "def f(a, b, c, d, e, f, g, h):\n    return a\n").unwrap();
    let (stdout, _stderr, code) = pulse_in_dir(&["check", "-a"], dir.path());
    assert_eq!(code, 1, "smelly dir → exit 1");
    assert!(stdout.contains("Excess Arguments"));
}

#[test]
fn check_double_dash_all_works() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("s.py"),
        "def f(a, b, c, d, e, f, g, h):\n    return a\n").unwrap();
    let (stdout, _stderr, code) = pulse_in_dir(&["check", "--all"], dir.path());
    assert_eq!(code, 1);
    assert!(stdout.contains("Excess Arguments"));
}

#[test]
fn bare_dash_a_alias_works() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("s.py"),
        "def f(a, b, c, d, e, f, g, h):\n    return a\n").unwrap();
    let (stdout, _stderr, code) = pulse_in_dir(&["-a"], dir.path());
    assert_eq!(code, 1);
    assert!(stdout.contains("Excess Arguments"));
}

#[test]
fn bare_double_dash_all_alias_works() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("s.py"),
        "def f(a, b, c, d, e, f, g, h):\n    return a\n").unwrap();
    let (stdout, _stderr, code) = pulse_in_dir(&["--all"], dir.path());
    assert_eq!(code, 1);
    assert!(stdout.contains("Excess Arguments"));
}

#[test]
fn check_a_short_t_includes_test_files() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("test_foo.py"),
        "def f(a, b, c, d, e, f, g, h):\n    return a\n").unwrap();
    let (stdout_no_t, _, _) = pulse_in_dir(&["check", "-a"], dir.path());
    assert!(stdout_no_t.is_empty(), "test files skipped by default");
    let (stdout_with_t, _, code_with_t) = pulse_in_dir(&["check", "-a", "-t"], dir.path());
    assert_eq!(code_with_t, 1);
    assert!(stdout_with_t.contains("Excess Arguments"));
}

#[test]
fn check_a_long_include_tests_works() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("test_foo.py"),
        "def f(a, b, c, d, e, f, g, h):\n    return a\n").unwrap();
    let (stdout, _, code) = pulse_in_dir(&["check", "-a", "--include-tests"], dir.path());
    assert_eq!(code, 1);
    assert!(stdout.contains("Excess Arguments"));
}

#[test]
fn debug_writes_to_stderr_not_stdout() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("t.py");
    std::fs::write(&p, "def f():\n    return 1\n").unwrap();
    let (stdout, stderr, code) = pulse(&["debug", p.to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(stdout.is_empty(), "debug output goes to stderr");
    assert!(stderr.contains("Module:"), "got: {stderr}");
}

#[test]
fn budget_with_file_writes_to_stderr() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("t.py");
    std::fs::write(&p, "def f():\n    return 1\n").unwrap();
    let (_stdout, stderr, code) = pulse(&["budget", p.to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(stderr.contains("functions:"), "got: {stderr}");
    assert!(stderr.contains("LOC:"));
}

#[test]
fn budget_new_writes_thresholds_to_stderr() {
    let (_stdout, stderr, code) = pulse(&["budget", "--new"]);
    assert_eq!(code, 0);
    assert!(stderr.contains("max functions:"));
}

#[test]
fn setup_subcommand_runs_and_exits_zero() {
    let dir = tempfile::tempdir().unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .arg("setup")
        .env("HOME", dir.path())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0), "setup must exit 0; stderr: {}",
        String::from_utf8_lossy(&out.stderr));
}

#[test]
fn unknown_subcommand_exits_nonzero() {
    let (_stdout, stderr, code) = pulse(&["bogus_subcommand_xyz"]);
    assert_ne!(code, 0, "unknown subcommand must not exit 0");
    assert!(!stderr.is_empty(), "unknown subcommand should produce a diagnostic");
}

#[test]
fn unknown_flag_to_check_exits_nonzero() {
    let (_stdout, stderr, code) = pulse(&["check", "--definitely-not-a-flag"]);
    assert_ne!(code, 0);
    assert!(!stderr.is_empty(), "got: {stderr}");
}

#[test]
fn no_args_exits_nonzero_with_help_or_usage() {
    let (_stdout, stderr, code) = pulse(&[]);
    assert_ne!(code, 0, "no args must not silently succeed");
    let combined = stderr.to_lowercase();
    assert!(
        combined.contains("usage") || combined.contains("error") || combined.contains("help"),
        "stderr should suggest usage/help, got: {stderr}",
    );
}
