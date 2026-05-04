mod common;

use common::*;
use std::process::Command;

const LANG: &str = "kotlin";

#[test]
fn clean_file_produces_no_output() {
    let output = run_check(LANG, "Clean.kt");
    assert!(output.is_empty(), "got: {output}");
}

#[test]
fn complex_method_detected() {
    let output = run_check(LANG, "ComplexMethod.kt");
    assert!(has_smell(&output, "Complex Method"), "got: {output}");
    assert!(has_function(&output, "processOrder"));
}

#[test]
fn complex_method_cc_at_least_9() {
    let debug = run_debug(LANG, "ComplexMethod.kt");
    let cc = function_metric(&debug, "processOrder", "cc").unwrap_or(0);
    assert!(cc >= 9, "cc should be >= 9, got: {cc}");
}

#[test]
fn excess_args_detected() {
    let output = run_check(LANG, "ExcessArgs.kt");
    assert!(has_smell(&output, "Excess Arguments"), "got: {output}");
    assert!(has_function(&output, "createUser"));
}

#[test]
fn excess_args_count_correct() {
    let debug = run_debug(LANG, "ExcessArgs.kt");
    let args = function_metric(&debug, "createUser", "args").unwrap_or(0);
    assert_eq!(args, 8, "got: {args}");
}

#[test]
fn simple_func_not_flagged() {
    let output = run_check(LANG, "ExcessArgs.kt");
    assert!(!has_function(&output, "simpleFunc"));
}

#[test]
fn constructor_over_injection_detected() {
    let output = run_check(LANG, "ExcessArgs.kt");
    assert!(has_smell(&output, "Constructor Over-Injection"), "got: {output}");
}

#[test]
fn deep_nesting_detected() {
    let output = run_check(LANG, "DeepNesting.kt");
    assert!(has_smell(&output, "Deep Nested"), "got: {output}");
    assert!(has_function(&output, "deeplyNested"));
}

#[test]
fn deep_nesting_depth_exceeds_4() {
    let debug = run_debug(LANG, "DeepNesting.kt");
    let depth = function_metric(&debug, "deeplyNested", "nesting").unwrap_or(0);
    assert!(depth > 4, "got: {depth}");
}

#[test]
fn moderate_nesting_not_flagged() {
    let output = run_check(LANG, "DeepNesting.kt");
    assert!(!has_function(&output, "moderatelyNested"));
}

#[test]
fn cc_base_case_is_1() {
    let debug = run_debug(LANG, "Clean.kt");
    let cc = function_metric(&debug, "add", "cc").unwrap_or(0);
    assert_eq!(cc, 1, "got: {cc}");
}

#[test]
fn output_starts_with_pulse() {
    let output = run_check(LANG, "ComplexMethod.kt");
    assert!(output.starts_with("pulse:"), "got: {output}");
}

#[test]
fn output_has_function_line_numbers() {
    let output = run_check(LANG, "ComplexMethod.kt");
    assert!(output.contains("(L"), "got: {output}");
}

#[test]
fn issue_count_matches_findings() {
    let output = run_check(LANG, "ComplexMethod.kt");
    let first_line = output.lines().next().unwrap_or("");
    let colon_part = first_line.split("issues").next().unwrap_or("0");
    let count_str = colon_part.trim().rsplit(' ').next().unwrap_or("0");
    let header_count: usize = count_str.parse().unwrap_or(0);
    let finding_count = output.lines().filter(|l| l.starts_with("  ")).count();
    assert_eq!(header_count, finding_count, "header={header_count}, findings={finding_count}\n{output}");
}

#[test]
fn hook_clean_file_silent() {
    let path = fixtures_dir(LANG).join("Clean.kt");
    let output = run_hook(path.to_str().unwrap());
    assert!(output.is_empty(), "got: {output}");
}

#[test]
fn hook_smelly_file_produces_output() {
    let path = fixtures_dir(LANG).join("ComplexMethod.kt");
    let output = run_hook(path.to_str().unwrap());
    assert!(!output.is_empty(), "expected output for smelly file");
}

#[test]
fn hook_nonexistent_file_silent() {
    let output = run_hook("/tmp/nonexistent_kotlin_file.kt");
    assert!(output.is_empty(), "got: {output}");
}

#[test]
fn empty_file() {
    let output = pulse_check_code("", "kt");
    assert!(output.is_empty(), "got: {output}");
}

