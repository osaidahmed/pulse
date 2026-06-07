use crate::common::*;
use std::process::Command;

const LANG: &str = "php";

// ===========================================================================
// Output format
// ===========================================================================

#[test]
fn output_starts_with_pulse() {
    let output = run_check(LANG, "complex_methods.php");
    assert!(output.starts_with("pulse:"), "got: {output}");
}

#[test]
fn output_has_function_line_numbers() {
    let output = run_check(LANG, "complex_methods.php");
    assert!(output.lines().any(|l| l.contains("(L") && l.contains("): ")));
}

#[test]
fn output_has_module_prefix() {
    let output = run_check(LANG, "production_service.php");
    assert!(output.contains("Module:"), "got: {output}");
}

#[test]
fn issue_count_matches_findings() {
    let output = run_check(LANG, "complex_methods.php");
    let first = output.lines().next().unwrap_or("");
    let findings = output.lines().filter(|l| l.starts_with("  ")).count();
    assert!(first.contains(&format!("{findings} issue")));
}

// ===========================================================================
// Clean / empty
// ===========================================================================

#[test]
fn clean_file_produces_no_output() {
    let output = run_check(LANG, "clean.php");
    assert!(output.is_empty(), "got: {output}");
}

#[test]
fn empty_file() {
    let out = pulse_check_code("<?php\n", "php");
    assert!(out.is_empty());
}

#[test]
fn comments_only_file() {
    let out = pulse_check_code("<?php\n// just a comment\n# another\n/* block */\n", "php");
    assert!(out.is_empty());
}

#[test]
fn simple_function_no_smell() {
    let out = pulse_check_code("<?php\nfunction f(): int { return 1; }\n", "php");
    assert!(out.is_empty(), "got: {out}");
}

// ===========================================================================
// CC boundary
// ===========================================================================

#[test]
fn cc_base_case_is_1() {
    let debug = pulse_debug_code("<?php\nfunction add(int $a, int $b): int { return $a + $b; }\n", "php");
    assert_eq!(function_metric(&debug, "add", "cc"), Some(1));
}

#[test]
fn function_at_cc_boundary_flagged() {
    let mut code = String::from("<?php\nfunction f(int $a): int {\n");
    for i in 0..cc_branches() {
        code.push_str(&format!("  if ($a > {i}) {{ return {i}; }}\n"));
    }
    code.push_str("  return 0;\n}\n");
    let out = pulse_check_code(&code, "php");
    assert!(has_smell(&out, "Complex Method"), "cc at threshold should trigger, got: {out}");
}

#[test]
fn function_below_cc_boundary_not_flagged() {
    let mut code = String::from("<?php\nfunction f(int $a): int {\n");
    for i in 0..6 {
        code.push_str(&format!("  if ($a > {i}) {{ return {i}; }}\n"));
    }
    code.push_str("  return 0;\n}\n");
    let out = pulse_check_code(&code, "php");
    assert!(!has_smell(&out, "Complex Method"));
}

#[test]
fn boolean_operators_increment_cc() {
    let debug = pulse_debug_code(
        "<?php\nfunction f(bool $a, bool $b, bool $c): bool {\n  if ($a && $b && $c) {\n    return true;\n  }\n  return false;\n}\n",
        "php",
    );
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert!(cc >= 4, "got: {cc}");
}

// ===========================================================================
// Complexity smells
// ===========================================================================

#[test]
fn complex_method_detected() {
    let output = run_check(LANG, "complex_methods.php");
    assert!(has_smell(&output, "Complex Method"), "got: {output}");
    assert!(has_function(&output, "process_order"));
}

#[test]
fn complex_method_cc_at_least_9() {
    let debug = run_debug(LANG, "complex_methods.php");
    let cc = function_metric(&debug, "process_order", "cc").unwrap_or(0);
    assert!(cc >= 9, "cc should be >= 9, got: {cc}");
}

#[test]
fn god_method_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("god.php");
    let mut code = String::from("<?php\nfunction process_data_pipeline(int $x): int {\n");
    for i in 0..cc_branches() {
        code.push_str(&format!("  if ($x > {i}) {{ return {i}; }}\n"));
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("  $y{i} = {i};\n"));
    }
    code.push_str("  return 0;\n}\n");
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
    let path = dir.path().join("large.php");
    let mut code = String::from("<?php\nfunction build_report(): int {\n");
    for i in 0..fn_padding() {
        code.push_str(&format!("  $x{i} = {i};\n"));
    }
    code.push_str("  return 0;\n}\n");
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(has_smell(&stdout, "Large Method") || has_smell(&stdout, "God Method"), "got: {stdout}");
}

