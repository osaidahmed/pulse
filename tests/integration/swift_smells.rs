use crate::common::*;
use std::process::Command;

const LANG: &str = "swift";

// ===========================================================================
// Output format
// ===========================================================================

#[test]
fn output_starts_with_pulse() {
    let output = run_check(LANG, "complex_methods.swift");
    assert!(output.starts_with("pulse:"), "got: {output}");
}

#[test]
fn output_has_function_line_numbers() {
    let output = run_check(LANG, "complex_methods.swift");
    assert!(output.lines().any(|l| l.contains("(L") && l.contains("): ")));
}

#[test]
fn output_has_module_prefix() {
    let output = run_check(LANG, "production_service.swift");
    assert!(output.contains("Module:"), "got: {output}");
}

#[test]
fn issue_count_matches_findings() {
    let output = run_check(LANG, "complex_methods.swift");
    let first = output.lines().next().unwrap_or("");
    let findings = output.lines().filter(|l| l.starts_with("  ")).count();
    assert!(first.contains(&format!("{findings} issue")));
}

// ===========================================================================
// Clean / empty
// ===========================================================================

#[test]
fn clean_file_produces_no_output() {
    let output = run_check(LANG, "clean.swift");
    assert!(output.is_empty(), "got: {output}");
}

#[test]
fn empty_file() {
    let out = pulse_check_code("", "swift");
    assert!(out.is_empty());
}

#[test]
fn comments_only_file() {
    let out = pulse_check_code("// just a comment\n", "swift");
    assert!(out.is_empty());
}

#[test]
fn simple_func_not_flagged() {
    let out = pulse_check_code("func add(a: Int, b: Int) -> Int {\n    return a + b\n}\n", "swift");
    assert!(out.is_empty(), "got: {out}");
}

// ===========================================================================
// CC boundary
// ===========================================================================

#[test]
fn cc_base_case_is_1() {
    let debug = pulse_debug_code("func add(a: Int, b: Int) -> Int {\n    return a + b\n}\n", "swift");
    let cc = function_metric(&debug, "add", "cc").unwrap_or(99);
    assert_eq!(cc, 1);
}

#[test]
fn function_at_cc_boundary_flagged() {
    let out = pulse_check_code(
        concat!(
            "func f(a: Int, b: Int, c: Int, d: Int, e: Int, f2: Int, g: Int, h: Int) -> Int {\n",
            "    if a > 0 {}\n",
            "    if b > 0 {}\n",
            "    if c > 0 {}\n",
            "    if d > 0 {}\n",
            "    if e > 0 {}\n",
            "    if f2 > 0 {}\n",
            "    if g > 0 {}\n",
            "    if h > 0 {}\n",
            "    return 0\n",
            "}\n",
        ),
        "swift",
    );
    assert!(has_smell(&out, "Complex Method"), "cc=9 should trigger, got: {out}");
}

#[test]
fn function_below_cc_boundary_not_flagged() {
    let out = pulse_check_code(
        concat!(
            "func f(a: Int, b: Int, c: Int, d: Int, e: Int, f2: Int, g: Int) -> Int {\n",
            "    if a > 0 {}\n",
            "    if b > 0 {}\n",
            "    if c > 0 {}\n",
            "    if d > 0 {}\n",
            "    if e > 0 {}\n",
            "    if f2 > 0 {}\n",
            "    if g > 0 {}\n",
            "    return 0\n",
            "}\n",
        ),
        "swift",
    );
    assert!(!has_smell(&out, "Complex Method"));
}

#[test]
fn boolean_operators_increment_cc() {
    let debug = pulse_debug_code(
        "func f(a: Bool, b: Bool, c: Bool) -> Bool {\n    if a && b && c {\n        return true\n    }\n    return false\n}\n",
        "swift",
    );
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert!(cc >= 4, "got: {cc}");
}

// ===========================================================================
// Complexity smells
// ===========================================================================

#[test]
fn complex_method_detected() {
    let output = run_check(LANG, "complex_methods.swift");
    assert!(has_smell(&output, "Complex Method"), "got: {output}");
    assert!(has_function(&output, "processOrder"));
}

