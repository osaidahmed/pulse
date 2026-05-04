mod common;

use common::*;
use std::process::Command;

const LANG: &str = "typescript";

#[test]
fn clean_file_produces_no_output() {
    let output = run_check(LANG, "clean.ts");
    assert!(
        output.is_empty(),
        "clean TS file should produce no output, got: {output}"
    );
}

#[test]
fn complex_method_detected() {
    let output = run_check(LANG, "complex_method.ts");
    assert!(
        has_smell(&output, "Complex Method"),
        "should detect complex method, got: {output}"
    );
    assert!(has_function(&output, "processOrder"));
}

#[test]
fn complex_method_cc_at_least_9() {
    let debug = run_debug(LANG, "complex_method.ts");
    let cc = function_metric(&debug, "processOrder", "cc").unwrap_or(0);
    assert!(cc >= 9, "cc should be >= 9, got: {cc}");
}

#[test]
fn deep_nesting_detected() {
    let output = run_check(LANG, "deep_nesting.ts");
    assert!(
        has_smell(&output, "Deep Nested"),
        "should detect deep nesting, got: {output}"
    );
    assert!(has_function(&output, "deeplyNested"));
}

#[test]
fn moderate_nesting_not_flagged() {
    let output = run_check(LANG, "deep_nesting.ts");
    assert!(!has_function(&output, "moderatelyNested"));
}

#[test]
fn excess_args_detected() {
    let output = run_check(LANG, "excess_args.ts");
    assert!(
        has_smell(&output, "Excess Arguments"),
        "should detect excess args, got: {output}"
    );
    assert!(has_function(&output, "createUser"));
}

#[test]
fn simple_func_not_flagged() {
    let output = run_check(LANG, "excess_args.ts");
    assert!(!has_function(&output, "simpleFunc"));
}

#[test]
fn constructor_over_injection_detected() {
    let output = run_check(LANG, "excess_args.ts");
    assert!(
        has_smell(&output, "Constructor Over-Injection"),
        "should detect constructor over-injection, got: {output}"
    );
    assert!(has_function(&output, "UserService.constructor"));
}

#[test]
fn primitive_obsession_detected_in_typescript() {
    let output = run_check(LANG, "excess_args.ts");
    assert!(
        has_smell(&output, "Primitive Obsession"),
        "should detect primitive obsession in TS, got: {output}"
    );
}

#[test]
fn code_duplication_detected() {
    let output = run_check(LANG, "code_duplication.ts");
    assert!(
        has_smell(&output, "Code Duplication"),
        "should detect duplication in TS, got: {output}"
    );
    assert!(has_function(&output, "processUserReport"));
    assert!(has_function(&output, "processAdminReport"));
    assert!(has_function(&output, "processVendorReport"));
}

#[test]
fn lcom4_detects_low_cohesion() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("low_cohesion.ts");
    std::fs::write(
        &path,
        r"
class KitchenSink {
    private users: any[] = [];
    private orders: any[] = [];
    private logs: any[] = [];

    addUser(user: any) { this.users.push(user); }
    getUser(id: number) { return this.users.find(u => u.id === id); }
    removeUser(id: number) { this.users = this.users.filter(u => u.id !== id); }

    addOrder(order: any) { this.orders.push(order); }
    getOrder(id: number) { return this.orders.find(o => o.id === id); }
    cancelOrder(id: number) { this.orders = this.orders.filter(o => o.id !== id); }

    logEvent(event: any) { this.logs.push(event); }
    getLogs() { return this.logs; }
    clearLogs() { this.logs = []; }
}
",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        has_smell(&stdout, "Low Cohesion"),
        "should detect low cohesion in TS class, got: {stdout}"
    );
}

#[test]
fn arrow_function_analyzed() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("arrow.ts");
    std::fs::write(
        &path,
        r"
const handler = (a: string, b: string, c: string, d: string, e: string, f: string, g: string) => {
    return a + b;
};
",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        has_smell(&stdout, "Excess Arguments"),
        "arrow function with 7 args should be flagged, got: {stdout}"
    );
}

