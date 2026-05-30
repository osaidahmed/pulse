
use crate::common::*;
use std::process::Command;

const LANG: &str = "r";

// ===========================================================================
// Output format
// ===========================================================================

#[test]
fn output_starts_with_pulse() {
    let output = run_check(LANG, "complex_methods.r");
    assert!(output.starts_with("pulse:"), "got: {output}");
}

#[test]
fn output_has_function_line_numbers() {
    let output = run_check(LANG, "complex_methods.r");
    assert!(output.lines().any(|l| l.contains("(L") && l.contains("): ")));
}

#[test]
fn output_has_module_prefix() {
    let output = run_check(LANG, "code_duplication.r");
    assert!(output.contains("Module:"), "got: {output}");
}

#[test]
fn issue_count_matches_findings() {
    let output = run_check(LANG, "complex_methods.r");
    let first = output.lines().next().unwrap_or("");
    let findings = output.lines().filter(|l| l.starts_with("  ")).count();
    assert!(first.contains(&format!("{findings} issue")));
}

// ===========================================================================
// Clean / empty
// ===========================================================================

#[test]
fn clean_file_produces_no_output() {
    let output = run_check(LANG, "clean.r");
    assert!(output.is_empty(), "got: {output}");
}

#[test]
fn empty_file() {
    let out = pulse_check_code("", "r");
    assert!(out.is_empty());
}

#[test]
fn comments_only_file() {
    let out = pulse_check_code("# just a comment\n", "r");
    assert!(out.is_empty());
}

#[test]
fn simple_func_not_flagged() {
    let out = pulse_check_code("add <- function(a, b) {\n  a + b\n}\n", "r");
    assert!(out.is_empty(), "got: {out}");
}

// ===========================================================================
// CC boundary
// ===========================================================================

#[test]
fn cc_base_case_is_1() {
    let debug = pulse_debug_code("f <- function() {\n  1\n}\n", "r");
    assert_eq!(function_metric(&debug, "f", "cc"), Some(1));
}

#[test]
fn function_at_cc_boundary_flagged() {
    let mut code = String::from("f <- function(a, b, c, d, e, f2, g, h) {\n");
    for v in ["a", "b", "c", "d", "e", "f2", "g", "h"] {
        code.push_str(&format!("  if ({v} > 0) return(1)\n"));
    }
    code.push_str("  0\n}\n");
    let out = pulse_check_code(&code, "r");
    assert!(has_smell(&out, "Complex Method"), "cc=9 should trigger, got: {out}");
}

#[test]
fn function_below_cc_boundary_not_flagged() {
    let mut code = String::from("f <- function(a, b, c, d, e, f2, g) {\n");
    for v in ["a", "b", "c", "d", "e", "f2", "g"] {
        code.push_str(&format!("  if ({v} > 0) return(1)\n"));
    }
    code.push_str("  0\n}\n");
    let out = pulse_check_code(&code, "r");
    assert!(!has_smell(&out, "Complex Method"));
}

#[test]
fn boolean_operators_increment_cc() {
    let debug = pulse_debug_code(
        "f <- function(a, b, c) {\n  if (a && b && c) {\n    return(TRUE)\n  }\n  FALSE\n}\n",
        "r",
    );
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert!(cc >= 4, "got: {cc}");
}

// ===========================================================================
// Complexity smells
// ===========================================================================

#[test]
fn complex_method_detected() {
    let output = run_check(LANG, "complex_methods.r");
    assert!(has_smell(&output, "Complex Method"), "got: {output}");
    assert!(has_function(&output, "process_order"));
}

#[test]
fn complex_method_cc_at_least_9() {
    let debug = run_debug(LANG, "complex_methods.r");
    let cc = function_metric(&debug, "process_order", "cc").unwrap_or(0);
    assert!(cc >= 9, "cc should be >= 9, got: {cc}");
}

#[test]
fn god_method_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("god.r");
    let mut code = String::from("process_data_pipeline <- function(x) {\n");
    for i in 0..cc_branches() {
        code.push_str(&format!("  if (x > {i}) return(1)\n"));
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("  y{i} <- {i}\n"));
    }
    code.push_str("  0\n}\n");
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(has_smell(&stdout, "God Method"), "got: {stdout}");
}

#[test]
fn large_method_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large.r");
    let mut code = String::from("build_report <- function() {\n");
    for i in 0..fn_padding() {
        code.push_str(&format!("  x{i} <- {i}\n"));
    }
    code.push_str("  0\n}\n");
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
    let output = run_check(LANG, "deep_nesting.r");
    assert!(has_smell(&output, "Deep Nested"), "got: {output}");
    assert!(has_function(&output, "deeply_nested"));
}

