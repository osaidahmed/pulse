mod common;

use common::*;

const LANG: &str = "csharp";

#[test]
fn clean_file_no_output() {
    let output = run_check(LANG, "clean.cs");
    assert!(output.is_empty(), "clean file should have no issues: {}", output);
}

#[test]
fn complex_method_detected() {
    let output = run_check(LANG, "complex_methods.cs");
    assert!(has_smell(&output, "God Method"), "got: {}", output);
    assert!(has_function(&output, "ProcessOrder"));
}

#[test]
fn excess_args_detected() {
    let output = run_check(LANG, "excess_args.cs");
    assert!(has_smell(&output, "Excess Arguments"), "got: {}", output);
    assert!(has_function(&output, "CreateUser"));
}

#[test]
fn deep_nesting_detected() {
    let output = run_check(LANG, "deep_nesting.cs");
    assert!(has_smell(&output, "Deep Nested"), "got: {}", output);
    assert!(has_function(&output, "DeeplyNested"));
}

#[test]
fn code_duplication_detected() {
    let output = run_check(LANG, "code_duplication.cs");
    assert!(has_smell(&output, "Code Duplication"), "got: {}", output);
}

#[test]
fn embedded_block_detected() {
    let output = run_check(LANG, "embedded_block.cs");
    assert!(has_smell(&output, "Large Embedded Block"), "got: {}", output);
    assert!(has_function(&output, "GetActiveUsers"));
}

#[test]
fn bumpy_road_detected() {
    let output = run_check(LANG, "bumpy_road.cs");
    assert!(has_smell(&output, "Nested Conditional Chunks"), "got: {}", output);
    assert!(has_function(&output, "ValidateAndProcess"));
}

#[test]
fn low_cohesion_detected() {
    let output = run_check(LANG, "low_cohesion.cs");
    assert!(has_smell(&output, "Low Cohesion"), "got: {}", output);
}

#[test]
fn primitive_obsession_detected() {
    let output = run_check(LANG, "primitive_obsession.cs");
    assert!(has_smell(&output, "Primitive Obsession"), "got: {}", output);
}

#[test]
fn production_service_has_issues() {
    let output = run_check(LANG, "production_service.cs");
    assert!(!output.is_empty(), "production service should have issues");
    assert!(has_smell(&output, "Constructor Over-Injection"), "got: {}", output);
    assert!(has_smell(&output, "God Method"), "got: {}", output);
    assert!(has_smell(&output, "Excess Arguments"), "got: {}", output);
}

// ===========================================================================
// Output format tests
// ===========================================================================

#[test]
fn output_starts_with_pulse() {
    let output = run_check(LANG, "complex_methods.cs");
    assert!(output.starts_with("pulse:"));
}

#[test]
fn output_has_function_line_numbers() {
    let output = run_check(LANG, "complex_methods.cs");
    assert!(output
        .lines()
        .any(|l| l.contains("(L") && l.contains("): ")));
}

#[test]
fn output_has_module_prefix() {
    let output = run_check(LANG, "production_service.cs");
    assert!(output.contains("Module:"), "got: {}", output);
}

#[test]
fn issue_count_matches_findings() {
    let output = run_check(LANG, "complex_methods.cs");
    let first = output.lines().next().unwrap_or("");
    let findings = output.lines().filter(|l| l.starts_with("  ")).count();
    assert!(first.contains(&format!("{} issue", findings)));
}

// ===========================================================================
// Clean / empty file tests
// ===========================================================================

#[test]
fn empty_file() {
    let out = pulse_check_code("", "cs");
    assert!(out.is_empty());
}

#[test]
fn comments_only_file() {
    let out = pulse_check_code("// just comments\n// nothing else\n", "cs");
    assert!(out.is_empty());
}

#[test]
fn simple_func_not_flagged() {
    let out = pulse_check_code(
        "public class T {\n    int Add(int a, int b) {\n        return a + b;\n    }\n}\n",
        "cs",
    );
    assert!(out.is_empty(), "got: {}", out);
}

// ===========================================================================
// CC boundary tests
// ===========================================================================

