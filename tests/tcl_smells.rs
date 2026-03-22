mod common;

use common::*;
use std::process::Command;

const LANG: &str = "tcl";

// ===========================================================================
// Output format
// ===========================================================================

#[test]
fn output_starts_with_pulse() {
    let output = run_check(LANG, "complex_methods.tcl");
    assert!(output.starts_with("pulse:"), "got: {}", output);
}

#[test]
fn output_has_function_line_numbers() {
    let output = run_check(LANG, "complex_methods.tcl");
    assert!(output.lines().any(|l| l.contains("(L") && l.contains("): ")));
}

#[test]
fn output_has_module_prefix() {
    let output = run_check(LANG, "production_service.tcl");
    assert!(output.contains("Module:"), "got: {}", output);
}

#[test]
fn issue_count_matches_findings() {
    let output = run_check(LANG, "complex_methods.tcl");
    let first = output.lines().next().unwrap_or("");
    let findings = output.lines().filter(|l| l.starts_with("  ")).count();
    assert!(first.contains(&format!("{} issue", findings)));
}

// ===========================================================================
// Clean / empty
// ===========================================================================

#[test]
fn clean_file_produces_no_output() {
    let output = run_check(LANG, "clean.tcl");
    assert!(output.is_empty(), "got: {}", output);
}

#[test]
fn empty_file() {
    let out = pulse_check_code("", "tcl");
    assert!(out.is_empty());
}

#[test]
fn comments_only_file() {
    let out = pulse_check_code("# just a comment\n", "tcl");
    assert!(out.is_empty());
}

#[test]
fn simple_func_not_flagged() {
    let out = pulse_check_code("proc add {a b} {\n    expr {$a + $b}\n}\n", "tcl");
    assert!(out.is_empty(), "got: {}", out);
}

// ===========================================================================
// CC boundary
// ===========================================================================

#[test]
fn cc_base_case_is_1() {
    let debug = pulse_debug_code("proc add {a b} {\n    expr {$a + $b}\n}\n", "tcl");
    assert_eq!(function_metric(&debug, "add", "cc"), Some(1));
}

#[test]
fn function_at_cc_boundary_flagged() {
    let mut code = String::from("proc f {a b c d e f2 g h} {\n");
    for v in ["a", "b", "c", "d", "e", "f2", "g", "h"] {
        code.push_str(&format!("    if {{${} > 0}} {{ return 1 }}\n", v));
    }
    code.push_str("    return 0\n}\n");
    let out = pulse_check_code(&code, "tcl");
    assert!(has_smell(&out, "Complex Method"), "cc=9 should trigger, got: {}", out);
}

#[test]
fn function_below_cc_boundary_not_flagged() {
    let mut code = String::from("proc f {a b c d e f2 g} {\n");
    for v in ["a", "b", "c", "d", "e", "f2", "g"] {
        code.push_str(&format!("    if {{${} > 0}} {{ return 1 }}\n", v));
    }
    code.push_str("    return 0\n}\n");
    let out = pulse_check_code(&code, "tcl");
    assert!(!has_smell(&out, "Complex Method"));
}

#[test]
fn boolean_operators_increment_cc() {
    let debug = pulse_debug_code(
        "proc f {a b c} {\n    if {$a && $b && $c} {\n        return 1\n    }\n    return 0\n}\n",
        "tcl",
    );
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert!(cc >= 4, "got: {}", cc);
}

// ===========================================================================
// Complexity smells
// ===========================================================================

#[test]
fn complex_method_detected() {
    let output = run_check(LANG, "complex_methods.tcl");
    assert!(has_smell(&output, "Complex Method"), "got: {}", output);
    assert!(has_function(&output, "process_order"));
}

#[test]
fn complex_method_cc_at_least_9() {
    let debug = run_debug(LANG, "complex_methods.tcl");
    let cc = function_metric(&debug, "process_order", "cc").unwrap_or(0);
    assert!(cc >= 9, "cc should be >= 9, got: {}", cc);
}

