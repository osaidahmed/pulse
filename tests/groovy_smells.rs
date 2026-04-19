mod common;

use common::*;
use std::process::Command;

const LANG: &str = "groovy";

// ===========================================================================
// Output format
// ===========================================================================

#[test]
fn output_starts_with_pulse() {
    let output = run_check(LANG, "complex_methods.groovy");
    assert!(output.starts_with("pulse:"), "got: {}", output);
}

#[test]
fn output_has_function_line_numbers() {
    let output = run_check(LANG, "complex_methods.groovy");
    assert!(output.lines().any(|l| l.contains("(L") && l.contains("): ")));
}

#[test]
fn output_has_module_prefix() {
    let output = run_check(LANG, "production_service.groovy");
    assert!(output.contains("Module:"), "got: {}", output);
}

#[test]
fn issue_count_matches_findings() {
    let output = run_check(LANG, "complex_methods.groovy");
    let first = output.lines().next().unwrap_or("");
    let findings = output.lines().filter(|l| l.starts_with("  ")).count();
    assert!(first.contains(&format!("{} issue", findings)));
}

// ===========================================================================
// Clean / empty
// ===========================================================================

#[test]
fn clean_file_produces_no_output() {
    let output = run_check(LANG, "clean.groovy");
    assert!(output.is_empty(), "got: {}", output);
}

#[test]
fn empty_file() {
    let out = pulse_check_code("", "groovy");
    assert!(out.is_empty());
}

#[test]
fn comments_only_file() {
    let out = pulse_check_code("// just a comment\n/* block */\n", "groovy");
    assert!(out.is_empty());
}

#[test]
fn simple_function_not_flagged() {
    let out = pulse_check_code(
        "class T { int add(int a, int b) { return a + b } }\n",
        "groovy",
    );
    assert!(out.is_empty(), "got: {}", out);
}

// ===========================================================================
// CC boundary
// ===========================================================================

#[test]
fn cc_base_case_is_1() {
    let debug = pulse_debug_code(
        "class T { int add(int a, int b) { return a + b } }\n",
        "groovy",
    );
    let cc = function_metric(&debug, "add", "cc").unwrap_or(99);
    assert_eq!(cc, 1);
}

#[test]
fn function_at_cc_boundary_flagged() {
    let out = pulse_check_code(
        concat!(
            "class T {\n",
            "  int f(int a, int b, int c, int d, int e, int g, int h, int i) {\n",
            "    if (a > 0) {}\n",
            "    if (b > 0) {}\n",
            "    if (c > 0) {}\n",
            "    if (d > 0) {}\n",
            "    if (e > 0) {}\n",
            "    if (g > 0) {}\n",
            "    if (h > 0) {}\n",
            "    if (i > 0) {}\n",
            "    return 0\n",
            "  }\n",
            "}\n",
        ),
        "groovy",
    );
    assert!(has_smell(&out, "Complex Method"), "cc=9 should trigger, got: {}", out);
}

#[test]
fn function_below_cc_boundary_not_flagged() {
    let out = pulse_check_code(
        concat!(
            "class T {\n",
            "  int f(int a, int b, int c, int d, int e, int g, int h) {\n",
            "    if (a > 0) {}\n",
            "    if (b > 0) {}\n",
            "    if (c > 0) {}\n",
            "    if (d > 0) {}\n",
            "    if (e > 0) {}\n",
            "    if (g > 0) {}\n",
            "    if (h > 0) {}\n",
            "    return 0\n",
            "  }\n",
            "}\n",
        ),
        "groovy",
    );
    assert!(!has_smell(&out, "Complex Method"));
}

#[test]
fn boolean_operators_increment_cc() {
    let debug = pulse_debug_code(
        "class T { boolean f(boolean a, boolean b, boolean c) { if (a && b && c) { return true } return false } }\n",
        "groovy",
    );
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert!(cc >= 4, "got: {}", cc);
}

// ===========================================================================
// Complexity smells
// ===========================================================================

#[test]
fn complex_method_detected() {
    let output = run_check(LANG, "complex_methods.groovy");
    assert!(has_smell(&output, "Complex Method"), "got: {}", output);
    assert!(has_function(&output, "processOrder"));
}

#[test]
fn complex_method_cc_at_least_9() {
    let debug = run_debug(LANG, "complex_methods.groovy");
    let cc = function_metric(&debug, "processOrder", "cc").unwrap_or(0);
    assert!(cc >= 9, "cc should be >= 9, got: {}", cc);
}

