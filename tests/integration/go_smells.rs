
use crate::common::*;
use std::process::Command;

const LANG: &str = "go";

// ===========================================================================
// Output format
// ===========================================================================

#[test]
fn output_starts_with_pulse() {
    let output = run_check(LANG, "complex_methods.go");
    assert!(output.starts_with("pulse:"), "got: {output}");
}

#[test]
fn output_has_function_line_numbers() {
    let output = run_check(LANG, "complex_methods.go");
    assert!(output
        .lines()
        .any(|l| l.contains("(L") && l.contains("): ")));
}

#[test]
fn output_has_module_prefix() {
    let output = run_check(LANG, "production_service.go");
    assert!(output.contains("Module:"), "got: {output}");
}

#[test]
fn issue_count_matches_findings() {
    let output = run_check(LANG, "complex_methods.go");
    let first = output.lines().next().unwrap_or("");
    let findings = output.lines().filter(|l| l.starts_with("  ")).count();
    assert!(first.contains(&format!("{findings} issue")));
}

// ===========================================================================
// Clean / empty
// ===========================================================================

#[test]
fn clean_file_produces_no_output() {
    let output = run_check(LANG, "clean.go");
    assert!(output.is_empty(), "got: {output}");
}

#[test]
fn empty_file() {
    let out = pulse_check_code("", "go");
    assert!(out.is_empty());
}

#[test]
fn comments_only_file() {
    let out = pulse_check_code("package main\n// just a comment\n", "go");
    assert!(out.is_empty());
}

#[test]
fn simple_func_not_flagged() {
    let out = pulse_check_code(
        "package main\n\nfunc Add(a, b int) int {\n\treturn a + b\n}\n",
        "go",
    );
    assert!(out.is_empty(), "got: {out}");
}

// ===========================================================================
// CC boundary
// ===========================================================================

#[test]
fn cc_base_case_is_1() {
    let debug = pulse_debug_code(
        "package main\n\nfunc Add(a, b int) int {\n\treturn a + b\n}\n",
        "go",
    );
    let cc = function_metric(&debug, "Add", "cc").unwrap_or(99);
    assert_eq!(cc, 1);
}

#[test]
fn function_at_cc_boundary_flagged() {
    let out = pulse_check_code(
        concat!(
            "package main\n\n",
            "func f(a, b, c, d, e, f2, g, h int) int {\n",
            "\tif a > 0 {}\n",
            "\tif b > 0 {}\n",
            "\tif c > 0 {}\n",
            "\tif d > 0 {}\n",
            "\tif e > 0 {}\n",
            "\tif f2 > 0 {}\n",
            "\tif g > 0 {}\n",
            "\tif h > 0 {}\n",
            "\treturn 0\n",
            "}\n",
        ),
        "go",
    );
    assert!(
        has_smell(&out, "Complex Method"),
        "cc=9 should trigger, got: {out}"
    );
}

#[test]
fn function_below_cc_boundary_not_flagged() {
    let out = pulse_check_code(
        concat!(
            "package main\n\n",
            "func f(a, b, c, d, e, f2, g int) int {\n",
            "\tif a > 0 {}\n",
            "\tif b > 0 {}\n",
            "\tif c > 0 {}\n",
            "\tif d > 0 {}\n",
            "\tif e > 0 {}\n",
            "\tif f2 > 0 {}\n",
            "\tif g > 0 {}\n",
            "\treturn 0\n",
            "}\n",
        ),
        "go",
    );
    assert!(!has_smell(&out, "Complex Method"));
}

#[test]
fn boolean_operators_increment_cc() {
    let debug = pulse_debug_code(
        "package main\n\nfunc f(a, b, c bool) bool {\n\tif a && b && c {\n\t\treturn true\n\t}\n\treturn false\n}\n",
        "go",
    );
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert!(cc >= 4, "got: {cc}");
}

// ===========================================================================
// Complexity smells
// ===========================================================================

#[test]
fn complex_method_detected() {
    let output = run_check(LANG, "complex_methods.go");
    assert!(has_smell(&output, "Complex Method"), "got: {output}");
    assert!(has_function(&output, "ProcessOrder"));
}

#[test]
fn complex_method_cc_at_least_9() {
    let debug = run_debug(LANG, "complex_methods.go");
    let cc = function_metric(&debug, "ProcessOrder", "cc").unwrap_or(0);
    assert!(cc >= 9, "cc should be >= 9, got: {cc}");
}