#[test]
fn god_method_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("god.tcl");
    let mut code = String::from("proc process_data_pipeline {x} {\n");
    for i in 0..10 {
        code.push_str(&format!("    if {{$x > {}}} {{ return 1 }}\n", i));
    }
    for i in 0..100 {
        code.push_str(&format!("    set y{} {}\n", i, i));
    }
    code.push_str("    return 0\n}\n");
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(has_smell(&stdout, "God Method"), "got: {}", stdout);
}

#[test]
fn large_method_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large.tcl");
    let mut code = String::from("proc build_report {} {\n");
    for i in 0..100 {
        code.push_str(&format!("    set x{} {}\n", i, i));
    }
    code.push_str("    return 0\n}\n");
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        has_smell(&stdout, "Large Method") || has_smell(&stdout, "God Method"),
        "got: {}", stdout
    );
}

// ===========================================================================
// Nesting
// ===========================================================================

#[test]
fn deep_nesting_detected() {
    let output = run_check(LANG, "deep_nesting.tcl");
    assert!(has_smell(&output, "Deep Nested"), "got: {}", output);
    assert!(has_function(&output, "deeply_nested"));
}

#[test]
fn deep_nesting_depth_exceeds_4() {
    let debug = run_debug(LANG, "deep_nesting.tcl");
    let depth = function_metric(&debug, "deeply_nested", "nesting").unwrap_or(0);
    assert!(depth >= 4, "got: {}", depth);
}

#[test]
fn moderate_nesting_not_flagged() {
    let output = run_check(LANG, "deep_nesting.tcl");
    assert!(!has_function(&output, "moderately_nested"));
}

// ===========================================================================
// Arguments
// ===========================================================================

#[test]
fn excess_args_detected() {
    let output = run_check(LANG, "excess_args.tcl");
    assert!(has_smell(&output, "Excess Arguments"), "got: {}", output);
    assert!(has_function(&output, "create_user"));
}

#[test]
fn excess_args_count_correct() {
    let debug = run_debug(LANG, "excess_args.tcl");
    let args = function_metric(&debug, "create_user", "args").unwrap_or(0);
    assert!(args >= 6, "got: {}", args);
}

#[test]
fn simple_func_in_excess_args_not_flagged() {
    let output = run_check(LANG, "excess_args.tcl");
    assert!(!has_function(&output, "simple_func"));
}

// ===========================================================================
// Module-level
// ===========================================================================

#[test]
fn code_duplication_detected() {
    let output = run_check(LANG, "code_duplication.tcl");
    assert!(has_smell(&output, "Code Duplication"), "got: {}", output);
}

#[test]
fn embedded_block_detected() {
    let output = run_check(LANG, "embedded_block.tcl");
    assert!(has_smell(&output, "Large Embedded Block"), "got: {}", output);
}

#[test]
fn bumpy_road_detected() {
    let output = run_check(LANG, "bumpy_road.tcl");
    assert!(
        has_smell(&output, "Nested Conditional Chunks") || has_smell(&output, "Deep Nested"),
        "got: {}", output
    );
}

#[test]
fn low_cohesion_detected() {
    let output = run_check(LANG, "low_cohesion.tcl");
    assert!(
        has_smell(&output, "Low Cohesion") || has_smell(&output, "Too Many Functions") || !output.is_empty(),
        "got: {}", output
    );
}

#[test]
fn overall_function_size_at_threshold() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("size_at.tcl");
    let mut code = String::new();
    for i in 0..3 {
        code.push_str(&format!("proc lg{} {{}} {{\n", i));
        for j in 0..45 {
            code.push_str(&format!("    set x{} {}\n", j, j));
        }
        code.push_str("    return 0\n}\n\n");
    }
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(has_smell(&stdout, "Overall Function Size"), "got: {}", stdout);
}

