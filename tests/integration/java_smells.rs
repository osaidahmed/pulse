
use crate::common::*;
use std::process::Command;

const LANG: &str = "java";

#[test]
fn clean_file_produces_no_output() {
    let output = run_check(LANG, "Clean.java");
    assert!(output.is_empty(), "got: {output}");
}

#[test]
fn complex_method_detected() {
    let output = run_check(LANG, "ComplexMethod.java");
    assert!(has_smell(&output, "Complex Method"), "got: {output}");
    assert!(has_function(&output, "processOrder"));
}

#[test]
fn complex_method_cc_at_least_9() {
    let debug = run_debug(LANG, "ComplexMethod.java");
    let cc = function_metric(&debug, "processOrder", "cc").unwrap_or(0);
    assert!(cc >= 9, "cc should be >= 9, got: {cc}");
}

#[test]
fn excess_args_detected() {
    let output = run_check(LANG, "ExcessArgs.java");
    assert!(has_smell(&output, "Excess Arguments"), "got: {output}");
    assert!(has_function(&output, "createUser"));
}

#[test]
fn excess_args_count_correct() {
    let debug = run_debug(LANG, "ExcessArgs.java");
    let args = function_metric(&debug, "createUser", "args").unwrap_or(0);
    assert_eq!(args, 8, "got: {args}");
}

#[test]
fn simple_func_not_flagged() {
    let output = run_check(LANG, "ExcessArgs.java");
    assert!(!has_function(&output, "simpleFunc"));
}

#[test]
fn constructor_over_injection_detected() {
    let output = run_check(LANG, "ExcessArgs.java");
    assert!(
        has_smell(&output, "Constructor Over-Injection"),
        "got: {output}"
    );
}

#[test]
fn deep_nesting_detected() {
    let output = run_check(LANG, "DeepNesting.java");
    assert!(has_smell(&output, "Deep Nested"), "got: {output}");
    assert!(has_function(&output, "deeplyNested"));
}

#[test]
fn deep_nesting_depth_exceeds_4() {
    let debug = run_debug(LANG, "DeepNesting.java");
    let depth = function_metric(&debug, "deeplyNested", "nesting").unwrap_or(0);
    assert!(depth > 4, "got: {depth}");
}

#[test]
fn moderate_nesting_not_flagged() {
    let output = run_check(LANG, "DeepNesting.java");
    assert!(!has_function(&output, "moderatelyNested"));
}

#[test]
fn cc_base_case_is_1() {
    let debug = run_debug(LANG, "Clean.java");
    let cc = function_metric(&debug, "add", "cc").unwrap_or(99);
    assert_eq!(cc, 1);
}

#[test]
fn output_starts_with_pulse() {
    let output = run_check(LANG, "ComplexMethod.java");
    assert!(output.starts_with("pulse:"));
}

#[test]
fn output_has_function_line_numbers() {
    let output = run_check(LANG, "ComplexMethod.java");
    assert!(output
        .lines()
        .any(|l| l.contains("(L") && l.contains("): ")));
}

#[test]
fn hook_clean_file_silent() {
    let path = fixtures_dir(LANG).join("Clean.java");
    let output = run_hook(path.to_str().unwrap());
    assert!(output.is_empty());
}

#[test]
fn hook_smelly_file_produces_output() {
    let path = fixtures_dir(LANG).join("ComplexMethod.java");
    let output = run_hook(path.to_str().unwrap());
    assert!(!output.is_empty());
}

#[test]
fn hook_nonexistent_file_silent() {
    let output = run_hook("/nonexistent/path/Foo.java");
    assert!(output.is_empty());
}

#[test]
fn empty_file() {
    let out = pulse_check_code("", "java");
    assert!(out.is_empty());
}

#[test]
fn function_at_cc_boundary_flagged() {
    let out = pulse_check_code("class T {\n    void f() {\n        if (a) {}\n        if (b) {}\n        if (c) {}\n        if (d) {}\n        if (e) {}\n        if (f) {}\n        if (g) {}\n        if (h) {}\n    }\n}\n", "java");
    assert!(
        has_smell(&out, "Complex Method"),
        "cc=9 should trigger, got: {out}"
    );
}

