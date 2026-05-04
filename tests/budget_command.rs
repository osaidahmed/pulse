mod common;

use common::*;
use std::process::Command;

fn run_budget(file_path: &str) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["budget", file_path])
        .output()
        .expect("failed to run pulse budget");
    String::from_utf8(output.stderr).unwrap()
}

fn run_check(file_path: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", file_path])
        .output()
        .expect("failed");
    String::from_utf8(out.stdout).unwrap()
}

// ===========================================================================
// Budget command output
// ===========================================================================

#[test]
fn budget_shows_function_count_and_room() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.py");
    let mut code = String::new();
    for i in 0..5 { code.push_str(&format!("def fn_{i}():\n    return {i}\n\n")); }
    std::fs::write(&path, &code).unwrap();
    let out = run_budget(path.to_str().unwrap());
    assert!(out.contains("functions: 5/20"), "should show 5/20, got: {out}");
    assert!(out.contains("room: 15"), "should show room 15, got: {out}");
}

#[test]
fn budget_shows_loc_and_room() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.py");
    std::fs::write(&path, "def f():\n    return 1\n").unwrap();
    let out = run_budget(path.to_str().unwrap());
    assert!(out.contains("LOC:"), "should show LOC line, got: {out}");
    let expected = format!("/{}", t().module.file_loc_warning);
    assert!(out.contains(&expected), "should show file LOC threshold, got: {out}");
}

#[test]
fn budget_shows_cc_and_room() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.py");
    std::fs::write(&path, "def f():\n    return 1\n").unwrap();
    let out = run_budget(path.to_str().unwrap());
    assert!(out.contains("total cc:"), "should show cc line, got: {out}");
    assert!(out.contains("/100"), "should show /100 threshold, got: {out}");
}

#[test]
fn budget_shows_per_function_limits() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.py");
    std::fs::write(&path, "def f():\n    return 1\n").unwrap();
    let out = run_budget(path.to_str().unwrap());
    let t = t();
    assert!(out.contains(&format!("cc<{}", t.function.cc_warning)), "should show cc limit, got: {out}");
    assert!(out.contains(&format!("cogc<{}", t.function.cogc_warning)), "should show cogc limit, got: {out}");
    assert!(out.contains(&format!("loc<{}", t.function.fn_loc_warning)), "should show loc limit, got: {out}");
    assert!(out.contains(&format!("args≤{}", t.function.arg_max)), "should show args limit, got: {out}");
}

#[test]
fn budget_zero_room_when_at_threshold() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.py");
    let mut code = String::new();
    for i in 0..20 { code.push_str(&format!("def fn_{i}():\n    return {i}\n\n")); }
    std::fs::write(&path, &code).unwrap();
    let out = run_budget(path.to_str().unwrap());
    assert!(out.contains("room: 0"), "20/20 should show room 0, got: {out}");
}

#[test]
fn budget_nonexistent_file() {
    let out = run_budget("/no/such/file.py");
    assert!(out.contains("unsupported or unreadable"), "got: {out}");
}

#[test]
fn budget_unsupported_extension() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.toml");
    std::fs::write(&path, "[x]\ny = 1\n").unwrap();
    let out = run_budget(path.to_str().unwrap());
    assert!(out.contains("unsupported or unreadable"), "got: {out}");
}

// ===========================================================================
// Threshold values in findings
// ===========================================================================

#[test]
fn check_excess_args_shows_threshold() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.py");
    std::fs::write(&path, "def f(a, b, c, d, e, f, g, h):\n    return a\n").unwrap();
    let out = run_check(path.to_str().unwrap());
    let expected = format!("threshold: {}", t().function.arg_max);
    assert!(out.contains(&expected), "excess args should show threshold, got: {out}");
}

#[test]
fn check_complex_method_shows_threshold() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.py");
    let mut code = String::from("def f(x):\n");
    for i in 0..cc_branches() { code.push_str(&format!("    if x > {i}:\n        pass\n")); }
    code.push_str("    return x\n");
    std::fs::write(&path, &code).unwrap();
    let out = run_check(path.to_str().unwrap());
    let t = t();
    let expected = format!("threshold: {}", t.function.cc_warning);
    let expected_cc = format!("cc threshold: {}", t.function.cc_warning);
    assert!(out.contains(&expected) || out.contains(&expected_cc), "cc should show threshold, got: {out}");
}

#[test]
fn check_deep_nesting_shows_threshold() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.py");
    std::fs::write(&path, concat!(
        "def f(a, b, c, d, e):\n",
        "    if a:\n",
        "        if b:\n",
        "            if c:\n",
        "                if d:\n",
        "                    if e:\n",
        "                        return 1\n",
        "    return 0\n",
    )).unwrap();
    let out = run_check(path.to_str().unwrap());
    let expected = format!("threshold: {}", t().function.nesting_depth);
    assert!(out.contains(&expected), "nesting should show threshold, got: {out}");
}

#[test]
fn check_file_too_large_shows_threshold() {
    let t = t();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.py");
    let mut code = String::new();
    for i in 0..(t.module.file_loc_warning as usize + 20) { code.push_str(&format!("x_{i} = {i}\n")); }
    std::fs::write(&path, &code).unwrap();
    let out = run_check(path.to_str().unwrap());
    let expected = format!("threshold: {}", t.module.file_loc_warning);
    assert!(out.contains(&expected), "file too large should show threshold, got: {out}");
}

#[test]
fn check_too_many_functions_shows_threshold() {
    let t = t();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.py");
    let mut code = String::new();
    for i in 0..(t.module.file_function_count as usize + 5) { code.push_str(&format!("def fn_{i}():\n    return {i}\n\n")); }
    std::fs::write(&path, &code).unwrap();
    let out = run_check(path.to_str().unwrap());
    let expected = format!("threshold: {}", t.module.file_function_count);
    assert!(out.contains(&expected), "too many fns should show threshold, got: {out}");
}

#[test]
fn check_large_method_shows_threshold() {
    let t = t();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.py");
    let mut code = String::from("def big():\n");
    for i in 0..(t.function.fn_loc_warning as usize + 10) { code.push_str(&format!("    x_{i} = {i}\n")); }
    code.push_str("    return 0\n");
    std::fs::write(&path, &code).unwrap();
    let out = run_check(path.to_str().unwrap());
    let expected = format!("threshold: {}", t.function.fn_loc_warning);
    assert!(out.contains(&expected), "large method should show threshold, got: {out}");
}

#[test]
fn check_embedded_block_shows_threshold() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.py");
    let mut code = String::from("def f():\n    s = '''\n");
    for i in 0..20 { code.push_str(&format!("    line {i}\n")); }
    code.push_str("    '''\n    return s\n");
    std::fs::write(&path, &code).unwrap();
    let out = run_check(path.to_str().unwrap());
    let expected = format!("threshold: {}", t().function.embedded_block_loc);
    assert!(out.contains(&expected), "embedded block should show threshold, got: {out}");
}