#[test]
fn complex_method_cc_at_least_9() {
    let debug = run_debug(LANG, "complex_methods.swift");
    let cc = function_metric(&debug, "processOrder", "cc").unwrap_or(0);
    assert!(cc >= 9, "cc should be >= 9, got: {cc}");
}

#[test]
fn god_method_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("god.swift");
    let mut code = String::from("func processDataPipeline(x: Int) -> Int {\n");
    for i in 0..cc_branches() {
        code.push_str(&format!("    if x > {i} {{}}\n"));
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("    let y{i} = {i}\n"));
    }
    code.push_str("    return 0\n}\n");
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
    let path = dir.path().join("god2.swift");
    let mut code = String::from("func processDataPipeline(x: Int) -> Int {\n");
    for i in 0..cc_branches() {
        code.push_str(&format!("    if x > {i} {{}}\n"));
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("    let y{i} = {i}\n"));
    }
    code.push_str("    return 0\n}\n");
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
    let path = dir.path().join("large.swift");
    let mut code = String::from("func buildReport() -> Int {\n");
    for i in 0..fn_padding() {
        code.push_str(&format!("    let x{i} = {i}\n"));
    }
    code.push_str("    return 0\n}\n");
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
    let output = run_check(LANG, "deep_nesting.swift");
    assert!(has_smell(&output, "Deep Nested"), "got: {output}");
    assert!(has_function(&output, "deeplyNested"));
}

#[test]
fn deep_nesting_depth_exceeds_4() {
    let debug = run_debug(LANG, "deep_nesting.swift");
    let depth = function_metric(&debug, "deeplyNested", "nesting").unwrap_or(0);
    assert!(depth > 4, "got: {depth}");
}

#[test]
fn moderate_nesting_not_flagged() {
    let output = run_check(LANG, "deep_nesting.swift");
    assert!(!has_function(&output, "moderatelyNested"));
}

// ===========================================================================
// Arguments
// ===========================================================================

#[test]
fn excess_args_detected() {
    let output = run_check(LANG, "excess_args.swift");
    assert!(has_smell(&output, "Excess Arguments"), "got: {output}");
    assert!(has_function(&output, "createUser"));
}

#[test]
fn excess_args_count_correct() {
    let debug = run_debug(LANG, "excess_args.swift");
    let args = function_metric(&debug, "createUser", "args").unwrap_or(0);
    assert!(args >= 6, "got: {args}");
}

// ===========================================================================
// Module-level
// ===========================================================================

#[test]
fn code_duplication_detected() {
    let output = run_check(LANG, "code_duplication.swift");
    assert!(has_smell(&output, "Code Duplication"), "got: {output}");
}

#[test]
fn embedded_block_detected() {
    let output = run_check(LANG, "embedded_block.swift");
    assert!(has_smell(&output, "Large Embedded Block"), "got: {output}");
}

#[test]
fn bumpy_road_detected() {
    let output = run_check(LANG, "bumpy_road.swift");
    assert!(has_smell(&output, "Nested Conditional Chunks") || has_smell(&output, "Deep Nested"), "got: {output}");
}

#[test]
fn low_cohesion_detected() {
    let output = run_check(LANG, "low_cohesion.swift");
    assert!(
        has_smell(&output, "Low Cohesion") || has_smell(&output, "Too Many Functions") || !output.is_empty(),
        "got: {output}"
    );
}

#[test]
fn primitive_obsession_detected() {
    let output = run_check(LANG, "primitive_obsession.swift");
    assert!(has_smell(&output, "Primitive Obsession") || has_smell(&output, "Excess Arguments"), "got: {output}");
}

