mod common;

use common::*;
use std::process::Command;

const LANG: &str = "d";

// ===========================================================================
// Output format
// ===========================================================================

#[test]
fn output_starts_with_pulse() {
    let output = run_check(LANG, "complex_methods.d");
    assert!(output.starts_with("pulse:"), "got: {}", output);
}

#[test]
fn output_has_function_line_numbers() {
    let output = run_check(LANG, "complex_methods.d");
    assert!(output
        .lines()
        .any(|l| l.contains("(L") && l.contains("): ")));
}

#[test]
fn output_has_module_prefix() {
    let output = run_check(LANG, "production_service.d");
    assert!(output.contains("Module:"), "got: {}", output);
}

#[test]
fn issue_count_matches_findings() {
    let output = run_check(LANG, "complex_methods.d");
    let first = output.lines().next().unwrap_or("");
    let findings = output.lines().filter(|l| l.starts_with("  ")).count();
    assert!(first.contains(&format!("{} issue", findings)));
}

// ===========================================================================
// Clean / empty
// ===========================================================================

#[test]
fn clean_file_produces_no_output() {
    let output = run_check(LANG, "clean.d");
    assert!(output.is_empty(), "got: {}", output);
}

#[test]
fn empty_file() {
    let out = pulse_check_code("", "d");
    assert!(out.is_empty());
}

#[test]
fn comments_only_file() {
    let out = pulse_check_code("// just a comment\n/* block */\n/+ nested +/\n", "d");
    assert!(out.is_empty());
}

#[test]
fn simple_function_not_flagged() {
    let out = pulse_check_code(
        "int add(int a, int b) {\n    return a + b;\n}\n",
        "d",
    );
    assert!(out.is_empty(), "got: {}", out);
}

// ===========================================================================
// CC boundary
// ===========================================================================

#[test]
fn cc_base_case_is_1() {
    let debug = pulse_debug_code(
        "int add(int a, int b) {\n    return a + b;\n}\n",
        "d",
    );
    let cc = function_metric(&debug, "add", "cc").unwrap_or(99);
    assert_eq!(cc, 1);
}

#[test]
fn function_at_cc_boundary_flagged() {
    let out = pulse_check_code(
        concat!(
            "int f(int a, int b, int c, int d, int e, int g, int h, int i) {\n",
            "    if (a > 0) {}\n",
            "    if (b > 0) {}\n",
            "    if (c > 0) {}\n",
            "    if (d > 0) {}\n",
            "    if (e > 0) {}\n",
            "    if (g > 0) {}\n",
            "    if (h > 0) {}\n",
            "    if (i > 0) {}\n",
            "    return 0;\n",
            "}\n",
        ),
        "d",
    );
    assert!(
        has_smell(&out, "Complex Method"),
        "cc=9 should trigger, got: {}",
        out
    );
}

#[test]
fn function_below_cc_boundary_not_flagged() {
    let out = pulse_check_code(
        concat!(
            "int f(int a, int b, int c, int d, int e, int g, int h) {\n",
            "    if (a > 0) {}\n",
            "    if (b > 0) {}\n",
            "    if (c > 0) {}\n",
            "    if (d > 0) {}\n",
            "    if (e > 0) {}\n",
            "    if (g > 0) {}\n",
            "    if (h > 0) {}\n",
            "    return 0;\n",
            "}\n",
        ),
        "d",
    );
    assert!(!has_smell(&out, "Complex Method"));
}

#[test]
fn boolean_operators_increment_cc() {
    let debug = pulse_debug_code(
        "bool f(bool a, bool b, bool c) {\n    if (a && b && c) {\n        return true;\n    }\n    return false;\n}\n",
        "d",
    );
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert!(cc >= 4, "got: {}", cc);
}

// ===========================================================================
// Complexity smells
// ===========================================================================

#[test]
fn complex_method_detected() {
    let output = run_check(LANG, "complex_methods.d");
    assert!(has_smell(&output, "Complex Method"), "got: {}", output);
    assert!(has_function(&output, "processOrder"));
}

#[test]
fn complex_method_cc_at_least_9() {
    let debug = run_debug(LANG, "complex_methods.d");
    let cc = function_metric(&debug, "processOrder", "cc").unwrap_or(0);
    assert!(cc >= 9, "cc should be >= 9, got: {}", cc);
}

#[test]
fn god_method_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("god.d");
    let mut code = String::from("int processDataPipeline(int x) {\n");
    for i in 0..cc_branches() {
        code.push_str(&format!("    if (x > {}) {{}}\n", i));
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("    int y{} = {};\n", i, i));
    }
    code.push_str("    return 0;\n}\n");
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(has_smell(&stdout, "God Method"), "got: {}", stdout);
}

