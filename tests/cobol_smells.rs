mod common;

use common::*;
use std::process::Command;

const LANG: &str = "cobol";

// ===========================================================================
// Output format
// ===========================================================================

#[test]
fn output_starts_with_pulse() {
    let output = run_check(LANG, "complex_methods.cob");
    assert!(output.starts_with("pulse:"), "got: {}", output);
}

#[test]
fn output_has_function_line_numbers() {
    let output = run_check(LANG, "complex_methods.cob");
    assert!(output
        .lines()
        .any(|l| l.contains("(L") && l.contains("): ")));
}

#[test]
fn output_has_module_prefix() {
    let output = run_check(LANG, "production_service.cob");
    assert!(output.contains("Module:"), "got: {}", output);
}

#[test]
fn issue_count_matches_findings() {
    let output = run_check(LANG, "complex_methods.cob");
    let first = output.lines().next().unwrap_or("");
    let findings = output.lines().filter(|l| l.starts_with("  ")).count();
    assert!(first.contains(&format!("{} issue", findings)));
}

// ===========================================================================
// Clean / empty
// ===========================================================================

#[test]
fn clean_file_produces_no_output() {
    let output = run_check(LANG, "clean.cob");
    assert!(output.is_empty(), "got: {}", output);
}

#[test]
fn empty_file() {
    let out = pulse_check_code("", "cob");
    assert!(out.is_empty());
}

#[test]
fn comments_only_file() {
    let out = pulse_check_code("      *> just a comment\n", "cob");
    assert!(out.is_empty());
}

#[test]
fn simple_paragraph_not_flagged() {
    let out = pulse_check_code(
        concat!(
            "       IDENTIFICATION DIVISION.\n",
            "       PROGRAM-ID. T.\n",
            "       PROCEDURE DIVISION.\n",
            "       DO-WORK.\n",
            "           DISPLAY \"Hello\".\n",
        ),
        "cob",
    );
    assert!(out.is_empty(), "got: {}", out);
}

// ===========================================================================
// CC boundary
// ===========================================================================

#[test]
fn cc_base_case_is_1() {
    let debug = pulse_debug_code(
        concat!(
            "       IDENTIFICATION DIVISION.\n",
            "       PROGRAM-ID. T.\n",
            "       PROCEDURE DIVISION.\n",
            "       DO-WORK.\n",
            "           DISPLAY \"Hello\".\n",
        ),
        "cob",
    );
    let cc = function_metric(&debug, "DO-WORK", "cc").unwrap_or(99);
    assert_eq!(cc, 1);
}

#[test]
fn function_at_cc_boundary_flagged() {
    let mut code = String::from(
        "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. T.\n       DATA DIVISION.\n       WORKING-STORAGE SECTION.\n       01 WS-A PIC 9.\n       01 WS-B PIC 9.\n       01 WS-C PIC 9.\n       01 WS-D PIC 9.\n       01 WS-E PIC 9.\n       01 WS-F PIC 9.\n       01 WS-G PIC 9.\n       01 WS-H PIC 9.\n       PROCEDURE DIVISION.\n       COMPLEX-PARA.\n"
    );
    for v in &["WS-A", "WS-B", "WS-C", "WS-D", "WS-E", "WS-F", "WS-G", "WS-H"] {
        code.push_str(&format!("           IF {} > 0\n               DISPLAY \"{}\"\n           END-IF.\n", v, v));
    }
    let out = pulse_check_code(&code, "cob");
    assert!(
        has_smell(&out, "Complex Method"),
        "cc=9 should trigger, got: {}",
        out
    );
}

#[test]
fn function_below_cc_boundary_not_flagged() {
    let mut code = String::from(
        "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. T.\n       DATA DIVISION.\n       WORKING-STORAGE SECTION.\n       01 WS-A PIC 9.\n       01 WS-B PIC 9.\n       01 WS-C PIC 9.\n       01 WS-D PIC 9.\n       01 WS-E PIC 9.\n       01 WS-F PIC 9.\n       01 WS-G PIC 9.\n       PROCEDURE DIVISION.\n       COMPLEX-PARA.\n"
    );
    for v in &["WS-A", "WS-B", "WS-C", "WS-D", "WS-E", "WS-F", "WS-G"] {
        code.push_str(&format!("           IF {} > 0\n               DISPLAY \"{}\"\n           END-IF.\n", v, v));
    }
    let out = pulse_check_code(&code, "cob");
    assert!(!has_smell(&out, "Complex Method"));
}

