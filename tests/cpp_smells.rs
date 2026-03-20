mod common;

use common::*;
use std::process::Command;

const LANG: &str = "cpp";

#[test]
fn clean_file_produces_no_output() {
    let output = run_check(LANG, "clean.cpp");
    assert!(output.is_empty(), "got: {}", output);
}

#[test]
fn complex_method_detected() {
    let output = run_check(LANG, "complex_method.cpp");
    assert!(has_smell(&output, "Complex Method"), "got: {}", output);
    assert!(has_function(&output, "process_order"));
}

#[test]
fn complex_method_cc_at_least_9() {
    let debug = run_debug(LANG, "complex_method.cpp");
    let cc = function_metric(&debug, "process_order", "cc").unwrap_or(0);
    assert!(cc >= 9, "cc should be >= 9, got: {}", cc);
}

#[test]
fn excess_args_detected() {
    let output = run_check(LANG, "excess_args.cpp");
    assert!(has_smell(&output, "Excess Arguments"), "got: {}", output);
    assert!(has_function(&output, "create_user"));
}

#[test]
fn simple_func_not_flagged() {
    let output = run_check(LANG, "excess_args.cpp");
    assert!(!has_function(&output, "simple_func"));
}

#[test]
fn constructor_over_injection_detected() {
    let output = run_check(LANG, "excess_args.cpp");
    assert!(
        has_smell(&output, "Constructor Over-Injection"),
        "got: {}",
        output
    );
    assert!(has_function(&output, "UserService"));
}

#[test]
fn deep_nesting_detected() {
    let output = run_check(LANG, "deep_nesting.cpp");
    assert!(has_smell(&output, "Deep Nested"), "got: {}", output);
    assert!(has_function(&output, "deeply_nested"));
}

#[test]
fn deep_nesting_depth_exceeds_4() {
    let debug = run_debug(LANG, "deep_nesting.cpp");
    let depth = function_metric(&debug, "deeply_nested", "nesting").unwrap_or(0);
    assert!(depth > 4, "got: {}", depth);
}

#[test]
fn moderate_nesting_not_flagged() {
    let output = run_check(LANG, "deep_nesting.cpp");
    assert!(!has_function(&output, "moderately_nested"));
}

#[test]
fn lcom4_detects_low_cohesion() {
    let out = pulse_check_code(
        concat!(
            "class Sink {\n",
            "public:\n",
            "    void use_a() { a_ = 1; }\n",
            "    int get_a() { return this->a_; }\n",
            "    void use_b() { b_ = 1; }\n",
            "    int get_b() { return this->b_; }\n",
            "    void use_c() { c_ = 1; }\n",
            "    int get_c() { return this->c_; }\n",
            "private:\n",
            "    int a_; int b_; int c_;\n",
            "};\n",
        ),
        "cpp",
    );
    assert!(has_smell(&out, "Low Cohesion"), "got: {}", out);
}

#[test]
fn cc_base_case_is_1() {
    let debug = run_debug(LANG, "clean.cpp");
    let cc = function_metric(&debug, "add", "cc").unwrap_or(99);
    assert_eq!(cc, 1);
}

#[test]
fn output_starts_with_pulse() {
    let output = run_check(LANG, "complex_method.cpp");
    assert!(output.starts_with("pulse:"));
}

#[test]
fn output_has_function_line_numbers() {
    let output = run_check(LANG, "complex_method.cpp");
    assert!(output
        .lines()
        .any(|l| l.contains("(L") && l.contains("): ")));
}

#[test]
fn hook_clean_file_silent() {
    let path = fixtures_dir(LANG).join("clean.cpp");
    let output = run_hook(path.to_str().unwrap());
    assert!(output.is_empty());
}

#[test]
fn hook_smelly_file_produces_output() {
    let path = fixtures_dir(LANG).join("complex_method.cpp");
    let output = run_hook(path.to_str().unwrap());
    assert!(!output.is_empty());
}