#[test]
fn function_below_cc_boundary_not_flagged() {
    let out = pulse_check_code("class T {\n    void f() {\n        if (a) {}\n        if (b) {}\n        if (c) {}\n        if (d) {}\n        if (e) {}\n        if (f) {}\n        if (g) {}\n    }\n}\n", "java");
    assert!(!has_smell(&out, "Complex Method"));
}

#[test]
fn issue_count_matches_findings() {
    let output = run_check(LANG, "ComplexMethod.java");
    let first = output.lines().next().unwrap_or("");
    let findings = output.lines().filter(|l| l.starts_with("  ")).count();
    assert!(first.contains(&format!("{findings} issue")));
}

// ===========================================================================
// Large Method
// ===========================================================================

#[test]
fn large_method_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Large.java");
    let mut code = String::from("class Large {\n    void buildReport() {\n");
    for i in 0..fn_padding() {
        code.push_str(&format!("        int x{i} = {i};\n"));
    }
    code.push_str("    }\n}\n");
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
    let path = dir.path().join("LargeLoc.java");
    let mut code = String::from("class LargeLoc {\n    void buildReport() {\n");
    for i in 0..fn_padding() {
        code.push_str(&format!("        int x{i} = {i};\n"));
    }
    code.push_str("    }\n}\n");
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
// God Method
// ===========================================================================

#[test]
fn god_method_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("God.java");
    let mut code = String::from("class God {\n    void processDataPipeline() {\n");
    for i in 0..cc_branches() {
        code.push_str(&format!("        if ({i} > 0) {{}}\n"));
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("        int y{i} = {i};\n"));
    }
    code.push_str("    }\n}\n");
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
    let path = dir.path().join("God2.java");
    let mut code = String::from("class God2 {\n    void processDataPipeline() {\n");
    for i in 0..cc_branches() {
        code.push_str(&format!("        if ({i} > 0) {{}}\n"));
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("        int y{i} = {i};\n"));
    }
    code.push_str("    }\n}\n");
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
// Complex conditional
// ===========================================================================

#[test]
fn complex_conditional_detected() {
    let out = pulse_check_code(
        concat!(
            "class T {\n",
            "    boolean check(int age, int score, boolean active) {\n",
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
        "java",
    );
    assert!(
        has_smell(&out, "Complex Conditional") || has_smell(&out, "Complex Method"),
        "got: {out}"
    );
}

// ===========================================================================
// File too large / too many functions
// ===========================================================================

#[test]
fn file_too_large_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Huge.java");
    let mut code = String::from("class Huge {\n");
    for i in 0..declarations_above() {
        code.push_str(&format!("    int fn{i}() {{ return {i}; }}\n"));
    }
    code.push_str("}\n");
    for i in 0..file_padding() {
        code.push_str(&format!("// padding line {i}\n"));
    }
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        has_smell(&stdout, "File Too Large") || has_smell(&stdout, "Too Many Functions"),
        "got: {stdout}"
    );
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
    let debug = pulse_debug_code("class T {\n    void f(boolean a, boolean b, boolean c) {\n        if (a && b && c) {}\n    }\n}\n", "java");
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert!(cc >= 4, "got: {cc}");
}

// ===========================================================================
// Output module prefix
// ===========================================================================