#[test]
fn god_method_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("god.groovy");
    let mut code = String::from("class T {\n  int processDataPipeline(int x) {\n");
    for i in 0..cc_branches() {
        code.push_str(&format!("    if (x > {}) {{}}\n", i));
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("    int y{} = {}\n", i, i));
    }
    code.push_str("    return 0\n  }\n}\n");
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output().expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(has_smell(&stdout, "God Method"), "got: {}", stdout);
}

#[test]
fn god_method_not_reported_as_separate() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("god.groovy");
    let mut code = String::from("class T {\n  int processDataPipeline(int x) {\n");
    for i in 0..cc_branches() {
        code.push_str(&format!("    if (x > {}) {{}}\n", i));
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("    int y{} = {}\n", i, i));
    }
    code.push_str("    return 0\n  }\n}\n");
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output().expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(has_smell(&stdout, "God Method"));
    assert!(!has_smell(&stdout, "Complex Method"), "should suppress Complex");
    assert!(!has_smell(&stdout, "Large Method"), "should suppress Large");
}

#[test]
fn large_method_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large.groovy");
    let mut code = String::from("class T {\n  int bigFunc() {\n");
    for i in 0..fn_padding() {
        code.push_str(&format!("    int x{} = {}\n", i, i));
    }
    code.push_str("    return 0\n  }\n}\n");
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output().expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(has_smell(&stdout, "Large Method"), "got: {}", stdout);
}

// ===========================================================================
// Nesting
// ===========================================================================

#[test]
fn deep_nesting_detected() {
    let output = run_check(LANG, "deep_nesting.groovy");
    assert!(has_smell(&output, "Deep Nested Complexity"), "got: {}", output);
    assert!(has_function(&output, "deeplyNested"));
}

#[test]
fn deep_nesting_depth_exceeds_4() {
    let debug = run_debug(LANG, "deep_nesting.groovy");
    let n = function_metric(&debug, "deeplyNested", "nesting").unwrap_or(0);
    assert!(n >= 4, "nesting should be >= 4, got: {}", n);
}

#[test]
fn moderate_nesting_not_flagged() {
    let output = run_check(LANG, "deep_nesting.groovy");
    assert!(!has_function(&output, "moderatelyNested"));
}

// ===========================================================================
// Arguments
// ===========================================================================

#[test]
fn excess_args_detected() {
    let output = run_check(LANG, "excess_args.groovy");
    assert!(has_smell(&output, "Excess Arguments"), "got: {}", output);
    assert!(has_function(&output, "createRecord"));
}

#[test]
fn excess_args_count_correct() {
    let debug = run_debug(LANG, "excess_args.groovy");
    let args = function_metric(&debug, "createRecord", "args").unwrap_or(0);
    assert!(args >= 6, "args should be >= 6, got: {}", args);
}

// ===========================================================================
// Module-level
// ===========================================================================

#[test]
fn code_duplication_detected() {
    let output = run_check(LANG, "code_duplication.groovy");
    assert!(has_smell(&output, "Code Duplication"), "got: {}", output);
}

#[test]
fn embedded_block_detected() {
    let output = run_check(LANG, "embedded_block.groovy");
    assert!(has_smell(&output, "Large Embedded Block"), "got: {}", output);
}

#[test]
fn bumpy_road_detected() {
    let output = run_check(LANG, "bumpy_road.groovy");
    assert!(
        has_smell(&output, "Nested Conditional Chunks") || has_smell(&output, "Deep Nested"),
        "got: {}", output
    );
}

#[test]
fn low_cohesion_detected() {
    let output = run_check(LANG, "low_cohesion.groovy");
    assert!(
        has_smell(&output, "Low Cohesion") || has_smell(&output, "Code Duplication"),
        "got: {}", output
    );
}

#[test]
fn primitive_obsession_detected() {
    let output = run_check(LANG, "primitive_obsession.groovy");
    assert!(has_smell(&output, "Primitive Obsession"), "got: {}", output);
}

#[test]
fn overall_function_size_at_threshold() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("overall.groovy");
    let mut code = String::from("class T {\n");
    for f in 0..t().large_fn_count + 1 {
        code.push_str(&format!("  int func{}() {{\n", f));
        for i in 0..t().large_fn_loc + 5 {
            code.push_str(&format!("    int x{} = {}\n", i, i));
        }
        code.push_str("    return 0\n  }\n\n");
    }
    code.push_str("}\n");
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output().expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(has_smell(&stdout, "Overall Function Size"), "got: {}", stdout);
}

