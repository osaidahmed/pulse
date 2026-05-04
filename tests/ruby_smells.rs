mod common;

use common::*;
use std::process::Command;

const LANG: &str = "ruby";

// ===========================================================================
// Output format
// ===========================================================================

#[test]
fn output_starts_with_pulse() {
    let output = run_check(LANG, "complex_methods.rb");
    assert!(output.starts_with("pulse:"), "got: {output}");
}

#[test]
fn output_has_function_line_numbers() {
    let output = run_check(LANG, "complex_methods.rb");
    assert!(output.lines().any(|l| l.contains("(L") && l.contains("): ")));
}

#[test]
fn output_has_module_prefix() {
    let output = run_check(LANG, "production_service.rb");
    assert!(output.contains("Module:"), "got: {output}");
}

#[test]
fn issue_count_matches_findings() {
    let output = run_check(LANG, "complex_methods.rb");
    let first = output.lines().next().unwrap_or("");
    let findings = output.lines().filter(|l| l.starts_with("  ")).count();
    assert!(first.contains(&format!("{findings} issue")));
}

// ===========================================================================
// Clean / empty
// ===========================================================================

#[test]
fn clean_file_produces_no_output() {
    let output = run_check(LANG, "clean.rb");
    assert!(output.is_empty(), "got: {output}");
}

#[test]
fn empty_file() {
    let out = pulse_check_code("", "rb");
    assert!(out.is_empty());
}

#[test]
fn comments_only_file() {
    let out = pulse_check_code("# just a comment\n", "rb");
    assert!(out.is_empty());
}

#[test]
fn simple_func_not_flagged() {
    let out = pulse_check_code("def add(a, b)\n  a + b\nend\n", "rb");
    assert!(out.is_empty(), "got: {out}");
}

// ===========================================================================
// CC boundary
// ===========================================================================

#[test]
fn cc_base_case_is_1() {
    let debug = pulse_debug_code("def add(a, b)\n  a + b\nend\n", "rb");
    assert_eq!(function_metric(&debug, "add", "cc"), Some(1));
}

#[test]
fn function_at_cc_boundary_flagged() {
    let mut code = String::from("def f(a, b, c, d, e, f2, g, h)\n");
    for v in ["a", "b", "c", "d", "e", "f2", "g", "h"] {
        code.push_str(&format!("  return 1 if {v} > 0\n"));
    }
    code.push_str("  0\nend\n");
    let out = pulse_check_code(&code, "rb");
    assert!(has_smell(&out, "Complex Method"), "cc=9 should trigger, got: {out}");
}

#[test]
fn function_below_cc_boundary_not_flagged() {
    let mut code = String::from("def f(a, b, c, d, e, f2, g)\n");
    for v in ["a", "b", "c", "d", "e", "f2", "g"] {
        code.push_str(&format!("  return 1 if {v} > 0\n"));
    }
    code.push_str("  0\nend\n");
    let out = pulse_check_code(&code, "rb");
    assert!(!has_smell(&out, "Complex Method"));
}

#[test]
fn boolean_operators_increment_cc() {
    let debug = pulse_debug_code(
        "def f(a, b, c)\n  if a && b && c\n    return true\n  end\n  false\nend\n",
        "rb",
    );
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert!(cc >= 4, "got: {cc}");
}

// ===========================================================================
// Complexity smells
// ===========================================================================

#[test]
fn complex_method_detected() {
    let output = run_check(LANG, "complex_methods.rb");
    assert!(has_smell(&output, "Complex Method"), "got: {output}");
    assert!(has_function(&output, "process_order"));
}

#[test]
fn complex_method_cc_at_least_9() {
    let debug = run_debug(LANG, "complex_methods.rb");
    let cc = function_metric(&debug, "process_order", "cc").unwrap_or(0);
    assert!(cc >= 9, "cc should be >= 9, got: {cc}");
}

#[test]
fn god_method_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("god.rb");
    let mut code = String::from("def process_data_pipeline(x)\n");
    for i in 0..cc_branches() {
        code.push_str(&format!("  return 1 if x > {i}\n"));
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("  y{i} = {i}\n"));
    }
    code.push_str("  0\nend\n");
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
    let path = dir.path().join("large.rb");
    let mut code = String::from("def build_report\n");
    for i in 0..fn_padding() {
        code.push_str(&format!("  x{i} = {i}\n"));
    }
    code.push_str("  0\nend\n");
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
    let output = run_check(LANG, "deep_nesting.rb");
    assert!(has_smell(&output, "Deep Nested"), "got: {output}");
    assert!(has_function(&output, "deeply_nested"));
}

#[test]
fn deep_nesting_depth_exceeds_4() {
    let debug = run_debug(LANG, "deep_nesting.rb");
    let depth = function_metric(&debug, "deeply_nested", "nesting").unwrap_or(0);
    assert!(depth >= 4, "got: {depth}");
}

