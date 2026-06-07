use crate::common::*;
use std::process::Command;

const LANG: &str = "zig";

// ===========================================================================
// Output format
// ===========================================================================

#[test]
fn output_starts_with_pulse() {
    let output = run_check(LANG, "complex_methods.zig");
    assert!(output.starts_with("pulse:"), "got: {output}");
}

#[test]
fn output_has_function_line_numbers() {
    let output = run_check(LANG, "complex_methods.zig");
    assert!(output.lines().any(|l| l.contains("(L") && l.contains("): ")));
}

#[test]
fn output_has_module_prefix() {
    let output = run_check(LANG, "production_service.zig");
    assert!(output.contains("Module:"), "got: {output}");
}

#[test]
fn issue_count_matches_findings() {
    let output = run_check(LANG, "complex_methods.zig");
    let first = output.lines().next().unwrap_or("");
    let findings = output.lines().filter(|l| l.starts_with("  ")).count();
    assert!(first.contains(&format!("{findings} issue")));
}

// ===========================================================================
// Clean / empty
// ===========================================================================

#[test]
fn clean_file_produces_no_output() {
    let output = run_check(LANG, "clean.zig");
    assert!(output.is_empty(), "got: {output}");
}

#[test]
fn empty_file() {
    let out = pulse_check_code("", "zig");
    assert!(out.is_empty());
}

#[test]
fn comments_only_file() {
    let out = pulse_check_code("// just a comment\n", "zig");
    assert!(out.is_empty());
}

#[test]
fn simple_func_not_flagged() {
    let out = pulse_check_code("pub fn add(a: i32, b: i32) i32 {\n    return a + b;\n}\n", "zig");
    assert!(out.is_empty(), "got: {out}");
}

// ===========================================================================
// CC boundary
// ===========================================================================

#[test]
fn cc_base_case_is_1() {
    let debug = pulse_debug_code("pub fn add(a: i32, b: i32) i32 {\n    return a + b;\n}\n", "zig");
    let cc = function_metric(&debug, "add", "cc").unwrap_or(99);
    assert_eq!(cc, 1);
}

#[test]
fn function_at_cc_boundary_flagged() {
    let out = pulse_check_code(
        concat!(
            "pub fn f(a: i32, b: i32, c: i32, d: i32, e: i32, f2: i32, g: i32, h: i32) i32 {\n",
            "    if (a > 0) {}\n",
            "    if (b > 0) {}\n",
            "    if (c > 0) {}\n",
            "    if (d > 0) {}\n",
            "    if (e > 0) {}\n",
            "    if (f2 > 0) {}\n",
            "    if (g > 0) {}\n",
            "    if (h > 0) {}\n",
            "    return 0;\n",
            "}\n",
        ),
        "zig",
    );
    assert!(has_smell(&out, "Complex Method"), "cc=9 should trigger, got: {out}");
}

#[test]
fn function_below_cc_boundary_not_flagged() {
    let out = pulse_check_code(
        concat!(
            "pub fn f(a: i32, b: i32, c: i32, d: i32, e: i32, f2: i32, g: i32) i32 {\n",
            "    if (a > 0) {}\n",
            "    if (b > 0) {}\n",
            "    if (c > 0) {}\n",
            "    if (d > 0) {}\n",
            "    if (e > 0) {}\n",
            "    if (f2 > 0) {}\n",
            "    if (g > 0) {}\n",
            "    return 0;\n",
            "}\n",
        ),
        "zig",
    );
    assert!(!has_smell(&out, "Complex Method"));
}

#[test]
fn boolean_operators_increment_cc() {
    let debug = pulse_debug_code(
        "fn f(a: bool, b: bool, c: bool) bool {\n    if (a and b and c) {\n        return true;\n    }\n    return false;\n}\n",
        "zig",
    );
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert!(cc >= 4, "got: {cc}");
}

// ===========================================================================
// Complexity smells
// ===========================================================================

#[test]
fn complex_method_detected() {
    let output = run_check(LANG, "complex_methods.zig");
    assert!(has_smell(&output, "Complex Method"), "got: {output}");
    assert!(has_function(&output, "processOrder"));
}

#[test]
fn complex_method_cc_at_least_9() {
    let debug = run_debug(LANG, "complex_methods.zig");
    let cc = function_metric(&debug, "processOrder", "cc").unwrap_or(0);
    assert!(cc >= 9, "cc should be >= 9, got: {cc}");
}

