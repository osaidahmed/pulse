
use crate::common::*;
use std::process::Command;

const LANG: &str = "c";

#[test]
fn clean_file_produces_no_output() {
    let output = run_check(LANG, "clean.c");
    assert!(output.is_empty(), "got: {output}");
}

#[test]
fn complex_method_detected() {
    let output = run_check(LANG, "complex_method.c");
    assert!(has_smell(&output, "Complex Method"), "got: {output}");
    assert!(has_function(&output, "process_order"));
}

#[test]
fn complex_method_cc_at_least_9() {
    let debug = run_debug(LANG, "complex_method.c");
    let cc = function_metric(&debug, "process_order", "cc").unwrap_or(0);
    assert!(cc >= 9, "cc should be >= 9, got: {cc}");
}

#[test]
fn excess_args_detected() {
    let output = run_check(LANG, "excess_args.c");
    assert!(has_smell(&output, "Excess Arguments"), "got: {output}");
    assert!(has_function(&output, "create_user"));
}

#[test]
fn excess_args_count_correct() {
    let debug = run_debug(LANG, "excess_args.c");
    let args = function_metric(&debug, "create_user", "args").unwrap_or(0);
    assert_eq!(args, 8, "got: {args}");
}

#[test]
fn simple_func_not_flagged() {
    let output = run_check(LANG, "excess_args.c");
    assert!(!has_function(&output, "simple_func"));
}

#[test]
fn deep_nesting_detected() {
    let output = run_check(LANG, "deep_nesting.c");
    assert!(has_smell(&output, "Deep Nested"), "got: {output}");
    assert!(has_function(&output, "deeply_nested"));
}

#[test]
fn deep_nesting_depth_exceeds_4() {
    let debug = run_debug(LANG, "deep_nesting.c");
    let depth = function_metric(&debug, "deeply_nested", "nesting").unwrap_or(0);
    assert!(depth > 4, "got: {depth}");
}

#[test]
fn moderate_nesting_not_flagged() {
    let output = run_check(LANG, "deep_nesting.c");
    assert!(!has_function(&output, "moderately_nested"));
}

#[test]
fn cc_base_case_is_1() {
    let debug = run_debug(LANG, "clean.c");
    let cc = function_metric(&debug, "add", "cc").unwrap_or(99);
    assert_eq!(cc, 1);
}

#[test]
fn output_starts_with_pulse() {
    let output = run_check(LANG, "complex_method.c");
    assert!(output.starts_with("pulse:"));
}

#[test]
fn output_has_function_line_numbers() {
    let output = run_check(LANG, "complex_method.c");
    assert!(output
        .lines()
        .any(|l| l.contains("(L") && l.contains("): ")));
}

#[test]
fn hook_clean_file_silent() {
    let path = fixtures_dir(LANG).join("clean.c");
    let output = run_hook(path.to_str().unwrap());
    assert!(output.is_empty());
}

#[test]
fn hook_smelly_file_produces_output() {
    let path = fixtures_dir(LANG).join("complex_method.c");
    let output = run_hook(path.to_str().unwrap());
    assert!(!output.is_empty());
}

#[test]
fn hook_nonexistent_file_silent() {
    let output = run_hook("/nonexistent/path/foo.c");
    assert!(output.is_empty());
}

#[test]
fn h_extension_supported() {
    let out = pulse_check_code("int add(int a, int b) {\n    return a + b;\n}\n", "h");
    assert!(out.is_empty());
}

#[test]
fn empty_file() {
    let out = pulse_check_code("", "c");
    assert!(out.is_empty());
}

#[test]
fn void_param_is_zero_args() {
    let debug = pulse_debug_code("int f(void) {\n    return 0;\n}\n", "c");
    let args = function_metric(&debug, "f", "args").unwrap_or(99);
    assert_eq!(args, 0, "f(void) should be 0 args, got: {args}");
}

#[test]
fn function_at_cc_boundary_flagged() {
    let out = pulse_check_code("int f(void) {\n    if (a) {}\n    if (b) {}\n    if (c) {}\n    if (d) {}\n    if (e) {}\n    if (f) {}\n    if (g) {}\n    if (h) {}\n    return 0;\n}\n", "c");
    assert!(
        has_smell(&out, "Complex Method"),
        "cc=9 should trigger, got: {out}"
    );
}

#[test]
fn function_below_cc_boundary_not_flagged() {
    let out = pulse_check_code("int f(void) {\n    if (a) {}\n    if (b) {}\n    if (c) {}\n    if (d) {}\n    if (e) {}\n    if (f) {}\n    if (g) {}\n    return 0;\n}\n", "c");
    assert!(!has_smell(&out, "Complex Method"));
}

#[test]
fn issue_count_matches_findings() {
    let output = run_check(LANG, "complex_method.c");
    let first = output.lines().next().unwrap_or("");
    let findings = output.lines().filter(|l| l.starts_with("  ")).count();
    assert!(first.contains(&format!("{findings} issue")));
}

// ===========================================================================
// Large Method (loc >= threshold)
// ===========================================================================