#[test]
fn god_method_not_reported_as_separate() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("god.d");
    let mut code = String::from("int processDataPipeline(int x) {\n");
    for i in 0..cc_branches() {
        code.push_str(&format!("    if (x > {}) {{}}\n", i));
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("    int y{} = {};\n", i, i));
    }
    code.push_str("    return 0;\n}\n");
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(has_smell(&stdout, "God Method"));
    assert!(!has_smell(&stdout, "Complex Method"), "should suppress Complex when God is reported");
    assert!(!has_smell(&stdout, "Large Method"), "should suppress Large when God is reported");
}

#[test]
fn large_method_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large.d");
    let mut code = String::from("int bigFunc() {\n");
    for i in 0..fn_padding() {
        code.push_str(&format!("    int x{} = {};\n", i, i));
    }
    code.push_str("    return 0;\n}\n");
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(has_smell(&stdout, "Large Method"), "got: {}", stdout);
}

// ===========================================================================
// Nesting
// ===========================================================================

#[test]
fn deep_nesting_detected() {
    let output = run_check(LANG, "deep_nesting.d");
    assert!(has_smell(&output, "Deep Nested Complexity"), "got: {}", output);
    assert!(has_function(&output, "deeplyNested"));
}

#[test]
fn deep_nesting_depth_exceeds_4() {
    let debug = run_debug(LANG, "deep_nesting.d");
    let n = function_metric(&debug, "deeplyNested", "nesting").unwrap_or(0);
    assert!(n >= 4, "nesting should be >= 4, got: {}", n);
}

#[test]
fn moderate_nesting_not_flagged() {
    let output = run_check(LANG, "deep_nesting.d");
    assert!(!has_function(&output, "moderatelyNested"), "moderate should not be flagged");
}

// ===========================================================================
// Arguments
// ===========================================================================

#[test]
fn excess_args_detected() {
    let output = run_check(LANG, "excess_args.d");
    assert!(has_smell(&output, "Excess Arguments"), "got: {}", output);
    assert!(has_function(&output, "createRecord"));
}

#[test]
fn excess_args_count_correct() {
    let debug = run_debug(LANG, "excess_args.d");
    let args = function_metric(&debug, "createRecord", "args").unwrap_or(0);
    assert!(args >= 6, "args should be >= 6, got: {}", args);
}

// ===========================================================================
// Module-level
// ===========================================================================

#[test]
fn code_duplication_detected() {
    let output = run_check(LANG, "code_duplication.d");
    assert!(has_smell(&output, "Code Duplication"), "got: {}", output);
}

#[test]
fn embedded_block_detected() {
    let output = run_check(LANG, "embedded_block.d");
    assert!(has_smell(&output, "Large Embedded Block"), "got: {}", output);
}

#[test]
fn bumpy_road_detected() {
    let output = run_check(LANG, "bumpy_road.d");
    assert!(
        has_smell(&output, "Nested Conditional Chunks") || has_smell(&output, "Deep Nested"),
        "got: {}",
        output
    );
}

#[test]
fn low_cohesion_detected() {
    let output = run_check(LANG, "low_cohesion.d");
    assert!(
        has_smell(&output, "Low Cohesion") || has_smell(&output, "Code Duplication"),
        "got: {}",
        output
    );
}

#[test]
fn primitive_obsession_detected() {
    let output = run_check(LANG, "primitive_obsession.d");
    assert!(has_smell(&output, "Primitive Obsession"), "got: {}", output);
}

#[test]
fn overall_function_size_at_threshold() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("overall.d");
    let mut code = String::new();
    for f in 0..t().large_fn_count + 1 {
        code.push_str(&format!("int func{}() {{\n", f));
        for i in 0..t().large_fn_loc + 5 {
            code.push_str(&format!("    int x{} = {};\n", i, i));
        }
        code.push_str("    return 0;\n}\n\n");
    }
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(has_smell(&stdout, "Overall Function Size"), "got: {}", stdout);
}

// ===========================================================================
// Hook
// ===========================================================================

#[test]
fn hook_clean_file_silent() {
    let dir = fixtures_dir(LANG);
    let path = dir.join("clean.d");
    let output = run_hook(path.to_str().unwrap());
    assert!(output.is_empty(), "got: {}", output);
}

#[test]
fn hook_smelly_file_produces_output() {
    let dir = fixtures_dir(LANG);
    let path = dir.join("complex_methods.d");
    let output = run_hook(path.to_str().unwrap());
    assert!(!output.is_empty(), "hook should find smells");
}