#[test]
fn switch_case_increments_cc() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("switch.ts");
    std::fs::write(
        &path,
        r#"
function handleAction(action: string): string {
    switch (action) {
        case "a": return "A";
        case "b": return "B";
        case "c": return "C";
        case "d": return "D";
        case "e": return "E";
        case "f": return "F";
        case "g": return "G";
        case "h": return "H";
        case "i": return "I";
        default: return "?";
    }
}
"#,
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        has_smell(&stdout, "Complex Method"),
        "9 switch cases should trigger cc >= 9, got: {stdout}"
    );
}

#[test]
fn hook_mode_works_with_ts() {
    let path = fixtures_dir(LANG).join("clean.ts");
    let output = run_hook(path.to_str().unwrap());
    assert!(output.is_empty(), "hook on clean TS file should be silent");
}

#[test]
fn hook_mode_detects_smells_in_ts() {
    let path = fixtures_dir(LANG).join("complex_method.ts");
    let output = run_hook(path.to_str().unwrap());
    assert!(
        !output.is_empty(),
        "hook on smelly TS file should produce output"
    );
}

#[test]
fn tsx_extension_supported() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("component.tsx");
    std::fs::write(&path, "function Component() {\n    return null;\n}\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    // Should not crash — clean file, no output
    assert!(out.stdout.is_empty());
    assert!(out.status.success());
}

#[test]
fn output_starts_with_pulse_prefix() {
    let output = run_check(LANG, "complex_method.ts");
    assert!(output.starts_with("pulse:"));
}

#[test]
fn performance_ts_under_500ms() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("big.ts");
    let mut code = String::new();
    for i in 0..30 {
        code.push_str(&format!(
            "function func{i}(data: any): any {{\n    const r: any = {{}};\n    r.id = data.id;\n    r.name = data.name;\n    r.value = data.value;\n    r.active = data.active;\n    r.category = data.category;\n    r.priority = data.priority;\n    return r;\n}}\n\n"
        ));
    }
    std::fs::write(&path, &code).unwrap();
    let start = std::time::Instant::now();
    let _ = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let elapsed = start.elapsed();
    assert!(
        elapsed.as_millis() < 500,
        "TS analysis should be under 500ms, took: {}ms",
        elapsed.as_millis()
    );
}

// ===========================================================================
// Additional coverage — match Python depth
// ===========================================================================

#[test]
fn deep_nesting_depth_exceeds_4() {
    let debug = run_debug(LANG, "deep_nesting.ts");
    let depth = function_metric(&debug, "deeplyNested", "nesting").unwrap_or(0);
    assert!(depth > 4, "nesting should be > 4, got: {depth}");
}

#[test]
fn nested_conditional_chunks_detected() {
    let output = run_check(LANG, "bumpy_road.ts");
    assert!(
        has_smell(&output, "Nested Conditional Chunks"),
        "got: {output}"
    );
    assert!(has_function(&output, "validateAndProcess"));
}

#[test]
fn embedded_block_detected() {
    let output = run_check(LANG, "embedded_block.ts");
    assert!(
        has_smell(&output, "Large Embedded Block"),
        "got: {output}"
    );
    assert!(has_function(&output, "getActiveUsers"));
}

#[test]
fn simple_query_not_flagged() {
    let output = run_check(LANG, "embedded_block.ts");
    assert!(!has_function(&output, "simpleQuery"));
}

#[test]
fn low_cohesion_from_fixture() {
    let output = run_check(LANG, "low_cohesion.ts");
    assert!(has_smell(&output, "Low Cohesion"), "got: {output}");
    assert!(output.contains("LCOM4=4"), "got: {output}");
}

#[test]
fn excess_args_count_correct() {
    let debug = run_debug(LANG, "excess_args.ts");
    let args = function_metric(&debug, "createUser", "args").unwrap_or(0);
    assert_eq!(args, 8, "got: {args}");
}

#[test]
fn cc_base_case_is_1() {
    let debug = run_debug(LANG, "clean.ts");
    let cc = function_metric(&debug, "add", "cc").unwrap_or(99);
    assert_eq!(cc, 1);
}