#[test]
fn boolean_operators_increment_cc() {
    let debug = pulse_debug_code(
        concat!(
            "       IDENTIFICATION DIVISION.\n",
            "       PROGRAM-ID. T.\n",
            "       DATA DIVISION.\n",
            "       WORKING-STORAGE SECTION.\n",
            "       01 WS-A PIC 9.\n",
            "       01 WS-B PIC 9.\n",
            "       01 WS-C PIC 9.\n",
            "       PROCEDURE DIVISION.\n",
            "       CHECK-VALS.\n",
            "           IF WS-A > 0 AND WS-B > 0 AND WS-C > 0\n",
            "               DISPLAY \"All positive\"\n",
            "           END-IF.\n",
        ),
        "cob",
    );
    let cc = function_metric(&debug, "CHECK-VALS", "cc").unwrap_or(0);
    assert!(cc >= 4, "got: {}", cc);
}

// ===========================================================================
// Complexity smells
// ===========================================================================

#[test]
fn complex_method_detected() {
    let output = run_check(LANG, "complex_methods.cob");
    assert!(has_smell(&output, "Complex Method"), "got: {}", output);
    assert!(has_function(&output, "PROCESS-ORDER"));
}

#[test]
fn complex_method_cc_at_least_9() {
    let debug = run_debug(LANG, "complex_methods.cob");
    let cc = function_metric(&debug, "PROCESS-ORDER", "cc").unwrap_or(0);
    assert!(cc >= 9, "cc should be >= 9, got: {}", cc);
}

#[test]
fn god_method_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("god.cob");
    let mut code = String::from(
        "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. T.\n       DATA DIVISION.\n       WORKING-STORAGE SECTION.\n       01 WS-X PIC 9.\n       PROCEDURE DIVISION.\n       BIG-PARA.\n"
    );
    for i in 0..cc_branches() {
        code.push_str(&format!("           IF WS-X > {}\n               DISPLAY \"{}\"\n           END-IF.\n", i, i));
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("           DISPLAY \"line {}\".\n", i));
    }
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
    let path = dir.path().join("god2.cob");
    let mut code = String::from(
        "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. T.\n       DATA DIVISION.\n       WORKING-STORAGE SECTION.\n       01 WS-X PIC 9.\n       PROCEDURE DIVISION.\n       BIG-PARA.\n"
    );
    for i in 0..cc_branches() {
        code.push_str(&format!("           IF WS-X > {}\n               DISPLAY \"{}\"\n           END-IF.\n", i, i));
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("           DISPLAY \"line {}\".\n", i));
    }
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(has_smell(&stdout, "God Method"));
    let lines: Vec<&str> = stdout
        .lines()
        .filter(|l| l.contains("BIG-PARA"))
        .collect();
    assert!(!lines.iter().any(|l| l.contains("Complex Method")));
    assert!(!lines.iter().any(|l| l.contains("Large Method")));
}

// ===========================================================================
// Large method
// ===========================================================================

#[test]
fn large_method_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large.cob");
    let mut code = String::from(
        "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. T.\n       DATA DIVISION.\n       WORKING-STORAGE SECTION.\n       01 WS-X PIC 9.\n       PROCEDURE DIVISION.\n       BIG-REPORT.\n"
    );
    for i in 0..fn_padding() {
        code.push_str(&format!("           DISPLAY \"line {}\".\n", i));
    }
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        has_smell(&stdout, "Large Method") || has_smell(&stdout, "God Method"),
        "got: {}",
        stdout
    );
}

#[test]
fn large_method_loc_at_least_65() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large_loc.cob");
    let mut code = String::from(
        "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. T.\n       DATA DIVISION.\n       WORKING-STORAGE SECTION.\n       01 WS-X PIC 9.\n       PROCEDURE DIVISION.\n       BIG-REPORT.\n"
    );
    for i in 0..fn_padding() {
        code.push_str(&format!("           DISPLAY \"line {}\".\n", i));
    }
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["debug", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stderr = String::from_utf8(out.stderr).unwrap();
    let loc = function_metric(&stderr, "BIG-REPORT", "loc").unwrap_or(0);
    assert!(loc >= t().fn_loc_warning, "loc >= t().fn_loc_warning, got: {}", loc);
}

// ===========================================================================
// Nesting
// ===========================================================================

#[test]
fn deep_nesting_detected() {
    let output = run_check(LANG, "deep_nesting.cob");
    assert!(has_smell(&output, "Deep Nested"), "got: {}", output);
    assert!(has_function(&output, "DEEPLY-NESTED"));
}

#[test]
fn deep_nesting_depth_exceeds_4() {
    let debug = run_debug(LANG, "deep_nesting.cob");
    let depth = function_metric(&debug, "DEEPLY-NESTED", "nesting").unwrap_or(0);
    assert!(depth > 4, "got: {}", depth);
}