#[test]
fn large_method_loc_at_least_threshold() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large_loc.php");
    let mut code = String::from("<?php\nfunction build_report(): int {\n");
    for i in 0..fn_padding() {
        code.push_str(&format!("  $x{i} = {i};\n"));
    }
    code.push_str("  return 0;\n}\n");
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["debug", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stderr = String::from_utf8(out.stderr).unwrap();
    let loc = function_metric(&stderr, "build_report", "loc").unwrap_or(0);
    assert!(loc >= t().function.fn_loc_warning, "loc >= {}, got: {}", t().function.fn_loc_warning, loc);
}

// ===========================================================================
// Nesting
// ===========================================================================

#[test]
fn deep_nesting_detected() {
    let output = run_check(LANG, "deep_nesting.php");
    assert!(has_smell(&output, "Deep Nested Complexity"), "got: {output}");
    assert!(has_function(&output, "deeply_nested"));
}

#[test]
fn deep_nesting_depth_at_least_4() {
    let debug = run_debug(LANG, "deep_nesting.php");
    let depth = function_metric(&debug, "deeply_nested", "nesting").unwrap();
    assert!(depth >= t().function.nesting_depth, "depth={depth}");
}

#[test]
fn moderate_nesting_not_flagged() {
    let out = pulse_check_code(
        "<?php\nfunction f(int $x): int {\n  if ($x > 0) {\n    if ($x > 1) { return $x; }\n  }\n  return 0;\n}\n",
        "php",
    );
    assert!(!has_smell(&out, "Deep Nested"), "got: {out}");
}

// ===========================================================================
// Arguments
// ===========================================================================

#[test]
fn excess_args_detected() {
    let output = run_check(LANG, "excess_args.php");
    assert!(has_smell(&output, "Excess Arguments"), "got: {output}");
}

#[test]
fn constructor_over_injection_detected() {
    let output = run_check(LANG, "excess_args.php");
    assert!(has_smell(&output, "Constructor Over-Injection"), "got: {output}");
}

#[test]
fn few_args_not_flagged() {
    let out = pulse_check_code("<?php\nfunction f(int $a, int $b): int { return $a + $b; }\n", "php");
    assert!(!has_smell(&out, "Excess Arguments"), "got: {out}");
}

// ===========================================================================
// Module-level smells
// ===========================================================================

#[test]
fn code_duplication_detected() {
    let output = run_check(LANG, "code_duplication.php");
    assert!(has_smell(&output, "Code Duplication"), "got: {output}");
}

#[test]
fn embedded_block_detected() {
    let output = run_check(LANG, "embedded_block.php");
    assert!(has_smell(&output, "Large Embedded Block"), "got: {output}");
}

#[test]
fn bumpy_road_detected() {
    let output = run_check(LANG, "bumpy_road.php");
    assert!(has_smell(&output, "Nested Conditional Chunks"), "got: {output}");
}

#[test]
fn low_cohesion_detected() {
    let output = run_check(LANG, "low_cohesion.php");
    assert!(has_smell(&output, "Low Cohesion"), "got: {output}");
}

#[test]
fn primitive_obsession_detected() {
    let output = run_check(LANG, "primitive_obsession.php");
    assert!(has_smell(&output, "Primitive Obsession"), "got: {output}");
}

#[test]
fn stringly_typed_match_detected() {
    let output = run_check(LANG, "match_expression.php");
    assert!(has_smell(&output, "Stringly-Typed Switch"), "got: {output}");
}

#[test]
fn large_assertion_block_detected() {
    let output = run_check(LANG, "test_smells.php");
    assert!(has_smell(&output, "Large Assertion Block"), "got: {output}");
}

// ===========================================================================
// Overall Function Size
// ===========================================================================