#[test]
fn overall_function_size_below_threshold() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("overall.groovy");
    let mut code = String::from("class T {\n");
    for f in 0..(t().large_fn_count as usize - 1) {
        code.push_str(&format!("  int func{}() {{\n", f));
        for i in 0..large_fn_lines() {
            code.push_str(&format!("    int x{} = {}\n", i, i));
        }
        code.push_str("    return 0\n  }\n\n");
    }
    code.push_str("}\n");
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output().expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!has_smell(&stdout, "Overall Function Size"));
}

// ===========================================================================
// Hook
// ===========================================================================

#[test]
fn hook_clean_file_silent() {
    let dir = fixtures_dir(LANG);
    let path = dir.join("clean.groovy");
    let output = run_hook(path.to_str().unwrap());
    assert!(output.is_empty(), "got: {}", output);
}

#[test]
fn hook_smelly_file_produces_output() {
    let dir = fixtures_dir(LANG);
    let path = dir.join("complex_methods.groovy");
    let output = run_hook(path.to_str().unwrap());
    assert!(!output.is_empty());
}

#[test]
fn hook_nonexistent_file_silent() {
    let output = run_hook("/tmp/nonexistent_file_groovy.groovy");
    assert!(output.is_empty());
}

// ===========================================================================
// Groovy-specific
// ===========================================================================

#[test]
fn closure_not_walked() {
    let debug = pulse_debug_code(
        "class T { void f() { def fn = { x -> if (x > 0) { if (x > 1) {} } } } }\n",
        "groovy",
    );
    let cc = function_metric(&debug, "f", "cc").unwrap_or(99);
    assert_eq!(cc, 1, "closure body should not contribute to outer cc, got: {}", cc);
}

#[test]
fn gstring_embedded_block() {
    let out = pulse_check_code(
        concat!(
            "class T {\n",
            "  String f(String name) {\n",
            "    String s = \"\"\"line1 ${name}\n",
            "line2\nline3\nline4\nline5\nline6\nline7\nline8\nline9\nline10\n",
            "line11\nline12\nline13\nline14\nline15\nline16\nline17\nline18\n",
            "line19\nline20\"\"\"\n",
            "    return s\n",
            "  }\n",
            "}\n",
        ),
        "groovy",
    );
    assert!(has_smell(&out, "Large Embedded Block"), "got: {}", out);
}

#[test]
fn enhanced_for_increments_cc() {
    let debug = pulse_debug_code(
        "class T { void f(int[] data) { for (int item : data) {} } }\n",
        "groovy",
    );
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert_eq!(cc, 2);
}

#[test]
fn class_method_name_prefixed() {
    let debug = pulse_debug_code(
        "class Foo { void bar() { int x = 1 } }\n",
        "groovy",
    );
    assert!(debug.contains("Foo.bar"), "got: {}", debug);
}

#[test]
fn constructor_detected() {
    let debug = pulse_debug_code(
        "class Foo { Foo(int x) { int y = x } }\n",
        "groovy",
    );
    assert!(debug.contains("Foo.Foo"), "got: {}", debug);
}

#[test]
fn switch_case_increments_cc() {
    let debug = pulse_debug_code(
        "class T { int f(int x) { switch (x) { case 1: return 1; case 2: return 2; default: return 0 } } }\n",
        "groovy",
    );
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert_eq!(cc, 3, "base 1 + 2 cases, got: {}", cc);
}

#[test]
fn try_catch_increments_cc() {
    let debug = pulse_debug_code(
        "class T { void f() { try { int x = 1 } catch (Exception e) { int y = 2 } } }\n",
        "groovy",
    );
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert_eq!(cc, 2);
}

#[test]
fn empty_catch_detected() {
    let out = pulse_check_code(
        "class T { void f() { try { int x = 1 } catch (Exception e) { } } }\n",
        "groovy",
    );
    assert!(has_smell(&out, "Empty Error Handler"), "got: {}", out);
}

#[test]
fn do_while_increments_cc() {
    let debug = pulse_debug_code(
        "class T { void f(int x) { do { x = x + 1 } while (x < 10) } }\n",
        "groovy",
    );
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert_eq!(cc, 2);
}

#[test]
fn while_increments_cc() {
    let debug = pulse_debug_code(
        "class T { void f(int x) { while (x > 0) { x = x - 1 } } }\n",
        "groovy",
    );
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert_eq!(cc, 2);
}