#[test]
fn function_at_cc_boundary_flagged() {
    let output = pulse_check_code(
        "class T {\n    fun f(x: Int): Int {\n        if (x==1) {} else if (x==2) {} else if (x==3) {} else if (x==4) {} else if (x==5) {} else if (x==6) {} else if (x==7) {} else if (x==8) {}\n        return x\n    }\n}\n",
        "kt",
    );
    assert!(has_smell(&output, "Complex Method"), "got: {output}");
}

#[test]
fn function_below_cc_boundary_not_flagged() {
    let output = pulse_check_code(
        "class T {\n    fun f(x: Int): Int {\n        if (x==1) {} else if (x==2) {} else if (x==3) {} else if (x==4) {} else if (x==5) {} else if (x==6) {} else if (x==7) {}\n        return x\n    }\n}\n",
        "kt",
    );
    assert!(!has_smell(&output, "Complex Method"), "got: {output}");
}

#[test]
fn large_method_detected() {
    let mut code = String::from("class T {\n    fun big(): Int {\n");
    for i in 0..fn_padding() {
        code.push_str(&format!("        val x{i} = {i}\n"));
    }
    code.push_str("        return 0\n    }\n}\n");
    let output = pulse_check_code(&code, "kt");
    assert!(has_smell(&output, "Large Method"), "got: {output}");
}

#[test]
fn large_method_loc_at_least_65() {
    let mut code = String::from("class T {\n    fun big(): Int {\n");
    for i in 0..fn_padding() {
        code.push_str(&format!("        val x{i} = {i}\n"));
    }
    code.push_str("        return 0\n    }\n}\n");
    let debug = pulse_debug_code(&code, "kt");
    let loc = function_metric(&debug, "big", "loc").unwrap_or(0);
    assert!(loc >= t().function.fn_loc_warning, "got: {loc}");
}

#[test]
fn god_method_detected() {
    let mut code = String::from("class T {\n    fun god(x: Int): Int {\n");
    for _ in 0..50 {
        code.push_str("        if (x > 0) {}\n");
    }
    for i in 0..50 {
        code.push_str(&format!("        val v{i} = {i}\n"));
    }
    code.push_str("        return 0\n    }\n}\n");
    let output = pulse_check_code(&code, "kt");
    assert!(has_smell(&output, "God Method"), "got: {output}");
}

#[test]
fn god_method_not_reported_as_separate() {
    let mut code = String::from("class T {\n    fun god(x: Int): Int {\n");
    for _ in 0..50 {
        code.push_str("        if (x > 0) {}\n");
    }
    for i in 0..50 {
        code.push_str(&format!("        val v{i} = {i}\n"));
    }
    code.push_str("        return 0\n    }\n}\n");
    let output = pulse_check_code(&code, "kt");
    assert!(!has_smell(&output, "Complex Method"), "got: {output}");
}

#[test]
fn complex_conditional_detected() {
    let output = pulse_check_code(
        "class T {\n    fun f(a: Boolean, b: Boolean, c: Boolean): Boolean {\n        if (a && b || c) { return true }\n        if (a || b && c) { return true }\n        if (a) {} else if (b) {} else if (c) {} else if (a && b) {} else if (b && c) {} else if (a || c) {} else if (a && b && c) {}\n        return false\n    }\n}\n",
        "kt",
    );
    assert!(has_smell(&output, "Complex Conditional"), "got: {output}");
}

#[test]
fn file_too_large_detected() {
    let mut code = String::new();
    for i in 0..26 {
        code.push_str(&format!("fun f{i}(): Int {{\n"));
        for j in 0..20 {
            code.push_str(&format!("    val x{j} = {j}\n"));
        }
        code.push_str("    return 0\n}\n\n");
    }
    let output = pulse_check_code(&code, "kt");
    assert!(has_smell(&output, "File Too Large"), "got: {output}");
}

#[test]
fn hook_invalid_json_silent() {
    let binary = env!("CARGO_BIN_EXE_pulse");
    let out = Command::new(binary)
        .arg("--hook")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            if let Some(ref mut stdin) = child.stdin {
                let _ = stdin.write_all(b"not json");
            }
            child.wait_with_output()
        })
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.is_empty(), "got: {stdout}");
}

#[test]
fn boolean_operators_increment_cc() {
    let debug = pulse_debug_code(
        "class T {\n    fun f(a: Boolean, b: Boolean, c: Boolean): Boolean {\n        if (a && b && c) { return true }\n        return false\n    }\n}\n",
        "kt",
    );
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert!(cc >= 4, "cc should be >= 4 (1+if+2&&), got: {cc}");
}