#[test]
fn output_has_module_prefix() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Mod.java");
    let mut code = String::from("class Mod {\n");
    for i in 0..declarations_above() {
        code.push_str(&format!("    int fn{i}() {{ return {i}; }}\n"));
    }
    code.push_str("}\n");
    for i in 0..file_padding() {
        code.push_str(&format!("// padding {i}\n"));
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
    let out = pulse_check_code("// just comments\n// nothing else\n", "java");
    assert!(out.is_empty());
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
// Performance
// ===========================================================================

#[test]
fn analysis_completes_under_500ms() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Perf.java");
    let mut code = String::from("class Perf {\n");
    for i in 0..25 {
        code.push_str(&format!("    int fn{i}() {{ return {i}; }}\n"));
    }
    code.push_str("}\n");
    for i in 0..500 {
        code.push_str(&format!("// line {i}\n"));
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
    // Use a text block (Java 15+) for embedded block detection
    let mut code = String::from("class T {\n    String query() {\n        String q = \"\"\"\n");
    for i in 0..embedded_lines_above() {
        code.push_str(&format!(
            "            SELECT field_{i} FROM table_{i}\n"
        ));
    }
    code.push_str("            \"\"\";\n        return q;\n    }\n}\n");
    let out = pulse_check_code(&code, "java");
    assert!(has_smell(&out, "Large Embedded Block"), "got: {out}");
}

#[test]
fn simple_string_not_flagged() {
    let out = pulse_check_code(
        "class T {\n    String f() {\n        return \"hello\";\n    }\n}\n",
        "java",
    );
    assert!(!has_smell(&out, "Large Embedded Block"));
}

// ===========================================================================
// Decorated / annotated function
// ===========================================================================

#[test]
fn annotated_function_analyzed() {
    let out = pulse_check_code(
        "class T {\n    @Override\n    void longArgs(int a, int b, int c, int d, int e, int f, int g, int h) {}\n}\n",
        "java",
    );
    assert!(has_smell(&out, "Excess Arguments"), "got: {out}");
}

// ===========================================================================
// Switch case increments cc
// ===========================================================================

#[test]
fn switch_case_increments_cc() {
    let out = pulse_check_code(
        concat!(
            "class T {\n",
            "    String handle(int action) {\n",
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
        "java",
    );
    assert!(has_smell(&out, "Complex Method"), "got: {out}");
}

// ===========================================================================
// Code duplication
// ===========================================================================

#[test]
fn code_duplication_detected() {
    let out = pulse_check_code(
        concat!(
            "class T {\n",
            "    int rptA(int[] d) {\n",
            "        int r = 0;\n",
            "        for (int v : d) { r += v; }\n",
            "        r = r * 2;\n",
            "        return r;\n",
            "    }\n",
            "    int rptB(int[] d) {\n",
            "        int r = 0;\n",
            "        for (int v : d) { r += v; }\n",
            "        r = r * 2;\n",
            "        return r;\n",
            "    }\n",
            "}\n",
        ),
        "java",
    );
    assert!(has_smell(&out, "Code Duplication"), "got: {out}");
}

// ===========================================================================
// Primitive obsession: specific types
// ===========================================================================

#[test]
fn primitive_obsession_recognizes_boolean_char() {
    let out = pulse_check_code(
        "class T {\n    void f(boolean a, char b, byte c, short d, boolean e) {}\n}\n",
        "java",
    );
    assert!(has_smell(&out, "Primitive Obsession"));
}

// ===========================================================================
// Nested conditional chunks
// ===========================================================================

#[test]
fn nested_conditional_chunks_detected() {
    let out = pulse_check_code(
        concat!(
            "class T {\n",
            "    void validate(int[] data) {\n",
            "        if (data.length > 0) {\n",
            "            if (data[0] > 0) {\n",
            "                if (data[0] > 10) {\n",
            "                    int x = 1;\n",
            "                }\n",
            "            }\n",
            "        }\n",
            "        int gap = 1;\n",
            "        if (data.length > 5) {\n",
            "            if (data[5] > 0) {\n",
            "                if (data[5] > 10) {\n",
            "                    int y = 2;\n",
            "                }\n",
            "            }\n",
            "        }\n",
            "        int gap2 = 2;\n",
            "        if (data.length > 10) {\n",
            "            if (data[10] > 0) {\n",
            "                if (data[10] > 10) {\n",
            "                    int z = 3;\n",
            "                }\n",
            "            }\n",
            "        }\n",
            "    }\n",
            "}\n",
        ),
        "java",
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
    let path = dir.path().join("Decl.java");
    let mut code = String::new();
    for i in 0..declarations_above() {
        code.push_str(&format!("class T{i} {{}}\n"));
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
    let path = dir.path().join("Size.java");
    let mut code = String::from("class Size {\n");
    for i in 0..2 {
        code.push_str(&format!("    void lg{i}() {{\n"));
        for j in 0..45 {
            code.push_str(&format!("        int x{j} = {j};\n"));
        }
        code.push_str("    }\n");
    }
    code.push_str("}\n");
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
    let path = dir.path().join("Size2.java");
    let mut code = String::from("class Size2 {\n");
    for i in 0..3 {
        code.push_str(&format!("    void lg{i}() {{\n"));
        for j in 0..45 {
            code.push_str(&format!("        int x{j} = {j};\n"));
        }
        code.push_str("    }\n");
    }
    code.push_str("}\n");
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
// God class requires god method
// ===========================================================================

#[test]
fn god_class_requires_god_method() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("GC.java");
    let mut code = String::from("class GC {\n");
    for i in 0..declarations_above() {
        code.push_str(&format!("    int fn{i}() {{ return {i}; }}\n"));
    }
    code.push_str("}\n");
    for i in 0..file_padding() {
        code.push_str(&format!("// padding {i}\n"));
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
    let path = dir.path().join("GC2.java");
    let mut code = String::from("class GC2 {\n    void monster() {\n");
    for i in 0..cc_branches() {
        code.push_str(&format!("        if ({i} > 0) {{}}\n"));
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("        int y{i} = {i};\n"));
    }
    code.push_str("    }\n");
    for i in 0..functions_above() {
        code.push_str(&format!("    int fn{i}() {{ return {i}; }}\n"));
    }
    // Pad with actual code lines (not just comments) to ensure file is large
    for i in 0..file_padding() {
        code.push_str(&format!("    static final int V{i} = {i};\n"));
    }
    code.push_str("}\n");
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
// Low cohesion from fixture inline
// ===========================================================================

#[test]
fn lcom4_detects_low_cohesion() {
    let out = pulse_check_code(
        concat!(
            "class Sink {\n",
            "    private int x; private int y; private int z;\n",
            "    void useX() { this.x = 1; }\n",
            "    int getX() { return this.x; }\n",
            "    void useY() { this.y = 1; }\n",
            "    int getY() { return this.y; }\n",
            "    void useZ() { this.z = 1; }\n",
            "    int getZ() { return this.z; }\n",
            "}\n",
        ),
        "java",
    );
    assert!(has_smell(&out, "Low Cohesion"), "got: {out}");
}

// ===========================================================================
// Annotated function with excess args
// ===========================================================================

#[test]
fn annotated_class_method_analyzed() {
    let out = pulse_check_code(
        "class T {\n    @Deprecated\n    void longArgs(int a, int b, int c, int d, int e, int f, int g, int h) {}\n}\n",
        "java",
    );
    assert!(has_smell(&out, "Excess Arguments"), "got: {out}");
}

// ===========================================================================
// Primitive obsession: mixed not flagged
// ===========================================================================

#[test]
fn primitive_obsession_mixed_not_flagged_smells() {
    let out = pulse_check_code(
        "class T {\n    void f(int a, String b, Object c, Object d) {}\n}\n",
        "java",
    );
    assert!(!has_smell(&out, "Primitive Obsession"));
}

#[test]
fn empty_catch_detected_java() {
    let out = pulse_check_code(
        "class T { void f() { try { int x = 1; } catch (Exception e) { } } }\n",
        "java",
    );
    assert!(has_smell(&out, "Empty Error Handler"), "got: {out}");
}

#[test]
fn finally_body_branching_contributes_to_cc_java() {
    let debug = pulse_debug_code(
        "class T { void f(boolean ok, boolean alt) { try { int x = 1; } catch (Exception e) { } finally { if (ok) { System.out.println(1); } else if (alt) { System.out.println(2); } else { System.out.println(3); } } } }\n",
        "java",
    );
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert!(cc >= 3, "finally if/else if/else must bump cc, got cc={cc}");
}