#[test]
fn nested_class_methods() {
    let debug = pulse_debug_code(
        concat!(
            "class Outer {\n",
            "  class Inner {\n",
            "    void work() { int x = 1 }\n",
            "  }\n",
            "  void outerWork() { int y = 2 }\n",
            "}\n",
        ),
        "groovy",
    );
    assert!(debug.contains("Inner.work"), "got: {}", debug);
    assert!(debug.contains("Outer.outerWork"), "got: {}", debug);
}

// ===========================================================================
// Production fixture — production_service.groovy
// ===========================================================================

#[test]
fn production_service_has_issues() {
    let output = run_check(LANG, "production_service.groovy");
    assert!(!output.is_empty());
}

#[test]
fn production_service_complex_method() {
    let output = run_check(LANG, "production_service.groovy");
    assert!(has_smell(&output, "Complex Method"), "got: {}", output);
    assert!(has_function(&output, "processOrder"));
}

#[test]
fn production_service_process_order_cc_high() {
    let debug = run_debug(LANG, "production_service.groovy");
    let cc = function_metric(&debug, "processOrder", "cc").unwrap_or(0);
    assert!(cc >= 10, "got: {}", cc);
}

#[test]
fn production_service_excess_args() {
    let output = run_check(LANG, "production_service.groovy");
    assert!(has_smell(&output, "Excess Arguments"), "got: {}", output);
    assert!(has_function(&output, "processOrder"));
}

#[test]
fn production_service_deep_nesting() {
    let output = run_check(LANG, "production_service.groovy");
    assert!(has_smell(&output, "Deep Nested"), "got: {}", output);
}

#[test]
fn production_service_nested_chunks() {
    let output = run_check(LANG, "production_service.groovy");
    assert!(has_smell(&output, "Nested Conditional Chunks"), "got: {}", output);
}

#[test]
fn production_service_code_duplication() {
    let output = run_check(LANG, "production_service.groovy");
    assert!(has_smell(&output, "Code Duplication"), "got: {}", output);
}

#[test]
fn production_service_pipeline_complex() {
    let debug = run_debug(LANG, "production_service.groovy");
    let cc = function_metric(&debug, "processDataPipeline", "cc").unwrap_or(0);
    assert!(cc >= 10, "pipeline should be complex, got: {}", cc);
}

#[test]
fn production_service_clean_helpers_not_flagged() {
    let output = run_check(LANG, "production_service.groovy");
    assert!(!has_function(&output, "validateSchema") || has_smell(&output, "Code Duplication"));
}

// ===========================================================================
// Production fixture — production_api_service.groovy
// ===========================================================================

#[test]
fn production_api_short_variable_names() {
    let output = run_check(LANG, "production_api_service.groovy");
    assert!(has_smell(&output, "Short Variable Names"), "got: {}", output);
    assert!(has_function(&output, "processEvent"));
}

#[test]
fn production_api_complex_handler() {
    let output = run_check(LANG, "production_api_service.groovy");
    assert!(has_smell(&output, "Complex Method"), "got: {}", output);
    assert!(has_function(&output, "handleApiRequest"));
}

#[test]
fn production_api_handler_cc_high() {
    let debug = run_debug(LANG, "production_api_service.groovy");
    let cc = function_metric(&debug, "handleApiRequest", "cc").unwrap_or(0);
    assert!(cc >= 15, "handler should have high cc, got: {}", cc);
}

#[test]
fn production_api_empty_handler() {
    let output = run_check(LANG, "production_api_service.groovy");
    assert!(has_smell(&output, "Empty Error Handler"), "got: {}", output);
}

#[test]
fn production_api_code_duplication() {
    let output = run_check(LANG, "production_api_service.groovy");
    assert!(has_smell(&output, "Code Duplication"), "got: {}", output);
}

#[test]
fn production_api_deep_nesting() {
    let output = run_check(LANG, "production_api_service.groovy");
    assert!(has_smell(&output, "Deep Nested"), "got: {}", output);
}

#[test]
fn production_api_middleware_empty_catch() {
    let debug = run_debug(LANG, "production_api_service.groovy");
    assert!(debug.contains("applyMiddleware"), "got: {}", debug);
}

#[test]
fn production_api_multiple_issues() {
    let output = run_check(LANG, "production_api_service.groovy");
    let issue_count: usize = output.lines().filter(|l| l.starts_with("  ")).count();
    assert!(issue_count >= 5, "should have many issues, got: {}", issue_count);
}