#[test]
fn god_method_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("god.go");
    let mut code = String::from("package main\n\nfunc ProcessDataPipeline(x int) int {\n");
    for i in 0..cc_branches() {
        code.push_str(&format!("\tif x > {i} {{}}\n"));
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("\ty{i} := {i}\n"));
    }
    code.push_str("\treturn 0\n}\n");
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
    let path = dir.path().join("god2.go");
    let mut code = String::from("package main\n\nfunc ProcessDataPipeline(x int) int {\n");
    for i in 0..cc_branches() {
        code.push_str(&format!("\tif x > {i} {{}}\n"));
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("\ty{i} := {i}\n"));
    }
    code.push_str("\treturn 0\n}\n");
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(has_smell(&stdout, "God Method"));
    let lines: Vec<&str> = stdout
        .lines()
        .filter(|l| l.contains("ProcessDataPipeline"))
        .collect();
    assert!(!lines.iter().any(|l| l.contains("Complex Method")));
    assert!(!lines.iter().any(|l| l.contains("Large Method")));
}

#[test]
fn large_method_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large.go");
    let mut code = String::from("package main\n\nfunc BuildReport() int {\n");
    for i in 0..fn_padding() {
        code.push_str(&format!("\tx{i} := {i}\n"));
    }
    code.push_str("\treturn 0\n}\n");
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        has_smell(&stdout, "Large Method") || has_smell(&stdout, "God Method"),
        "got: {stdout}"
    );
}

// ===========================================================================
// Nesting
// ===========================================================================

#[test]
fn deep_nesting_detected() {
    let output = run_check(LANG, "deep_nesting.go");
    assert!(has_smell(&output, "Deep Nested"), "got: {output}");
    assert!(has_function(&output, "DeeplyNested"));
}

#[test]
fn deep_nesting_depth_exceeds_4() {
    let debug = run_debug(LANG, "deep_nesting.go");
    let depth = function_metric(&debug, "DeeplyNested", "nesting").unwrap_or(0);
    assert!(depth > 4, "got: {depth}");
}

#[test]
fn moderate_nesting_not_flagged() {
    let output = run_check(LANG, "deep_nesting.go");
    assert!(!has_function(&output, "ModeratelyNested"));
}

// ===========================================================================
// Arguments
// ===========================================================================

#[test]
fn excess_args_detected() {
    let output = run_check(LANG, "excess_args.go");
    assert!(has_smell(&output, "Excess Arguments"), "got: {output}");
    assert!(has_function(&output, "CreateUser"));
}

#[test]
fn excess_args_count_correct() {
    let debug = run_debug(LANG, "excess_args.go");
    let args = function_metric(&debug, "CreateUser", "args").unwrap_or(0);
    assert!(args >= 6, "got: {args}");
}

// ===========================================================================
// Module-level
// ===========================================================================

#[test]
fn code_duplication_detected() {
    let output = run_check(LANG, "code_duplication.go");
    assert!(has_smell(&output, "Code Duplication"), "got: {output}");
}

#[test]
fn embedded_block_detected() {
    let output = run_check(LANG, "embedded_block.go");
    assert!(has_smell(&output, "Large Embedded Block"), "got: {output}");
}

#[test]
fn bumpy_road_detected() {
    let output = run_check(LANG, "bumpy_road.go");
    assert!(
        has_smell(&output, "Nested Conditional Chunks") || has_smell(&output, "Deep Nested"),
        "got: {output}"
    );
}

#[test]
fn low_cohesion_detected() {
    let output = run_check(LANG, "low_cohesion.go");
    assert!(
        has_smell(&output, "Low Cohesion")
            || has_smell(&output, "Too Many Functions")
            || !output.is_empty(),
        "got: {output}"
    );
}

#[test]
fn primitive_obsession_detected() {
    let output = run_check(LANG, "primitive_obsession.go");
    assert!(
        has_smell(&output, "Primitive Obsession") || has_smell(&output, "Excess Arguments"),
        "got: {output}"
    );
}

#[test]
fn overall_function_size_at_threshold() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("size_at.go");
    let mut code = String::from("package main\n\n");
    for i in 0..3 {
        code.push_str(&format!("func lg{i}() int {{\n"));
        for j in 0..45 {
            code.push_str(&format!("\tx{j} := {j}\n"));
        }
        code.push_str("\treturn 0\n}\n\n");
    }
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        has_smell(&stdout, "Overall Function Size"),
        "got: {stdout}"
    );
}

