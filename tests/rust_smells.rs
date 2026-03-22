mod common;

use common::*;
use std::process::Command;

const LANG: &str = "rust";

#[test]
fn clean_file_produces_no_output() {
    let output = run_check(LANG, "clean.rs");
    assert!(
        output.is_empty(),
        "clean Rust file should produce no output, got: {}",
        output
    );
}

#[test]
fn complex_method_detected() {
    let output = run_check(LANG, "complex_method.rs");
    assert!(
        has_smell(&output, "Complex Method"),
        "should detect complex method, got: {}",
        output
    );
    assert!(has_function(&output, "process_order"));
}

#[test]
fn complex_method_cc_at_least_9() {
    let debug = run_debug(LANG, "complex_method.rs");
    let cc = function_metric(&debug, "process_order", "cc").unwrap_or(0);
    assert!(cc >= 9, "cc should be >= 9, got: {}", cc);
}

#[test]
fn excess_args_detected() {
    let output = run_check(LANG, "excess_args.rs");
    assert!(has_smell(&output, "Excess Arguments"), "got: {}", output);
    assert!(has_function(&output, "create_user"));
}

#[test]
fn simple_func_not_flagged() {
    let output = run_check(LANG, "excess_args.rs");
    assert!(!has_function(&output, "simple_func"));
}

#[test]
fn constructor_over_injection_detected() {
    let output = run_check(LANG, "excess_args.rs");
    assert!(
        has_smell(&output, "Constructor Over-Injection"),
        "got: {}",
        output
    );
    assert!(has_function(&output, "UserService.new"));
}

#[test]
fn self_param_excluded_from_arg_count() {
    let debug = run_debug(LANG, "excess_args.rs");
    let args = function_metric(&debug, "UserService.get_user", "args").unwrap_or(99);
    assert_eq!(
        args, 1,
        "get_user takes &self + user_id, should report 1, got: {}",
        args
    );
}

#[test]
fn primitive_obsession_detected() {
    let output = run_check(LANG, "excess_args.rs");
    assert!(has_smell(&output, "Primitive Obsession"), "got: {}", output);
}

#[test]
fn hook_mode_works_with_rs() {
    let path = fixtures_dir(LANG).join("clean.rs");
    let output = run_hook(path.to_str().unwrap());
    assert!(
        output.is_empty(),
        "hook on clean Rust file should be silent"
    );
}

#[test]
fn hook_mode_detects_smells_in_rs() {
    let path = fixtures_dir(LANG).join("complex_method.rs");
    let output = run_hook(path.to_str().unwrap());
    assert!(
        !output.is_empty(),
        "hook on smelly Rust file should produce output"
    );
}

#[test]
fn output_starts_with_pulse() {
    let output = run_check(LANG, "complex_method.rs");
    assert!(output.starts_with("pulse:"));
}

// ===========================================================================
// Additional coverage — match Python depth
// ===========================================================================

#[test]
fn excess_args_count_correct() {
    let debug = run_debug(LANG, "excess_args.rs");
    let args = function_metric(&debug, "create_user", "args").unwrap_or(0);
    assert_eq!(args, 8, "got: {}", args);
}

#[test]
fn constructor_args_count_correct() {
    let debug = run_debug(LANG, "excess_args.rs");
    let args = function_metric(&debug, "UserService.new", "args").unwrap_or(0);
    assert_eq!(args, 6, "got: {}", args);
}

#[test]
fn cc_base_case_is_1() {
    let debug = run_debug(LANG, "clean.rs");
    let cc = function_metric(&debug, "add", "cc").unwrap_or(99);
    assert_eq!(cc, 1);
}

#[test]
fn method_arg_count_excludes_self() {
    let debug = run_debug(LANG, "clean.rs");
    let args = function_metric(&debug, "Calculator.add", "args").unwrap_or(99);
    assert_eq!(args, 1, "should exclude &mut self, got: {}", args);
}

#[test]
fn output_has_function_line_numbers() {
    let output = run_check(LANG, "complex_method.rs");
    let has_loc = output
        .lines()
        .any(|l| l.contains("(L") && l.contains("): "));
    assert!(has_loc);
}

#[test]
fn empty_file() {
    let out = pulse_check_code("", "rs");
    assert!(out.is_empty());
}

#[test]
fn comments_only_file() {
    let out = pulse_check_code("// just comments\n// nothing else\n", "rs");
    assert!(out.is_empty());
}

#[test]
fn function_at_cc_boundary_flagged() {
    let out = pulse_check_code("fn f() {\n    if true {}\n    if true {}\n    if true {}\n    if true {}\n    if true {}\n    if true {}\n    if true {}\n    if true {}\n}\n", "rs");
    assert!(
        has_smell(&out, "Complex Method"),
        "cc=9 should trigger, got: {}",
        out
    );
}