#[test]
fn cc_base_case_is_1() {
    let debug = run_debug(LANG, "clean.cs");
    let cc = function_metric(&debug, "Add", "cc").unwrap_or(99);
    assert_eq!(cc, 1);
}

#[test]
fn function_at_cc_boundary_flagged() {
    let out = pulse_check_code(
        concat!(
            "public class T {\n",
            "    void F() {\n",
            "        if (a) {}\n",
            "        if (b) {}\n",
            "        if (c) {}\n",
            "        if (d) {}\n",
            "        if (e) {}\n",
            "        if (f) {}\n",
            "        if (g) {}\n",
            "        if (h) {}\n",
            "    }\n",
            "}\n",
        ),
        "cs",
    );
    assert!(
        has_smell(&out, "Complex Method"),
        "cc=9 should trigger, got: {}",
        out
    );
}

#[test]
fn function_below_cc_boundary_not_flagged() {
    let out = pulse_check_code(
        concat!(
            "public class T {\n",
            "    void F() {\n",
            "        if (a) {}\n",
            "        if (b) {}\n",
            "        if (c) {}\n",
            "        if (d) {}\n",
            "        if (e) {}\n",
            "        if (f) {}\n",
            "        if (g) {}\n",
            "    }\n",
            "}\n",
        ),
        "cs",
    );
    assert!(!has_smell(&out, "Complex Method"));
}

#[test]
fn boolean_operators_increment_cc() {
    let debug = pulse_debug_code(
        "public class T {\n    void F(bool a, bool b, bool c) {\n        if (a && b && c) {}\n    }\n}\n",
        "cs",
    );
    let cc = function_metric(&debug, "F", "cc").unwrap_or(0);
    assert!(cc >= 4, "got: {}", cc);
}

#[test]
fn switch_case_increments_cc() {
    let out = pulse_check_code(
        concat!(
            "public class T {\n",
            "    string Handle(int action) {\n",
            "        switch (action) {\n",
            "            case 1: return \"a\";\n",
            "            case 2: return \"b\";\n",
            "            case 3: return \"c\";\n",
            "            case 4: return \"d\";\n",
            "            case 5: return \"e\";\n",
            "            case 6: return \"f\";\n",
            "            case 7: return \"g\";\n",
            "            case 8: return \"h\";\n",
            "            case 9: return \"i\";\n",
            "            default: return \"?\";\n",
            "        }\n",
            "    }\n",
            "}\n",
        ),
        "cs",
    );
    assert!(has_smell(&out, "Complex Method"), "got: {}", out);
}

// ===========================================================================
// Complexity smell tests
// ===========================================================================

#[test]
fn complex_method_cc_at_least_9() {
    let debug = run_debug(LANG, "complex_methods.cs");
    let cc = function_metric(&debug, "ProcessOrder", "cc").unwrap_or(0);
    assert!(cc >= 9, "cc should be >= 9, got: {}", cc);
}

#[test]
fn god_method_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("God.cs");
    let mut code = String::from("public class God {\n    void ProcessDataPipeline() {\n");
    for i in 0..10 {
        code.push_str(&format!("        if ({} > 0) {{}}\n", i));
    }
    for i in 0..45 {
        code.push_str(&format!("        int y{} = {};\n", i, i));
    }
    code.push_str("    }\n}\n");
    std::fs::write(&path, &code).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(has_smell(&stdout, "God Method"), "got: {}", stdout);
}

#[test]
fn god_method_not_reported_as_separate() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("God2.cs");
    let mut code = String::from("public class God2 {\n    void ProcessDataPipeline() {\n");
    for i in 0..10 {
        code.push_str(&format!("        if ({} > 0) {{}}\n", i));
    }
    for i in 0..45 {
        code.push_str(&format!("        int y{} = {};\n", i, i));
    }
    code.push_str("    }\n}\n");
    std::fs::write(&path, &code).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_pulse"))
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
    let path = dir.path().join("Large.cs");
    let mut code = String::from("public class Large {\n    void BuildReport() {\n");
    for i in 0..55 {
        code.push_str(&format!("        int x{} = {};\n", i, i));
    }
    code.push_str("    }\n}\n");
    std::fs::write(&path, &code).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_pulse"))
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
    let path = dir.path().join("LargeLoc.cs");
    let mut code = String::from("public class LargeLoc {\n    void BuildReport() {\n");
    for i in 0..55 {
        code.push_str(&format!("        int x{} = {};\n", i, i));
    }
    code.push_str("    }\n}\n");
    std::fs::write(&path, &code).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["debug", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stderr = String::from_utf8(out.stderr).unwrap();
    let loc = function_metric(&stderr, "BuildReport", "loc").unwrap_or(0);
    assert!(loc >= 50, "loc >= 50, got: {}", loc);
}

