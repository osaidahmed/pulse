
use crate::common::*;
use std::process::Command;

const LANG: &str = "python";

// ===========================================================================
// Clean file — zero findings
// ===========================================================================

#[test]
fn clean_file_produces_no_output() {
    let output = run_check(LANG, "clean.py");
    assert!(
        output.is_empty(),
        "clean file should produce no output, got: {output}"
    );
}

// ===========================================================================
// Complex Method (cc >= threshold)
// ===========================================================================

#[test]
fn complex_method_detected() {
    let output = run_check(LANG, "complex_method.py");
    assert!(
        has_smell(&output, "Complex Method") || has_smell(&output, "God Method"),
        "should detect complex method, got: {output}"
    );
    assert!(has_function(&output, "process_order"));
}

#[test]
fn complex_method_cc_at_least_9() {
    let debug = run_debug(LANG, "complex_method.py");
    let cc = function_metric(&debug, "process_order", "cc");
    assert!(cc.is_some(), "should compute cc for process_order");
    assert!(cc.unwrap() >= t().function.cc_warning, "cc should be >= t().function.cc_warning, got: {}", cc.unwrap());
}

// ===========================================================================
// Large Method (loc >= threshold)
// ===========================================================================

#[test]
fn large_method_detected() {
    let mut code = String::from("def build_report(data):\n");
    for i in 0..fn_padding() { code.push_str(&format!("    x_{i} = {i}\n")); }
    code.push_str("    return x_0\n");
    let out = pulse_check_code(&code, "py");
    assert!(
        has_smell(&out, "Large Method") || has_smell(&out, "God Method"),
        "should detect large method, got: {out}"
    );
}

#[test]
fn large_method_loc_exceeds_threshold() {
    let mut code = String::from("def build_report(data):\n");
    for i in 0..fn_padding() { code.push_str(&format!("    x_{i} = {i}\n")); }
    code.push_str("    return x_0\n");
    let debug = pulse_debug_code(&code, "py");
    let loc = function_metric(&debug, "build_report", "loc").unwrap_or(0);
    assert!(loc >= t().function.fn_loc_warning, "loc should exceed threshold, got: {loc}");
}

// ===========================================================================
// God Method (complex AND large)
// ===========================================================================

#[test]
fn god_method_detected() {
    let output = run_check(LANG, "brain_method.py");
    assert!(
        has_smell(&output, "God Method"),
        "should detect god method, got: {output}"
    );
    assert!(has_function(&output, "process_data_pipeline"));
}

#[test]
fn god_method_has_high_cc_and_loc() {
    let debug = run_debug(LANG, "brain_method.py");
    let cc = function_metric(&debug, "process_data_pipeline", "cc").unwrap_or(0);
    let loc = function_metric(&debug, "process_data_pipeline", "loc").unwrap_or(0);
    assert!(cc >= t().function.cc_warning, "god method cc should be >= t().function.cc_warning, got: {cc}");
    assert!(loc >= t().function.fn_loc_warning, "god method loc should be >= 50, got: {loc}");
}

#[test]
fn god_method_not_reported_as_separate_complex_and_large() {
    let output = run_check(LANG, "brain_method.py");
    assert!(has_smell(&output, "God Method"));
    // When God Method is detected, Complex Method and Large Method should NOT appear for same function
    let lines: Vec<&str> = output
        .lines()
        .filter(|l| l.contains("process_data_pipeline"))
        .collect();
    assert!(
        !lines.iter().any(|l| l.contains("Complex Method")),
        "should not separately report Complex Method for a God Method"
    );
    assert!(
        !lines.iter().any(|l| l.contains("Large Method")),
        "should not separately report Large Method for a God Method"
    );
}

// ===========================================================================
// Nested Conditional Chunks (bumpy road pattern)
// ===========================================================================

#[test]
fn nested_conditional_chunks_detected() {
    let output = run_check(LANG, "bumpy_road.py");
    assert!(
        has_smell(&output, "Nested Conditional Chunks"),
        "should detect nested conditional chunks, got: {output}"
    );
    assert!(has_function(&output, "validate_and_process"));
}

#[test]
fn nested_conditional_chunks_bump_count() {
    let debug = run_debug(LANG, "bumpy_road.py");
    let bumps = function_metric(&debug, "validate_and_process", "bumps").unwrap_or(0);
    assert!(bumps >= 2, "should have >= 2 bumps, got: {bumps}");
}

// ===========================================================================
// Deep Nested Complexity
// ===========================================================================

#[test]
fn deep_nesting_detected() {
    let output = run_check(LANG, "deep_nesting.py");
    assert!(
        has_smell(&output, "Deep Nested"),
        "should detect deep nesting, got: {output}"
    );
    assert!(has_function(&output, "deeply_nested"));
}

