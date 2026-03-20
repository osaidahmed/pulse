mod common;

use common::*;
use std::process::Command;

const LANG: &str = "javascript";

#[test]
fn clean_file_produces_no_output() {
    let output = run_check(LANG, "clean.js");
    assert!(output.is_empty(), "clean JS file should produce no output, got: {}", output);
}

#[test]
fn complex_method_detected() {
    let output = run_check(LANG, "complex_method.js");
    assert!(has_smell(&output, "Complex Method"), "should detect complex method in JS, got: {}", output);
    assert!(has_function(&output, "processOrder"));
}

#[test]
fn excess_args_detected() {
    let output = run_check(LANG, "excess_args.js");
    assert!(has_smell(&output, "Excess Arguments"), "should detect excess args in JS, got: {}", output);
    assert!(has_function(&output, "createUser"));
}

#[test]
fn constructor_over_injection_detected() {
    let output = run_check(LANG, "excess_args.js");
    assert!(has_smell(&output, "Constructor Over-Injection"), "should detect constructor over-injection in JS, got: {}", output);
}

#[test]
fn simple_func_not_flagged() {
    let output = run_check(LANG, "excess_args.js");
    assert!(!has_function(&output, "simpleFunc"));
}

#[test]
fn primitive_obsession_never_triggers_in_javascript() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("many_args.js");
    std::fs::write(&path, "function f(a, b, c, d, e, f, g, h, i) {\n    return a;\n}\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!has_smell(&stdout, "Primitive Obsession"), "JS has no types — should never trigger, got: {}", stdout);
}

#[test]
fn hook_mode_works_with_js() {
    let path = fixtures_dir(LANG).join("clean.js");
    let output = run_hook(path.to_str().unwrap());
    assert!(output.is_empty());
}

#[test]
fn jsx_extension_supported() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("component.jsx");
    std::fs::write(&path, "function Component() {\n    return null;\n}\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    assert!(out.stdout.is_empty());
    assert!(out.status.success());
}

#[test]
fn mjs_extension_supported() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("module.mjs");
    std::fs::write(&path, "export function add(a, b) {\n    return a + b;\n}\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    assert!(out.stdout.is_empty());
    assert!(out.status.success());
}

#[test]
fn cjs_extension_supported() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("module.cjs");
    std::fs::write(&path, "function add(a, b) {\n    return a + b;\n}\nmodule.exports = { add };\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    assert!(out.stdout.is_empty());
    assert!(out.status.success());
}

#[test]
fn output_starts_with_pulse_prefix() {
    let output = run_check(LANG, "complex_method.js");
    assert!(output.starts_with("pulse:"));
}

// ===========================================================================
// Additional coverage — match Python depth
// ===========================================================================

#[test]
fn complex_method_cc_at_least_9() {
    let debug = run_debug(LANG, "complex_method.js");
    let cc = function_metric(&debug, "processOrder", "cc").unwrap_or(0);
    assert!(cc >= 9, "cc should be >= 9, got: {}", cc);
}

#[test]
fn deep_nesting_detected() {
    let output = run_check(LANG, "deep_nesting.js");
    assert!(has_smell(&output, "Deep Nested"), "got: {}", output);
    assert!(has_function(&output, "deeplyNested"));
}

#[test]
fn deep_nesting_depth_exceeds_4() {
    let debug = run_debug(LANG, "deep_nesting.js");
    let depth = function_metric(&debug, "deeplyNested", "nesting").unwrap_or(0);
    assert!(depth > 4, "got: {}", depth);
}

#[test]
fn moderate_nesting_not_flagged() {
    let output = run_check(LANG, "deep_nesting.js");
    assert!(!has_function(&output, "moderatelyNested"));
}

#[test]
fn embedded_block_detected() {
    let output = run_check(LANG, "embedded_block.js");
    assert!(has_smell(&output, "Large Embedded Block"), "got: {}", output);
    assert!(has_function(&output, "getActiveUsers"));
}

#[test]
fn simple_query_not_flagged() {
    let output = run_check(LANG, "embedded_block.js");
    assert!(!has_function(&output, "simpleQuery"));
}

#[test]
fn code_duplication_detected() {
    let output = run_check(LANG, "code_duplication.js");
    assert!(has_smell(&output, "Code Duplication"), "got: {}", output);
    assert!(has_function(&output, "processUserReport"));
    assert!(has_function(&output, "processAdminReport"));
    assert!(has_function(&output, "processVendorReport"));
}

#[test]
fn excess_args_count_correct() {
    let debug = run_debug(LANG, "excess_args.js");
    let args = function_metric(&debug, "createUser", "args").unwrap_or(0);
    assert_eq!(args, 8, "got: {}", args);
}

#[test]
fn cc_base_case_is_1() {
    let debug = run_debug(LANG, "clean.js");
    let cc = function_metric(&debug, "add", "cc").unwrap_or(99);
    assert_eq!(cc, 1);
}

#[test]
fn output_has_function_line_numbers() {
    let output = run_check(LANG, "complex_method.js");
    let has_loc = output.lines().any(|l| l.contains("(L") && l.contains("): "));
    assert!(has_loc);
}

#[test]
fn output_has_module_prefix() {
    let output = run_check(LANG, "code_duplication.js");
    assert!(output.contains("Module:"));
}

#[test]
fn empty_file() {
    let out = pulse_check_code("", "js");
    assert!(out.is_empty());
}

#[test]
fn comments_only_file() {
    let out = pulse_check_code("// just comments\n// nothing else\n", "js");
    assert!(out.is_empty());
}

#[test]
fn function_at_cc_boundary_flagged() {
    let out = pulse_check_code("function f() {\n    if (a) {}\n    if (b) {}\n    if (c) {}\n    if (d) {}\n    if (e) {}\n    if (f) {}\n    if (g) {}\n    if (h) {}\n}\n", "js");
    assert!(has_smell(&out, "Complex Method"), "cc=9 should trigger, got: {}", out);
}

#[test]
fn function_below_cc_boundary_not_flagged() {
    let out = pulse_check_code("function f() {\n    if (a) {}\n    if (b) {}\n    if (c) {}\n    if (d) {}\n    if (e) {}\n    if (f) {}\n    if (g) {}\n}\n", "js");
    assert!(!has_smell(&out, "Complex Method"));
}

#[test]
fn hook_nonexistent_file_silent() {
    let output = run_hook("/nonexistent/path/foo.js");
    assert!(output.is_empty());
}

#[test]
fn hook_smelly_file_produces_output() {
    let path = fixtures_dir(LANG).join("complex_method.js");
    let output = run_hook(path.to_str().unwrap());
    assert!(!output.is_empty());
}