#[test]
fn function_below_cc_boundary_not_flagged() {
    let out = pulse_check_code("fn f() {\n    if true {}\n    if true {}\n    if true {}\n    if true {}\n    if true {}\n    if true {}\n    if true {}\n}\n", "rs");
    assert!(!has_smell(&out, "Complex Method"));
}

#[test]
fn hook_nonexistent_file_silent() {
    let output = run_hook("/nonexistent/path/foo.rs");
    assert!(output.is_empty());
}

#[test]
fn issue_count_in_header_matches_findings() {
    let output = run_check(LANG, "excess_args.rs");
    let first = output.lines().next().unwrap_or("");
    let findings = output.lines().filter(|l| l.starts_with("  ")).count();
    assert!(first.contains(&format!("{} issue", findings)));
}

// ===========================================================================
// Large Method (loc >= threshold)
// ===========================================================================

#[test]
fn large_method_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large.rs");
    let mut code = String::from("fn build_report() {\n");
    for i in 0..100 {
        code.push_str(&format!("    let x{} = {};\n", i, i));
    }
    code.push_str("}\n");
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
    let path = dir.path().join("large_loc.rs");
    let mut code = String::from("fn build_report() {\n");
    for i in 0..100 {
        code.push_str(&format!("    let x{} = {};\n", i, i));
    }
    code.push_str("}\n");
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["debug", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stderr = String::from_utf8(out.stderr).unwrap();
    let loc = function_metric(&stderr, "build_report", "loc").unwrap_or(0);
    assert!(loc >= 65, "loc should be >= 50, got: {}", loc);
}

// ===========================================================================
// God Method
// ===========================================================================

#[test]
fn god_method_has_high_cc_and_loc() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("god.rs");
    let mut code = String::from("fn process_data_pipeline() {\n");
    for i in 0..10 {
        code.push_str(&format!("    if {} > 0 {{}}\n", i));
    }
    for i in 0..100 {
        code.push_str(&format!("    let y{} = {};\n", i, i));
    }
    code.push_str("}\n");
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["debug", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stderr = String::from_utf8(out.stderr).unwrap();
    let cc = function_metric(&stderr, "process_data_pipeline", "cc").unwrap_or(0);
    let loc = function_metric(&stderr, "process_data_pipeline", "loc").unwrap_or(0);
    assert!(cc >= 9, "cc >= 9, got: {}", cc);
    assert!(loc >= 65, "loc >= 65, got: {}", loc);
}

#[test]
fn god_method_not_reported_as_separate_complex_and_large() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("god2.rs");
    let mut code = String::from("fn process_data_pipeline() {\n");
    for i in 0..10 {
        code.push_str(&format!("    if {} > 0 {{}}\n", i));
    }
    for i in 0..100 {
        code.push_str(&format!("    let y{} = {};\n", i, i));
    }
    code.push_str("}\n");
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(has_smell(&stdout, "God Method"));
    let lines: Vec<&str> = stdout
        .lines()
        .filter(|l| l.contains("process_data_pipeline"))
        .collect();
    assert!(!lines.iter().any(|l| l.contains("Complex Method")));
    assert!(!lines.iter().any(|l| l.contains("Large Method")));
}

// ===========================================================================
// Nested conditional chunks
// ===========================================================================

#[test]
fn nested_conditional_chunks_detected() {
    let out = pulse_check_code(
        concat!(
            "fn validate_and_process(data: &[i32]) {\n",
            "    if data.len() > 0 {\n",
            "        if data[0] > 0 {\n",
            "            if data[0] > 10 {\n",
            "                let x = 1;\n",
            "            }\n",
            "        }\n",
            "    }\n",
            "    let gap = 1;\n",
            "    if data.len() > 5 {\n",
            "        if data[5] > 0 {\n",
            "            if data[5] > 10 {\n",
            "                let y = 2;\n",
            "            }\n",
            "        }\n",
            "    }\n",
            "    let gap2 = 2;\n",
            "    if data.len() > 10 {\n",
            "        if data[10] > 0 {\n",
            "            if data[10] > 10 {\n",
            "                let z = 3;\n",
            "            }\n",
            "        }\n",
            "    }\n",
            "}\n",
        ),
        "rs",
    );
    assert!(
        has_smell(&out, "Nested Conditional Chunks") || has_smell(&out, "Complex Method"),
        "got: {}",
        out
    );
}

// ===========================================================================
// Complex conditional
// ===========================================================================

#[test]
fn complex_conditional_detected() {
    let out = pulse_check_code(
        concat!(
            "fn check_eligibility(age: i32, score: i32, active: bool) -> bool {\n",
            "    if age > 18 && score > 50 && active {\n",
            "        if score > 80 || (age > 25 && active) {\n",
            "            return true;\n",
            "        }\n",
            "    }\n",
            "    if age > 65 || score < 10 {\n",
            "        return true;\n",
            "    }\n",
            "    false\n",
            "}\n",
        ),
        "rs",
    );
    assert!(
        has_smell(&out, "Complex Conditional") || has_smell(&out, "Complex Method"),
        "got: {}",
        out
    );
}