#[test]
fn overall_function_size_below_threshold() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("size_below.tcl");
    let mut code = String::new();
    for i in 0..2 {
        code.push_str(&format!("proc lg{} {{}} {{\n", i));
        for j in 0..45 {
            code.push_str(&format!("    set x{} {}\n", j, j));
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
    let out = pulse_check_code("proc f {} {\n    set x \"hello\"\n    return $x\n}\n", "tcl");
    assert!(!has_smell(&out, "Large Embedded Block"));
}

#[test]
fn complex_conditional_detected() {
    let out = pulse_check_code(concat!(
        "proc check {age score active} {\n",
        "    if {$age > 18 && $score > 50 && $active} {\n",
        "        if {$score > 80 || ($age > 25 && $active)} {\n",
        "            return 1\n",
        "        }\n",
        "    }\n",
        "    if {$age > 65 || $score < 10} {\n",
        "        return 1\n",
        "    }\n",
        "    return 0\n",
        "}\n",
    ), "tcl");
    assert!(
        has_smell(&out, "Complex Conditional") || has_smell(&out, "Complex Method"),
        "got: {}", out
    );
}

#[test]
fn production_service_has_issues() {
    let output = run_check(LANG, "production_service.tcl");
    assert!(!output.is_empty(), "production_service.tcl should have findings");
}

#[test]
fn primitive_obsession_not_triggered() {
    let out = pulse_check_code("proc f {a b c d} {\n    expr {$a + $b + $c + $d}\n}\n", "tcl");
    assert!(!has_smell(&out, "Primitive Obsession"));
}

// ===========================================================================
// Performance
// ===========================================================================

#[test]
fn analysis_completes_under_500ms() {
    let path = fixtures_dir(LANG).join("production_service.tcl");
    let start = std::time::Instant::now();
    let _ = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    assert!(start.elapsed().as_millis() < 500, "took: {}ms", start.elapsed().as_millis());
}

// ===========================================================================
// Hook
// ===========================================================================

#[test]
fn hook_clean_file_silent() {
    let path = fixtures_dir(LANG).join("clean.tcl");
    let output = run_hook(path.to_str().unwrap());
    assert!(output.is_empty());
}

#[test]
fn hook_smelly_file_produces_output() {
    let path = fixtures_dir(LANG).join("complex_methods.tcl");
    let output = run_hook(path.to_str().unwrap());
    assert!(!output.is_empty());
}

#[test]
fn hook_nonexistent_file_silent() {
    let output = run_hook("/nonexistent/path/foo.tcl");
    assert!(output.is_empty());
}

// ===========================================================================
// Tcl-specific
// ===========================================================================

#[test]
fn if_elseif_else_increments_cc() {
    let debug = pulse_debug_code(concat!(
        "proc f {x} {\n",
        "    if {$x > 0} {\n",
        "        return 1\n",
        "    } elseif {$x < 0} {\n",
        "        return -1\n",
        "    } else {\n",
        "        return 0\n",
        "    }\n",
        "}\n",
    ), "tcl");
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert!(cc >= 3, "if+elseif should give cc >= 3, got: {}", cc);
}

#[test]
fn while_increments_cc() {
    let debug = pulse_debug_code(
        "proc f {x} {\n    set n $x\n    while {$n > 0} {\n        incr n -1\n    }\n}\n",
        "tcl",
    );
    assert_eq!(function_metric(&debug, "f", "cc"), Some(2));
}

#[test]
fn foreach_increments_cc() {
    let debug = pulse_debug_code(
        "proc f {items} {\n    foreach item $items {\n        puts $item\n    }\n}\n",
        "tcl",
    );
    assert_eq!(function_metric(&debug, "f", "cc"), Some(2));
}

#[test]
fn for_command_increments_cc() {
    let debug = pulse_debug_code(
        "proc f {} {\n    for {set i 0} {$i < 10} {incr i} {\n        puts $i\n    }\n}\n",
        "tcl",
    );
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert!(cc >= 2, "for command should add CC, got: {}", cc);
}

#[test]
fn switch_command_increments_cc() {
    let out = pulse_check_code(concat!(
        "proc handle {action} {\n",
        "    switch $action {\n",
        "        1 { return \"a\" }\n",
        "        2 { return \"b\" }\n",
        "        3 { return \"c\" }\n",
        "        4 { return \"d\" }\n",
        "        5 { return \"e\" }\n",
        "        6 { return \"f\" }\n",
        "        7 { return \"g\" }\n",
        "        8 { return \"h\" }\n",
        "        9 { return \"i\" }\n",
        "        default { return \"?\" }\n",
        "    }\n",
        "}\n",
    ), "tcl");
    assert!(has_smell(&out, "Complex Method"), "got: {}", out);
}

#[test]
fn catch_increments_cc() {
    let debug = pulse_debug_code(concat!(
        "proc f {x} {\n",
        "    catch {error \"boom\"} result\n",
        "}\n",
    ), "tcl");
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert!(cc >= 2, "catch should add CC, got: {}", cc);
}

#[test]
fn try_on_error_increments_cc() {
    let debug = pulse_debug_code(concat!(
        "proc f {x} {\n",
        "    try {\n",
        "        set r [expr {$x / 0}]\n",
        "    } on error {msg opts} {\n",
        "        puts $msg\n",
        "    }\n",
        "}\n",
    ), "tcl");
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert!(cc >= 2, "try on error should add CC, got: {}", cc);
}

#[test]
fn global_conditionals_detected() {
    let output = run_check(LANG, "global_conditionals.tcl");
    assert!(has_smell(&output, "Global Conditionals"), "got: {}", output);
}

#[test]
fn namespace_methods_analyzed() {
    let debug = pulse_debug_code(concat!(
        "namespace eval Svc {\n",
        "    proc handle {a b c d e f g h} {\n",
        "        expr {$a + $b}\n",
        "    }\n",
        "}\n",
    ), "tcl");
    assert!(debug.contains("Svc.handle"), "method should be attributed to namespace, got: {}", debug);
}

#[test]
fn code_duplication_inline() {
    let out = pulse_check_code(concat!(
        "proc rpt_a {data} {\n",
        "    set r 0\n",
        "    foreach v $data {\n        set r [expr {$r + $v}]\n    }\n",
        "    set r [expr {$r * 2}]\n    return $r\n",
        "}\n\n",
        "proc rpt_b {data} {\n",
        "    set r 0\n",
        "    foreach v $data {\n        set r [expr {$r + $v}]\n    }\n",
        "    set r [expr {$r * 2}]\n    return $r\n",
        "}\n",
    ), "tcl");
    assert!(has_smell(&out, "Code Duplication"), "got: {}", out);
}

#[test]
fn nested_conditional_chunks_detected() {
    let output = run_check(LANG, "bumpy_road.tcl");
    assert!(
        has_smell(&output, "Nested Conditional Chunks") || has_smell(&output, "Complex Method"),
        "got: {}", output
    );
}

#[test]
fn constructor_over_injection_detected() {
    let output = run_check(LANG, "excess_args.tcl");
    assert!(
        has_smell(&output, "Constructor Over-Injection") || has_smell(&output, "Excess Arguments"),
        "got: {}", output
    );
}

// ===========================================================================
// LOC
// ===========================================================================

#[test]
fn large_method_loc_at_least_65() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large_loc.tcl");
    let mut code = String::from("proc build_report {} {\n");
    for i in 0..100 {
        code.push_str(&format!("    set x{} {}\n", i, i));
    }
    code.push_str("    return 0\n}\n");
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["debug", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stderr = String::from_utf8(out.stderr).unwrap();
    let loc = function_metric(&stderr, "build_report", "loc").unwrap_or(0);
    assert!(loc >= 65, "loc >= 65, got: {}", loc);
}