#[test]
fn deep_nesting_depth_exceeds_4() {
    let debug = run_debug(LANG, "deep_nesting.py");
    let depth = function_metric(&debug, "deeply_nested", "nesting").unwrap_or(0);
    assert!(depth > 4, "nesting should be > 4, got: {depth}");
}

#[test]
fn moderate_nesting_not_flagged() {
    let output = run_check(LANG, "deep_nesting.py");
    assert!(
        !has_function(&output, "moderately_nested"),
        "moderate nesting should not be flagged"
    );
}

// ===========================================================================
// Complex Conditional
// ===========================================================================

#[test]
fn complex_conditional_detected() {
    let output = run_check(LANG, "complex_conditional.py");
    assert!(
        has_smell(&output, "Complex Conditional") || has_smell(&output, "Complex Method"),
        "should detect complex conditional, got: {output}"
    );
    assert!(has_function(&output, "check_eligibility"));
}

#[test]
fn simple_check_not_flagged() {
    let output = run_check(LANG, "complex_conditional.py");
    assert!(
        !has_function(&output, "simple_check"),
        "simple_check should not be flagged"
    );
}

// ===========================================================================
// Excess Function Arguments
// ===========================================================================

#[test]
fn excess_args_detected() {
    let output = run_check(LANG, "excess_args.py");
    assert!(
        has_smell(&output, "Excess Arguments"),
        "should detect excess args, got: {output}"
    );
    assert!(has_function(&output, "create_user"));
}

#[test]
fn excess_args_count_is_correct() {
    let debug = run_debug(LANG, "excess_args.py");
    let args = function_metric(&debug, "create_user", "args").unwrap_or(0);
    assert_eq!(args, 8, "create_user has 8 args, got: {args}");
}

#[test]
fn simple_func_not_flagged() {
    let output = run_check(LANG, "excess_args.py");
    assert!(
        !has_function(&output, "simple_func"),
        "simple_func should not be flagged"
    );
}

// ===========================================================================
// Constructor Over-Injection
// ===========================================================================

#[test]
fn constructor_over_injection_detected() {
    let output = run_check(LANG, "excess_args.py");
    assert!(
        has_smell(&output, "Constructor Over-Injection") || has_smell(&output, "Excess Arguments"),
        "should detect constructor over-injection, got: {output}"
    );
    assert!(has_function(&output, "UserService.__init__"));
}

#[test]
fn constructor_args_exclude_self() {
    let debug = run_debug(LANG, "excess_args.py");
    let args = function_metric(&debug, "UserService.__init__", "args").unwrap_or(0);
    assert_eq!(
        args, 6,
        "constructor args should be 6 (excluding self), got: {args}"
    );
}

// ===========================================================================
// Large Embedded Block
// ===========================================================================

#[test]
fn embedded_block_detected() {
    let output = run_check(LANG, "embedded_block.py");
    assert!(
        has_smell(&output, "Large Embedded Block"),
        "should detect embedded block, got: {output}"
    );
    assert!(has_function(&output, "get_active_users"));
}

#[test]
fn simple_query_not_flagged() {
    let output = run_check(LANG, "embedded_block.py");
    assert!(
        !has_function(&output, "simple_query"),
        "simple_query should not be flagged"
    );
}

// ===========================================================================
// Global Conditionals
// ===========================================================================

#[test]
fn global_conditionals_detected() {
    let output = run_check(LANG, "global_conditionals.py");
    assert!(
        has_smell(&output, "Global Conditionals"),
        "should detect global conditionals, got: {output}"
    );
}

// ===========================================================================
// File Too Large (module-level)
// ===========================================================================

#[test]
fn file_too_large_detected() {
    let mut code = String::new();
    for i in 0..file_padding() { code.push_str(&format!("x_{i} = {i}\n")); }
    let out = pulse_check_code(&code, "py");
    assert!(
        has_smell(&out, "File Too Large"),
        "should detect file too large, got: {out}"
    );
}

#[test]
fn too_many_functions_detected() {
    let output = run_check(LANG, "file_too_large.py");
    assert!(
        has_smell(&output, "Too Many Functions"),
        "should detect too many functions, got: {output}"
    );
}

// ===========================================================================
// Hook mode
// ===========================================================================

#[test]
fn hook_clean_file_is_silent() {
    let path = fixtures_dir(LANG).join("clean.py");
    let output = run_hook(path.to_str().unwrap());
    assert!(
        output.is_empty(),
        "hook on clean file should be silent, got: {output}"
    );
}

#[test]
fn hook_smelly_file_produces_output() {
    let path = fixtures_dir(LANG).join("brain_method.py");
    let output = run_hook(path.to_str().unwrap());
    assert!(
        !output.is_empty(),
        "hook on smelly file should produce output"
    );
    assert!(has_smell(&output, "god method"));
}