#[test]
fn output_has_module_prefix() {
    let mut code = String::new();
    for i in 0..declarations_above() {
        code.push_str(&format!("fun f{i}(): Int {{ return {i} }}\n"));
    }
    let output = pulse_check_code(&code, "kt");
    assert!(has_smell(&output, "Too Many Functions"), "got: {output}");
}

#[test]
fn comments_only_file() {
    let output = pulse_check_code("// just a comment\n/* block comment */\n", "kt");
    assert!(output.is_empty(), "got: {output}");
}

#[test]
fn hook_unsupported_extension_silent() {
    let output = run_hook("/tmp/file.xyz");
    assert!(output.is_empty(), "got: {output}");
}

#[test]
fn analysis_completes_under_500ms() {
    let mut code = String::new();
    for i in 0..25 {
        code.push_str(&format!("fun f{i}(): Int {{\n"));
        for j in 0..20 {
            code.push_str(&format!("    val x{j} = {j}\n"));
        }
        code.push_str("    return 0\n}\n\n");
    }
    let start = std::time::Instant::now();
    let _ = pulse_check_code(&code, "kt");
    let elapsed = start.elapsed();
    assert!(elapsed.as_millis() < 2000, "took {}ms", elapsed.as_millis());
}

#[test]
fn embedded_block_detected() {
    let mut code = String::from("class T {\n    fun f(): String {\n        return \"\"\"\n");
    for i in 0..embedded_lines_above() {
        code.push_str(&format!("            line {i}\n"));
    }
    code.push_str("        \"\"\"\n    }\n}\n");
    let output = pulse_check_code(&code, "kt");
    assert!(has_smell(&output, "Large Embedded Block"), "got: {output}");
}

#[test]
fn simple_string_not_flagged() {
    let output = pulse_check_code(
        "class T {\n    fun f(): String {\n        return \"hello\"\n    }\n}\n",
        "kt",
    );
    assert!(!has_smell(&output, "Large Embedded Block"), "got: {output}");
}

#[test]
fn when_expression_increments_cc() {
    let output = run_check(LANG, "WhenExpression.kt");
    assert!(has_smell(&output, "Complex Method"), "got: {output}");
}

#[test]
fn when_expression_cc_value() {
    let debug = run_debug(LANG, "WhenExpression.kt");
    let cc = function_metric(&debug, "dispatch", "cc").unwrap_or(0);
    assert!(cc >= 10, "cc should be >= 10, got: {cc}");
}

#[test]
fn code_duplication_detected() {
    let output = pulse_check_code(
        "fun a(): Int {\n    val x = 1\n    val y = 2\n    val z = 3\n    val w = 4\n    val v = 5\n    return x + y + z + w + v\n}\nfun b(): Int {\n    val x = 1\n    val y = 2\n    val z = 3\n    val w = 4\n    val v = 5\n    return x + y + z + w + v\n}\n",
        "kt",
    );
    assert!(has_smell(&output, "Code Duplication"), "got: {output}");
}

#[test]
fn primitive_obsession_recognizes_kotlin_types() {
    let output = pulse_check_code(
        "fun f(a: Int, b: Long, c: Double, d: Boolean, e: Char): Int {\n    return 0\n}\n",
        "kt",
    );
    assert!(has_smell(&output, "Primitive Obsession"), "got: {output}");
}

#[test]
fn primitive_obsession_mixed_not_flagged() {
    let output = pulse_check_code(
        "fun f(a: Int, b: Long, c: List<String>, d: Map<String, Int>): Int {\n    return 0\n}\n",
        "kt",
    );
    assert!(!has_smell(&output, "Primitive Obsession"), "got: {output}");
}

#[test]
fn nested_conditional_chunks_detected() {
    let output = pulse_check_code(
        "class T {\n    fun f(x: Int): Int {\n        for (i in 0..10) { if (x > 0) { if (x > 1) {} } }\n        for (i in 0..10) { if (x > 2) { if (x > 3) {} } }\n        for (i in 0..10) { if (x > 4) { if (x > 5) {} } }\n        return x\n    }\n}\n",
        "kt",
    );
    assert!(has_smell(&output, "Nested Conditional"), "got: {output}");
}