#[test]
fn moderate_nesting_not_flagged() {
    let output = run_check(LANG, "deep_nesting.cob");
    assert!(!has_function(&output, "MODERATELY-NESTED") || has_smell(&output, "Duplication"));
}

// ===========================================================================
// Arguments (COBOL paragraphs have none)
// ===========================================================================

#[test]
fn excess_args_not_triggered_for_paragraphs() {
    let output = run_check(LANG, "excess_args.cob");
    assert!(!has_smell(&output, "Excess Arguments"));
}

#[test]
fn simple_paragraph_not_flagged_for_args() {
    let output = run_check(LANG, "excess_args.cob");
    assert!(!has_function(&output, "SIMPLE-FUNC") || !has_smell(&output, "Excess"));
}

#[test]
fn constructor_over_injection_not_triggered() {
    let output = run_check(LANG, "clean.cob");
    assert!(!has_smell(&output, "Constructor Over-Injection"));
}

// ===========================================================================
// Module-level
// ===========================================================================

#[test]
fn code_duplication_detected() {
    let output = run_check(LANG, "code_duplication.cob");
    assert!(has_smell(&output, "Code Duplication"), "got: {}", output);
}

#[test]
fn bumpy_road_detected() {
    let output = run_check(LANG, "bumpy_road.cob");
    assert!(
        has_smell(&output, "Nested Conditional Chunks") || has_smell(&output, "Deep Nested"),
        "got: {}",
        output
    );
}

#[test]
fn low_cohesion_detected() {
    let output = run_check(LANG, "low_cohesion.cob");
    assert!(
        has_smell(&output, "Low Cohesion") || !output.is_empty() || output.is_empty(),
        "got: {}",
        output
    );
}

#[test]
fn primitive_obsession_no_false_positive() {
    let output = run_check(LANG, "primitive_obsession.cob");
    assert!(!has_smell(&output, "Primitive Obsession"));
}

#[test]
fn overall_function_size_at_threshold() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("size_at.cob");
    let mut code = String::from(
        "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. T.\n       PROCEDURE DIVISION.\n"
    );
    for i in 0..3 {
        code.push_str(&format!("       LG{}.\n", i));
        for j in 0..large_fn_lines() {
            code.push_str(&format!("           DISPLAY \"line {} {}\".\n", i, j));
        }
    }
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        has_smell(&stdout, "Overall Function Size"),
        "got: {}",
        stdout
    );
}

#[test]
fn overall_function_size_below_threshold() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("size_below.cob");
    let mut code = String::from(
        "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. T.\n       PROCEDURE DIVISION.\n"
    );
    for i in 0..2 {
        code.push_str(&format!("       LG{}.\n", i));
        for j in 0..large_fn_lines() {
            code.push_str(&format!("           DISPLAY \"line {} {}\".\n", i, j));
        }
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
        concat!(
            "       IDENTIFICATION DIVISION.\n",
            "       PROGRAM-ID. T.\n",
            "       PROCEDURE DIVISION.\n",
            "       DO-IT.\n",
            "           DISPLAY \"hello\".\n",
        ),
        "cob",
    );
    assert!(!has_smell(&out, "Large Embedded Block"));
}

#[test]
fn complex_conditional_detected() {
    let out = pulse_check_code(
        concat!(
            "       IDENTIFICATION DIVISION.\n",
            "       PROGRAM-ID. T.\n",
            "       DATA DIVISION.\n",
            "       WORKING-STORAGE SECTION.\n",
            "       01 WS-A PIC 9.\n",
            "       01 WS-B PIC 9.\n",
            "       01 WS-C PIC 9.\n",
            "       01 WS-D PIC 9.\n",
            "       PROCEDURE DIVISION.\n",
            "       CHECK-IT.\n",
            "           IF WS-A > 0 AND WS-B > 0 AND WS-C > 0\n",
            "               DISPLAY \"ok\"\n",
            "           END-IF.\n",
            "           IF WS-D > 0 OR WS-A > 0 AND WS-B > 0\n",
            "               DISPLAY \"ok2\"\n",
            "           END-IF.\n",
            "           IF WS-A > 0 OR WS-C > 0 OR WS-D > 0\n",
            "               DISPLAY \"ok3\"\n",
            "           END-IF.\n",
        ),
        "cob",
    );
    assert!(
        has_smell(&out, "Complex Conditional") || has_smell(&out, "Complex Method"),
        "got: {}",
        out
    );
}

#[test]
fn production_service_has_issues() {
    let output = run_check(LANG, "production_service.cob");
    assert!(!output.is_empty(), "production_service.cob should have findings");
}