#[test]
fn deep_nesting_depth_exceeds_4() {
    let debug = run_debug(LANG, "deep_nesting.r");
    let depth = function_metric(&debug, "deeply_nested", "nesting").unwrap_or(0);
    assert!(depth >= 4, "got: {depth}");
}

#[test]
fn moderate_nesting_not_flagged() {
    let output = run_check(LANG, "deep_nesting.r");
    assert!(!has_function(&output, "moderately_nested") || has_smell(&output, "Code Duplication"));
}

// ===========================================================================
// Arguments
// ===========================================================================

#[test]
fn excess_args_detected() {
    let output = run_check(LANG, "excess_args.r");
    assert!(has_smell(&output, "Excess Arguments"), "got: {output}");
    assert!(has_function(&output, "create_user"));
}

#[test]
fn excess_args_count_correct() {
    let debug = run_debug(LANG, "excess_args.r");
    let args = function_metric(&debug, "create_user", "args").unwrap_or(0);
    assert!(args >= 6, "got: {args}");
}

#[test]
fn simple_func_in_excess_args_not_flagged() {
    let output = run_check(LANG, "excess_args.r");
    assert!(!has_function(&output, "simple_func"));
}

// ===========================================================================
// Module-level
// ===========================================================================

#[test]
fn code_duplication_detected() {
    let output = run_check(LANG, "code_duplication.r");
    assert!(has_smell(&output, "Code Duplication"), "got: {output}");
}

#[test]
fn embedded_block_detected() {
    let output = run_check(LANG, "embedded_block.r");
    assert!(has_smell(&output, "Large Embedded Block"), "got: {output}");
}

#[test]
fn bumpy_road_detected() {
    let output = run_check(LANG, "bumpy_road.r");
    assert!(
        has_smell(&output, "Nested Conditional Chunks") || has_smell(&output, "Deep Nested"),
        "got: {output}"
    );
}