#[test]
fn boolean_operators_increment_cc() {
    let _output = pulse_check_code("function f(): void {\n    if (a && b && c) {}\n}\n", "ts");
    let debug = pulse_debug_code("function f(): void {\n    if (a && b && c) {}\n}\n", "ts");
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert!(
        cc >= 4,
        "boolean operators should increment cc, got: {cc}"
    );
}

#[test]
fn output_has_function_line_numbers() {
    let output = run_check(LANG, "complex_method.ts");
    let has_loc = output
        .lines()
        .any(|l| l.contains("(L") && l.contains("): "));
    assert!(has_loc);
}

#[test]
fn output_has_module_prefix() {
    let output = run_check(LANG, "low_cohesion.ts");
    assert!(output.contains("Module:"));
}

#[test]
fn issue_count_in_header_matches_findings() {
    let output = run_check(LANG, "excess_args.ts");
    let first = output.lines().next().unwrap_or("");
    let findings = output.lines().filter(|l| l.starts_with("  ")).count();
    assert!(first.contains(&format!("{findings} issue")));
}

#[test]
fn empty_file() {
    let out = pulse_check_code("", "ts");
    assert!(out.is_empty());
}

#[test]
fn comments_only_file() {
    let out = pulse_check_code("// just comments\n// nothing else\n", "ts");
    assert!(out.is_empty());
}

#[test]
fn function_at_cc_boundary_flagged() {
    let out = pulse_check_code("function f(): void {\n    if (a) {}\n    if (b) {}\n    if (c) {}\n    if (d) {}\n    if (e) {}\n    if (f) {}\n    if (g) {}\n    if (h) {}\n}\n", "ts");
    assert!(
        has_smell(&out, "Complex Method"),
        "cc=9 should trigger, got: {out}"
    );
}

#[test]
fn function_below_cc_boundary_not_flagged() {
    let out = pulse_check_code("function f(): void {\n    if (a) {}\n    if (b) {}\n    if (c) {}\n    if (d) {}\n    if (e) {}\n    if (f) {}\n    if (g) {}\n}\n", "ts");
    assert!(
        !has_smell(&out, "Complex Method"),
        "cc=8 should not trigger, got: {out}"
    );
}

#[test]
fn hook_nonexistent_file_silent() {
    let output = run_hook("/nonexistent/path/foo.ts");
    assert!(output.is_empty());
}

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
// Large Method (loc >= threshold)
// ===========================================================================

#[test]
fn large_method_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large.ts");
    let mut code = String::from("function buildReport(): void {\n");
    for i in 0..fn_padding() {
        code.push_str(&format!("    const x{i} = {i};\n"));
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
        "should detect large method, got: {stdout}"
    );
}

// ===========================================================================
// God Method (complex AND large)
// ===========================================================================

#[test]
fn god_method_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("god.ts");
    let mut code = String::from("function processDataPipeline(): void {\n");
    for i in 0..cc_branches() {
        code.push_str(&format!("    if (x === {i}) {{}}\n"));
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("    const y{i} = {i};\n"));
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
    let path = dir.path().join("god2.ts");
    let mut code = String::from("function processDataPipeline(): void {\n");
    for i in 0..cc_branches() {
        code.push_str(&format!("    if (x === {i}) {{}}\n"));
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("    const y{i} = {i};\n"));
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
    assert!(cc >= 9, "god method cc >= 9, got: {cc}");
    assert!(loc >= t().function.fn_loc_warning, "god method loc >= t().function.fn_loc_warning, got: {loc}");
}

#[test]
fn god_method_not_reported_as_separate_complex_and_large() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("god3.ts");
    let mut code = String::from("function processDataPipeline(): void {\n");
    for i in 0..cc_branches() {
        code.push_str(&format!("    if (x === {i}) {{}}\n"));
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("    const y{i} = {i};\n"));
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
    assert!(
        !lines.iter().any(|l| l.contains("Complex Method")),
        "should not separately report Complex Method for a God Method"
    );
    assert!(
        !lines.iter().any(|l| l.contains("Large Method")),
        "should not separately report Large Method for a God Method"
    );
}

