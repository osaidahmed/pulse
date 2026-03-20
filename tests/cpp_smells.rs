mod common;

use common::*;
use std::process::Command;

const LANG: &str = "cpp";

#[test]
fn clean_file_produces_no_output() {
    let output = run_check(LANG, "clean.cpp");
    assert!(output.is_empty(), "got: {}", output);
}

#[test]
fn complex_method_detected() {
    let output = run_check(LANG, "complex_method.cpp");
    assert!(has_smell(&output, "Complex Method"), "got: {}", output);
    assert!(has_function(&output, "process_order"));
}

#[test]
fn complex_method_cc_at_least_9() {
    let debug = run_debug(LANG, "complex_method.cpp");
    let cc = function_metric(&debug, "process_order", "cc").unwrap_or(0);
    assert!(cc >= 9, "cc should be >= 9, got: {}", cc);
}

#[test]
fn excess_args_detected() {
    let output = run_check(LANG, "excess_args.cpp");
    assert!(has_smell(&output, "Excess Arguments"), "got: {}", output);
    assert!(has_function(&output, "create_user"));
}

#[test]
fn simple_func_not_flagged() {
    let output = run_check(LANG, "excess_args.cpp");
    assert!(!has_function(&output, "simple_func"));
}

#[test]
fn constructor_over_injection_detected() {
    let output = run_check(LANG, "excess_args.cpp");
    assert!(has_smell(&output, "Constructor Over-Injection"), "got: {}", output);
    assert!(has_function(&output, "UserService"));
}

#[test]
fn deep_nesting_detected() {
    let output = run_check(LANG, "deep_nesting.cpp");
    assert!(has_smell(&output, "Deep Nested"), "got: {}", output);
    assert!(has_function(&output, "deeply_nested"));
}

#[test]
fn deep_nesting_depth_exceeds_4() {
    let debug = run_debug(LANG, "deep_nesting.cpp");
    let depth = function_metric(&debug, "deeply_nested", "nesting").unwrap_or(0);
    assert!(depth > 4, "got: {}", depth);
}

#[test]
fn moderate_nesting_not_flagged() {
    let output = run_check(LANG, "deep_nesting.cpp");
    assert!(!has_function(&output, "moderately_nested"));
}

#[test]
fn lcom4_detects_low_cohesion() {
    let out = pulse_check_code(concat!(
        "class Sink {\n",
        "public:\n",
        "    void use_a() { a_ = 1; }\n",
        "    int get_a() { return this->a_; }\n",
        "    void use_b() { b_ = 1; }\n",
        "    int get_b() { return this->b_; }\n",
        "    void use_c() { c_ = 1; }\n",
        "    int get_c() { return this->c_; }\n",
        "private:\n",
        "    int a_; int b_; int c_;\n",
        "};\n",
    ), "cpp");
    assert!(has_smell(&out, "Low Cohesion"), "got: {}", out);
}

#[test]
fn cc_base_case_is_1() {
    let debug = run_debug(LANG, "clean.cpp");
    let cc = function_metric(&debug, "add", "cc").unwrap_or(99);
    assert_eq!(cc, 1);
}

#[test]
fn output_starts_with_pulse() {
    let output = run_check(LANG, "complex_method.cpp");
    assert!(output.starts_with("pulse:"));
}

#[test]
fn output_has_function_line_numbers() {
    let output = run_check(LANG, "complex_method.cpp");
    assert!(output.lines().any(|l| l.contains("(L") && l.contains("): ")));
}

#[test]
fn hook_clean_file_silent() {
    let path = fixtures_dir(LANG).join("clean.cpp");
    let output = run_hook(path.to_str().unwrap());
    assert!(output.is_empty());
}

#[test]
fn hook_smelly_file_produces_output() {
    let path = fixtures_dir(LANG).join("complex_method.cpp");
    let output = run_hook(path.to_str().unwrap());
    assert!(!output.is_empty());
}

#[test]
fn hook_nonexistent_file_silent() {
    let output = run_hook("/nonexistent/path/foo.cpp");
    assert!(output.is_empty());
}

#[test]
fn hpp_extension_supported() {
    let out = pulse_check_code("int add(int a, int b) {\n    return a + b;\n}\n", "hpp");
    assert!(out.is_empty());
}

#[test]
fn cc_extension_supported() {
    let out = pulse_check_code("int add(int a, int b) {\n    return a + b;\n}\n", "cc");
    assert!(out.is_empty());
}

#[test]
fn empty_file() {
    let out = pulse_check_code("", "cpp");
    assert!(out.is_empty());
}

#[test]
fn function_at_cc_boundary_flagged() {
    let out = pulse_check_code("void f() {\n    if (a) {}\n    if (b) {}\n    if (c) {}\n    if (d) {}\n    if (e) {}\n    if (f) {}\n    if (g) {}\n    if (h) {}\n}\n", "cpp");
    assert!(has_smell(&out, "Complex Method"), "cc=9 should trigger, got: {}", out);
}

#[test]
fn function_below_cc_boundary_not_flagged() {
    let out = pulse_check_code("void f() {\n    if (a) {}\n    if (b) {}\n    if (c) {}\n    if (d) {}\n    if (e) {}\n    if (f) {}\n    if (g) {}\n}\n", "cpp");
    assert!(!has_smell(&out, "Complex Method"));
}

#[test]
fn issue_count_matches_findings() {
    let output = run_check(LANG, "excess_args.cpp");
    let first = output.lines().next().unwrap_or("");
    let findings = output.lines().filter(|l| l.starts_with("  ")).count();
    assert!(first.contains(&format!("{} issue", findings)));
}