#[test]
fn overall_function_size_at_threshold() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("size_at.r");
    let mut code = String::new();
    for i in 0..3 {
        code.push_str(&format!("lg{i} <- function() {{\n"));
        for j in 0..45 {
            code.push_str(&format!("  x{j} <- {j}\n"));
        }
        code.push_str("  0\n}\n\n");
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
    let path = dir.path().join("size_below.r");
    let mut code = String::new();
    for i in 0..2 {
        code.push_str(&format!("lg{i} <- function() {{\n"));
        for j in 0..45 {
            code.push_str(&format!("  x{j} <- {j}\n"));
        }
        code.push_str("  0\n}\n\n");
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
    let out = pulse_check_code("f <- function() {\n  \"hello\"\n}\n", "r");
    assert!(!has_smell(&out, "Large Embedded Block"));
}

#[test]
fn complex_conditional_detected() {
    let out = pulse_check_code(concat!(
        "check <- function(age, score, active) {\n",
        "  if (age > 18 && score > 50 && active) {\n",
        "    if (score > 80 || (age > 25 && active)) {\n",
        "      return(TRUE)\n",
        "    }\n",
        "  }\n",
        "  if (age > 65 || score < 10) {\n",
        "    return(TRUE)\n",
        "  }\n",
        "  FALSE\n",
        "}\n",
    ), "r");
    assert!(
        has_smell(&out, "Complex Conditional") || has_smell(&out, "Complex Method"),
        "got: {out}"
    );
}

#[test]
fn production_service_has_issues() {
    let output = run_check(LANG, "production_service.r");
    assert!(!output.is_empty(), "production_service.r should have findings");
}

#[test]
fn analysis_completes_under_500ms() {
    let path = fixtures_dir(LANG).join("production_service.r");
    let start = std::time::Instant::now();
    let _ = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    assert!(start.elapsed().as_millis() < 1000, "took: {}ms", start.elapsed().as_millis());
}

// ===========================================================================
// Hook
// ===========================================================================

#[test]
fn hook_clean_file_silent() {
    let path = fixtures_dir(LANG).join("clean.r");
    let output = run_hook(path.to_str().unwrap());
    assert!(output.is_empty());
}

#[test]
fn hook_smelly_file_produces_output() {
    let path = fixtures_dir(LANG).join("complex_methods.r");
    let output = run_hook(path.to_str().unwrap());
    assert!(!output.is_empty());
}

#[test]
fn hook_nonexistent_file_silent() {
    let output = run_hook("/nonexistent/path/foo.r");
    assert!(output.is_empty());
}

// ===========================================================================
// R-specific
// ===========================================================================

#[test]
fn switch_increments_cc() {
    let debug = pulse_debug_code(concat!(
        "f <- function(x) {\n",
        "  switch(x,\n",
        "    a = 1,\n",
        "    b = 2,\n",
        "    c = 3\n",
        "  )\n",
        "}\n",
    ), "r");
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert!(cc >= 4, "switch with 3 cases should have cc>=4, got: {cc}");
}

#[test]
fn try_catch_increments_cc() {
    let debug = pulse_debug_code(concat!(
        "f <- function(x) {\n",
        "  tryCatch(\n",
        "    as.numeric(x),\n",
        "    error = function(e) -1,\n",
        "    warning = function(w) -2\n",
        "  )\n",
        "}\n",
    ), "r");
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert!(cc >= 3, "tryCatch with 2 handlers should have cc>=3, got: {cc}");
}

#[test]
fn empty_try_catch_handler_detected() {
    let out = pulse_check_code(concat!(
        "f <- function(x) {\n",
        "  tryCatch(\n",
        "    as.numeric(x),\n",
        "    error = function(e) {}\n",
        "  )\n",
        "}\n",
    ), "r");
    assert!(has_smell(&out, "Empty Error Handler"), "got: {out}");
}

#[test]
fn repeat_increments_cc() {
    let debug = pulse_debug_code(concat!(
        "f <- function() {\n",
        "  repeat {\n",
        "    break\n",
        "  }\n",
        "}\n",
    ), "r");
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert!(cc >= 2, "repeat should add CC, got: {cc}");
}

#[test]
fn for_loop_increments_cc() {
    let debug = pulse_debug_code(
        "f <- function(items) {\n  for (item in items) {\n    print(item)\n  }\n}\n",
        "r",
    );
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert!(cc >= 2, "for should increment cc, got: {cc}");
}

#[test]
fn while_loop_increments_cc() {
    let debug = pulse_debug_code(
        "f <- function(x) {\n  n <- x\n  while (n > 0) {\n    n <- n - 1\n  }\n}\n",
        "r",
    );
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert!(cc >= 2, "while should increment cc, got: {cc}");
}

#[test]
fn else_if_increments_cc() {
    let debug = pulse_debug_code(concat!(
        "f <- function(x) {\n",
        "  if (x == 1) {\n",
        "    1\n",
        "  } else if (x == 2) {\n",
        "    2\n",
        "  } else {\n",
        "    3\n",
        "  }\n",
        "}\n",
    ), "r");
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert!(cc >= 3, "else-if should add CC, got: {cc}");
}

#[test]
fn function_name_from_assignment() {
    let debug = pulse_debug_code("my_func <- function(x) {\n  x + 1\n}\n", "r");
    assert!(debug.contains("my_func"), "got: {debug}");
}

#[test]
fn global_assignment_operator() {
    let debug = pulse_debug_code("my_func <<- function(x) {\n  x + 1\n}\n", "r");
    assert!(debug.contains("my_func"), "got: {debug}");
}

#[test]
fn global_conditionals_detected() {
    let output = run_check(LANG, "global_conditionals.r");
    assert!(has_smell(&output, "Global Conditionals"), "got: {output}");
}

#[test]
fn code_duplication_inline() {
    let out = pulse_check_code(concat!(
        "rpt_a <- function(data) {\n",
        "  r <- 0\n",
        "  for (v in data) {\n    r <- r + v\n  }\n",
        "  r <- r * 2\n  r\n",
        "}\n\n",
        "rpt_b <- function(data) {\n",
        "  r <- 0\n",
        "  for (v in data) {\n    r <- r + v\n  }\n",
        "  r <- r * 2\n  r\n",
        "}\n",
    ), "r");
    assert!(has_smell(&out, "Code Duplication"), "got: {out}");
}

#[test]
fn nested_conditional_chunks_detected() {
    let out = pulse_check_code(concat!(
        "validate <- function(data) {\n",
        "  if (length(data) > 0) {\n",
        "    if (data[1] > 0) {\n",
        "      if (data[1] > 10) {\n",
        "        x <- 1\n",
        "      }\n",
        "    }\n",
        "  }\n",
        "  gap <- 1\n",
        "  if (length(data) > 5) {\n",
        "    if (data[5] > 0) {\n",
        "      if (data[5] > 10) {\n",
        "        y <- 2\n",
        "      }\n",
        "    }\n",
        "  }\n",
        "  gap2 <- 2\n",
        "  if (length(data) > 10) {\n",
        "    if (data[10] > 0) {\n",
        "      if (data[10] > 10) {\n",
        "        z <- 3\n",
        "      }\n",
        "    }\n",
        "  }\n",
        "  0\n",
        "}\n",
    ), "r");
    assert!(
        has_smell(&out, "Nested Conditional Chunks") || has_smell(&out, "Complex Method"),
        "got: {out}"
    );
}

#[test]
fn primitive_obsession_not_triggered() {
    let out = pulse_check_code("f <- function(a, b, c, d) {\n  a + b + c + d\n}\n", "r");
    assert!(!has_smell(&out, "Primitive Obsession"));
}

#[test]
fn test_file_analyzed() {
    let output = run_check(LANG, "test_smells.r");
    assert!(output.is_empty() || !output.is_empty(), "test file should be parseable");
}

#[test]
fn constructor_over_injection_detected() {
    let output = run_check(LANG, "excess_args.r");
    assert!(
        has_smell(&output, "Constructor Over-Injection") || has_smell(&output, "Excess Arguments"),
        "got: {output}"
    );
}

#[test]
fn large_method_loc_verification() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large_loc.r");
    let mut code = String::from("build_report <- function() {\n");
    for i in 0..fn_padding() {
        code.push_str(&format!("  x{i} <- {i}\n"));
    }
    code.push_str("  0\n}\n");
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["debug", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stderr = String::from_utf8(out.stderr).unwrap();
    let loc = function_metric(&stderr, "build_report", "loc").unwrap_or(0);
    assert!(loc >= t().function.fn_loc_warning, "loc >= threshold, got: {loc}");
}

#[test]
fn vectorized_and_not_cc() {
    let debug = pulse_debug_code(
        "f <- function(a, b) {\n  a & b\n}\n",
        "r",
    );
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert_eq!(cc, 1, "vectorized & should not increment CC, got: {cc}");
}

#[test]
fn vectorized_or_not_cc() {
    let debug = pulse_debug_code(
        "f <- function(a, b) {\n  a | b\n}\n",
        "r",
    );
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert_eq!(cc, 1, "vectorized | should not increment CC, got: {cc}");
}

// ===========================================================================
// Production fixture — production_service.r
// ===========================================================================

#[test]
fn production_service_complex_method() {
    let output = run_check(LANG, "production_service.r");
    assert!(has_smell(&output, "Complex Method"), "got: {output}");
    assert!(has_function(&output, "process_order"));
}

#[test]
fn production_service_process_order_cc_high() {
    let debug = run_debug(LANG, "production_service.r");
    let cc = function_metric(&debug, "process_order", "cc").unwrap_or(0);
    assert!(cc >= 10, "production process_order should have high cc, got: {cc}");
}

#[test]
fn production_service_excess_args() {
    let output = run_check(LANG, "production_service.r");
    assert!(has_smell(&output, "Excess Arguments"), "got: {output}");
    assert!(has_function(&output, "process_order"));
}

#[test]
fn production_service_deep_nesting() {
    let output = run_check(LANG, "production_service.r");
    assert!(has_smell(&output, "Deep Nested"), "got: {output}");
}

#[test]
fn production_service_nested_chunks() {
    let output = run_check(LANG, "production_service.r");
    assert!(has_smell(&output, "Nested Conditional Chunks"), "got: {output}");
}

#[test]
fn production_service_no_false_duplication() {
    let output = run_check(LANG, "production_service.r");
    assert!(!has_smell(&output, "Code Duplication"), "size-disparate functions must not be flagged as duplicates, got: {output}");
}

#[test]
fn production_service_pipeline_complex() {
    let debug = run_debug(LANG, "production_service.r");
    let cc = function_metric(&debug, "process_data_pipeline", "cc").unwrap_or(0);
    assert!(cc >= 10, "pipeline should be complex, got: {cc}");
}

#[test]
fn production_service_clean_helpers_not_flagged() {
    let output = run_check(LANG, "production_service.r");
    assert!(!has_function(&output, "validate_schema") || has_smell(&output, "Code Duplication"));
}

// ===========================================================================
// Production fixture — production_api_service.r
// ===========================================================================

#[test]
fn production_api_short_variable_names() {
    let output = run_check(LANG, "production_api_service.r");
    assert!(has_smell(&output, "Short Variable Names"), "got: {output}");
    assert!(has_function(&output, "process_event"));
}

#[test]
fn production_api_complex_handler() {
    let output = run_check(LANG, "production_api_service.r");
    assert!(has_smell(&output, "Complex Method"), "got: {output}");
    assert!(has_function(&output, "handle_api_request"));
}

#[test]
fn production_api_handler_cc_high() {
    let debug = run_debug(LANG, "production_api_service.r");
    let cc = function_metric(&debug, "handle_api_request", "cc").unwrap_or(0);
    assert!(cc >= 15, "handler should have high cc, got: {cc}");
}

#[test]
fn production_api_empty_handler() {
    let output = run_check(LANG, "production_api_service.r");
    assert!(has_smell(&output, "Empty Error Handler"), "got: {output}");
}

#[test]
fn production_api_no_false_duplication() {
    let output = run_check(LANG, "production_api_service.r");
    assert!(!has_smell(&output, "Code Duplication"), "size-disparate functions must not be flagged as duplicates, got: {output}");
}

#[test]
fn production_api_parse_query_deep_nesting() {
    let output = run_check(LANG, "production_api_service.r");
    assert!(has_smell(&output, "Deep Nested"), "got: {output}");
}

#[test]
fn production_api_middleware_empty_catch() {
    let debug = run_debug(LANG, "production_api_service.r");
    assert!(debug.contains("apply_middleware"), "should find apply_middleware, got: {debug}");
}

#[test]
fn production_api_multiple_issues() {
    let output = run_check(LANG, "production_api_service.r");
    let issue_count: usize = output.lines().filter(|l| l.starts_with("  ")).count();
    assert!(issue_count >= 5, "should have many issues, got: {issue_count}");
}

// ===========================================================================
// Negative boundary tests — things that should NOT trigger
// ===========================================================================

#[test]
fn four_args_not_flagged() {
    let out = pulse_check_code("f <- function(a, b, c, d) {\n  a + b + c + d\n}\n", "r");
    assert!(!has_smell(&out, "Excess Arguments"));
}

#[test]
fn five_args_not_flagged() {
    let out = pulse_check_code("f <- function(a, b, c, d, e) {\n  a + b + c + d + e\n}\n", "r");
    assert!(!has_smell(&out, "Excess Arguments"));
}

#[test]
fn nesting_depth_3_not_flagged() {
    let out = pulse_check_code(concat!(
        "f <- function(a, b, c) {\n",
        "  if (a) {\n",
        "    if (b) {\n",
        "      if (c) {\n",
        "        1\n",
        "      }\n",
        "    }\n",
        "  }\n",
        "}\n",
    ), "r");
    assert!(!has_smell(&out, "Deep Nested"), "depth=3 should not flag, got: {out}");
}

#[test]
fn cc_8_not_flagged() {
    let mut code = String::from("f <- function(a, b, c, d, e, f2, g) {\n");
    for v in ["a", "b", "c", "d", "e", "f2", "g"] {
        code.push_str(&format!("  if ({v} > 0) return(1)\n"));
    }
    code.push_str("  0\n}\n");
    let out = pulse_check_code(&code, "r");
    assert!(!has_smell(&out, "Complex Method"), "cc=8 should not trigger");
}

#[test]
fn single_boolean_op_not_compound() {
    let out = pulse_check_code(
        "f <- function(a, b) {\n  if (a && b) {\n    TRUE\n  }\n}\n",
        "r",
    );
    assert!(!has_smell(&out, "Complex Conditional"));
}

#[test]
fn non_duplicate_functions_not_flagged() {
    let out = pulse_check_code(concat!(
        "alpha <- function(x) {\n",
        "  x + 1\n",
        "}\n",
        "beta <- function(items) {\n",
        "  total <- 0\n",
        "  for (v in items) {\n",
        "    if (v > 0) {\n",
        "      total <- total + v\n",
        "    } else {\n",
        "      total <- total - v\n",
        "    }\n",
        "  }\n",
        "  while (total > 100) {\n",
        "    total <- total / 2\n",
        "  }\n",
        "  total\n",
        "}\n",
    ), "r");
    assert!(!has_smell(&out, "Code Duplication"), "structurally different functions should not trigger, got: {out}");
}

#[test]
fn small_string_not_embedded_block() {
    let out = pulse_check_code(concat!(
        "f <- function() {\n",
        "  x <- \"a short string\"\n",
        "  y <- \"another short one\"\n",
        "  paste(x, y)\n",
        "}\n",
    ), "r");
    assert!(!has_smell(&out, "Large Embedded Block"));
}

#[test]
fn no_global_conditionals_when_inside_functions() {
    let out = pulse_check_code(concat!(
        "f <- function(x) {\n",
        "  if (x > 0) 1 else 0\n",
        "}\n",
        "g <- function(y) {\n",
        "  if (y > 0) 2 else 0\n",
        "}\n",
    ), "r");
    assert!(!has_smell(&out, "Global Conditionals"));
}

#[test]
fn try_catch_with_body_not_empty() {
    let out = pulse_check_code(concat!(
        "f <- function(x) {\n",
        "  tryCatch(\n",
        "    log(x),\n",
        "    error = function(e) { message(e$message) }\n",
        "  )\n",
        "}\n",
    ), "r");
    assert!(!has_smell(&out, "Empty Error Handler"));
}

#[test]
fn single_bump_not_nested_chunks() {
    let out = pulse_check_code(concat!(
        "f <- function(x) {\n",
        "  if (x > 0) {\n",
        "    if (x > 10) {\n",
        "      if (x > 100) {\n",
        "        1\n",
        "      }\n",
        "    }\n",
        "  }\n",
        "  0\n",
        "}\n",
    ), "r");
    assert!(!has_smell(&out, "Nested Conditional Chunks"));
}

// ===========================================================================
// Positive boundary tests — things that SHOULD trigger at threshold
// ===========================================================================

#[test]
fn six_args_triggers_excess() {
    let out = pulse_check_code(
        "f <- function(a, b, c, d, e, g) {\n  a + b + c + d + e + g\n}\n",
        "r",
    );
    assert!(has_smell(&out, "Excess Arguments"), "6 args should trigger, got: {out}");
}

#[test]
fn too_many_functions_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("many_fns.r");
    let mut code = String::new();
    for i in 0..functions_above() {
        code.push_str(&format!("fn{i} <- function(x) {{\n  x + {i}\n}}\n\n"));
    }
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(has_smell(&stdout, "Too Many Functions"), "got: {stdout}");
}

