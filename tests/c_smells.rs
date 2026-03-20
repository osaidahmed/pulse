mod common;

use common::*;
use std::process::Command;

const LANG: &str = "c";

#[test]
fn clean_file_produces_no_output() {
    let output = run_check(LANG, "clean.c");
    assert!(output.is_empty(), "got: {}", output);
}

#[test]
fn complex_method_detected() {
    let output = run_check(LANG, "complex_method.c");
    assert!(has_smell(&output, "Complex Method"), "got: {}", output);
    assert!(has_function(&output, "process_order"));
}

#[test]
fn complex_method_cc_at_least_9() {
    let debug = run_debug(LANG, "complex_method.c");
    let cc = function_metric(&debug, "process_order", "cc").unwrap_or(0);
    assert!(cc >= 9, "cc should be >= 9, got: {}", cc);
}

#[test]
fn excess_args_detected() {
    let output = run_check(LANG, "excess_args.c");
    assert!(has_smell(&output, "Excess Arguments"), "got: {}", output);
    assert!(has_function(&output, "create_user"));
}

#[test]
fn excess_args_count_correct() {
    let debug = run_debug(LANG, "excess_args.c");
    let args = function_metric(&debug, "create_user", "args").unwrap_or(0);
    assert_eq!(args, 8, "got: {}", args);
}

#[test]
fn simple_func_not_flagged() {
    let output = run_check(LANG, "excess_args.c");
    assert!(!has_function(&output, "simple_func"));
}

#[test]
fn deep_nesting_detected() {
    let output = run_check(LANG, "deep_nesting.c");
    assert!(has_smell(&output, "Deep Nested"), "got: {}", output);
    assert!(has_function(&output, "deeply_nested"));
}

#[test]
fn deep_nesting_depth_exceeds_4() {
    let debug = run_debug(LANG, "deep_nesting.c");
    let depth = function_metric(&debug, "deeply_nested", "nesting").unwrap_or(0);
    assert!(depth > 4, "got: {}", depth);
}

#[test]
fn moderate_nesting_not_flagged() {
    let output = run_check(LANG, "deep_nesting.c");
    assert!(!has_function(&output, "moderately_nested"));
}

#[test]
fn cc_base_case_is_1() {
    let debug = run_debug(LANG, "clean.c");
    let cc = function_metric(&debug, "add", "cc").unwrap_or(99);
    assert_eq!(cc, 1);
}

#[test]
fn output_starts_with_pulse() {
    let output = run_check(LANG, "complex_method.c");
    assert!(output.starts_with("pulse:"));
}

#[test]
fn output_has_function_line_numbers() {
    let output = run_check(LANG, "complex_method.c");
    assert!(output.lines().any(|l| l.contains("(L") && l.contains("): ")));
}

#[test]
fn hook_clean_file_silent() {
    let path = fixtures_dir(LANG).join("clean.c");
    let output = run_hook(path.to_str().unwrap());
    assert!(output.is_empty());
}

#[test]
fn hook_smelly_file_produces_output() {
    let path = fixtures_dir(LANG).join("complex_method.c");
    let output = run_hook(path.to_str().unwrap());
    assert!(!output.is_empty());
}

#[test]
fn hook_nonexistent_file_silent() {
    let output = run_hook("/nonexistent/path/foo.c");
    assert!(output.is_empty());
}

#[test]
fn h_extension_supported() {
    let out = pulse_check_code("int add(int a, int b) {\n    return a + b;\n}\n", "h");
    assert!(out.is_empty());
}

#[test]
fn empty_file() {
    let out = pulse_check_code("", "c");
    assert!(out.is_empty());
}

#[test]
fn void_param_is_zero_args() {
    let debug = pulse_debug_code("int f(void) {\n    return 0;\n}\n", "c");
    let args = function_metric(&debug, "f", "args").unwrap_or(99);
    assert_eq!(args, 0, "f(void) should be 0 args, got: {}", args);
}

#[test]
fn function_at_cc_boundary_flagged() {
    let out = pulse_check_code("int f(void) {\n    if (a) {}\n    if (b) {}\n    if (c) {}\n    if (d) {}\n    if (e) {}\n    if (f) {}\n    if (g) {}\n    if (h) {}\n    return 0;\n}\n", "c");
    assert!(has_smell(&out, "Complex Method"), "cc=9 should trigger, got: {}", out);
}

#[test]
fn function_below_cc_boundary_not_flagged() {
    let out = pulse_check_code("int f(void) {\n    if (a) {}\n    if (b) {}\n    if (c) {}\n    if (d) {}\n    if (e) {}\n    if (f) {}\n    if (g) {}\n    return 0;\n}\n", "c");
    assert!(!has_smell(&out, "Complex Method"));
}

#[test]
fn issue_count_matches_findings() {
    let output = run_check(LANG, "complex_method.c");
    let first = output.lines().next().unwrap_or("");
    let findings = output.lines().filter(|l| l.starts_with("  ")).count();
    assert!(first.contains(&format!("{} issue", findings)));
}