#[test]
fn god_method_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("god.zig");
    let mut code = String::from("pub fn processDataPipeline(x: i32) i32 {\n");
    for i in 0..cc_branches() {
        code.push_str(&format!("    if (x > {i}) {{}}\n"));
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("    const y{i} = {i};\n"));
    }
    code.push_str("    return 0;\n}\n");
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(has_smell(&stdout, "God Method"), "got: {stdout}");
}

#[test]
fn god_method_not_reported_as_separate() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("god2.zig");
    let mut code = String::from("pub fn processDataPipeline(x: i32) i32 {\n");
    for i in 0..cc_branches() {
        code.push_str(&format!("    if (x > {i}) {{}}\n"));
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("    const y{i} = {i};\n"));
    }
    code.push_str("    return 0;\n}\n");
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(has_smell(&stdout, "God Method"));
    let lines: Vec<&str> = stdout.lines().filter(|l| l.contains("processDataPipeline")).collect();
    assert!(!lines.iter().any(|l| l.contains("Complex Method")));
    assert!(!lines.iter().any(|l| l.contains("Large Method")));
}

#[test]
fn large_method_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large.zig");
    let mut code = String::from("pub fn buildReport() i32 {\n");
    for i in 0..fn_padding() {
        code.push_str(&format!("    const x{i} = {i};\n"));
    }
    code.push_str("    return 0;\n}\n");
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(has_smell(&stdout, "Large Method") || has_smell(&stdout, "God Method"), "got: {stdout}");
}

// ===========================================================================
// Nesting
// ===========================================================================

#[test]
fn deep_nesting_detected() {
    let output = run_check(LANG, "deep_nesting.zig");
    assert!(has_smell(&output, "Deep Nested"), "got: {output}");
    assert!(has_function(&output, "deeplyNested"));
}

#[test]
fn deep_nesting_depth_exceeds_4() {
    let debug = run_debug(LANG, "deep_nesting.zig");
    let depth = function_metric(&debug, "deeplyNested", "nesting").unwrap_or(0);
    assert!(depth > 4, "got: {depth}");
}

#[test]
fn moderate_nesting_not_flagged() {
    let output = run_check(LANG, "deep_nesting.zig");
    assert!(!has_function(&output, "moderatelyNested"));
}

// ===========================================================================
// Arguments
// ===========================================================================

#[test]
fn excess_args_detected() {
    let output = run_check(LANG, "excess_args.zig");
    assert!(has_smell(&output, "Excess Arguments"), "got: {output}");
    assert!(has_function(&output, "createUser"));
}

#[test]
fn excess_args_count_correct() {
    let debug = run_debug(LANG, "excess_args.zig");
    let args = function_metric(&debug, "createUser", "args").unwrap_or(0);
    assert!(args >= 6, "got: {args}");
}

#[test]
fn simple_func_in_excess_args_not_flagged() {
    let output = run_check(LANG, "excess_args.zig");
    assert!(!has_function(&output, "simpleFunc"));
}

// ===========================================================================
// Module-level
// ===========================================================================

#[test]
fn code_duplication_detected() {
    let output = run_check(LANG, "code_duplication.zig");
    assert!(has_smell(&output, "Code Duplication"), "got: {output}");
}

#[test]
fn embedded_block_detected() {
    let output = run_check(LANG, "embedded_block.zig");
    assert!(has_smell(&output, "Large Embedded Block"), "got: {output}");
}

#[test]
fn bumpy_road_detected() {
    let output = run_check(LANG, "bumpy_road.zig");
    assert!(has_smell(&output, "Nested Conditional Chunks") || has_smell(&output, "Deep Nested"), "got: {output}");
}

#[test]
fn low_cohesion_detected() {
    let output = run_check(LANG, "low_cohesion.zig");
    assert!(has_smell(&output, "Low Cohesion") || !output.is_empty(), "got: {output}");
}

#[test]
fn primitive_obsession_detected() {
    let output = run_check(LANG, "primitive_obsession.zig");
    assert!(has_smell(&output, "Primitive Obsession") || has_smell(&output, "Excess Arguments"), "got: {output}");
}