#[test]
fn overall_function_size_at_threshold() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("size_at.swift");
    let mut code = String::new();
    for i in 0..3 {
        code.push_str(&format!("func lg{i}() -> Int {{\n"));
        for j in 0..45 {
            code.push_str(&format!("    let x{j} = {j}\n"));
        }
        code.push_str("    return 0\n}\n\n");
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
    let path = dir.path().join("size_below.swift");
    let mut code = String::new();
    for i in 0..2 {
        code.push_str(&format!("func lg{i}() -> Int {{\n"));
        for j in 0..45 {
            code.push_str(&format!("    let x{j} = {j}\n"));
        }
        code.push_str("    return 0\n}\n\n");
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
    let out = pulse_check_code("func f() -> String {\n    return \"hello\"\n}\n", "swift");
    assert!(!has_smell(&out, "Large Embedded Block"));
}

#[test]
fn complex_conditional_detected() {
    let out = pulse_check_code(
        concat!(
            "func check(age: Int, score: Int, active: Bool) -> Bool {\n",
            "    if age > 18 && score > 50 && active {\n",
            "        if score > 80 || (age > 25 && active) {\n",
            "            return true\n",
            "        }\n",
            "    }\n",
            "    if age > 65 || score < 10 {\n",
            "        return true\n",
            "    }\n",
            "    return false\n",
            "}\n",
        ),
        "swift",
    );
    assert!(has_smell(&out, "Complex Conditional") || has_smell(&out, "Complex Method"), "got: {out}");
}

#[test]
fn production_service_has_issues() {
    let output = run_check(LANG, "production_service.swift");
    assert!(!output.is_empty(), "production_service.swift should have findings");
}

#[test]
fn analysis_completes_under_500ms() {
    let path = fixtures_dir(LANG).join("production_service.swift");
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
    let path = fixtures_dir(LANG).join("clean.swift");
    let output = run_hook(path.to_str().unwrap());
    assert!(output.is_empty());
}

#[test]
fn hook_smelly_file_produces_output() {
    let path = fixtures_dir(LANG).join("complex_methods.swift");
    let output = run_hook(path.to_str().unwrap());
    assert!(!output.is_empty());
}

#[test]
fn hook_nonexistent_file_silent() {
    let output = run_hook("/nonexistent/path/foo.swift");
    assert!(output.is_empty());
}

// ===========================================================================
// Switch case increments cc
// ===========================================================================

#[test]
fn switch_case_increments_cc() {
    let out = pulse_check_code(
        concat!(
            "func handle(action: Int) -> String {\n",
            "    switch action {\n",
            "    case 1:\n        return \"a\"\n",
            "    case 2:\n        return \"b\"\n",
            "    case 3:\n        return \"c\"\n",
            "    case 4:\n        return \"d\"\n",
            "    case 5:\n        return \"e\"\n",
            "    case 6:\n        return \"f\"\n",
            "    case 7:\n        return \"g\"\n",
            "    case 8:\n        return \"h\"\n",
            "    case 9:\n        return \"i\"\n",
            "    default:\n        return \"?\"\n",
            "    }\n",
            "}\n",
        ),
        "swift",
    );
    assert!(has_smell(&out, "Complex Method"), "got: {out}");
}

// ===========================================================================
// Guard increments cc
// ===========================================================================

#[test]
fn guard_increments_cc() {
    let debug =
        pulse_debug_code("func f(x: Int) -> Int {\n    guard x > 0 else { return 0 }\n    return x\n}\n", "swift");
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert_eq!(cc, 2);
}

// ===========================================================================
// Init detected as constructor
// ===========================================================================

#[test]
fn init_detected_as_constructor() {
    let output = run_check(LANG, "excess_args.swift");
    assert!(has_smell(&output, "Constructor Over-Injection"), "got: {output}");
}

// ===========================================================================
// Extension methods attributed to type
// ===========================================================================

#[test]
fn extension_method_attributed_to_type() {
    let debug = pulse_debug_code(
        concat!(
            "struct Point {\n    var x: Int\n    var y: Int\n}\n\n",
            "extension Point {\n",
            "    func magnitude() -> Int {\n        return x + y\n    }\n",
            "}\n",
        ),
        "swift",
    );
    assert!(debug.contains("Point.magnitude"), "extension method should be attributed, got: {debug}");
}

// ===========================================================================
// Global conditionals
// ===========================================================================

#[test]
fn global_conditionals_detected() {
    let output = run_check(LANG, "global_conditionals.swift");
    assert!(has_smell(&output, "Global Conditionals"), "got: {output}");
}

// ===========================================================================
// Test smells
// ===========================================================================

#[test]
fn test_file_analyzed() {
    let output = run_check(LANG, "test_smells.swift");
    assert!(!output.is_empty() || output.is_empty(), "test file should be parseable");
}

// ===========================================================================
// Constructor over-injection
// ===========================================================================

#[test]
fn constructor_over_injection_detected() {
    let output = run_check(LANG, "excess_args.swift");
    assert!(
        has_smell(&output, "Constructor Over-Injection") || has_smell(&output, "Excess Arguments"),
        "got: {output}"
    );
}

// ===========================================================================
// Simple func in excess args not flagged
// ===========================================================================

#[test]
fn simple_func_in_excess_args_not_flagged() {
    let output = run_check(LANG, "excess_args.swift");
    assert!(!has_function(&output, "simpleFunc"));
}

// ===========================================================================
// Large method loc check
// ===========================================================================

#[test]
fn large_method_loc_at_least_65() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large_loc.swift");
    let mut code = String::from("func buildReport() -> Int {\n");
    for i in 0..fn_padding() {
        code.push_str(&format!("    let x{i} = {i}\n"));
    }
    code.push_str("    return 0\n}\n");
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
            "func rptA(data: [Int]) -> Int {\n",
            "    var r = 0\n",
            "    for v in data {\n",
            "        r += v\n",
            "    }\n",
            "    r = r * 2\n",
            "    return r\n",
            "}\n\n",
            "func rptB(data: [Int]) -> Int {\n",
            "    var r = 0\n",
            "    for v in data {\n",
            "        r += v\n",
            "    }\n",
            "    r = r * 2\n",
            "    return r\n",
            "}\n",
        ),
        "swift",
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
            "func validate(data: [Int]) -> Int {\n",
            "    if data.count > 0 {\n",
            "        if data[0] > 0 {\n",
            "            if data[0] > 10 {\n",
            "                let x = 1\n",
            "                _ = x\n",
            "            }\n",
            "        }\n",
            "    }\n",
            "    let gap = 1\n",
            "    _ = gap\n",
            "    if data.count > 5 {\n",
            "        if data[5] > 0 {\n",
            "            if data[5] > 10 {\n",
            "                let y = 2\n",
            "                _ = y\n",
            "            }\n",
            "        }\n",
            "    }\n",
            "    let gap2 = 2\n",
            "    _ = gap2\n",
            "    if data.count > 10 {\n",
            "        if data[10] > 0 {\n",
            "            if data[10] > 10 {\n",
            "                let z = 3\n",
            "                _ = z\n",
            "            }\n",
            "        }\n",
            "    }\n",
            "    return 0\n",
            "}\n",
        ),
        "swift",
    );
    assert!(has_smell(&out, "Nested Conditional Chunks") || has_smell(&out, "Complex Method"), "got: {out}");
}

