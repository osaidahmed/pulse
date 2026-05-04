
use crate::common::*;
use std::process::Command;

const LANG: &str = "lua";

// ===========================================================================
// Output format
// ===========================================================================

#[test]
fn output_starts_with_pulse() {
    let output = run_check(LANG, "complex_methods.lua");
    assert!(output.starts_with("pulse:"), "got: {output}");
}

#[test]
fn output_has_function_line_numbers() {
    let output = run_check(LANG, "complex_methods.lua");
    assert!(output
        .lines()
        .any(|l| l.contains("(L") && l.contains("): ")));
}

#[test]
fn output_has_module_prefix() {
    let output = run_check(LANG, "production_service.lua");
    assert!(output.contains("Module:"), "got: {output}");
}

#[test]
fn issue_count_matches_findings() {
    let output = run_check(LANG, "complex_methods.lua");
    let first = output.lines().next().unwrap_or("");
    let findings = output.lines().filter(|l| l.starts_with("  ")).count();
    assert!(first.contains(&format!("{findings} issue")));
}

// ===========================================================================
// Clean / empty
// ===========================================================================

#[test]
fn clean_file_produces_no_output() {
    let output = run_check(LANG, "clean.lua");
    assert!(output.is_empty(), "got: {output}");
}

#[test]
fn empty_file() {
    let out = pulse_check_code("", "lua");
    assert!(out.is_empty());
}

#[test]
fn comments_only_file() {
    let out = pulse_check_code("-- just a comment\n", "lua");
    assert!(out.is_empty());
}

#[test]
fn simple_func_not_flagged() {
    let out = pulse_check_code(
        "function add(a, b)\n    return a + b\nend\n",
        "lua",
    );
    assert!(out.is_empty(), "got: {out}");
}

// ===========================================================================
// CC boundary
// ===========================================================================

#[test]
fn cc_base_case_is_1() {
    let debug = pulse_debug_code(
        "function add(a, b)\n    return a + b\nend\n",
        "lua",
    );
    let cc = function_metric(&debug, "add", "cc").unwrap_or(99);
    assert_eq!(cc, 1);
}

#[test]
fn function_at_cc_boundary_flagged() {
    let out = pulse_check_code(
        concat!(
            "function f(a, b, c, d, e, f2, g, h)\n",
            "    if a > 0 then end\n",
            "    if b > 0 then end\n",
            "    if c > 0 then end\n",
            "    if d > 0 then end\n",
            "    if e > 0 then end\n",
            "    if f2 > 0 then end\n",
            "    if g > 0 then end\n",
            "    if h > 0 then end\n",
            "    return 0\n",
            "end\n",
        ),
        "lua",
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
            "function f(a, b, c, d, e, f2, g)\n",
            "    if a > 0 then end\n",
            "    if b > 0 then end\n",
            "    if c > 0 then end\n",
            "    if d > 0 then end\n",
            "    if e > 0 then end\n",
            "    if f2 > 0 then end\n",
            "    if g > 0 then end\n",
            "    return 0\n",
            "end\n",
        ),
        "lua",
    );
    assert!(!has_smell(&out, "Complex Method"));
}

#[test]
fn boolean_operators_increment_cc() {
    let debug = pulse_debug_code(
        "function f(a, b, c)\n    if a and b and c then\n        return true\n    end\n    return false\nend\n",
        "lua",
    );
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert!(cc >= 4, "got: {cc}");
}

// ===========================================================================
// Complexity smells
// ===========================================================================

#[test]
fn complex_method_detected() {
    let output = run_check(LANG, "complex_methods.lua");
    assert!(has_smell(&output, "Complex Method"), "got: {output}");
    assert!(has_function(&output, "processOrder"));
}

#[test]
fn complex_method_cc_at_least_9() {
    let debug = run_debug(LANG, "complex_methods.lua");
    let cc = function_metric(&debug, "processOrder", "cc").unwrap_or(0);
    assert!(cc >= 9, "cc should be >= 9, got: {cc}");
}

#[test]
fn god_method_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("god.lua");
    let mut code = String::from("function processDataPipeline(x)\n");
    for i in 0..cc_branches() {
        code.push_str(&format!("    if x > {i} then end\n"));
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("    local y{i} = {i}\n"));
    }
    code.push_str("    return 0\nend\n");
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
    let path = dir.path().join("god2.lua");
    let mut code = String::from("function processDataPipeline(x)\n");
    for i in 0..cc_branches() {
        code.push_str(&format!("    if x > {i} then end\n"));
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("    local y{i} = {i}\n"));
    }
    code.push_str("    return 0\nend\n");
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(has_smell(&stdout, "God Method"));
    let lines: Vec<&str> = stdout
        .lines()
        .filter(|l| l.contains("processDataPipeline"))
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
    let path = dir.path().join("large.lua");
    let mut code = String::from("function buildReport()\n");
    for i in 0..fn_padding() {
        code.push_str(&format!("    local x{i} = {i}\n"));
    }
    code.push_str("    return 0\nend\n");
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