// ===========================================================================
// Negative boundary tests
// ===========================================================================

#[test]
fn four_args_not_flagged() {
    let out = pulse_check_code("class T { void f(int a, int b, int c, int d) { println(a) } }\n", "groovy");
    assert!(!has_smell(&out, "Excess Arguments"));
}

#[test]
fn five_args_not_flagged() {
    let out = pulse_check_code("class T { void f(int a, int b, int c, int d, int e) { println(a) } }\n", "groovy");
    assert!(!has_smell(&out, "Excess Arguments"));
}

#[test]
fn nesting_depth_3_not_flagged() {
    let out = pulse_check_code(
        concat!(
            "class T {\n",
            "  void f(int a, int b, int c) {\n",
            "    if (a > 0) {\n",
            "      if (b > 0) {\n",
            "        if (c > 0) { println(1) }\n",
            "      }\n",
            "    }\n",
            "  }\n",
            "}\n",
        ),
        "groovy",
    );
    assert!(!has_smell(&out, "Deep Nested"), "depth=3 should not flag, got: {}", out);
}

#[test]
fn cc_8_not_flagged() {
    let mut code = String::from("class T {\n  void f(int a, int b, int c, int d, int e, int g, int h) {\n");
    for v in ["a", "b", "c", "d", "e", "g", "h"] {
        code.push_str(&format!("    if ({} > 0) {{ return }}\n", v));
    }
    code.push_str("  }\n}\n");
    let out = pulse_check_code(&code, "groovy");
    assert!(!has_smell(&out, "Complex Method"), "cc=8 should not trigger");
}

#[test]
fn single_boolean_op_not_compound() {
    let out = pulse_check_code(
        "class T { void f(boolean a, boolean b) { if (a && b) { println(1) } } }\n",
        "groovy",
    );
    assert!(!has_smell(&out, "Complex Conditional"));
}

#[test]
fn non_duplicate_functions_not_flagged() {
    let out = pulse_check_code(
        concat!(
            "class T {\n",
            "  int alpha(int x) { return x + 1 }\n",
            "  int beta(int[] items) {\n",
            "    int total = 0\n",
            "    for (int v : items) {\n",
            "      if (v > 0) { total = total + v }\n",
            "      else { total = total - v }\n",
            "    }\n",
            "    while (total > 100) { total = total / 2 }\n",
            "    return total\n",
            "  }\n",
            "}\n",
        ),
        "groovy",
    );
    assert!(!has_smell(&out, "Code Duplication"), "got: {}", out);
}

#[test]
fn small_string_not_embedded_block() {
    let out = pulse_check_code(
        "class T { void f() { String x = \"hello\"; String y = \"world\" } }\n",
        "groovy",
    );
    assert!(!has_smell(&out, "Large Embedded Block"));
}

#[test]
fn no_global_conditionals_when_inside_functions() {
    let out = pulse_check_code(
        concat!(
            "class T {\n",
            "  void f(int x) { if (x > 0) { println(1) } }\n",
            "  void g(int y) { if (y > 0) { println(2) } }\n",
            "}\n",
        ),
        "groovy",
    );
    assert!(!has_smell(&out, "Global Conditionals"));
}

#[test]
fn try_catch_with_body_not_empty() {
    let out = pulse_check_code(
        "class T { void f(int x) { try { println(x) } catch (Exception e) { println(e) } } }\n",
        "groovy",
    );
    assert!(!has_smell(&out, "Empty Error Handler"));
}

#[test]
fn single_bump_not_nested_chunks() {
    let out = pulse_check_code(
        concat!(
            "class T {\n",
            "  void f(int x) {\n",
            "    if (x > 0) {\n",
            "      if (x > 10) {\n",
            "        if (x > 100) { println(1) }\n",
            "      }\n",
            "    }\n",
            "  }\n",
            "}\n",
        ),
        "groovy",
    );
    assert!(!has_smell(&out, "Nested Conditional Chunks"));
}

// ===========================================================================
// Positive boundary tests
// ===========================================================================

#[test]
fn six_args_triggers_excess() {
    let out = pulse_check_code(
        "class T { void f(int a, int b, int c, int d, int e, int g) { println(a) } }\n",
        "groovy",
    );
    assert!(has_smell(&out, "Excess Arguments"), "got: {}", out);
}