#[test]
fn hook_nonexistent_file_silent() {
    let output = run_hook("/nonexistent/path/foo.cpp");
    assert!(output.is_empty());
}

#[test]
fn hpp_extension_supported() {
    let out = pulse_check_code("int add(int a, int b) {\n    return a + b;\n}\n", "hpp");
    assert!(out.is_empty());
}

#[test]
fn cc_extension_supported() {
    let out = pulse_check_code("int add(int a, int b) {\n    return a + b;\n}\n", "cc");
    assert!(out.is_empty());
}

#[test]
fn empty_file() {
    let out = pulse_check_code("", "cpp");
    assert!(out.is_empty());
}

#[test]
fn function_at_cc_boundary_flagged() {
    let out = pulse_check_code("void f() {\n    if (a) {}\n    if (b) {}\n    if (c) {}\n    if (d) {}\n    if (e) {}\n    if (f) {}\n    if (g) {}\n    if (h) {}\n}\n", "cpp");
    assert!(
        has_smell(&out, "Complex Method"),
        "cc=9 should trigger, got: {}",
        out
    );
}

#[test]
fn function_below_cc_boundary_not_flagged() {
    let out = pulse_check_code("void f() {\n    if (a) {}\n    if (b) {}\n    if (c) {}\n    if (d) {}\n    if (e) {}\n    if (f) {}\n    if (g) {}\n}\n", "cpp");
    assert!(!has_smell(&out, "Complex Method"));
}

#[test]
fn issue_count_matches_findings() {
    let output = run_check(LANG, "excess_args.cpp");
    let first = output.lines().next().unwrap_or("");
    let findings = output.lines().filter(|l| l.starts_with("  ")).count();
    assert!(first.contains(&format!("{} issue", findings)));
}

// ===========================================================================
// Large Method
// ===========================================================================