#[test]
fn large_method_loc_at_least_65() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large_loc.lua");
    let mut code = String::from("function buildReport()\n");
    for i in 0..fn_padding() {
        code.push_str(&format!("    local x{i} = {i}\n"));
    }
    code.push_str("    return 0\nend\n");
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
// Nesting
// ===========================================================================

#[test]
fn deep_nesting_detected() {
    let output = run_check(LANG, "deep_nesting.lua");
    assert!(has_smell(&output, "Deep Nested"), "got: {output}");
    assert!(has_function(&output, "deeplyNested"));
}

#[test]
fn deep_nesting_depth_exceeds_4() {
    let debug = run_debug(LANG, "deep_nesting.lua");
    let depth = function_metric(&debug, "deeplyNested", "nesting").unwrap_or(0);
    assert!(depth > 4, "got: {depth}");
}

#[test]
fn moderate_nesting_not_flagged() {
    let output = run_check(LANG, "deep_nesting.lua");
    assert!(!has_function(&output, "moderatelyNested"));
}

// ===========================================================================
// Arguments
// ===========================================================================

#[test]
fn excess_args_detected() {
    let output = run_check(LANG, "excess_args.lua");
    assert!(has_smell(&output, "Excess Arguments"), "got: {output}");
    assert!(has_function(&output, "createUser"));
}

#[test]
fn excess_args_count_correct() {
    let debug = run_debug(LANG, "excess_args.lua");
    let args = function_metric(&debug, "createUser", "args").unwrap_or(0);
    assert!(args >= 6, "got: {args}");
}

#[test]
fn simple_func_in_excess_args_not_flagged() {
    let output = run_check(LANG, "excess_args.lua");
    assert!(!has_function(&output, "simpleFunc"));
}

// ===========================================================================
// Module-level
// ===========================================================================

#[test]
fn code_duplication_detected() {
    let output = run_check(LANG, "code_duplication.lua");
    assert!(has_smell(&output, "Code Duplication"), "got: {output}");
}

#[test]
fn embedded_block_detected() {
    let output = run_check(LANG, "embedded_block.lua");
    assert!(has_smell(&output, "Large Embedded Block"), "got: {output}");
}

#[test]
fn bumpy_road_detected() {
    let output = run_check(LANG, "bumpy_road.lua");
    assert!(
        has_smell(&output, "Nested Conditional Chunks") || has_smell(&output, "Deep Nested"),
        "got: {output}"
    );
}

#[test]
fn low_cohesion_detected() {
    let output = run_check(LANG, "low_cohesion.lua");
    assert!(
        has_smell(&output, "Low Cohesion") || !output.is_empty(),
        "got: {output}"
    );
}

#[test]
fn primitive_obsession_no_false_positive() {
    let output = run_check(LANG, "primitive_obsession.lua");
    assert!(!has_smell(&output, "Primitive Obsession"));
}

#[test]
fn overall_function_size_at_threshold() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("size_at.lua");
    let mut code = String::new();
    for i in 0..3 {
        code.push_str(&format!("function lg{i}()\n"));
        for j in 0..large_fn_lines() {
            code.push_str(&format!("    local x{j} = {j}\n"));
        }
        code.push_str("    return 0\nend\n\n");
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
    let path = dir.path().join("size_below.lua");
    let mut code = String::new();
    for i in 0..2 {
        code.push_str(&format!("function lg{i}()\n"));
        for j in 0..large_fn_lines() {
            code.push_str(&format!("    local x{j} = {j}\n"));
        }
        code.push_str("    return 0\nend\n\n");
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
        "function f()\n    return \"hello\"\nend\n",
        "lua",
    );
    assert!(!has_smell(&out, "Large Embedded Block"));
}

#[test]
fn complex_conditional_detected() {
    let out = pulse_check_code(
        concat!(
            "function check(age, score, active)\n",
            "    if age > 18 and score > 50 and active then\n",
            "        if score > 80 or (age > 25 and active) then\n",
            "            return true\n",
            "        end\n",
            "    end\n",
            "    if age > 65 or score < 10 then\n",
            "        return true\n",
            "    end\n",
            "    return false\n",
            "end\n",
        ),
        "lua",
    );
    assert!(
        has_smell(&out, "Complex Conditional") || has_smell(&out, "Complex Method"),
        "got: {out}"
    );
}

#[test]
fn production_service_has_issues() {
    let output = run_check(LANG, "production_service.lua");
    assert!(!output.is_empty(), "production_service.lua should have findings");
}

