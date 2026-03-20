mod common;

use common::*;
use std::process::Command;

const LANG: &str = "java";

#[test]
fn clean_file_produces_no_output() {
    let output = run_check(LANG, "Clean.java");
    assert!(output.is_empty(), "got: {}", output);
}

#[test]
fn complex_method_detected() {
    let output = run_check(LANG, "ComplexMethod.java");
    assert!(has_smell(&output, "Complex Method"), "got: {}", output);
    assert!(has_function(&output, "processOrder"));
}

#[test]
fn complex_method_cc_at_least_9() {
    let debug = run_debug(LANG, "ComplexMethod.java");
    let cc = function_metric(&debug, "processOrder", "cc").unwrap_or(0);
    assert!(cc >= 9, "cc should be >= 9, got: {}", cc);
}

#[test]
fn excess_args_detected() {
    let output = run_check(LANG, "ExcessArgs.java");
    assert!(has_smell(&output, "Excess Arguments"), "got: {}", output);
    assert!(has_function(&output, "createUser"));
}

#[test]
fn excess_args_count_correct() {
    let debug = run_debug(LANG, "ExcessArgs.java");
    let args = function_metric(&debug, "createUser", "args").unwrap_or(0);
    assert_eq!(args, 8, "got: {}", args);
}

#[test]
fn simple_func_not_flagged() {
    let output = run_check(LANG, "ExcessArgs.java");
    assert!(!has_function(&output, "simpleFunc"));
}

#[test]
fn constructor_over_injection_detected() {
    let output = run_check(LANG, "ExcessArgs.java");
    assert!(has_smell(&output, "Constructor Over-Injection"), "got: {}", output);
}

#[test]
fn deep_nesting_detected() {
    let output = run_check(LANG, "DeepNesting.java");
    assert!(has_smell(&output, "Deep Nested"), "got: {}", output);
    assert!(has_function(&output, "deeplyNested"));
}

#[test]
fn deep_nesting_depth_exceeds_4() {
    let debug = run_debug(LANG, "DeepNesting.java");
    let depth = function_metric(&debug, "deeplyNested", "nesting").unwrap_or(0);
    assert!(depth > 4, "got: {}", depth);
}

#[test]
fn moderate_nesting_not_flagged() {
    let output = run_check(LANG, "DeepNesting.java");
    assert!(!has_function(&output, "moderatelyNested"));
}

#[test]
fn cc_base_case_is_1() {
    let debug = run_debug(LANG, "Clean.java");
    let cc = function_metric(&debug, "add", "cc").unwrap_or(99);
    assert_eq!(cc, 1);
}

#[test]
fn output_starts_with_pulse() {
    let output = run_check(LANG, "ComplexMethod.java");
    assert!(output.starts_with("pulse:"));
}

#[test]
fn output_has_function_line_numbers() {
    let output = run_check(LANG, "ComplexMethod.java");
    assert!(output.lines().any(|l| l.contains("(L") && l.contains("): ")));
}

#[test]
fn hook_clean_file_silent() {
    let path = fixtures_dir(LANG).join("Clean.java");
    let output = run_hook(path.to_str().unwrap());
    assert!(output.is_empty());
}

#[test]
fn hook_smelly_file_produces_output() {
    let path = fixtures_dir(LANG).join("ComplexMethod.java");
    let output = run_hook(path.to_str().unwrap());
    assert!(!output.is_empty());
}

#[test]
fn hook_nonexistent_file_silent() {
    let output = run_hook("/nonexistent/path/Foo.java");
    assert!(output.is_empty());
}

#[test]
fn empty_file() {
    let out = pulse_check_code("", "java");
    assert!(out.is_empty());
}

#[test]
fn function_at_cc_boundary_flagged() {
    let out = pulse_check_code("class T {\n    void f() {\n        if (a) {}\n        if (b) {}\n        if (c) {}\n        if (d) {}\n        if (e) {}\n        if (f) {}\n        if (g) {}\n        if (h) {}\n    }\n}\n", "java");
    assert!(has_smell(&out, "Complex Method"), "cc=9 should trigger, got: {}", out);
}

#[test]
fn function_below_cc_boundary_not_flagged() {
    let out = pulse_check_code("class T {\n    void f() {\n        if (a) {}\n        if (b) {}\n        if (c) {}\n        if (d) {}\n        if (e) {}\n        if (f) {}\n        if (g) {}\n    }\n}\n", "java");
    assert!(!has_smell(&out, "Complex Method"));
}

#[test]
fn issue_count_matches_findings() {
    let output = run_check(LANG, "ComplexMethod.java");
    let first = output.lines().next().unwrap_or("");
    let findings = output.lines().filter(|l| l.starts_with("  ")).count();
    assert!(first.contains(&format!("{} issue", findings)));
}