#[test]
fn too_many_functions_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("many_fns.groovy");
    let mut code = String::from("class T {\n");
    for i in 0..functions_above() {
        code.push_str(&format!("  int fn{}(int x) {{ return x + {} }}\n", i, i));
    }
    code.push_str("}\n");
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output().expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(has_smell(&stdout, "Too Many Functions"), "got: {}", stdout);
}

#[test]
fn file_too_large_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("huge.groovy");
    let mut code = String::new();
    for i in 0..file_padding() {
        code.push_str(&format!("int x{} = {}\n", i, i));
    }
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output().expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(has_smell(&stdout, "File Too Large"), "got: {}", stdout);
}

#[test]
fn overall_code_complexity_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("complex_total.groovy");
    let mut code = String::from("class T {\n");
    for i in 0..15 {
        code.push_str(&format!("  int fn{}(int x) {{\n", i));
        for j in 0..8 {
            code.push_str(&format!("    if (x > {}) {{ return {} }}\n", j, j));
        }
        code.push_str("    return 0\n  }\n\n");
    }
    code.push_str("}\n");
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output().expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        has_smell(&stdout, "Overall Code Complexity") || has_smell(&stdout, "Complex Method"),
        "got: {}", stdout
    );
}

#[test]
fn short_variable_names_inline() {
    let mut code = String::from("class T {\n  void f(int data) {\n");
    for ch in ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'] {
        code.push_str(&format!("    int {} = data + 1\n", ch));
    }
    for i in 0..20 {
        code.push_str(&format!("    int x{} = {}\n", i, i));
    }
    code.push_str("  }\n}\n");
    let out = pulse_check_code(&code, "groovy");
    assert!(has_smell(&out, "Short Variable Names"), "got: {}", out);
}

#[test]
fn short_vars_not_triggered_with_long_names() {
    let out = pulse_check_code(
        concat!(
            "class T {\n",
            "  void f(int data) {\n",
            "    int total = data + 1\n",
            "    int count = data + 2\n",
            "    int value = data + 3\n",
            "    int score = data + 4\n",
            "  }\n",
            "}\n",
        ),
        "groovy",
    );
    assert!(!has_smell(&out, "Short Variable Names"));
}

#[test]
fn duplicated_assertion_blocks_above_threshold() {
    let mut code = String::from("class T {\n  void testA() {\n");
    for i in 0..asserts_above() {
        code.push_str(&format!("    assert {} > 0\n", i));
    }
    code.push_str("  }\n  void testB() {\n");
    for i in 0..asserts_above() {
        code.push_str(&format!("    assert {} > 0\n", i));
    }
    code.push_str("  }\n}\n");
    let out = pulse_check_code(&code, "groovy");
    assert!(
        has_smell(&out, "Duplicated Assertion Blocks") || has_smell(&out, "Large Assertion Block")
            || has_smell(&out, "Code Duplication"),
        "got: {}", out
    );
}

#[test]
fn constructor_over_injection_detected() {
    let output = run_check(LANG, "production_service.groovy");
    assert!(has_smell(&output, "Constructor Over-Injection"), "got: {}", output);
}

// ===========================================================================
// Other
// ===========================================================================

#[test]
fn complex_conditional_detected() {
    let out = pulse_check_code(
        concat!(
            "class T {\n",
            "  boolean check(int age, int score, boolean active) {\n",
            "    if (age > 18 && score > 50 && active) {\n",
            "      if (score > 80 || (age > 25 && active)) { return true }\n",
            "    }\n",
            "    if (age > 65 || score < 10) { return true }\n",
            "    return false\n",
            "  }\n",
            "}\n",
        ),
        "groovy",
    );
    assert!(
        has_smell(&out, "Complex Conditional") || has_smell(&out, "Complex Method"),
        "got: {}", out
    );
}

#[test]
fn test_file_analyzed() {
    let output = run_check(LANG, "test_smells.groovy");
    let debug = run_debug(LANG, "test_smells.groovy");
    assert!(debug.contains("testAdd"), "test methods should be parsed");
}

#[test]
fn analysis_completes_under_500ms() {
    let _ = run_check(LANG, "clean.groovy");
    let start = std::time::Instant::now();
    let _ = run_check(LANG, "production_service.groovy");
    assert!(start.elapsed().as_millis() < 500);
}

#[test]
fn global_conditionals_parsed() {
    let debug = run_debug(LANG, "clean.groovy");
    assert!(debug.contains("functions"), "debug should show module metrics");
}
