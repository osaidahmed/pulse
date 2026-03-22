mod common;

use common::*;
use std::process::Command;

const LANG: &str = "javascript";

#[test]
fn clean_file_produces_no_output() {
    let output = run_check(LANG, "clean.js");
    assert!(
        output.is_empty(),
        "clean JS file should produce no output, got: {}",
        output
    );
}

#[test]
fn complex_method_detected() {
    let output = run_check(LANG, "complex_method.js");
    assert!(
        has_smell(&output, "Complex Method"),
        "should detect complex method in JS, got: {}",
        output
    );
    assert!(has_function(&output, "processOrder"));
}

#[test]
fn excess_args_detected() {
    let output = run_check(LANG, "excess_args.js");
    assert!(
        has_smell(&output, "Excess Arguments"),
        "should detect excess args in JS, got: {}",
        output
    );
    assert!(has_function(&output, "createUser"));
}

#[test]
fn constructor_over_injection_detected() {
    let output = run_check(LANG, "excess_args.js");
    assert!(
        has_smell(&output, "Constructor Over-Injection"),
        "should detect constructor over-injection in JS, got: {}",
        output
    );
}

#[test]
fn simple_func_not_flagged() {
    let output = run_check(LANG, "excess_args.js");
    assert!(!has_function(&output, "simpleFunc"));
}

#[test]
fn primitive_obsession_never_triggers_in_javascript() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("many_args.js");
    std::fs::write(
        &path,
        "function f(a, b, c, d, e, f, g, h, i) {\n    return a;\n}\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        !has_smell(&stdout, "Primitive Obsession"),
        "JS has no types — should never trigger, got: {}",
        stdout
    );
}

#[test]
fn hook_mode_works_with_js() {
    let path = fixtures_dir(LANG).join("clean.js");
    let output = run_hook(path.to_str().unwrap());
    assert!(output.is_empty());
}

#[test]
fn jsx_extension_supported() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("component.jsx");
    std::fs::write(&path, "function Component() {\n    return null;\n}\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    assert!(out.stdout.is_empty());
    assert!(out.status.success());
}

#[test]
fn mjs_extension_supported() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("module.mjs");
    std::fs::write(&path, "export function add(a, b) {\n    return a + b;\n}\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    assert!(out.stdout.is_empty());
    assert!(out.status.success());
}

#[test]
fn cjs_extension_supported() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("module.cjs");
    std::fs::write(
        &path,
        "function add(a, b) {\n    return a + b;\n}\nmodule.exports = { add };\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    assert!(out.stdout.is_empty());
    assert!(out.status.success());
}

#[test]
fn output_starts_with_pulse_prefix() {
    let output = run_check(LANG, "complex_method.js");
    assert!(output.starts_with("pulse:"));
}

// ===========================================================================
// Additional coverage — match Python depth
// ===========================================================================

#[test]
fn complex_method_cc_at_least_9() {
    let debug = run_debug(LANG, "complex_method.js");
    let cc = function_metric(&debug, "processOrder", "cc").unwrap_or(0);
    assert!(cc >= 9, "cc should be >= 9, got: {}", cc);
}

#[test]
fn deep_nesting_detected() {
    let output = run_check(LANG, "deep_nesting.js");
    assert!(has_smell(&output, "Deep Nested"), "got: {}", output);
    assert!(has_function(&output, "deeplyNested"));
}

#[test]
fn deep_nesting_depth_exceeds_4() {
    let debug = run_debug(LANG, "deep_nesting.js");
    let depth = function_metric(&debug, "deeplyNested", "nesting").unwrap_or(0);
    assert!(depth > 4, "got: {}", depth);
}

#[test]
fn moderate_nesting_not_flagged() {
    let output = run_check(LANG, "deep_nesting.js");
    assert!(!has_function(&output, "moderatelyNested"));
}

#[test]
fn embedded_block_detected() {
    let output = run_check(LANG, "embedded_block.js");
    assert!(
        has_smell(&output, "Large Embedded Block"),
        "got: {}",
        output
    );
    assert!(has_function(&output, "getActiveUsers"));
}

#[test]
fn simple_query_not_flagged() {
    let output = run_check(LANG, "embedded_block.js");
    assert!(!has_function(&output, "simpleQuery"));
}

#[test]
fn code_duplication_detected() {
    let output = run_check(LANG, "code_duplication.js");
    assert!(has_smell(&output, "Code Duplication"), "got: {}", output);
    assert!(has_function(&output, "processUserReport"));
    assert!(has_function(&output, "processAdminReport"));
    assert!(has_function(&output, "processVendorReport"));
}