// ===========================================================================
// Nesting tests
// ===========================================================================

#[test]
fn deep_nesting_depth_exceeds_4() {
    let debug = run_debug(LANG, "deep_nesting.cs");
    let depth = function_metric(&debug, "DeeplyNested", "nesting").unwrap_or(0);
    assert!(depth > 4, "got: {}", depth);
}

#[test]
fn moderate_nesting_not_flagged() {
    let output = run_check(LANG, "deep_nesting.cs");
    assert!(!has_function(&output, "ModeratelyNested"));
}

// ===========================================================================
// Argument tests
// ===========================================================================

#[test]
fn excess_args_count_correct() {
    let debug = run_debug(LANG, "excess_args.cs");
    let args = function_metric(&debug, "CreateUser", "args").unwrap_or(0);
    assert_eq!(args, 8, "got: {}", args);
}

#[test]
fn constructor_over_injection_detected() {
    let output = run_check(LANG, "excess_args.cs");
    assert!(
        has_smell(&output, "Constructor Over-Injection"),
        "got: {}",
        output
    );
}

// ===========================================================================
// Module-level tests
// ===========================================================================

#[test]
fn file_too_large_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Huge.cs");
    let mut code = String::from("public class Huge {\n");
    for i in 0..25 {
        code.push_str(&format!("    int Fn{}() {{ return {}; }}\n", i, i));
    }
    code.push_str("}\n");
    for i in 0..500 {
        code.push_str(&format!("// padding line {}\n", i));
    }
    std::fs::write(&path, &code).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        has_smell(&stdout, "File Too Large") || has_smell(&stdout, "Too Many Functions"),
        "got: {}",
        stdout
    );
}

#[test]
fn declarations_above_threshold() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Decl.cs");
    let mut code = String::new();
    for i in 0..25 {
        code.push_str(&format!("public class T{} {{}}\n", i));
    }
    std::fs::write(&path, &code).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(has_smell(&stdout, "Declarations"), "got: {}", stdout);
}

#[test]
fn overall_function_size_at_threshold() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Size2.cs");
    let mut code = String::from("public class Size2 {\n");
    for i in 0..3 {
        code.push_str(&format!("    void Lg{}() {{\n", i));
        for j in 0..45 {
            code.push_str(&format!("        int x{} = {};\n", j, j));
        }
        code.push_str("    }\n");
    }
    code.push_str("}\n");
    std::fs::write(&path, &code).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_pulse"))
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
    let path = dir.path().join("Size.cs");
    let mut code = String::from("public class Size {\n");
    for i in 0..2 {
        code.push_str(&format!("    void Lg{}() {{\n", i));
        for j in 0..45 {
            code.push_str(&format!("        int x{} = {};\n", j, j));
        }
        code.push_str("    }\n");
    }
    code.push_str("}\n");
    std::fs::write(&path, &code).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!has_smell(&stdout, "Overall Function Size"));
}

#[test]
fn god_class_triggers_with_god_method() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("GC2.cs");
    let mut code = String::from("public class GC2 {\n    void Monster() {\n");
    for i in 0..10 {
        code.push_str(&format!("        if ({} > 0) {{}}\n", i));
    }
    for i in 0..40 {
        code.push_str(&format!("        int y{} = {};\n", i, i));
    }
    code.push_str("    }\n");
    for i in 0..21 {
        code.push_str(&format!("    int Fn{}() {{ return {}; }}\n", i, i));
    }
    for i in 0..350 {
        code.push_str(&format!("    static readonly int V{} = {};\n", i, i));
    }
    code.push_str("}\n");
    std::fs::write(&path, &code).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(has_smell(&stdout, "God Method"));
    assert!(has_smell(&stdout, "God Class"));
}