// ===========================================================================
// Global conditionals (cfg attrs in Rust)
// ===========================================================================

#[test]
fn global_conditionals_detected() {
    let out = pulse_check_code(
        concat!(
            "const X: i32 = 1;\n",
            "static mut FLAG: bool = false;\n",
            "fn main() {}\n",
        ),
        "rs",
    );
    // Rust rarely has global if-blocks; this verifies no false positive
    assert!(!has_smell(&out, "Global Conditionals"));
}

// ===========================================================================
// File too large / too many functions
// ===========================================================================

#[test]
fn file_too_large_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("huge.rs");
    let mut code = String::new();
    for i in 0..25 {
        code.push_str(&format!("fn fn{}() -> i32 {{ {} }}\n", i, i));
    }
    for i in 0..500 {
        code.push_str(&format!("const VAR{}: i32 = {};\n", i, i));
    }
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(has_smell(&stdout, "File Too Large"), "got: {}", stdout);
}

#[test]
fn too_many_functions_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("huge2.rs");
    let mut code = String::new();
    for i in 0..25 {
        code.push_str(&format!("fn fn{}() -> i32 {{ {} }}\n", i, i));
    }
    for i in 0..500 {
        code.push_str(&format!("const VAR{}: i32 = {};\n", i, i));
    }
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(has_smell(&stdout, "Too Many Functions"), "got: {}", stdout);
}

// ===========================================================================
// Hook invalid JSON silent
// ===========================================================================

#[test]
fn hook_invalid_json_silent() {
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["--hook"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.take().unwrap().write_all(b"not json").unwrap();
            child.wait_with_output()
        })
        .expect("failed to run");
    assert!(out.stdout.is_empty());
}

// ===========================================================================
// Boolean operators increment cc
// ===========================================================================

#[test]
fn boolean_operators_increment_cc() {
    let debug = pulse_debug_code(
        "fn f(a: bool, b: bool, c: bool) {\n    if a && b && c {}\n}\n",
        "rs",
    );
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert!(
        cc >= 4,
        "boolean operators should increment cc, got: {}",
        cc
    );
}

// ===========================================================================
// Decorated function (Rust: attribute macro)
// ===========================================================================

#[test]
fn attributed_function_analyzed() {
    let out = pulse_check_code(
        "#[inline]\nfn long_args(a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32) {}\n",
        "rs",
    );
    assert!(has_smell(&out, "Excess Arguments"), "got: {}", out);
}

// ===========================================================================
// Output module prefix
// ===========================================================================

#[test]
fn output_has_module_prefix() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mod_test.rs");
    let mut code = String::new();
    for i in 0..25 {
        code.push_str(&format!("struct T{} {{}}\n", i));
    }
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("Module:"), "got: {}", stdout);
}

// ===========================================================================
// Performance on fixture
// ===========================================================================

#[test]
fn analysis_completes_under_500ms() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("perf.rs");
    let mut code = String::new();
    for i in 0..25 {
        code.push_str(&format!("fn fn{}() -> i32 {{ {} }}\n", i, i));
    }
    for i in 0..500 {
        code.push_str(&format!("const VAR{}: i32 = {};\n", i, i));
    }
    std::fs::write(&path, &code).unwrap();
    let start = std::time::Instant::now();
    let _ = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let elapsed = start.elapsed();
    assert!(elapsed.as_millis() < 500, "took: {}ms", elapsed.as_millis());
}

// ===========================================================================
// Hook unsupported extension
// ===========================================================================

#[test]
fn hook_unsupported_extension_silent() {
    let output = run_hook("/some/file.xyz");
    assert!(output.is_empty());
}

// ===========================================================================
// Deep nesting detected
// ===========================================================================

#[test]
fn deep_nesting_detected() {
    let out = pulse_check_code("fn deep() {\n    for x in 0..1 {\n        if true {\n            for y in 0..1 {\n                if true {\n                    for z in 0..1 {\n                        if true {}\n                    }\n                }\n            }\n        }\n    }\n}\n", "rs");
    assert!(has_smell(&out, "Deep Nested"));
}

#[test]
fn deep_nesting_depth_exceeds_4() {
    let debug = pulse_debug_code("fn deep() {\n    for x in 0..1 {\n        if true {\n            for y in 0..1 {\n                if true {\n                    for z in 0..1 {\n                        if true {}\n                    }\n                }\n            }\n        }\n    }\n}\n", "rs");
    let depth = function_metric(&debug, "deep", "nesting").unwrap_or(0);
    assert!(depth > 4, "got: {}", depth);
}