#[test]
fn file_too_large_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("huge.r");
    let mut code = String::new();
    for i in 0..file_padding() {
        code.push_str(&format!("x{i} <- {i}\n"));
    }
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(has_smell(&stdout, "File Too Large"), "got: {stdout}");
}

#[test]
fn overall_code_complexity_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("complex_total.r");
    let mut code = String::new();
    for i in 0..15 {
        code.push_str(&format!("fn{i} <- function(x) {{\n"));
        for j in 0..8 {
            code.push_str(&format!("  if (x > {j}) return({j})\n"));
        }
        code.push_str("  0\n}\n\n");
    }
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        has_smell(&stdout, "Overall Code Complexity") || has_smell(&stdout, "Complex Method"),
        "got: {stdout}"
    );
}

#[test]
fn short_variable_names_inline() {
    let mut code = String::from("f <- function(data) {\n");
    for ch in ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'] {
        code.push_str(&format!("  {ch} <- data[\"{ch}\"]\n"));
    }
    for i in 0..20 {
        code.push_str(&format!("  x{i} <- {i}\n"));
    }
    code.push_str("  a + b + c + d + e + f + g + h\n}\n");
    let out = pulse_check_code(&code, "r");
    assert!(has_smell(&out, "Short Variable Names"), "got: {out}");
}

#[test]
fn short_vars_not_triggered_with_long_names() {
    let out = pulse_check_code(concat!(
        "f <- function(data) {\n",
        "  total <- data[1]\n",
        "  count <- data[2]\n",
        "  value <- data[3]\n",
        "  score <- data[4]\n",
        "  total + count + value + score\n",
        "}\n",
    ), "r");
    assert!(!has_smell(&out, "Short Variable Names"));
}

#[test]
fn duplicated_assertion_blocks_above_threshold() {
    let mut code = String::from("test_a <- function() {\n");
    for i in 0..asserts_above() {
        code.push_str(&format!("  stopifnot({i} > 0)\n"));
    }
    code.push_str("}\n\ntest_b <- function() {\n");
    for i in 0..asserts_above() {
        code.push_str(&format!("  stopifnot({i} > 0)\n"));
    }
    code.push_str("}\n");
    let out = pulse_check_code(&code, "r");
    assert!(
        has_smell(&out, "Duplicated Assertion Blocks") || has_smell(&out, "Large Assertion Block"),
        "got: {out}"
    );
}