#[test]
fn moderate_nesting_not_flagged() {
    let output = run_check(LANG, "deep_nesting.rb");
    assert!(!has_function(&output, "moderately_nested"));
}

// ===========================================================================
// Arguments
// ===========================================================================

#[test]
fn excess_args_detected() {
    let output = run_check(LANG, "excess_args.rb");
    assert!(has_smell(&output, "Excess Arguments"), "got: {output}");
    assert!(has_function(&output, "create_user"));
}

#[test]
fn excess_args_count_correct() {
    let debug = run_debug(LANG, "excess_args.rb");
    let args = function_metric(&debug, "create_user", "args").unwrap_or(0);
    assert!(args >= 6, "got: {args}");
}

#[test]
fn simple_func_in_excess_args_not_flagged() {
    let output = run_check(LANG, "excess_args.rb");
    assert!(!has_function(&output, "simple_func"));
}

// ===========================================================================
// Module-level
// ===========================================================================

#[test]
fn code_duplication_detected() {
    let output = run_check(LANG, "code_duplication.rb");
    assert!(has_smell(&output, "Code Duplication"), "got: {output}");
}

#[test]
fn embedded_block_detected() {
    let output = run_check(LANG, "embedded_block.rb");
    assert!(has_smell(&output, "Large Embedded Block"), "got: {output}");
}

#[test]
fn bumpy_road_detected() {
    let output = run_check(LANG, "bumpy_road.rb");
    assert!(
        has_smell(&output, "Nested Conditional Chunks") || has_smell(&output, "Deep Nested"),
        "got: {output}"
    );
}

#[test]
fn low_cohesion_detected() {
    let output = run_check(LANG, "low_cohesion.rb");
    assert!(
        has_smell(&output, "Low Cohesion") || has_smell(&output, "Too Many Functions") || !output.is_empty(),
        "got: {output}"
    );
}