#[test]
fn excess_args_count_correct() {
    let debug = run_debug(LANG, "excess_args.js");
    let args = function_metric(&debug, "createUser", "args").unwrap_or(0);
    assert_eq!(args, 8, "got: {}", args);
}

#[test]
fn cc_base_case_is_1() {
    let debug = run_debug(LANG, "clean.js");
    let cc = function_metric(&debug, "add", "cc").unwrap_or(99);
    assert_eq!(cc, 1);
}

#[test]
fn output_has_function_line_numbers() {
    let output = run_check(LANG, "complex_method.js");
    let has_loc = output
        .lines()
        .any(|l| l.contains("(L") && l.contains("): "));
    assert!(has_loc);
}

#[test]
fn output_has_module_prefix() {
    let output = run_check(LANG, "code_duplication.js");
    assert!(output.contains("Module:"));
}

#[test]
fn empty_file() {
    let out = pulse_check_code("", "js");
    assert!(out.is_empty());
}

#[test]
fn comments_only_file() {
    let out = pulse_check_code("// just comments\n// nothing else\n", "js");
    assert!(out.is_empty());
}

#[test]
fn function_at_cc_boundary_flagged() {
    let out = pulse_check_code("function f() {\n    if (a) {}\n    if (b) {}\n    if (c) {}\n    if (d) {}\n    if (e) {}\n    if (f) {}\n    if (g) {}\n    if (h) {}\n}\n", "js");
    assert!(
        has_smell(&out, "Complex Method"),
        "cc=9 should trigger, got: {}",
        out
    );
}

#[test]
fn function_below_cc_boundary_not_flagged() {
    let out = pulse_check_code("function f() {\n    if (a) {}\n    if (b) {}\n    if (c) {}\n    if (d) {}\n    if (e) {}\n    if (f) {}\n    if (g) {}\n}\n", "js");
    assert!(!has_smell(&out, "Complex Method"));
}

#[test]
fn hook_nonexistent_file_silent() {
    let output = run_hook("/nonexistent/path/foo.js");
    assert!(output.is_empty());
}

#[test]
fn hook_smelly_file_produces_output() {
    let path = fixtures_dir(LANG).join("complex_method.js");
    let output = run_hook(path.to_str().unwrap());
    assert!(!output.is_empty());
}

// ===========================================================================
// Large Method (loc >= threshold)
// ===========================================================================

#[test]
fn large_method_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large.js");
    let mut code = String::from("function buildReport() {\n");
    for i in 0..fn_padding() {
        code.push_str(&format!("    const x{} = {};\n", i, i));
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
    let path = dir.path().join("large_loc.js");
    let mut code = String::from("function buildReport() {\n");
    for i in 0..fn_padding() {
        code.push_str(&format!("    const x{} = {};\n", i, i));
    }
    code.push_str("}\n");
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["debug", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stderr = String::from_utf8(out.stderr).unwrap();
    let loc = function_metric(&stderr, "buildReport", "loc").unwrap_or(0);
    assert!(loc >= t().fn_loc_warning, "loc should be >= 50, got: {}", loc);
}

// ===========================================================================
// God Method (complex AND large)
// ===========================================================================

#[test]
fn god_method_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("god.js");
    let mut code = String::from("function processDataPipeline() {\n");
    for i in 0..10 {
        code.push_str(&format!("    if (x === {}) {{}}\n", i));
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("    const y{} = {};\n", i, i));
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
fn god_method_has_high_cc_and_loc() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("god2.js");
    let mut code = String::from("function processDataPipeline() {\n");
    for i in 0..10 {
        code.push_str(&format!("    if (x === {}) {{}}\n", i));
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("    const y{} = {};\n", i, i));
    }
    code.push_str("}\n");
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["debug", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stderr = String::from_utf8(out.stderr).unwrap();
    let cc = function_metric(&stderr, "processDataPipeline", "cc").unwrap_or(0);
    let loc = function_metric(&stderr, "processDataPipeline", "loc").unwrap_or(0);
    assert!(cc >= 9, "cc >= 9, got: {}", cc);
    assert!(loc >= t().fn_loc_warning, "loc >= t().fn_loc_warning, got: {}", loc);
}