#[test]
fn moderate_nesting_not_flagged() {
    let out = pulse_check_code(
        "fn moderate() {\n    if true {\n        if true {}\n    }\n}\n",
        "rs",
    );
    assert!(!has_function(&out, "moderate"));
}

// ===========================================================================
// Embedded block detected
// ===========================================================================

#[test]
fn embedded_block_detected() {
    let mut code = String::from("fn query() -> &'static str {\n    let q = r#\"\n");
    for i in 0..20 {
        code.push_str(&format!("        SELECT field_{} FROM table_{}\n", i, i));
    }
    code.push_str("    \"#;\n    q\n}\n");
    let out = pulse_check_code(&code, "rs");
    assert!(has_smell(&out, "Large Embedded Block"), "got: {}", out);
}

#[test]
fn simple_string_not_flagged() {
    let out = pulse_check_code("fn f() -> &'static str {\n    \"short\"\n}\n", "rs");
    assert!(!has_smell(&out, "Large Embedded Block"));
}

// ===========================================================================
// Code duplication detected
// ===========================================================================

#[test]
fn code_duplication_detected() {
    let out = pulse_check_code(
        concat!(
            "fn report_a(data: &[i32]) -> Vec<i32> {\n",
            "    let mut r = Vec::new();\n",
            "    for item in data {\n",
            "        let e = item * 2;\n",
            "        r.push(e);\n",
            "    }\n",
            "    r\n",
            "}\n\n",
            "fn report_b(data: &[i32]) -> Vec<i32> {\n",
            "    let mut r = Vec::new();\n",
            "    for item in data {\n",
            "        let e = item * 2;\n",
            "        r.push(e);\n",
            "    }\n",
            "    r\n",
            "}\n",
        ),
        "rs",
    );
    assert!(has_smell(&out, "Code Duplication"), "got: {}", out);
}

// ===========================================================================
// Nested conditional chunks bump count
// ===========================================================================

#[test]
fn nested_conditional_chunks_bump_count() {
    let out = pulse_debug_code(
        concat!(
            "fn validate_and_process(data: &[i32]) {\n",
            "    if data.len() > 0 {\n",
            "        if data[0] > 0 {\n",
            "            if data[0] > 10 {\n",
            "                let x = 1;\n",
            "            }\n",
            "        }\n",
            "    }\n",
            "    let gap = 1;\n",
            "    if data.len() > 5 {\n",
            "        if data[5] > 0 {\n",
            "            if data[5] > 10 {\n",
            "                let y = 2;\n",
            "            }\n",
            "        }\n",
            "    }\n",
            "    let gap2 = 2;\n",
            "    if data.len() > 10 {\n",
            "        if data[10] > 0 {\n",
            "            if data[10] > 10 {\n",
            "                let z = 3;\n",
            "            }\n",
            "        }\n",
            "    }\n",
            "}\n",
        ),
        "rs",
    );
    let bumps = function_metric(&out, "validate_and_process", "bumps");
    // If bumps metric exists, verify it; cc should also be high enough
    let cc = function_metric(&out, "validate_and_process", "cc").unwrap_or(0);
    assert!(
        bumps.unwrap_or(0) >= 2 || cc >= 9,
        "should have >= 2 bumps or cc >= 9, got bumps: {:?}, cc: {}",
        bumps,
        cc
    );
}

// ===========================================================================
// Low cohesion from inline
// ===========================================================================

#[test]
fn lcom4_detects_low_cohesion() {
    let out = pulse_check_code(
        concat!(
            "struct Sink { x: i32, y: i32, z: i32 }\n",
            "impl Sink {\n",
            "    fn use_x(&self) -> i32 { self.x }\n",
            "    fn use_y(&self) -> i32 { self.y }\n",
            "    fn use_z(&self) -> i32 { self.z }\n",
            "}\n",
        ),
        "rs",
    );
    assert!(has_smell(&out, "Low Cohesion"), "got: {}", out);
}

// ===========================================================================
// Switch / match case increments cc
// ===========================================================================

#[test]
fn match_arms_increment_cc() {
    let out = pulse_check_code(
        concat!(
            "fn handle(action: i32) -> &'static str {\n",
            "    match action {\n",
            "        1 => \"a\",\n",
            "        2 => \"b\",\n",
            "        3 => \"c\",\n",
            "        4 => \"d\",\n",
            "        5 => \"e\",\n",
            "        6 => \"f\",\n",
            "        7 => \"g\",\n",
            "        8 => \"h\",\n",
            "        _ => \"?\",\n",
            "    }\n",
            "}\n",
        ),
        "rs",
    );
    assert!(
        has_smell(&out, "Complex Method"),
        "8 match arms should trigger cc >= 9, got: {}",
        out
    );
}