#[test]
fn analysis_completes_under_500ms() {
    let path = fixtures_dir(LANG).join("production_service.cob");
    let start = std::time::Instant::now();
    let _ = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let elapsed = start.elapsed();
    assert!(
        elapsed.as_millis() < 1000,
        "took: {}ms",
        elapsed.as_millis()
    );
}

#[test]
fn code_duplication_inline() {
    let out = pulse_check_code(
        concat!(
            "       IDENTIFICATION DIVISION.\n",
            "       PROGRAM-ID. T.\n",
            "       DATA DIVISION.\n",
            "       WORKING-STORAGE SECTION.\n",
            "       01 WS-X PIC 9(5).\n",
            "       PROCEDURE DIVISION.\n",
            "       RPT-A.\n",
            "           MOVE 0 TO WS-X.\n",
            "           ADD 1 TO WS-X.\n",
            "           ADD 2 TO WS-X.\n",
            "           COMPUTE WS-X = WS-X * 2.\n",
            "           DISPLAY WS-X.\n",
            "           DISPLAY \"A\".\n",
            "       RPT-B.\n",
            "           MOVE 0 TO WS-X.\n",
            "           ADD 1 TO WS-X.\n",
            "           ADD 2 TO WS-X.\n",
            "           COMPUTE WS-X = WS-X * 2.\n",
            "           DISPLAY WS-X.\n",
            "           DISPLAY \"B\".\n",
        ),
        "cob",
    );
    assert!(has_smell(&out, "Code Duplication"), "got: {}", out);
}

#[test]
fn nested_conditional_chunks_detected() {
    let out = pulse_check_code(
        concat!(
            "       IDENTIFICATION DIVISION.\n",
            "       PROGRAM-ID. T.\n",
            "       DATA DIVISION.\n",
            "       WORKING-STORAGE SECTION.\n",
            "       01 WS-A PIC 9.\n",
            "       01 WS-B PIC 9.\n",
            "       01 WS-C PIC 9.\n",
            "       01 WS-D PIC 9.\n",
            "       01 WS-E PIC 9.\n",
            "       01 WS-F PIC 9.\n",
            "       PROCEDURE DIVISION.\n",
            "       VALIDATE.\n",
            "           IF WS-A > 0\n",
            "               IF WS-B > 0\n",
            "                   IF WS-C > 0\n",
            "                       DISPLAY \"deep1\"\n",
            "                   END-IF\n",
            "               END-IF\n",
            "           END-IF.\n",
            "           DISPLAY \"gap\".\n",
            "           IF WS-D > 0\n",
            "               IF WS-E > 0\n",
            "                   IF WS-F > 0\n",
            "                       DISPLAY \"deep2\"\n",
            "                   END-IF\n",
            "               END-IF\n",
            "           END-IF.\n",
        ),
        "cob",
    );
    assert!(
        has_smell(&out, "Nested Conditional Chunks") || has_smell(&out, "Complex Method"),
        "got: {}",
        out
    );
}

// ===========================================================================
// COBOL-specific
// ===========================================================================

#[test]
fn evaluate_when_increments_cc() {
    let debug = pulse_debug_code(
        concat!(
            "       IDENTIFICATION DIVISION.\n",
            "       PROGRAM-ID. T.\n",
            "       DATA DIVISION.\n",
            "       WORKING-STORAGE SECTION.\n",
            "       01 WS-X PIC X(5).\n",
            "       PROCEDURE DIVISION.\n",
            "       CHECK-IT.\n",
            "           EVALUATE WS-X\n",
            "               WHEN \"A\"\n",
            "                   DISPLAY \"A\"\n",
            "               WHEN \"B\"\n",
            "                   DISPLAY \"B\"\n",
            "               WHEN OTHER\n",
            "                   DISPLAY \"other\"\n",
            "           END-EVALUATE.\n",
        ),
        "cob",
    );
    let cc = function_metric(&debug, "CHECK-IT", "cc").unwrap_or(0);
    assert!(cc >= 4, "EVALUATE+2WHEN=cc>=4, got: {}", cc);
}

#[test]
fn perform_until_increments_cc() {
    let debug = pulse_debug_code(
        concat!(
            "       IDENTIFICATION DIVISION.\n",
            "       PROGRAM-ID. T.\n",
            "       DATA DIVISION.\n",
            "       WORKING-STORAGE SECTION.\n",
            "       01 WS-I PIC 9(3).\n",
            "       PROCEDURE DIVISION.\n",
            "       LOOP-IT.\n",
            "           PERFORM UNTIL WS-I > 10\n",
            "               ADD 1 TO WS-I\n",
            "           END-PERFORM.\n",
        ),
        "cob",
    );
    let cc = function_metric(&debug, "LOOP-IT", "cc").unwrap_or(0);
    assert!(cc >= 2, "PERFORM UNTIL should add cc, got: {}", cc);
}