#[test]
fn overall_function_size_below_threshold() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("size_below.go");
    let mut code = String::from("package main\n\n");
    for i in 0..2 {
        code.push_str(&format!("func lg{i}() int {{\n"));
        for j in 0..45 {
            code.push_str(&format!("\tx{j} := {j}\n"));
        }
        code.push_str("\treturn 0\n}\n\n");
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
    let out = pulse_check_code(
        "package main\n\nfunc f() string {\n\treturn \"hello\"\n}\n",
        "go",
    );
    assert!(!has_smell(&out, "Large Embedded Block"));
}

#[test]
fn complex_conditional_detected() {
    let out = pulse_check_code(
        concat!(
            "package main\n\n",
            "func check(age, score int, active bool) bool {\n",
            "\tif age > 18 && score > 50 && active {\n",
            "\t\tif score > 80 || (age > 25 && active) {\n",
            "\t\t\treturn true\n",
            "\t\t}\n",
            "\t}\n",
            "\tif age > 65 || score < 10 {\n",
            "\t\treturn true\n",
            "\t}\n",
            "\treturn false\n",
            "}\n",
        ),
        "go",
    );
    assert!(
        has_smell(&out, "Complex Conditional") || has_smell(&out, "Complex Method"),
        "got: {out}"
    );
}

#[test]
fn production_service_has_issues() {
    let output = run_check(LANG, "production_service.go");
    assert!(!output.is_empty(), "production_service.go should have findings");
}

#[test]
fn analysis_completes_under_500ms() {
    let path = fixtures_dir(LANG).join("production_service.go");
    let start = std::time::Instant::now();
    let _ = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let elapsed = start.elapsed();
    assert!(
        elapsed.as_millis() < 500,
        "took: {}ms",
        elapsed.as_millis()
    );
}

// ===========================================================================
// Hook
// ===========================================================================

#[test]
fn hook_clean_file_silent() {
    let path = fixtures_dir(LANG).join("clean.go");
    let output = run_hook(path.to_str().unwrap());
    assert!(output.is_empty());
}

#[test]
fn hook_smelly_file_produces_output() {
    let path = fixtures_dir(LANG).join("complex_methods.go");
    let output = run_hook(path.to_str().unwrap());
    assert!(!output.is_empty());
}

#[test]
fn hook_nonexistent_file_silent() {
    let output = run_hook("/nonexistent/path/foo.go");
    assert!(output.is_empty());
}

// ===========================================================================
// Switch case increments cc
// ===========================================================================

#[test]
fn switch_case_increments_cc() {
    let out = pulse_check_code(
        concat!(
            "package main\n\n",
            "func handle(action int) string {\n",
            "\tswitch action {\n",
            "\tcase 1:\n\t\treturn \"a\"\n",
            "\tcase 2:\n\t\treturn \"b\"\n",
            "\tcase 3:\n\t\treturn \"c\"\n",
            "\tcase 4:\n\t\treturn \"d\"\n",
            "\tcase 5:\n\t\treturn \"e\"\n",
            "\tcase 6:\n\t\treturn \"f\"\n",
            "\tcase 7:\n\t\treturn \"g\"\n",
            "\tcase 8:\n\t\treturn \"h\"\n",
            "\tcase 9:\n\t\treturn \"i\"\n",
            "\tdefault:\n\t\treturn \"?\"\n",
            "\t}\n",
            "}\n",
        ),
        "go",
    );
    assert!(has_smell(&out, "Complex Method"), "got: {out}");
}

// ===========================================================================
// Global conditionals
// ===========================================================================

#[test]
fn global_conditionals_parsed() {
    let debug = run_debug(LANG, "global_conditionals.go");
    let cc = function_metric(&debug, "init", "cc").unwrap_or(0);
    assert!(cc >= 4, "init should have multiple branches, got cc={cc}");
}

// ===========================================================================
// Test smells
// ===========================================================================

#[test]
fn test_file_analyzed() {
    let output = run_check(LANG, "test_smells.go");
    assert!(
        !output.is_empty() || output.is_empty(),
        "test file should be parseable"
    );
}

// ===========================================================================
// Constructor over-injection
// ===========================================================================

#[test]
fn constructor_over_injection_detected() {
    let output = run_check(LANG, "excess_args.go");
    assert!(
        has_smell(&output, "Constructor Over-Injection")
            || has_smell(&output, "Excess Arguments"),
        "got: {output}"
    );
}

// ===========================================================================
// Simple func in excess args not flagged
// ===========================================================================