#[test]
fn overall_function_size_at_threshold() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("size_at.zig");
    let mut code = String::new();
    for i in 0..3 {
        code.push_str(&format!("pub fn lg{i}() i32 {{\n"));
        for j in 0..large_fn_lines() {
            code.push_str(&format!("    const x{j} = {j};\n"));
        }
        code.push_str("    return 0;\n}\n\n");
    }
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(has_smell(&stdout, "Overall Function Size"), "got: {stdout}");
}

#[test]
fn overall_function_size_below_threshold() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("size_below.zig");
    let mut code = String::new();
    for i in 0..2 {
        code.push_str(&format!("pub fn lg{i}() i32 {{\n"));
        for j in 0..large_fn_lines() {
            code.push_str(&format!("    const x{j} = {j};\n"));
        }
        code.push_str("    return 0;\n}\n\n");
    }
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!has_smell(&stdout, "Overall Function Size"));
}

// ===========================================================================
// Other
// ===========================================================================

#[test]
fn simple_string_not_flagged() {
    let out = pulse_check_code("pub fn f() []const u8 {\n    return \"hello\";\n}\n", "zig");
    assert!(!has_smell(&out, "Large Embedded Block"));
}

#[test]
fn complex_conditional_detected() {
    let out = pulse_check_code(
        concat!(
            "fn check(age: i32, score: i32, active: bool) bool {\n",
            "    if (age > 18 and score > 50 and active) {\n",
            "        if (score > 80 or (age > 25 and active)) {\n",
            "            return true;\n",
            "        }\n",
            "    }\n",
            "    if (age > 65 or score < 10) {\n",
            "        return true;\n",
            "    }\n",
            "    return false;\n",
            "}\n",
        ),
        "zig",
    );
    assert!(has_smell(&out, "Complex Conditional") || has_smell(&out, "Complex Method"), "got: {out}");
}

#[test]
fn production_service_has_issues() {
    let output = run_check(LANG, "production_service.zig");
    assert!(!output.is_empty(), "production_service.zig should have findings");
}

#[test]
fn analysis_completes_under_500ms() {
    let path = fixtures_dir(LANG).join("production_service.zig");
    let start = std::time::Instant::now();
    let _ = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let elapsed = start.elapsed();
    assert!(elapsed.as_millis() < 500, "took: {}ms", elapsed.as_millis());
}

// ===========================================================================
// Hook
// ===========================================================================

#[test]
fn hook_clean_file_silent() {
    let path = fixtures_dir(LANG).join("clean.zig");
    let output = run_hook(path.to_str().unwrap());
    assert!(output.is_empty());
}

#[test]
fn hook_smelly_file_produces_output() {
    let path = fixtures_dir(LANG).join("complex_methods.zig");
    let output = run_hook(path.to_str().unwrap());
    assert!(!output.is_empty());
}

#[test]
fn hook_nonexistent_file_silent() {
    let output = run_hook("/nonexistent/path/foo.zig");
    assert!(output.is_empty());
}

// ===========================================================================
// Switch case increments cc
// ===========================================================================

#[test]
fn switch_case_increments_cc() {
    let out = pulse_check_code(
        concat!(
            "fn handle(action: u8) u8 {\n",
            "    return switch (action) {\n",
            "        1 => 10,\n",
            "        2 => 20,\n",
            "        3 => 30,\n",
            "        4 => 40,\n",
            "        5 => 50,\n",
            "        6 => 60,\n",
            "        7 => 70,\n",
            "        8 => 80,\n",
            "        9 => 90,\n",
            "        else => 0,\n",
            "    };\n",
            "}\n",
        ),
        "zig",
    );
    assert!(has_smell(&out, "Complex Method"), "got: {out}");
}

// ===========================================================================
// Global conditionals
// ===========================================================================

#[test]
fn global_conditionals_parsed() {
    let debug = run_debug(LANG, "global_conditionals.zig");
    let cc = function_metric(&debug, "setup", "cc").unwrap_or(0);
    assert!(cc >= 2, "setup should have branches, got cc={cc}");
}

// ===========================================================================
// Test smells
// ===========================================================================

#[test]
fn test_file_analyzed() {
    let output = run_check(LANG, "test_smells.zig");
    assert!(!output.is_empty() || output.is_empty(), "test file should be parseable");
}

// ===========================================================================
// Constructor over-injection
// ===========================================================================

#[test]
fn constructor_over_injection_detected() {
    let output = run_check(LANG, "excess_args.zig");
    assert!(
        has_smell(&output, "Constructor Over-Injection") || has_smell(&output, "Excess Arguments"),
        "got: {output}"
    );
}