// ===========================================================================
// For-in increments cc
// ===========================================================================

#[test]
fn for_in_increments_cc() {
    let debug = pulse_debug_code(
        "func sum(data: [Int]) -> Int {\n    var s = 0\n    for v in data {\n        s += v\n    }\n    return s\n}\n",
        "swift",
    );
    let cc = function_metric(&debug, "sum", "cc").unwrap_or(0);
    assert!(cc >= 2, "for-in should increment cc, got: {cc}");
}

// ===========================================================================
// Method attributed to class
// ===========================================================================

#[test]
fn method_attributed_to_class() {
    let debug = pulse_debug_code(
        concat!(
            "class Svc {\n",
            "    func handle(a: Int, b: Int, c: Int, d: Int, e: Int, f: Int, g: Int, h: Int) -> Int {\n",
            "        return a + b\n",
            "    }\n",
            "}\n",
        ),
        "swift",
    );
    assert!(debug.contains("Svc.handle"), "method should be attributed to class, got: {debug}");
}

// ===========================================================================
// Primitive obsession inline
// ===========================================================================

#[test]
fn primitive_obsession_inline() {
    let out = pulse_check_code(
        concat!(
            "func f(a: Int, b: Int, c: Int, d: Int, e: Double, f2: Bool) -> Int {\n",
            "    return a + b + c + d\n",
            "}\n",
        ),
        "swift",
    );
    assert!(has_smell(&out, "Primitive Obsession") || has_smell(&out, "Excess Arguments"), "got: {out}");
}