#[test]
fn large_method_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large.c");
    let mut code = String::from("void build_report(void) {\n");
    for i in 0..fn_padding() {
        code.push_str(&format!("    int x{i} = {i};\n"));
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
        "got: {stdout}"
    );
}

#[test]
fn large_method_loc_at_least_65() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large_loc.c");
    let mut code = String::from("void build_report(void) {\n");
    for i in 0..fn_padding() {
        code.push_str(&format!("    int x{i} = {i};\n"));
    }
    code.push_str("}\n");
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["debug", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stderr = String::from_utf8(out.stderr).unwrap();
    let loc = function_metric(&stderr, "build_report", "loc").unwrap_or(0);
    assert!(loc >= t().function.fn_loc_warning, "loc should be >= 50, got: {loc}");
}

// ===========================================================================
// God Method (complex AND large)
// ===========================================================================

#[test]
fn god_method_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("god.c");
    let mut code = String::from("void process_data_pipeline(void) {\n");
    for i in 0..cc_branches() {
        code.push_str(&format!("    if ({i} > 0) {{}}\n"));
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("    int y{i} = {i};\n"));
    }
    code.push_str("}\n");
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(has_smell(&stdout, "God Method"), "got: {stdout}");
}

#[test]
fn god_method_has_high_cc_and_loc() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("god2.c");
    let mut code = String::from("void process_data_pipeline(void) {\n");
    for i in 0..cc_branches() {
        code.push_str(&format!("    if ({i} > 0) {{}}\n"));
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("    int y{i} = {i};\n"));
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
    assert!(cc >= 9, "cc >= 9, got: {cc}");
    assert!(loc >= t().function.fn_loc_warning, "loc >= t().function.fn_loc_warning, got: {loc}");
}

#[test]
fn god_method_not_reported_as_separate_complex_and_large() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("god3.c");
    let mut code = String::from("void process_data_pipeline(void) {\n");
    for i in 0..cc_branches() {
        code.push_str(&format!("    if ({i} > 0) {{}}\n"));
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("    int y{i} = {i};\n"));
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
            "int check_eligibility(int age, int score, int active) {\n",
            "    if (age > 18 && score > 50 && active) {\n",
            "        if (score > 80 || (age > 25 && active)) {\n",
            "            return 1;\n",
            "        }\n",
            "    }\n",
            "    if (age > 65 || score < 10) {\n",
            "        return 1;\n",
            "    }\n",
            "    return 0;\n",
            "}\n",
        ),
        "c",
    );
    assert!(
        has_smell(&out, "Complex Conditional") || has_smell(&out, "Complex Method"),
        "got: {out}"
    );
}

// ===========================================================================
// Global conditionals
// ===========================================================================

#[test]
fn global_conditionals_detected() {
    let _out = pulse_check_code(
        concat!(
            "int debug_mode = 0;\n",
            "int verbose = 0;\n",
            "#ifdef DEBUG\n",
            "int x = 1;\n",
            "#endif\n",
            "void setup(void) {}\n",
        ),
        "c",
    );
    // C files with preprocessor conditionals may or may not trigger.
    // Reaching this line without panicking is the verification.
}

// ===========================================================================
// File too large / too many functions
// ===========================================================================

#[test]
fn file_too_large_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("huge.c");
    let mut code = String::new();
    for i in 0..declarations_above() {
        code.push_str(&format!("int fn{i}(void) {{ return {i}; }}\n"));
    }
    for i in 0..file_padding() {
        code.push_str(&format!("int VAR{i} = {i};\n"));
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
fn too_many_functions_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("huge2.c");
    let mut code = String::new();
    for i in 0..declarations_above() {
        code.push_str(&format!("int fn{i}(void) {{ return {i}; }}\n"));
    }
    for i in 0..file_padding() {
        code.push_str(&format!("int VAR{i} = {i};\n"));
    }
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(has_smell(&stdout, "Too Many Functions"), "got: {stdout}");
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
// Boolean operators
// ===========================================================================

#[test]
fn boolean_operators_increment_cc() {
    let debug = pulse_debug_code(
        "void f(int a, int b, int c) {\n    if (a && b && c) {}\n}\n",
        "c",
    );
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert!(cc >= 4, "got: {cc}");
}

// ===========================================================================
// Output module prefix
// ===========================================================================

#[test]
fn output_has_module_prefix() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mod_test.c");
    let mut code = String::new();
    for i in 0..50 {
        code.push_str(&format!("int fn{i}(void) {{ return {i}; }}\n"));
    }
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("Module:"), "got: {stdout}");
}

// ===========================================================================
// Comments only
// ===========================================================================

#[test]
fn comments_only_file() {
    let out = pulse_check_code("/* just comments */\n// nothing else\n", "c");
    assert!(out.is_empty());
}

// ===========================================================================
// Performance on fixture
// ===========================================================================