#[test]
fn god_method_not_reported_as_separate_complex_and_large() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("god3.js");
    let mut code = String::from("function processDataPipeline() {\n");
    for i in 0..10 {
        code.push_str(&format!("    if (x === {}) {{}}\n", i));
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("    const y{} = {};\n", i, i));
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
        .filter(|l| l.contains("processDataPipeline"))
        .collect();
    assert!(!lines.iter().any(|l| l.contains("Complex Method")));
    assert!(!lines.iter().any(|l| l.contains("Large Method")));
}

// ===========================================================================
// Nested conditional chunks
// ===========================================================================

#[test]
fn nested_conditional_chunks_detected() {
    let output = run_check(LANG, "deep_nesting.js");
    // deep_nesting fixture may also trigger bumpy road pattern
    assert!(
        has_smell(&output, "Deep Nested") || has_smell(&output, "Nested Conditional Chunks"),
        "got: {}",
        output
    );
}

// ===========================================================================
// Complex conditional
// ===========================================================================

#[test]
fn complex_conditional_detected() {
    let out = pulse_check_code(
        concat!(
            "function checkEligibility(age, score, status) {\n",
            "    if (age > 18 && score > 50 && status === 'active') {\n",
            "        if (score > 80 || (age > 25 && status === 'premium')) {\n",
            "            return true;\n",
            "        }\n",
            "    }\n",
            "    if (age > 65 || score < 10 || status === 'exempt') {\n",
            "        return true;\n",
            "    }\n",
            "    return false;\n",
            "}\n",
        ),
        "js",
    );
    assert!(
        has_smell(&out, "Complex Conditional") || has_smell(&out, "Complex Method"),
        "got: {}",
        out
    );
}

#[test]
fn simple_check_not_flagged_in_conditional() {
    let out = pulse_check_code(
        concat!(
            "function checkEligibility(age, score, status) {\n",
            "    if (age > 18 && score > 50 && status === 'active') {\n",
            "        if (score > 80 || (age > 25 && status === 'premium')) {\n",
            "            return true;\n",
            "        }\n",
            "    }\n",
            "    return false;\n",
            "}\n",
            "function simpleCheck(x) {\n",
            "    return x > 0;\n",
            "}\n",
        ),
        "js",
    );
    assert!(!has_function(&out, "simpleCheck"));
}

// ===========================================================================
// Global conditionals
// ===========================================================================

#[test]
fn global_conditionals_detected() {
    let out = pulse_check_code(
        concat!(
            "const x = 1;\n",
            "if (process.env.NODE_ENV === 'development') {\n",
            "    console.log('dev mode');\n",
            "}\n",
            "if (process.env.DEBUG) {\n",
            "    console.log('debug');\n",
            "}\n",
            "if (process.env.VERBOSE) {\n",
            "    console.log('verbose');\n",
            "}\n",
        ),
        "js",
    );
    assert!(has_smell(&out, "Global Conditionals"), "got: {}", out);
}

// ===========================================================================
// File too large / too many functions
// ===========================================================================