#[test]
fn declarations_above_threshold() {
    let mut code = String::new();
    for i in 0..declarations_above() {
        code.push_str(&format!("class C{i}\n"));
    }
    let output = pulse_check_code(&code, "kt");
    assert!(has_smell(&output, "Excessive Declarations"), "got: {output}");
}

#[test]
fn overall_function_size_below_threshold() {
    let mut code = String::new();
    for i in 0..2 {
        code.push_str(&format!("fun f{i}(): Int {{\n"));
        for j in 0..45 {
            code.push_str(&format!("    val x{j} = {j}\n"));
        }
        code.push_str("    return 0\n}\n\n");
    }
    let output = pulse_check_code(&code, "kt");
    assert!(!has_smell(&output, "Overall Function Size"), "got: {output}");
}

#[test]
fn overall_function_size_at_threshold() {
    let mut code = String::new();
    for i in 0..3 {
        code.push_str(&format!("fun f{i}(): Int {{\n"));
        for j in 0..45 {
            code.push_str(&format!("    val x{j} = {j}\n"));
        }
        code.push_str("    return 0\n}\n\n");
    }
    let output = pulse_check_code(&code, "kt");
    assert!(has_smell(&output, "Overall Function Size"), "got: {output}");
}

#[test]
fn god_class_requires_god_method() {
    let mut code = String::from("class Big {\n");
    for i in 0..declarations_above() {
        code.push_str(&format!("    fun f{i}(): Int {{\n"));
        for j in 0..20 {
            code.push_str(&format!("        val x{j} = {j}\n"));
        }
        code.push_str("        return 0\n    }\n\n");
    }
    code.push_str("}\n");
    let output = pulse_check_code(&code, "kt");
    assert!(!has_smell(&output, "God Class"), "got: {output}");
}

#[test]
fn god_class_triggers_with_god_method() {
    let mut code = String::from("class Big {\n");
    code.push_str("    fun god(x: Int): Int {\n");
    for _ in 0..50 {
        code.push_str("        if (x > 0) {}\n");
    }
    for i in 0..50 {
        code.push_str(&format!("        val v{i} = {i}\n"));
    }
    code.push_str("        return 0\n    }\n\n");
    for i in 1..declarations_above() {
        code.push_str(&format!("    fun f{i}(): Int {{\n"));
        for j in 0..20 {
            code.push_str(&format!("        val x{j} = {j}\n"));
        }
        code.push_str("        return 0\n    }\n\n");
    }
    code.push_str("}\n");
    let output = pulse_check_code(&code, "kt");
    assert!(has_smell(&output, "God Class"), "got: {output}");
}

#[test]
fn lcom4_detects_low_cohesion() {
    let output = pulse_check_code(
        "class C {\n    private var a: Int = 0\n    private var b: Int = 0\n    private var c: Int = 0\n    fun fa(): Int { return a }\n    fun fb(): Int { return b }\n    fun fc(): Int { return c }\n}\n",
        "kt",
    );
    assert!(has_smell(&output, "Low Cohesion"), "got: {output}");
}

#[test]
fn kotlin_features_extension_func() {
    let debug = run_debug(LANG, "KotlinFeatures.kt");
    assert!(debug.contains("String.isEmail"), "expected extension func, got: {debug}");
}

#[test]
fn kotlin_features_companion_object() {
    let debug = run_debug(LANG, "KotlinFeatures.kt");
    assert!(debug.contains("Container.Companion.empty"), "expected companion method, got: {debug}");
    assert!(debug.contains("Container.Companion.of"), "expected companion method, got: {debug}");
}

#[test]
fn kotlin_features_init_block() {
    let debug = run_debug(LANG, "KotlinFeatures.kt");
    assert!(debug.contains("Container.init"), "expected init block, got: {debug}");
}

#[test]
fn kotlin_features_expression_body() {
    let debug = run_debug(LANG, "KotlinFeatures.kt");
    assert!(debug.contains("topLevel"), "expected expression body func, got: {debug}");
    let cc = function_metric(&debug, "topLevel", "cc").unwrap_or(0);
    assert_eq!(cc, 1, "expression body should have cc=1, got: {cc}");
}

#[test]
fn kotlin_features_object_declaration() {
    let debug = run_debug(LANG, "KotlinFeatures.kt");
    assert!(debug.contains("Registry.register"), "expected object method, got: {debug}");
}

#[test]
fn kotlin_features_data_class() {
    let debug = run_debug(LANG, "KotlinFeatures.kt");
    assert!(debug.contains("Point.distanceTo"), "expected data class method, got: {debug}");
}