#[test]
fn god_class_requires_god_method() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("GC.cs");
    let mut code = String::from("public class GC {\n");
    for i in 0..25 {
        code.push_str(&format!("    int Fn{}() {{ return {}; }}\n", i, i));
    }
    code.push_str("}\n");
    for i in 0..200 {
        code.push_str(&format!("// padding {}\n", i));
    }
    std::fs::write(&path, &code).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!has_smell(&stdout, "God Class"));
}

// ===========================================================================
// Other tests
// ===========================================================================

#[test]
fn simple_string_not_flagged() {
    let out = pulse_check_code(
        "public class T {\n    string F() {\n        return \"hello\";\n    }\n}\n",
        "cs",
    );
    assert!(!has_smell(&out, "Large Embedded Block"));
}

#[test]
fn complex_conditional_detected() {
    let out = pulse_check_code(
        concat!(
            "public class T {\n",
            "    bool Check(int age, int score, bool active) {\n",
            "        if (age > 18 && score > 50 && active) {\n",
            "            if (score > 80 || (age > 25 && active)) {\n",
            "                return true;\n",
            "            }\n",
            "        }\n",
            "        if (age > 65 || score < 10) {\n",
            "            return true;\n",
            "        }\n",
            "        return false;\n",
            "    }\n",
            "}\n",
        ),
        "cs",
    );
    assert!(
        has_smell(&out, "Complex Conditional") || has_smell(&out, "Complex Method"),
        "got: {}",
        out
    );
}

#[test]
fn lcom4_detects_low_cohesion() {
    let out = pulse_check_code(
        concat!(
            "public class Sink {\n",
            "    private int x;\n",
            "    private int y;\n",
            "    private int z;\n",
            "    void UseX() { this.x = 1; }\n",
            "    int GetX() { return this.x; }\n",
            "    void UseY() { this.y = 1; }\n",
            "    int GetY() { return this.y; }\n",
            "    void UseZ() { this.z = 1; }\n",
            "    int GetZ() { return this.z; }\n",
            "}\n",
        ),
        "cs",
    );
    assert!(has_smell(&out, "Low Cohesion"), "got: {}", out);
}

#[test]
fn primitive_obsession_mixed_not_flagged() {
    let out = pulse_check_code(
        "public class T {\n    void F(int a, MyType b, MyOther c, SomeObj d) {}\n}\n",
        "cs",
    );
    assert!(!has_smell(&out, "Primitive Obsession"));
}

// ===========================================================================
// Hook tests
// ===========================================================================

#[test]
fn hook_clean_file_silent() {
    let path = fixtures_dir(LANG).join("clean.cs");
    let output = run_hook(path.to_str().unwrap());
    assert!(output.is_empty());
}

#[test]
fn hook_smelly_file_produces_output() {
    let path = fixtures_dir(LANG).join("complex_methods.cs");
    let output = run_hook(path.to_str().unwrap());
    assert!(!output.is_empty());
}

#[test]
fn hook_nonexistent_file_silent() {
    let output = run_hook("/nonexistent/path/Foo.cs");
    assert!(output.is_empty());
}

#[test]
fn hook_unsupported_extension_silent() {
    let output = run_hook("/some/file.xyz");
    assert!(output.is_empty());
}

// ===========================================================================
// Performance
// ===========================================================================

#[test]
fn analysis_completes_under_500ms() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Perf.cs");
    let mut code = String::from("public class Perf {\n");
    for i in 0..25 {
        code.push_str(&format!("    int Fn{}() {{ return {}; }}\n", i, i));
    }
    code.push_str("}\n");
    for i in 0..500 {
        code.push_str(&format!("// line {}\n", i));
    }
    std::fs::write(&path, &code).unwrap();
    let start = std::time::Instant::now();
    let _ = std::process::Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let elapsed = start.elapsed();
    assert!(elapsed.as_millis() < 500, "took: {}ms", elapsed.as_millis());
}