#[test]
fn hook_nonexistent_file_silent() {
    let output = run_hook("/tmp/nonexistent_file_d.d");
    assert!(output.is_empty());
}

// ===========================================================================
// D-specific
// ===========================================================================

#[test]
fn foreach_increments_cc() {
    let debug = pulse_debug_code(
        "void f(int[] data) {\n    foreach (item; data) {}\n}\n",
        "d",
    );
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert_eq!(cc, 2);
}

#[test]
fn switch_case_increments_cc() {
    let debug = pulse_debug_code(
        "int f(int x) {\n    switch (x) {\n        case 1: return 1;\n        case 2: return 2;\n        default: return 0;\n    }\n}\n",
        "d",
    );
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert_eq!(cc, 3, "base 1 + 2 cases, got: {}", cc);
}

#[test]
fn try_catch_increments_cc() {
    let debug = pulse_debug_code(
        "void f() {\n    try {\n        int x = 1;\n    } catch (Exception e) {\n        int y = 2;\n    }\n}\n",
        "d",
    );
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert_eq!(cc, 2);
}

#[test]
fn class_method_name_prefixed() {
    let debug = pulse_debug_code(
        "class Foo {\n    void bar() {\n        int x = 1;\n    }\n}\n",
        "d",
    );
    assert!(debug.contains("Foo.bar"), "got: {}", debug);
}

#[test]
fn struct_method_name_prefixed() {
    let debug = pulse_debug_code(
        "struct Point {\n    void draw() {\n        int x = 1;\n    }\n}\n",
        "d",
    );
    assert!(debug.contains("Point.draw"), "got: {}", debug);
}

#[test]
fn constructor_detected() {
    let debug = pulse_debug_code(
        "class Foo {\n    this(int x) {\n        int y = x;\n    }\n}\n",
        "d",
    );
    assert!(debug.contains("Foo.this"), "got: {}", debug);
}

#[test]
fn destructor_detected() {
    let debug = pulse_debug_code(
        "class Foo {\n    ~this() {\n        int x = 0;\n    }\n}\n",
        "d",
    );
    assert!(debug.contains("Foo.~this"), "got: {}", debug);
}

#[test]
fn unittest_block_detected() {
    let debug = pulse_debug_code(
        "unittest {\n    int x = 1;\n}\n",
        "d",
    );
    assert!(debug.contains("unittest_L"), "got: {}", debug);
}

#[test]
fn scope_guard_no_cc() {
    let debug = pulse_debug_code(
        "void f() {\n    scope(exit) int x = 1;\n    int y = 2;\n}\n",
        "d",
    );
    let cc = function_metric(&debug, "f", "cc").unwrap_or(99);
    assert_eq!(cc, 1, "scope guard should not add cc, got: {}", cc);
}

#[test]
fn do_while_increments_cc() {
    let debug = pulse_debug_code(
        "void f(int x) {\n    do {\n        x = x + 1;\n    } while (x < 10);\n}\n",
        "d",
    );
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert_eq!(cc, 2);
}

#[test]
fn while_increments_cc() {
    let debug = pulse_debug_code(
        "void f(int x) {\n    while (x > 0) {\n        x = x - 1;\n    }\n}\n",
        "d",
    );
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert_eq!(cc, 2);
}

// ===========================================================================
// Other
// ===========================================================================

#[test]
fn constructor_over_injection_detected() {
    let output = run_check(LANG, "production_service.d");
    assert!(has_smell(&output, "Constructor Over-Injection"), "got: {}", output);
}

#[test]
fn test_file_analyzed() {
    let output = run_check(LANG, "test_smells.d");
    let debug = run_debug(LANG, "test_smells.d");
    assert!(debug.contains("unittest_L"), "unittest blocks should be parsed");
}

#[test]
fn analysis_completes_under_500ms() {
    // Warm up
    let _ = run_check(LANG, "clean.d");
    let start = std::time::Instant::now();
    let _ = run_check(LANG, "production_service.d");
    assert!(start.elapsed().as_millis() < 500);
}

#[test]
fn empty_catch_detected() {
    let out = pulse_check_code(
        "void f() {\n    try {\n        int x = 1;\n    } catch (Exception e) {\n    }\n}\n",
        "d",
    );
    assert!(has_smell(&out, "Empty Error Handler"), "got: {}", out);
}

#[test]
fn global_conditionals_parsed() {
    let debug = run_debug(LANG, "clean.d");
    assert!(debug.contains("functions"), "debug should show module metrics");
}