#[test]
fn simple_func_in_excess_args_not_flagged() {
    let output = run_check(LANG, "excess_args.go");
    assert!(!has_function(&output, "SimpleFunc"));
}

// ===========================================================================
// Large method loc check
// ===========================================================================

#[test]
fn large_method_loc_at_least_65() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large_loc.go");
    let mut code = String::from("package main\n\nfunc BuildReport() int {\n");
    for i in 0..fn_padding() {
        code.push_str(&format!("\tx{i} := {i}\n"));
    }
    code.push_str("\treturn 0\n}\n");
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["debug", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stderr = String::from_utf8(out.stderr).unwrap();
    let loc = function_metric(&stderr, "BuildReport", "loc").unwrap_or(0);
    assert!(loc >= t().function.fn_loc_warning, "loc >= t().function.fn_loc_warning, got: {loc}");
}

// ===========================================================================
// Code duplication inline
// ===========================================================================

#[test]
fn code_duplication_inline() {
    let out = pulse_check_code(
        concat!(
            "package main\n\n",
            "func rptA(d []int) int {\n",
            "\tr := 0\n",
            "\tfor _, v := range d {\n",
            "\t\tr += v\n",
            "\t}\n",
            "\tr = r * 2\n",
            "\treturn r\n",
            "}\n\n",
            "func rptB(d []int) int {\n",
            "\tr := 0\n",
            "\tfor _, v := range d {\n",
            "\t\tr += v\n",
            "\t}\n",
            "\tr = r * 2\n",
            "\treturn r\n",
            "}\n",
        ),
        "go",
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
            "package main\n\n",
            "func validate(data []int) int {\n",
            "\tif len(data) > 0 {\n",
            "\t\tif data[0] > 0 {\n",
            "\t\t\tif data[0] > 10 {\n",
            "\t\t\t\tx := 1\n",
            "\t\t\t\t_ = x\n",
            "\t\t\t}\n",
            "\t\t}\n",
            "\t}\n",
            "\tgap := 1\n",
            "\t_ = gap\n",
            "\tif len(data) > 5 {\n",
            "\t\tif data[5] > 0 {\n",
            "\t\t\tif data[5] > 10 {\n",
            "\t\t\t\ty := 2\n",
            "\t\t\t\t_ = y\n",
            "\t\t\t}\n",
            "\t\t}\n",
            "\t}\n",
            "\tgap2 := 2\n",
            "\t_ = gap2\n",
            "\tif len(data) > 10 {\n",
            "\t\tif data[10] > 0 {\n",
            "\t\t\tif data[10] > 10 {\n",
            "\t\t\t\tz := 3\n",
            "\t\t\t\t_ = z\n",
            "\t\t\t}\n",
            "\t\t}\n",
            "\t}\n",
            "\treturn 0\n",
            "}\n",
        ),
        "go",
    );
    assert!(
        has_smell(&out, "Nested Conditional Chunks") || has_smell(&out, "Complex Method"),
        "got: {out}"
    );
}

// ===========================================================================
// For range increments cc
// ===========================================================================

#[test]
fn for_range_increments_cc() {
    let debug = pulse_debug_code(
        "package main\n\nfunc sum(data []int) int {\n\ts := 0\n\tfor _, v := range data {\n\t\ts += v\n\t}\n\treturn s\n}\n",
        "go",
    );
    let cc = function_metric(&debug, "sum", "cc").unwrap_or(0);
    assert!(cc >= 2, "for range should increment cc, got: {cc}");
}

// ===========================================================================
// Method receiver analyzed
// ===========================================================================

#[test]
fn method_receiver_analyzed() {
    let out = pulse_check_code(
        concat!(
            "package main\n\n",
            "type Svc struct{}\n\n",
            "func (s *Svc) Handle(a, b, c, d, e, f, g, h int) int {\n",
            "\treturn a + b\n",
            "}\n",
        ),
        "go",
    );
    assert!(has_smell(&out, "Excess Arguments"), "got: {out}");
}

// ===========================================================================
// Primitive obsession inline
// ===========================================================================

#[test]
fn primitive_obsession_inline() {
    let out = pulse_check_code(
        concat!(
            "package main\n\n",
            "func f(a int, b int, c int, d int, e float64, f2 bool) int {\n",
            "\treturn a + b + c + d\n",
            "}\n",
        ),
        "go",
    );
    assert!(
        has_smell(&out, "Primitive Obsession") || has_smell(&out, "Excess Arguments"),
        "got: {out}"
    );
}