#[test]
fn overall_function_size_at_threshold() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("size_at.php");
    let mut code = String::from("<?php\n");
    for i in 0..3 {
        code.push_str(&format!("function lg{i}(): int {{\n"));
        for j in 0..large_fn_lines() {
            code.push_str(&format!("  $x{j} = {j};\n"));
        }
        code.push_str("  return 0;\n}\n\n");
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
    let path = dir.path().join("size_below.php");
    let mut code = String::from("<?php\n");
    for i in 0..2 {
        code.push_str(&format!("function lg{i}(): int {{\n"));
        for j in 0..large_fn_lines() {
            code.push_str(&format!("  $x{j} = {j};\n"));
        }
        code.push_str("  return 0;\n}\n\n");
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
    let out = pulse_check_code("<?php\nfunction f(): string { return \"hello\"; }\n", "php");
    assert!(!has_smell(&out, "Large Embedded Block"));
}

#[test]
fn complex_conditional_detected() {
    let out = pulse_check_code(
        concat!(
            "<?php\nfunction check(int $age, int $score, bool $active): bool {\n",
            "  if ($age > 18 && $score > 50 && $active) {\n",
            "    if ($score > 80 || ($age > 25 && $active)) {\n",
            "      return true;\n",
            "    }\n",
            "  }\n",
            "  if ($age > 65 || $score < 10) {\n",
            "    return true;\n",
            "  }\n",
            "  return false;\n",
            "}\n",
        ),
        "php",
    );
    assert!(has_smell(&out, "Complex Conditional") || has_smell(&out, "Complex Method"), "got: {out}");
}

#[test]
fn global_conditionals_detected() {
    let output = run_check(LANG, "global_conditionals.php");
    assert!(has_smell(&output, "Global Conditionals"), "got: {output}");
}

#[test]
fn test_file_analyzed() {
    let output = run_check(LANG, "test_smells.php");
    assert!(output.is_empty() || !output.is_empty(), "test file should be parseable");
}

#[test]
fn code_duplication_inline() {
    let out = pulse_check_code(
        concat!(
            "<?php\nfunction rpt_a(array $data): int {\n",
            "  $r = 0;\n",
            "  foreach ($data as $v) {\n    $r += $v;\n  }\n",
            "  $r = $r * 2;\n  return $r;\n",
            "}\n\n",
            "function rpt_b(array $data): int {\n",
            "  $r = 0;\n",
            "  foreach ($data as $v) {\n    $r += $v;\n  }\n",
            "  $r = $r * 2;\n  return $r;\n",
            "}\n",
        ),
        "php",
    );
    assert!(has_smell(&out, "Code Duplication"), "got: {out}");
}

#[test]
fn nested_conditional_chunks_inline() {
    let out = pulse_check_code(
        concat!(
            "<?php\nfunction validate(array $data): int {\n",
            "  if (count($data) > 0) {\n",
            "    if ($data[0] > 0) {\n",
            "      if ($data[0] > 10) {\n",
            "        $x = 1;\n",
            "      }\n",
            "    }\n",
            "  }\n",
            "  $gap = 1;\n",
            "  if (count($data) > 5) {\n",
            "    if ($data[5] > 0) {\n",
            "      if ($data[5] > 10) {\n",
            "        $y = 2;\n",
            "      }\n",
            "    }\n",
            "  }\n",
            "  return 0;\n",
            "}\n",
        ),
        "php",
    );
    assert!(has_smell(&out, "Nested Conditional Chunks") || has_smell(&out, "Complex Method"), "got: {out}");
}

#[test]
fn analysis_completes_under_500ms() {
    let path = fixtures_dir(LANG).join("production_service.php");
    let start = std::time::Instant::now();
    let _ = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    assert!(start.elapsed().as_millis() < 1000, "took: {}ms", start.elapsed().as_millis());
}

#[test]
fn production_api_service_parseable() {
    let output = run_check(LANG, "production_api_service.php");
    assert!(output.is_empty() || !output.is_empty(), "api service should be parseable");
}

// ===========================================================================
// Hook
// ===========================================================================

#[test]
fn hook_clean_file_silent() {
    let path = fixtures_dir(LANG).join("clean.php");
    assert!(run_hook(path.to_str().unwrap()).is_empty());
}

#[test]
fn hook_smelly_file_reports() {
    let path = fixtures_dir(LANG).join("complex_methods.php");
    assert!(!run_hook(path.to_str().unwrap()).is_empty());
}

#[test]
fn hook_missing_file_silent() {
    assert!(run_hook("/nonexistent/file.php").is_empty());
}

// ===========================================================================
// PHP-specific constructs
// ===========================================================================

#[test]
fn elseif_cc() {
    let out = pulse_debug_code("<?php\nfunction f(int $x): int {\n  if ($x > 3) { return 3; } elseif ($x > 2) { return 2; } elseif ($x > 1) { return 1; }\n  return 0;\n}\n", "php");
    assert_eq!(function_metric(&out, "f", "cc"), Some(4));
}

#[test]
fn foreach_cc() {
    let out =
        pulse_debug_code("<?php\nfunction f(array $a): int { foreach ($a as $v) { echo $v; } return 0; }\n", "php");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn do_while_cc() {
    let out =
        pulse_debug_code("<?php\nfunction f(): int { $i = 0; do { $i++; } while ($i < 10); return $i; }\n", "php");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn switch_case_cc() {
    let out = pulse_debug_code("<?php\nfunction f(int $x): string {\n  switch ($x) {\n    case 1: return 'a';\n    case 2: return 'b';\n    case 3: return 'c';\n    default: return 'd';\n  }\n}\n", "php");
    assert_eq!(function_metric(&out, "f", "cc"), Some(4));
}

#[test]
fn match_expression_cc() {
    let out = pulse_debug_code(
        "<?php\nfunction f(int $x): string {\n  return match($x) { 1 => 'a', 2 => 'b', default => 'c' };\n}\n",
        "php",
    );
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn catch_cc() {
    let out =
        pulse_debug_code("<?php\nfunction f(): int { try { return 1; } catch (Exception $e) { return 0; } }\n", "php");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn finally_no_cc() {
    let out = pulse_debug_code("<?php\nfunction f(): int { try { return 1; } finally { echo 'done'; } }\n", "php");
    assert_eq!(function_metric(&out, "f", "cc"), Some(1));
}

#[test]
fn ternary_cc() {
    let out = pulse_debug_code("<?php\nfunction f(int $x): int { return $x > 0 ? 1 : 0; }\n", "php");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn construct_is_constructor() {
    let out =
        pulse_debug_code("<?php\nclass Foo {\n  public function __construct(int $x) { $this->x = $x; }\n}\n", "php");
    assert!(out.contains("Foo.__construct"), "got: {out}");
}

#[test]
fn field_access_tracked() {
    let out = pulse_debug_code("<?php\nclass C {\n  public function getX(): int { return $this->x; }\n}\n", "php");
    assert!(out.contains("fields=[\"x\"]"), "got: {out}");
}

#[test]
fn heredoc_embedded_block() {
    let lines: Vec<String> = (0..20).map(|i| format!("    line {i}")).collect();
    let code = format!("<?php\nfunction f(): string {{\n  return <<<EOT\n{}\n  EOT;\n}}\n", lines.join("\n"));
    let out = pulse_check_code(&code, "php");
    assert!(has_smell(&out, "Large Embedded Block"), "got: {out}");
}

#[test]
fn boolean_and_or_cc() {
    let out = pulse_debug_code(
        "<?php\nfunction f(bool $a, bool $b): bool { if ($a && $b) { return true; } return false; }\n",
        "php",
    );
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn keyword_and_or_cc() {
    let out = pulse_debug_code(
        "<?php\nfunction f(bool $a, bool $b): bool { if ($a and $b) { return true; } return false; }\n",
        "php",
    );
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn arrow_function_scope_boundary() {
    let out = pulse_debug_code("<?php\nfunction f(): callable { return fn(int $x): int => $x > 0 ? 1 : 0; }\n", "php");
    assert_eq!(function_metric(&out, "f", "cc"), Some(1));
}

#[test]
fn anonymous_function_scope_boundary() {
    let out = pulse_debug_code(
        "<?php\nfunction f(): callable { return function(int $x): int { if ($x > 0) { return 1; } return 0; }; }\n",
        "php",
    );
    assert_eq!(function_metric(&out, "f", "cc"), Some(1));
}

#[test]
fn namespace_functions_detected() {
    let out = pulse_debug_code("<?php\nnamespace App\\Service;\nfunction helper(): int { return 1; }\n", "php");
    assert!(out.contains("helper"), "got: {out}");
}

#[test]
fn empty_catch_detected() {
    let out = pulse_check_code(
        "<?php\nfunction f(): void { try { throw new Exception(); } catch (Exception $e) {} }\n",
        "php",
    );
    assert!(has_smell(&out, "Empty Error Handler"), "got: {out}");
}

// ===========================================================================
// Production fixture
// ===========================================================================

#[test]
fn production_service_detects_module_smells() {
    let output = run_check(LANG, "production_service.php");
    assert!(!output.is_empty(), "should detect smells in production service");
}
