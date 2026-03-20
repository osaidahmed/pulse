mod common;

use common::*;
use std::process::Command;

const LANG: &str = "rust";

#[test]
fn clean_file_produces_no_output() {
    let output = run_check(LANG, "clean.rs");
    assert!(output.is_empty(), "clean Rust file should produce no output, got: {}", output);
}

#[test]
fn complex_method_detected() {
    let output = run_check(LANG, "complex_method.rs");
    assert!(has_smell(&output, "Complex Method"), "should detect complex method, got: {}", output);
    assert!(has_function(&output, "process_order"));
}

#[test]
fn complex_method_cc_at_least_9() {
    let debug = run_debug(LANG, "complex_method.rs");
    let cc = function_metric(&debug, "process_order", "cc").unwrap_or(0);
    assert!(cc >= 9, "cc should be >= 9, got: {}", cc);
}

#[test]
fn excess_args_detected() {
    let output = run_check(LANG, "excess_args.rs");
    assert!(has_smell(&output, "Excess Arguments"), "got: {}", output);
    assert!(has_function(&output, "create_user"));
}

#[test]
fn simple_func_not_flagged() {
    let output = run_check(LANG, "excess_args.rs");
    assert!(!has_function(&output, "simple_func"));
}

#[test]
fn constructor_over_injection_detected() {
    let output = run_check(LANG, "excess_args.rs");
    assert!(has_smell(&output, "Constructor Over-Injection"), "got: {}", output);
    assert!(has_function(&output, "UserService.new"));
}

#[test]
fn self_param_excluded_from_arg_count() {
    let debug = run_debug(LANG, "excess_args.rs");
    let args = function_metric(&debug, "UserService.get_user", "args").unwrap_or(99);
    assert_eq!(args, 1, "get_user takes &self + user_id, should report 1, got: {}", args);
}

#[test]
fn primitive_obsession_detected() {
    let output = run_check(LANG, "excess_args.rs");
    assert!(has_smell(&output, "Primitive Obsession"), "got: {}", output);
}

#[test]
fn hook_mode_works_with_rs() {
    let path = fixtures_dir(LANG).join("clean.rs");
    let output = run_hook(path.to_str().unwrap());
    assert!(output.is_empty(), "hook on clean Rust file should be silent");
}

#[test]
fn hook_mode_detects_smells_in_rs() {
    let path = fixtures_dir(LANG).join("complex_method.rs");
    let output = run_hook(path.to_str().unwrap());
    assert!(!output.is_empty(), "hook on smelly Rust file should produce output");
}

#[test]
fn output_starts_with_pulse() {
    let output = run_check(LANG, "complex_method.rs");
    assert!(output.starts_with("pulse:"));
}

// ===========================================================================
// Additional coverage — match Python depth
// ===========================================================================

#[test]
fn excess_args_count_correct() {
    let debug = run_debug(LANG, "excess_args.rs");
    let args = function_metric(&debug, "create_user", "args").unwrap_or(0);
    assert_eq!(args, 8, "got: {}", args);
}

#[test]
fn constructor_args_count_correct() {
    let debug = run_debug(LANG, "excess_args.rs");
    let args = function_metric(&debug, "UserService.new", "args").unwrap_or(0);
    assert_eq!(args, 6, "got: {}", args);
}

#[test]
fn cc_base_case_is_1() {
    let debug = run_debug(LANG, "clean.rs");
    let cc = function_metric(&debug, "add", "cc").unwrap_or(99);
    assert_eq!(cc, 1);
}

#[test]
fn method_arg_count_excludes_self() {
    let debug = run_debug(LANG, "clean.rs");
    let args = function_metric(&debug, "Calculator.add", "args").unwrap_or(99);
    assert_eq!(args, 1, "should exclude &mut self, got: {}", args);
}

#[test]
fn output_has_function_line_numbers() {
    let output = run_check(LANG, "complex_method.rs");
    let has_loc = output.lines().any(|l| l.contains("(L") && l.contains("): "));
    assert!(has_loc);
}

#[test]
fn empty_file() {
    let out = pulse_check_code("", "rs");
    assert!(out.is_empty());
}

#[test]
fn comments_only_file() {
    let out = pulse_check_code("// just comments\n// nothing else\n", "rs");
    assert!(out.is_empty());
}

#[test]
fn function_at_cc_boundary_flagged() {
    let out = pulse_check_code("fn f() {\n    if true {}\n    if true {}\n    if true {}\n    if true {}\n    if true {}\n    if true {}\n    if true {}\n    if true {}\n}\n", "rs");
    assert!(has_smell(&out, "Complex Method"), "cc=9 should trigger, got: {}", out);
}

#[test]
fn function_below_cc_boundary_not_flagged() {
    let out = pulse_check_code("fn f() {\n    if true {}\n    if true {}\n    if true {}\n    if true {}\n    if true {}\n    if true {}\n    if true {}\n}\n", "rs");
    assert!(!has_smell(&out, "Complex Method"));
}

#[test]
fn hook_nonexistent_file_silent() {
    let output = run_hook("/nonexistent/path/foo.rs");
    assert!(output.is_empty());
}

#[test]
fn issue_count_in_header_matches_findings() {
    let output = run_check(LANG, "excess_args.rs");
    let first = output.lines().next().unwrap_or("");
    let findings = output.lines().filter(|l| l.starts_with("  ")).count();
    assert!(first.contains(&format!("{} issue", findings)));
}