#[test]
fn hook_nonexistent_file_is_silent() {
    let output = run_hook("/nonexistent/path/foo.py");
    assert!(output.is_empty());
}

#[test]
fn hook_unsupported_extension_is_silent() {
    let output = run_hook("/some/file.rs");
    assert!(output.is_empty());
}

#[test]
fn hook_invalid_json_is_silent() {
    let output = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["--hook"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.take().unwrap().write_all(b"not json").unwrap();
            child.wait_with_output()
        })
        .expect("failed to run");
    assert!(output.stdout.is_empty());
}

// ===========================================================================
// Metric accuracy
// ===========================================================================

#[test]
fn cc_base_case_is_1() {
    let debug = run_debug(LANG, "clean.py");
    let cc = function_metric(&debug, "add", "cc").unwrap_or(99);
    assert_eq!(cc, 1, "function with no branches should have cc=1");
}

#[test]
fn method_arg_count_excludes_self() {
    let debug = run_debug(LANG, "clean.py");
    let args = function_metric(&debug, "Calculator.add", "args").unwrap_or(99);
    assert_eq!(
        args, 1,
        "method arg count should exclude self, got: {args}"
    );
}

#[test]
fn boolean_operators_increment_cc() {
    let debug = run_debug(LANG, "complex_conditional.py");
    let cc = function_metric(&debug, "check_eligibility", "cc").unwrap_or(0);
    // 1 base + 2 if-statements + boolean operators from conditions
    assert!(
        cc >= 7,
        "boolean operators should increment cc, got: {cc}"
    );
}

// ===========================================================================
// Output format
// ===========================================================================

#[test]
fn output_starts_with_pulse_prefix() {
    let output = run_check(LANG, "brain_method.py");
    assert!(output.starts_with("pulse:"));
    assert!(output.contains("brain_method.py"));
}

#[test]
fn output_has_function_line_numbers() {
    let output = run_check(LANG, "brain_method.py");
    let has_location = output
        .lines()
        .any(|l| l.contains("(L") && l.contains("): "));
    assert!(
        has_location,
        "should contain function locations with line numbers"
    );
}

#[test]
fn output_has_module_prefix() {
    let output = run_check(LANG, "file_too_large.py");
    assert!(
        output.contains("Module:"),
        "module smells should have 'Module:' prefix"
    );
}

#[test]
fn issue_count_in_header_matches_findings() {
    let output = run_check(LANG, "excess_args.py");
    let first_line = output.lines().next().unwrap_or("");
    let finding_lines = output.lines().filter(|l| l.starts_with("  ")).count();
    assert!(
        first_line.contains(&format!("{finding_lines} issue")),
        "header should report correct issue count"
    );
}

// ===========================================================================
// Edge cases
// ===========================================================================

#[test]
fn empty_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("empty.py");
    std::fs::write(&path, "").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    assert!(out.stdout.is_empty());
}

#[test]
fn comments_only_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("comments.py");
    std::fs::write(&path, "# just comments\n# nothing else\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    assert!(out.stdout.is_empty());
}

#[test]
fn decorated_function_analyzed() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("deco.py");
    std::fs::write(
        &path,
        "def d(f):\n    return f\n\n@d\ndef long_deco(a, b, c, d, e, f, g, h):\n    return a\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        has_smell(&stdout, "Excess Arguments") || has_function(&stdout, "long_deco"),
        "decorated function with 8 args should be flagged, got: {stdout}"
    );
}

#[test]
fn function_at_threshold_boundary_is_flagged() {
    // cc_warning = 9, so cc=9 should trigger
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("boundary.py");
    // 9 branches: base(1) + 8 if-statements = cc=9
    let mut code = "def boundary():\n".to_string();
    for i in 0..8 {
        code.push_str(&format!("    if x == {i}:\n        pass\n"));
    }
    code.push_str("    return True\n");
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        has_smell(&stdout, "Complex Method"),
        "cc=9 should trigger Complex Method, got: {stdout}"
    );
}

#[test]
fn function_below_threshold_not_flagged() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("below.py");
    // 7 branches = cc=8 (below 9 threshold)
    let mut code = "def below():\n".to_string();
    for i in 0..7 {
        code.push_str(&format!("    if x == {i}:\n        pass\n"));
    }
    code.push_str("    return True\n");
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    assert!(out.stdout.is_empty(), "cc=8 should not trigger anything");
}

// ===========================================================================
// Performance
// ===========================================================================

#[test]
fn analysis_completes_under_500ms() {
    let path = fixtures_dir(LANG).join("file_too_large.py");
    let start = std::time::Instant::now();
    let _ = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let elapsed = start.elapsed();
    assert!(
        elapsed.as_millis() < 500,
        "should complete under 500ms, took: {}ms",
        elapsed.as_millis()
    );
}