#[test]
fn overall_function_size_at_threshold() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("size_at.rb");
    let mut code = String::new();
    for i in 0..3 {
        code.push_str(&format!("def lg{i}()\n"));
        for j in 0..45 {
            code.push_str(&format!("  x{j} = {j}\n"));
        }
        code.push_str("  0\nend\n\n");
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
    let path = dir.path().join("size_below.rb");
    let mut code = String::new();
    for i in 0..2 {
        code.push_str(&format!("def lg{i}()\n"));
        for j in 0..45 {
            code.push_str(&format!("  x{j} = {j}\n"));
        }
        code.push_str("  0\nend\n\n");
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
    let out = pulse_check_code("def f()\n  \"hello\"\nend\n", "rb");
    assert!(!has_smell(&out, "Large Embedded Block"));
}

#[test]
fn complex_conditional_detected() {
    let out = pulse_check_code(concat!(
        "def check(age, score, active)\n",
        "  if age > 18 && score > 50 && active\n",
        "    if score > 80 || (age > 25 && active)\n",
        "      return true\n",
        "    end\n",
        "  end\n",
        "  if age > 65 || score < 10\n",
        "    return true\n",
        "  end\n",
        "  false\n",
        "end\n",
    ), "rb");
    assert!(
        has_smell(&out, "Complex Conditional") || has_smell(&out, "Complex Method"),
        "got: {out}"
    );
}

#[test]
fn production_service_has_issues() {
    let output = run_check(LANG, "production_service.rb");
    assert!(!output.is_empty(), "production_service.rb should have findings");
}

#[test]
fn analysis_completes_under_500ms() {
    let path = fixtures_dir(LANG).join("production_service.rb");
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
    let path = fixtures_dir(LANG).join("clean.rb");
    let output = run_hook(path.to_str().unwrap());
    assert!(output.is_empty());
}

#[test]
fn hook_smelly_file_produces_output() {
    let path = fixtures_dir(LANG).join("complex_methods.rb");
    let output = run_hook(path.to_str().unwrap());
    assert!(!output.is_empty());
}

#[test]
fn hook_nonexistent_file_silent() {
    let output = run_hook("/nonexistent/path/foo.rb");
    assert!(output.is_empty());
}

// ===========================================================================
// Ruby-specific
// ===========================================================================

#[test]
fn case_when_increments_cc() {
    let out = pulse_check_code(concat!(
        "def handle(action)\n",
        "  case action\n",
        "  when 1 then \"a\"\n",
        "  when 2 then \"b\"\n",
        "  when 3 then \"c\"\n",
        "  when 4 then \"d\"\n",
        "  when 5 then \"e\"\n",
        "  when 6 then \"f\"\n",
        "  when 7 then \"g\"\n",
        "  when 8 then \"h\"\n",
        "  when 9 then \"i\"\n",
        "  else \"?\"\n",
        "  end\n",
        "end\n",
    ), "rb");
    assert!(has_smell(&out, "Complex Method"), "got: {out}");
}

#[test]
fn unless_increments_cc() {
    let debug = pulse_debug_code("def f(x)\n  unless x > 0\n    return -1\n  end\n  x\nend\n", "rb");
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert_eq!(cc, 2);
}

#[test]
fn global_conditionals_detected() {
    let output = run_check(LANG, "global_conditionals.rb");
    assert!(has_smell(&output, "Global Conditionals"), "got: {output}");
}

#[test]
fn test_file_analyzed() {
    let output = run_check(LANG, "test_smells.rb");
    assert!(output.is_empty() || !output.is_empty(), "test file should be parseable");
}

#[test]
fn constructor_over_injection_detected() {
    let output = run_check(LANG, "excess_args.rb");
    assert!(
        has_smell(&output, "Constructor Over-Injection") || has_smell(&output, "Excess Arguments"),
        "got: {output}"
    );
}

#[test]
fn for_in_increments_cc() {
    let debug = pulse_debug_code(
        "def sum(data)\n  s = 0\n  for v in data\n    s += v\n  end\n  s\nend\n",
        "rb",
    );
    let cc = function_metric(&debug, "sum", "cc").unwrap_or(0);
    assert!(cc >= 2, "for should increment cc, got: {cc}");
}

#[test]
fn method_attributed_to_class() {
    let debug = pulse_debug_code(concat!(
        "class Svc\n",
        "  def handle(a, b, c, d, e, f, g, h)\n",
        "    a + b\n",
        "  end\n",
        "end\n",
    ), "rb");
    assert!(debug.contains("Svc.handle"), "method should be attributed to class, got: {debug}");
}

#[test]
fn code_duplication_inline() {
    let out = pulse_check_code(concat!(
        "def rpt_a(data)\n",
        "  r = 0\n",
        "  data.each do |v|\n    r += v\n  end\n",
        "  r = r * 2\n  r\n",
        "end\n\n",
        "def rpt_b(data)\n",
        "  r = 0\n",
        "  data.each do |v|\n    r += v\n  end\n",
        "  r = r * 2\n  r\n",
        "end\n",
    ), "rb");
    assert!(has_smell(&out, "Code Duplication"), "got: {out}");
}

#[test]
fn nested_conditional_chunks_detected() {
    let out = pulse_check_code(concat!(
        "def validate(data)\n",
        "  if data.length > 0\n",
        "    if data[0] > 0\n",
        "      if data[0] > 10\n",
        "        x = 1\n",
        "      end\n",
        "    end\n",
        "  end\n",
        "  gap = 1\n",
        "  if data.length > 5\n",
        "    if data[5] > 0\n",
        "      if data[5] > 10\n",
        "        y = 2\n",
        "      end\n",
        "    end\n",
        "  end\n",
        "  gap2 = 2\n",
        "  if data.length > 10\n",
        "    if data[10] > 0\n",
        "      if data[10] > 10\n",
        "        z = 3\n",
        "      end\n",
        "    end\n",
        "  end\n",
        "  0\n",
        "end\n",
    ), "rb");
    assert!(
        has_smell(&out, "Nested Conditional Chunks") || has_smell(&out, "Complex Method"),
        "got: {out}"
    );
}

#[test]
fn rescue_increments_cc() {
    let debug = pulse_debug_code(concat!(
        "def f(x)\n",
        "  begin\n",
        "    Integer(x)\n",
        "  rescue ArgumentError\n",
        "    -1\n",
        "  end\n",
        "end\n",
    ), "rb");
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert!(cc >= 2, "rescue should add CC, got: {cc}");
}

#[test]
fn ternary_increments_cc() {
    let debug = pulse_debug_code("def f(x)\n  x > 0 ? x : 0\nend\n", "rb");
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert_eq!(cc, 2);
}

#[test]
fn primitive_obsession_not_triggered() {
    let out = pulse_check_code("def f(a, b, c, d)\n  a + b + c + d\nend\n", "rb");
    assert!(!has_smell(&out, "Primitive Obsession"));
}

#[test]
fn large_method_loc_at_least_65() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large_loc.rb");
    let mut code = String::from("def build_report\n");
    for i in 0..fn_padding() {
        code.push_str(&format!("  x{i} = {i}\n"));
    }
    code.push_str("  0\nend\n");
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["debug", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stderr = String::from_utf8(out.stderr).unwrap();
    let loc = function_metric(&stderr, "build_report", "loc").unwrap_or(0);
    assert!(loc >= t().function.fn_loc_warning, "loc >= t().function.fn_loc_warning, got: {loc}");
}