// ===========================================================================
// Nested conditional chunks bump count
// ===========================================================================

#[test]
fn nested_conditional_chunks_bump_count() {
    let debug = run_debug(LANG, "bumpy_road.ts");
    let bumps = function_metric(&debug, "validateAndProcess", "bumps").unwrap_or(0);
    assert!(bumps >= 2, "should have >= 2 bumps, got: {bumps}");
}

// ===========================================================================
// Complex conditional
// ===========================================================================

#[test]
fn complex_conditional_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cc.ts");
    std::fs::write(
        &path,
        concat!(
            "function checkEligibility(age: number, score: number, status: string): boolean {\n",
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
            "function simpleCheck(x: number): boolean {\n",
            "    return x > 0;\n",
            "}\n",
        ),
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        has_smell(&stdout, "Complex Conditional") || has_smell(&stdout, "Complex Method"),
        "got: {stdout}"
    );
    assert!(has_function(&stdout, "checkEligibility"));
}

#[test]
fn simple_check_not_flagged() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cc2.ts");
    std::fs::write(
        &path,
        concat!(
            "function checkEligibility(age: number, score: number, status: string): boolean {\n",
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
            "function simpleCheck(x: number): boolean {\n",
            "    return x > 0;\n",
            "}\n",
        ),
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        !has_function(&stdout, "simpleCheck"),
        "simpleCheck should not be flagged"
    );
}

// ===========================================================================
// Global conditionals
// ===========================================================================

#[test]
fn global_conditionals_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("gc.ts");
    std::fs::write(
        &path,
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
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(has_smell(&stdout, "Global Conditionals"), "got: {stdout}");
}

// ===========================================================================
// File too large / too many functions
// ===========================================================================

#[test]
fn file_too_large_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("huge.ts");
    let mut code = String::new();
    for i in 0..declarations_above() {
        code.push_str(&format!("function fn{i}(): number {{ return {i}; }}\n"));
    }
    for i in 0..file_padding() {
        code.push_str(&format!("const VAR{i} = {i};\n"));
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
    let path = dir.path().join("huge2.ts");
    let mut code = String::new();
    for i in 0..declarations_above() {
        code.push_str(&format!("function fn{i}(): number {{ return {i}; }}\n"));
    }
    for i in 0..file_padding() {
        code.push_str(&format!("const VAR{i} = {i};\n"));
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
// Decorated / annotated function analyzed
// ===========================================================================

#[test]
fn decorated_function_analyzed() {
    let out = pulse_check_code(concat!(
        "function decorator(target: any) { return target; }\n\n",
        "function longDeco(a: any, b: any, c: any, d: any, e: any, f: any, g: any, h: any): any {\n",
        "    return a;\n",
        "}\n",
    ), "ts");
    assert!(
        has_smell(&out, "Excess Arguments") || has_function(&out, "longDeco"),
        "function with 8 args should be flagged, got: {out}"
    );
}

// ===========================================================================
// Performance on fixture
// ===========================================================================

#[test]
fn analysis_completes_under_500ms() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("perf.ts");
    let mut code = String::new();
    for i in 0..25 {
        code.push_str(&format!("function fn{i}(): number {{ return {i}; }}\n"));
    }
    for i in 0..500 {
        code.push_str(&format!("const VAR{i} = {i};\n"));
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

#[test]
fn large_method_loc_at_least_65() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large_loc.ts");
    let mut code = String::from("function buildReport(): void {\n");
    for i in 0..fn_padding() {
        code.push_str(&format!("    const x{i} = {i};\n"));
    }
    code.push_str("}\n");
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["debug", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stderr = String::from_utf8(out.stderr).unwrap();
    let loc = function_metric(&stderr, "buildReport", "loc").unwrap_or(0);
    assert!(loc >= t().function.fn_loc_warning, "loc should be >= 50, got: {loc}");
}