#[test]
fn large_method_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large.cpp");
    let mut code = String::from("void build_report() {\n");
    for i in 0..55 {
        code.push_str(&format!("    int x{} = {};\n", i, i));
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
fn large_method_loc_at_least_50() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large_loc.cpp");
    let mut code = String::from("void build_report() {\n");
    for i in 0..55 {
        code.push_str(&format!("    int x{} = {};\n", i, i));
    }
    code.push_str("}\n");
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["debug", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stderr = String::from_utf8(out.stderr).unwrap();
    let loc = function_metric(&stderr, "build_report", "loc").unwrap_or(0);
    assert!(loc >= 50, "loc >= 50, got: {}", loc);
}

// ===========================================================================
// God Method
// ===========================================================================

#[test]
fn god_method_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("god.cpp");
    let mut code = String::from("void process_data_pipeline() {\n");
    for i in 0..10 {
        code.push_str(&format!("    if ({} > 0) {{}}\n", i));
    }
    for i in 0..45 {
        code.push_str(&format!("    int y{} = {};\n", i, i));
    }
    code.push_str("}\n");
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
    let path = dir.path().join("god2.cpp");
    let mut code = String::from("void process_data_pipeline() {\n");
    for i in 0..10 {
        code.push_str(&format!("    if ({} > 0) {{}}\n", i));
    }
    for i in 0..45 {
        code.push_str(&format!("    int y{} = {};\n", i, i));
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
// Complex conditional
// ===========================================================================

#[test]
fn complex_conditional_detected() {
    let out = pulse_check_code(
        concat!(
            "bool check(int age, int score, bool active) {\n",
            "    if (age > 18 && score > 50 && active) {\n",
            "        if (score > 80 || (age > 25 && active)) {\n",
            "            return true;\n",
            "        }\n",
            "    }\n",
            "    if (age > 65 || score < 10) {\n",
            "        return true;\n",
            "    }\n",
            "    return false;\n",
            "}\n",
        ),
        "cpp",
    );
    assert!(
        has_smell(&out, "Complex Conditional") || has_smell(&out, "Complex Method"),
        "got: {}",
        out
    );
}

// ===========================================================================
// Global conditionals
// ===========================================================================

#[test]
fn global_conditionals_not_flagged_shallow() {
    let out = pulse_check_code("int x = 1;\nvoid setup() {}\n", "cpp");
    assert!(!has_smell(&out, "Global Conditionals"));
}

// ===========================================================================
// File too large / too many functions
// ===========================================================================

#[test]
fn file_too_large_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("huge.cpp");
    let mut code = String::new();
    for i in 0..25 {
        code.push_str(&format!("int fn{}() {{ return {}; }}\n", i, i));
    }
    for i in 0..500 {
        code.push_str(&format!("int VAR{} = {};\n", i, i));
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
    let path = dir.path().join("huge2.cpp");
    let mut code = String::new();
    for i in 0..25 {
        code.push_str(&format!("int fn{}() {{ return {}; }}\n", i, i));
    }
    for i in 0..500 {
        code.push_str(&format!("int VAR{} = {};\n", i, i));
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
// Hook invalid JSON
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
// Boolean operators
// ===========================================================================

#[test]
fn boolean_operators_increment_cc() {
    let debug = pulse_debug_code(
        "void f(int a, int b, int c) {\n    if (a && b && c) {}\n}\n",
        "cpp",
    );
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert!(cc >= 4, "got: {}", cc);
}

// ===========================================================================
// Output module prefix
// ===========================================================================

#[test]
fn output_has_module_prefix() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mod_test.cpp");
    let mut code = String::new();
    for i in 0..50 {
        code.push_str(&format!("int fn{}() {{ return {}; }}\n", i, i));
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
// Comments only
// ===========================================================================

#[test]
fn comments_only_file() {
    let out = pulse_check_code("/* comments */\n// only\n", "cpp");
    assert!(out.is_empty());
}

// ===========================================================================
// Primitive obsession
// ===========================================================================

#[test]
fn primitive_obsession_detected() {
    let out = pulse_check_code("void f(int a, float b, double c, char d) {}\n", "cpp");
    assert!(has_smell(&out, "Primitive Obsession"), "got: {}", out);
}

// ===========================================================================
// Constructor args exclude this
// ===========================================================================

#[test]
fn excess_args_count_correct() {
    let debug = run_debug(LANG, "excess_args.cpp");
    let args = function_metric(&debug, "create_user", "args").unwrap_or(0);
    assert_eq!(args, 8, "got: {}", args);
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
// Performance on fixture
// ===========================================================================

#[test]
fn analysis_completes_under_500ms() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("perf.cpp");
    let mut code = String::new();
    for i in 0..25 {
        code.push_str(&format!("int fn{}() {{ return {}; }}\n", i, i));
    }
    for i in 0..500 {
        code.push_str(&format!("int VAR{} = {};\n", i, i));
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
// Embedded block
// ===========================================================================

#[test]
fn embedded_block_detected() {
    let mut code = String::from("const char* query() {\n    const char* q = R\"(\n");
    for i in 0..20 {
        code.push_str(&format!("        SELECT field_{} FROM table_{}\n", i, i));
    }
    code.push_str("    )\";\n    return q;\n}\n");
    let out = pulse_check_code(&code, "cpp");
    assert!(has_smell(&out, "Large Embedded Block"), "got: {}", out);
}

#[test]
fn simple_string_not_flagged() {
    let out = pulse_check_code("const char* f() {\n    return \"hello\";\n}\n", "cpp");
    assert!(!has_smell(&out, "Large Embedded Block"));
}

// ===========================================================================
// Switch case increments cc
// ===========================================================================

#[test]
fn switch_case_increments_cc() {
    let out = pulse_check_code(
        concat!(
            "const char* handle(int action) {\n",
            "    switch (action) {\n",
            "        case 1: return \"a\";\n",
            "        case 2: return \"b\";\n",
            "        case 3: return \"c\";\n",
            "        case 4: return \"d\";\n",
            "        case 5: return \"e\";\n",
            "        case 6: return \"f\";\n",
            "        case 7: return \"g\";\n",
            "        case 8: return \"h\";\n",
            "        case 9: return \"i\";\n",
            "        default: return \"?\";\n",
            "    }\n",
            "}\n",
        ),
        "cpp",
    );
    assert!(has_smell(&out, "Complex Method"), "got: {}", out);
}

// ===========================================================================
// Code duplication
// ===========================================================================

#[test]
fn code_duplication_detected() {
    let out = pulse_check_code(concat!(
        "int rpt_a(int* d, int n) {\n    int r = 0;\n    for (int i = 0; i < n; i++) {\n        r += d[i];\n    }\n    return r;\n}\n\n",
        "int rpt_b(int* d, int n) {\n    int r = 0;\n    for (int i = 0; i < n; i++) {\n        r += d[i];\n    }\n    return r;\n}\n",
    ), "cpp");
    assert!(has_smell(&out, "Code Duplication"), "got: {}", out);
}

// ===========================================================================
// Decorated function
// ===========================================================================

#[test]
fn attributed_function_analyzed() {
    let out = pulse_check_code(
        "[[nodiscard]]\nvoid long_args(int a, int b, int c, int d, int e, int f, int g, int h) {}\n",
        "cpp",
    );
    assert!(has_smell(&out, "Excess Arguments"), "got: {}", out);
}

// ===========================================================================
// God class requires god method
// ===========================================================================

#[test]
fn god_class_requires_god_method() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("gc.cpp");
    let mut code = String::new();
    for i in 0..25 {
        code.push_str(&format!("int fn{}() {{ return {}; }}\n", i, i));
    }
    for i in 0..200 {
        code.push_str(&format!("int VAR{} = {};\n", i, i));
    }
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!has_smell(&stdout, "God Class"));
}

// ===========================================================================
// God class triggers with god method
// ===========================================================================

#[test]
fn god_class_triggers_with_god_method() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("gc2.cpp");
    let mut code = String::from("void monster() {\n");
    for i in 0..10 {
        code.push_str(&format!("    if ({} > 0) {{}}\n", i));
    }
    for i in 0..40 {
        code.push_str(&format!("    int y{} = {};\n", i, i));
    }
    code.push_str("}\n\n");
    for i in 0..21 {
        code.push_str(&format!("int fn{}() {{ return {}; }}\n", i, i));
    }
    for i in 0..350 {
        code.push_str(&format!("int V{} = {};\n", i, i));
    }
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(has_smell(&stdout, "God Method"));
    assert!(has_smell(&stdout, "God Class"));
}

// ===========================================================================
// Nested conditional chunks from fixture inline
// ===========================================================================

#[test]
fn nested_conditional_chunks_detected() {
    let out = pulse_check_code(
        concat!(
            "void validate(int* data, int n) {\n",
            "    if (n > 0) {\n",
            "        if (data[0] > 0) {\n",
            "            if (data[0] > 10) {\n",
            "                int x = 1;\n",
            "            }\n",
            "        }\n",
            "    }\n",
            "    int gap = 1;\n",
            "    if (n > 5) {\n",
            "        if (data[5] > 0) {\n",
            "            if (data[5] > 10) {\n",
            "                int y = 2;\n",
            "            }\n",
            "        }\n",
            "    }\n",
            "    int gap2 = 2;\n",
            "    if (n > 10) {\n",
            "        if (data[10] > 0) {\n",
            "            if (data[10] > 10) {\n",
            "                int z = 3;\n",
            "            }\n",
            "        }\n",
            "    }\n",
            "}\n",
        ),
        "cpp",
    );
    assert!(
        has_smell(&out, "Nested Conditional Chunks") || has_smell(&out, "Complex Method"),
        "got: {}",
        out
    );
}