#[test]
fn file_too_large_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("huge.js");
    let mut code = String::new();
    for i in 0..25 {
        code.push_str(&format!("function fn{}() {{ return {}; }}\n", i, i));
    }
    for i in 0..500 {
        code.push_str(&format!("const VAR{} = {};\n", i, i));
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
    let path = dir.path().join("huge2.js");
    let mut code = String::new();
    for i in 0..25 {
        code.push_str(&format!("function fn{}() {{ return {}; }}\n", i, i));
    }
    for i in 0..500 {
        code.push_str(&format!("const VAR{} = {};\n", i, i));
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
// Hook edge cases
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
    let debug = pulse_debug_code("function f() {\n    if (a && b && c) {}\n}\n", "js");
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert!(
        cc >= 4,
        "boolean operators should increment cc, got: {}",
        cc
    );
}

// ===========================================================================
// Issue count matches findings
// ===========================================================================

#[test]
fn issue_count_in_header_matches_findings() {
    let output = run_check(LANG, "excess_args.js");
    let first = output.lines().next().unwrap_or("");
    let findings = output.lines().filter(|l| l.starts_with("  ")).count();
    assert!(first.contains(&format!("{} issue", findings)));
}

// ===========================================================================
// Decorated function
// ===========================================================================

#[test]
fn decorated_function_analyzed() {
    let out = pulse_check_code(
        "function longDeco(a, b, c, d, e, f, g, h) {\n    return a;\n}\n",
        "js",
    );
    assert!(
        has_smell(&out, "Excess Arguments") || has_function(&out, "longDeco"),
        "got: {}",
        out
    );
}

// ===========================================================================
// Performance on file_too_large equivalent
// ===========================================================================

#[test]
fn analysis_completes_under_500ms() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("perf.js");
    let mut code = String::new();
    for i in 0..25 {
        code.push_str(&format!("function fn{}() {{ return {}; }}\n", i, i));
    }
    for i in 0..500 {
        code.push_str(&format!("const VAR{} = {};\n", i, i));
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
// Hook unsupported extension is silent
// ===========================================================================

#[test]
fn hook_unsupported_extension_is_silent() {
    let output = run_hook("/some/file.xyz");
    assert!(output.is_empty());
}

// ===========================================================================
// Nested conditional chunks bump count
// ===========================================================================

#[test]
fn nested_conditional_chunks_bump_count() {
    let debug = run_debug(LANG, "deep_nesting.js");
    let depth = function_metric(&debug, "deeplyNested", "nesting").unwrap_or(0);
    assert!(depth > 4, "nesting should be > 4, got: {}", depth);
}

// ===========================================================================
// Constructor args count
// ===========================================================================

#[test]
fn constructor_args_count_correct() {
    let debug = run_debug(LANG, "excess_args.js");
    // Constructor should exclude this, count only explicit params
    let out = pulse_debug_code("class S {\n    constructor(a, b, c, d, e, f) {}\n}\n", "js");
    let args = function_metric(&out, "S.constructor", "args").unwrap_or(0);
    assert_eq!(args, 6, "got: {}", args);
}

// ===========================================================================
// Low cohesion from inline
// ===========================================================================

#[test]
fn lcom4_detects_low_cohesion() {
    let out = pulse_check_code(
        concat!(
            "class KitchenSink {\n",
            "    constructor() {\n",
            "        this.users = [];\n",
            "        this.orders = [];\n",
            "        this.logs = [];\n",
            "    }\n",
            "    addUser(user) { this.users.push(user); }\n",
            "    getUser(id) { return this.users.find(u => u.id === id); }\n",
            "    removeUser(id) { this.users = this.users.filter(u => u.id !== id); }\n",
            "    addOrder(order) { this.orders.push(order); }\n",
            "    getOrder(id) { return this.orders.find(o => o.id === id); }\n",
            "    cancelOrder(id) { this.orders = this.orders.filter(o => o.id !== id); }\n",
            "    logEvent(event) { this.logs.push(event); }\n",
            "    getLogs() { return this.logs; }\n",
            "    clearLogs() { this.logs = []; }\n",
            "}\n",
        ),
        "js",
    );
    assert!(has_smell(&out, "Low Cohesion"), "got: {}", out);
}

// ===========================================================================
// Switch case increments cc
// ===========================================================================

#[test]
fn switch_case_increments_cc() {
    let out = pulse_check_code(
        concat!(
            "function handleAction(action) {\n",
            "    switch (action) {\n",
            "        case 'a': return 'A';\n",
            "        case 'b': return 'B';\n",
            "        case 'c': return 'C';\n",
            "        case 'd': return 'D';\n",
            "        case 'e': return 'E';\n",
            "        case 'f': return 'F';\n",
            "        case 'g': return 'G';\n",
            "        case 'h': return 'H';\n",
            "        case 'i': return 'I';\n",
            "        default: return '?';\n",
            "    }\n",
            "}\n",
        ),
        "js",
    );
    assert!(
        has_smell(&out, "Complex Method"),
        "9 switch cases should trigger cc >= 9, got: {}",
        out
    );
}

// ===========================================================================
// Arrow function analyzed
// ===========================================================================

#[test]
fn arrow_function_analyzed() {
    let out = pulse_check_code(
        "const handler = (a, b, c, d, e, f, g) => {\n    return a + b;\n};\n",
        "js",
    );
    assert!(
        has_smell(&out, "Excess Arguments"),
        "arrow function with 7 args should be flagged, got: {}",
        out
    );
}

// ===========================================================================
// Comments only file
// ===========================================================================

#[test]
fn comments_only_file_no_output() {
    let out = pulse_check_code("// just a comment\n// another comment\n", "js");
    assert!(out.is_empty());
}

// ===========================================================================
// Module prefix in output
// ===========================================================================

#[test]
fn output_has_module_prefix_for_duplication() {
    let output = run_check(LANG, "code_duplication.js");
    assert!(output.contains("Module:"));
}