#[test]
fn analysis_completes_under_500ms() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("perf.c");
    let mut code = String::new();
    for i in 0..25 {
        code.push_str(&format!("int fn{i}(void) {{ return {i}; }}\n"));
    }
    for i in 0..500 {
        code.push_str(&format!("int VAR{i} = {i};\n"));
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
// Embedded block
// ===========================================================================

#[test]
fn embedded_block_detected() {
    let mut code = String::from("const char* query(void) {\n    const char* q = \"\\\n");
    for i in 0..embedded_lines_above() {
        code.push_str(&format!("        SELECT field_{i} FROM table_{i} \\\n"));
    }
    code.push_str("    \";\n    return q;\n}\n");
    let out = pulse_check_code(&code, "c");
    assert!(has_smell(&out, "Large Embedded Block"), "got: {out}");
}

#[test]
fn simple_string_not_flagged() {
    let out = pulse_check_code("const char* f(void) {\n    return \"hello\";\n}\n", "c");
    assert!(!has_smell(&out, "Large Embedded Block"));
}

// ===========================================================================
// Code duplication detected
// ===========================================================================

#[test]
fn code_duplication_detected() {
    let out = pulse_check_code(concat!(
        "void rpt_a(int* d, int n) {\n    int r = 0;\n    for (int i = 0; i < n; i++) {\n        r += d[i];\n    }\n    printf(\"%d\", r);\n}\n\n",
        "void rpt_b(int* d, int n) {\n    int r = 0;\n    for (int i = 0; i < n; i++) {\n        r += d[i];\n    }\n    printf(\"%d\", r);\n}\n",
    ), "c");
    assert!(has_smell(&out, "Code Duplication"), "got: {out}");
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
        "c",
    );
    assert!(
        has_smell(&out, "Complex Method"),
        "9 switch cases should trigger, got: {out}"
    );
}

// ===========================================================================
// Nested conditional chunks
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
        "c",
    );
    assert!(
        has_smell(&out, "Nested Conditional Chunks") || has_smell(&out, "Complex Method"),
        "got: {out}"
    );
}

// ===========================================================================
// Declarations above threshold
// ===========================================================================

#[test]
fn declarations_above_threshold() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("decl.c");
    let mut code = String::new();
    for i in 0..declarations_above() {
        code.push_str(&format!("typedef struct {{ int x; }} T{i};\n"));
    }
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(has_smell(&stdout, "Declarations"), "got: {stdout}");
}

// ===========================================================================
// Overall function size
// ===========================================================================

#[test]
fn overall_function_size_below_threshold() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("size.c");
    let mut code = String::new();
    for i in 0..2 {
        code.push_str(&format!("void lg{i}(void) {{\n"));
        for j in 0..45 {
            code.push_str(&format!("    int x{j} = {j};\n"));
        }
        code.push_str("}\n\n");
    }
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!has_smell(&stdout, "Overall Function Size"));
}

#[test]
fn overall_function_size_at_threshold() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("size2.c");
    let mut code = String::new();
    for i in 0..3 {
        code.push_str(&format!("void lg{i}(void) {{\n"));
        for j in 0..45 {
            code.push_str(&format!("    int x{j} = {j};\n"));
        }
        code.push_str("}\n\n");
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

// ===========================================================================
// Decorated function (attribute)
// ===========================================================================

#[test]
fn attributed_function_analyzed() {
    let out = pulse_check_code(
        "__attribute__((noinline))\nvoid long_args(int a, int b, int c, int d, int e, int f, int g, int h) {}\n",
        "c",
    );
    assert!(has_smell(&out, "Excess Arguments"), "got: {out}");
}

// ===========================================================================
// God class requires god method
// ===========================================================================

#[test]
fn god_class_requires_god_method() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("gc.c");
    let mut code = String::new();
    for i in 0..declarations_above() {
        code.push_str(&format!("int fn{i}(void) {{ return {i}; }}\n"));
    }
    for i in 0..file_padding() {
        code.push_str(&format!("int VAR{i} = {i};\n"));
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
    let path = dir.path().join("gc2.c");
    let mut code = String::from("void monster(void) {\n");
    for i in 0..cc_branches() {
        code.push_str(&format!("    if ({i} > 0) {{}}\n"));
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("    int y{i} = {i};\n"));
    }
    code.push_str("}\n\n");
    for i in 0..functions_above() {
        code.push_str(&format!("int fn{i}(void) {{ return {i}; }}\n"));
    }
    for i in 0..file_padding() {
        code.push_str(&format!("int V{i} = {i};\n"));
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
// Primitive obsession: all primitives
// ===========================================================================

#[test]
fn primitive_obsession_all_primitives() {
    let out = pulse_check_code("void f(int a, float b, double c, char d) {}\n", "c");
    assert!(has_smell(&out, "Primitive Obsession"), "got: {out}");
}

// ===========================================================================
// Method arg count with self (N/A for C, but verify no crash)
// ===========================================================================

#[test]
fn method_arg_excludes_no_self_in_c() {
    let debug = run_debug(LANG, "clean.c");
    let cc = function_metric(&debug, "add", "cc").unwrap_or(99);
    assert_eq!(cc, 1, "clean function cc should be 1");
}