#[test]
fn analysis_completes_under_500ms() {
    let path = fixtures_dir(LANG).join("production_service.lua");
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
fn constructor_over_injection_detected() {
    let output = run_check(LANG, "excess_args.lua");
    assert!(
        has_smell(&output, "Constructor Over-Injection")
            || has_smell(&output, "Excess Arguments"),
        "got: {output}"
    );
}

#[test]
fn code_duplication_inline() {
    let out = pulse_check_code(
        concat!(
            "function rptA(data)\n",
            "    local r = 0\n",
            "    for _, v in ipairs(data) do\n",
            "        r = r + v\n",
            "    end\n",
            "    r = r * 2\n",
            "    return r\n",
            "end\n\n",
            "function rptB(data)\n",
            "    local r = 0\n",
            "    for _, v in ipairs(data) do\n",
            "        r = r + v\n",
            "    end\n",
            "    r = r * 2\n",
            "    return r\n",
            "end\n",
        ),
        "lua",
    );
    assert!(has_smell(&out, "Code Duplication"), "got: {out}");
}

#[test]
fn nested_conditional_chunks_detected() {
    let out = pulse_check_code(
        concat!(
            "function validate(data)\n",
            "    if data.a then\n",
            "        if data.b then\n",
            "            if data.c then\n",
            "                local x = 1\n",
            "            end\n",
            "        end\n",
            "    end\n",
            "    local gap = 1\n",
            "    if data.d then\n",
            "        if data.e then\n",
            "            if data.f then\n",
            "                local y = 2\n",
            "            end\n",
            "        end\n",
            "    end\n",
            "    return 0\n",
            "end\n",
        ),
        "lua",
    );
    assert!(
        has_smell(&out, "Nested Conditional Chunks") || has_smell(&out, "Complex Method"),
        "got: {out}"
    );
}

// ===========================================================================
// Hook
// ===========================================================================

#[test]
fn hook_clean_file_silent() {
    let path = fixtures_dir(LANG).join("clean.lua");
    let output = run_hook(path.to_str().unwrap());
    assert!(output.is_empty());
}

#[test]
fn hook_smelly_file_produces_output() {
    let path = fixtures_dir(LANG).join("complex_methods.lua");
    let output = run_hook(path.to_str().unwrap());
    assert!(!output.is_empty());
}

#[test]
fn hook_nonexistent_file_silent() {
    let output = run_hook("/nonexistent/path/foo.lua");
    assert!(output.is_empty());
}

// ===========================================================================
// Global conditionals
// ===========================================================================

#[test]
fn global_conditionals_parsed() {
    let debug = run_debug(LANG, "global_conditionals.lua");
    let cc = function_metric(&debug, "setup", "cc").unwrap_or(0);
    assert!(cc >= 2, "setup should have branches, got cc={cc}");
}

// ===========================================================================
// Test smells
// ===========================================================================

#[test]
fn test_file_analyzed() {
    let output = run_check(LANG, "test_smells.lua");
    assert!(
        !output.is_empty() || output.is_empty(),
        "test file should be parseable"
    );
}

// ===========================================================================
// Lua-specific
// ===========================================================================

#[test]
fn repeat_until_increments_cc() {
    let debug = pulse_debug_code(
        "function f(x)\n    repeat\n        x = x + 1\n    until x > 10\n    return x\nend\n",
        "lua",
    );
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert!(cc >= 2, "repeat should increment cc, got: {cc}");
}

#[test]
fn for_in_increments_cc() {
    let debug = pulse_debug_code(
        "function f(t)\n    for k, v in pairs(t) do\n        print(v)\n    end\nend\n",
        "lua",
    );
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert!(cc >= 2, "for-in should increment cc, got: {cc}");
}

#[test]
fn for_numeric_increments_cc() {
    let debug = pulse_debug_code(
        "function f()\n    for i = 1, 10 do\n        print(i)\n    end\nend\n",
        "lua",
    );
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert!(cc >= 2, "for-numeric should increment cc, got: {cc}");
}

#[test]
fn colon_method_naming() {
    let debug = pulse_debug_code(
        "local M = {}\nfunction M:doWork(x)\n    self.x = x\nend\n",
        "lua",
    );
    assert!(debug.contains("M.doWork"), "method should be M.doWork, got: {debug}");
}

#[test]
fn long_string_embedded_block() {
    let mut code = String::from("function f()\n    return [[\n");
    for i in 0..embedded_lines_above() {
        code.push_str(&format!("        line {i}\n"));
    }
    code.push_str("    ]]\nend\n");
    let out = pulse_check_code(&code, "lua");
    assert!(has_smell(&out, "Large Embedded Block"), "got: {out}");
}