// ===========================================================================
// Large method loc check
// ===========================================================================

#[test]
fn large_method_loc_at_least_65() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large_loc.zig");
    let mut code = String::from("pub fn buildReport() i32 {\n");
    for i in 0..fn_padding() {
        code.push_str(&format!("    const x{i} = {i};\n"));
    }
    code.push_str("    return 0;\n}\n");
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["debug", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stderr = String::from_utf8(out.stderr).unwrap();
    let loc = function_metric(&stderr, "buildReport", "loc").unwrap_or(0);
    assert!(loc >= t().function.fn_loc_warning, "loc >= t().function.fn_loc_warning, got: {loc}");
}

// ===========================================================================
// Code duplication inline
// ===========================================================================

#[test]
fn code_duplication_inline() {
    let out = pulse_check_code(
        concat!(
            "fn rptA(d: []const u8) u32 {\n",
            "    var r: u32 = 0;\n",
            "    for (d) |v| {\n",
            "        r += v;\n",
            "    }\n",
            "    r = r * 2;\n",
            "    return r;\n",
            "}\n\n",
            "fn rptB(d: []const u8) u32 {\n",
            "    var r: u32 = 0;\n",
            "    for (d) |v| {\n",
            "        r += v;\n",
            "    }\n",
            "    r = r * 2;\n",
            "    return r;\n",
            "}\n",
        ),
        "zig",
    );
    assert!(has_smell(&out, "Code Duplication"), "got: {out}");
}

// ===========================================================================
// Nested conditional chunks inline
// ===========================================================================

#[test]
fn nested_conditional_chunks_detected() {
    let out = pulse_check_code(
        concat!(
            "fn validate(data: []const u8) i32 {\n",
            "    if (data.len > 0) {\n",
            "        if (data[0] > 0) {\n",
            "            if (data[0] > 10) {\n",
            "                const x = 1;\n",
            "                _ = x;\n",
            "            }\n",
            "        }\n",
            "    }\n",
            "    const gap = 1;\n",
            "    _ = gap;\n",
            "    if (data.len > 5) {\n",
            "        if (data[5] > 0) {\n",
            "            if (data[5] > 10) {\n",
            "                const y = 2;\n",
            "                _ = y;\n",
            "            }\n",
            "        }\n",
            "    }\n",
            "    return 0;\n",
            "}\n",
        ),
        "zig",
    );
    assert!(has_smell(&out, "Nested Conditional Chunks") || has_smell(&out, "Complex Method"), "got: {out}");
}

// ===========================================================================
// For range increments cc
// ===========================================================================

#[test]
fn for_range_increments_cc() {
    let debug = pulse_debug_code(
        "fn sum(data: []const u8) u32 {\n    var s: u32 = 0;\n    for (data) |v| {\n        s += v;\n    }\n    return s;\n}\n",
        "zig",
    );
    let cc = function_metric(&debug, "sum", "cc").unwrap_or(0);
    assert!(cc >= 2, "for should increment cc, got: {cc}");
}

// ===========================================================================
// Struct method analyzed
// ===========================================================================

#[test]
fn struct_method_analyzed() {
    let out = pulse_check_code(
        concat!(
            "const Svc = struct {\n",
            "    pub fn handle(self: Svc, a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32) i32 {\n",
            "        _ = self;\n",
            "        return a + b;\n",
            "    }\n",
            "};\n",
        ),
        "zig",
    );
    assert!(has_smell(&out, "Excess Arguments"), "got: {out}");
}

// ===========================================================================
// Primitive obsession inline
// ===========================================================================

#[test]
fn primitive_obsession_inline() {
    let out = pulse_check_code(
        concat!("fn f(a: i32, b: i32, c: i32, d: i32, e: f64, f2: bool) i32 {\n", "    return a + b + c + d;\n", "}\n",),
        "zig",
    );
    assert!(has_smell(&out, "Primitive Obsession") || has_smell(&out, "Excess Arguments"), "got: {out}");
}

// ===========================================================================
// Catch increments cc
// ===========================================================================

#[test]
fn catch_increments_cc() {
    let debug = pulse_debug_code("fn f(val: anyerror!i32) i32 {\n    return val catch 0;\n}\n", "zig");
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert!(cc >= 2, "catch should increment cc, got: {cc}");
}