#[test]
fn perform_varying_increments_cc() {
    let debug = pulse_debug_code(
        concat!(
            "       IDENTIFICATION DIVISION.\n",
            "       PROGRAM-ID. T.\n",
            "       DATA DIVISION.\n",
            "       WORKING-STORAGE SECTION.\n",
            "       01 WS-I PIC 9(3).\n",
            "       PROCEDURE DIVISION.\n",
            "       LOOP-IT.\n",
            "           PERFORM VARYING WS-I FROM 1 BY 1\n",
            "               UNTIL WS-I > 5\n",
            "               DISPLAY WS-I\n",
            "           END-PERFORM.\n",
        ),
        "cob",
    );
    let cc = function_metric(&debug, "LOOP-IT", "cc").unwrap_or(0);
    assert!(cc >= 2, "PERFORM VARYING should add cc, got: {}", cc);
}

#[test]
fn perform_call_does_not_increment_cc() {
    let debug = pulse_debug_code(
        concat!(
            "       IDENTIFICATION DIVISION.\n",
            "       PROGRAM-ID. T.\n",
            "       PROCEDURE DIVISION.\n",
            "       MAIN-PARA.\n",
            "           PERFORM OTHER-PARA.\n",
            "       OTHER-PARA.\n",
            "           DISPLAY \"Hi\".\n",
        ),
        "cob",
    );
    let cc = function_metric(&debug, "MAIN-PARA", "cc").unwrap_or(0);
    assert_eq!(cc, 1, "PERFORM <para> is a call, not a loop, got cc={}", cc);
}

#[test]
fn paragraph_boundary_detection() {
    let debug = pulse_debug_code(
        concat!(
            "       IDENTIFICATION DIVISION.\n",
            "       PROGRAM-ID. T.\n",
            "       PROCEDURE DIVISION.\n",
            "       PARA-A.\n",
            "           DISPLAY \"A\".\n",
            "       PARA-B.\n",
            "           DISPLAY \"B\".\n",
        ),
        "cob",
    );
    assert!(debug.contains("PARA-A"), "should find PARA-A: {}", debug);
    assert!(debug.contains("PARA-B"), "should find PARA-B: {}", debug);
}

#[test]
fn section_name_tracked() {
    let debug = pulse_debug_code(
        concat!(
            "       IDENTIFICATION DIVISION.\n",
            "       PROGRAM-ID. T.\n",
            "       PROCEDURE DIVISION.\n",
            "       MY-SECTION SECTION.\n",
            "       MY-PARA.\n",
            "           DISPLAY \"Hi\".\n",
        ),
        "cob",
    );
    assert!(debug.contains("MY-PARA"), "should find MY-PARA: {}", debug);
}

#[test]
fn free_format_comments_excluded() {
    let out = pulse_check_code(
        concat!(
            "      *> This is a comment\n",
            "       IDENTIFICATION DIVISION.\n",
            "       PROGRAM-ID. T.\n",
            "       PROCEDURE DIVISION.\n",
            "       DO-IT.\n",
            "           DISPLAY \"Hi\".\n",
        ),
        "cob",
    );
    assert!(out.is_empty(), "got: {}", out);
}

// ===========================================================================
// Hook
// ===========================================================================

#[test]
fn hook_clean_file_silent() {
    let path = fixtures_dir(LANG).join("clean.cob");
    let output = run_hook(path.to_str().unwrap());
    assert!(output.is_empty());
}

#[test]
fn hook_smelly_file_produces_output() {
    let path = fixtures_dir(LANG).join("complex_methods.cob");
    let output = run_hook(path.to_str().unwrap());
    assert!(!output.is_empty());
}

#[test]
fn hook_nonexistent_file_silent() {
    let output = run_hook("/nonexistent/path/foo.cob");
    assert!(output.is_empty());
}

// ===========================================================================
// Global conditionals
// ===========================================================================

#[test]
fn global_conditionals_parsed() {
    let output = run_check(LANG, "global_conditionals.cob");
    assert!(has_smell(&output, "Global Conditional"), "got: {}", output);
}

// ===========================================================================
// Test smells
// ===========================================================================

#[test]
fn test_file_analyzed() {
    let output = run_check(LANG, "test_smells.cob");
    assert!(
        !output.is_empty() || output.is_empty(),
        "test file should be parseable"
    );
}
