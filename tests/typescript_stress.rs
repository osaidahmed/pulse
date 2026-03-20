mod common;

use common::*;
use std::process::Command;

fn check(code: &str) -> String { pulse_check_code(code, "ts") }
fn debug(code: &str) -> String { pulse_debug_code(code, "ts") }

// ===========================================================================
// CC counting precision
// ===========================================================================

#[test]
fn cc_counts_if() {
    let out = debug("function f(): void {\n    if (x) {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_else_if() {
    let out = debug("function f(): void {\n    if (x) {} else if (y) {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_counts_for() {
    let out = debug("function f(): void {\n    for (const x of y) {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_for_in() {
    let out = debug("function f(): void {\n    for (const x in y) {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_while() {
    let out = debug("function f(): void {\n    while (x) {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_catch() {
    let out = debug("function f(): void {\n    try {} catch (e) {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_and_operator() {
    let out = debug("function f(): void {\n    if (a && b) {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_counts_or_operator() {
    let out = debug("function f(): void {\n    if (a || b) {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_counts_nullish_coalescing() {
    let out = debug("function f(): void {\n    if (a ?? b) {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_counts_ternary() {
    let out = debug("function f(): number {\n    const x = a ? 1 : 2;\n    return x;\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_chained_boolean() {
    let out = debug("function f(): void {\n    if (a && b && c && d) {}\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(cc >= 4, "chained boolean should increase cc, got: {}", cc);
}

#[test]
fn cc_counts_switch_cases() {
    let out = debug("function f(x: string): string {\n    switch (x) {\n        case 'a': return 'A';\n        case 'b': return 'B';\n        case 'c': return 'C';\n        default: return '?';\n    }\n}\n");
    // base(1) + 3 cases (default doesn't count) = 4
    assert_eq!(function_metric(&out, "f", "cc"), Some(4));
}

#[test]
fn cc_nested_if_in_for() {
    let out = debug("function f(): void {\n    for (const x of y) {\n        if (x) {}\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

// ===========================================================================
// Nesting depth precision
// ===========================================================================

#[test]
fn nesting_depth_0_for_flat_function() {
    let out = debug("function f(): number {\n    return 1;\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(0));
}

#[test]
fn nesting_depth_1_for_single_if() {
    let out = debug("function f(): void {\n    if (x) {}\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(1));
}

#[test]
fn nesting_depth_2_for_nested_if() {
    let out = debug("function f(): void {\n    if (x) {\n        if (y) {}\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(2));
}

#[test]
fn nesting_depth_tracks_for_in_if() {
    let out = debug("function f(): void {\n    if (x) {\n        for (const i of y) {\n            if (z) {}\n        }\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(3));
}

// ===========================================================================
// Argument counting precision
// ===========================================================================

#[test]
fn args_counts_positional() {
    let out = debug("function f(a: any, b: any, c: any): void {}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(3));
}

#[test]
fn args_counts_defaults() {
    let out = debug("function f(a: number = 0, b: string = ''): void {}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(2));
}

#[test]
fn args_counts_rest() {
    let out = debug("function f(a: any, ...rest: any[]): void {}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(2));
}

#[test]
fn args_zero_for_no_params() {
    let out = debug("function f(): void {}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(0));
}

// ===========================================================================
// Primitive obsession precision (TS-specific — has types)
// ===========================================================================

#[test]
fn primitive_obsession_all_string() {
    let out = check("function f(a: string, b: string, c: string, d: string, e: string): void {}\n");
    assert!(has_smell(&out, "Primitive Obsession"));
}

#[test]
fn primitive_obsession_mixed_below_threshold() {
    let out = check("function f(a: string, b: string, c: MyObj, d: OtherObj): void {}\n");
    assert!(!has_smell(&out, "Primitive Obsession"));
}

#[test]
fn primitive_obsession_untyped_not_counted() {
    // If params have no type annotations, typed_count is 0 → never triggers
    let out = check("function f(a, b, c, d, e): void {}\n");
    assert!(!has_smell(&out, "Primitive Obsession"));
}

#[test]
fn primitive_obsession_below_min_typed() {
    let out = check("function f(a: string, b: number, c: boolean): void {}\n");
    assert!(!has_smell(&out, "Primitive Obsession"));
}

#[test]
fn primitive_obsession_recognizes_number_boolean() {
    let out = check("function f(a: number, b: boolean, c: bigint, d: void): void {}\n");
    assert!(has_smell(&out, "Primitive Obsession"));
}

// ===========================================================================
// LCOM4 precision
// ===========================================================================

#[test]
fn lcom4_single_method_class_not_flagged() {
    let out = check("class T {\n    x = 1;\n    get() { return this.x; }\n}\n");
    assert!(!has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_all_methods_share_field() {
    let out = check("class C {\n    d: any[] = [];\n    add(x: any) { this.d.push(x); }\n    get() { return this.d; }\n    clear() { this.d = []; }\n}\n");
    assert!(!has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_three_disconnected_groups() {
    let out = check(concat!(
        "class M {\n",
        "    x = 1; y = 2; z = 3;\n",
        "    useX() { return this.x; }\n",
        "    useY() { return this.y; }\n",
        "    useZ() { return this.z; }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_transitive_connection() {
    let out = check(concat!(
        "class C {\n",
        "    a = 1; b = 2;\n",
        "    m1() { return this.a; }\n",
        "    m2() { return this.a + this.b; }\n",
        "    m3() { return this.b; }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"));
}

// ===========================================================================
// Code duplication edge cases
// ===========================================================================

#[test]
fn duplication_two_functions_is_minimum() {
    let out = check(concat!(
        "function rptA(data: any): any {\n    const r: any = {};\n    r.id = data.id;\n    r.name = data.name;\n    r.val = data.val;\n    r.status = 'active';\n    return r;\n}\n\n",
        "function rptB(data: any): any {\n    const r: any = {};\n    r.id = data.id;\n    r.name = data.name;\n    r.val = data.val;\n    r.status = 'active';\n    return r;\n}\n",
    ));
    assert!(has_smell(&out, "Code Duplication"));
}

#[test]
fn duplication_test_functions_suppressed() {
    let out = check(concat!(
        "function test_a(data: any): any {\n    const r: any = {};\n    r.id = data.id;\n    r.name = data.name;\n    r.val = data.val;\n    r.ok = data.ok;\n    return r;\n}\n\n",
        "function test_b(data: any): any {\n    const r: any = {};\n    r.id = data.id;\n    r.name = data.name;\n    r.val = data.val;\n    r.ok = data.ok;\n    return r;\n}\n",
    ));
    assert!(!has_smell(&out, "Code Duplication"));
}

// ===========================================================================
// God Method / God Class interaction
// ===========================================================================

#[test]
fn god_class_requires_god_method() {
    let mut code = String::new();
    for i in 0..25 {
        code.push_str(&format!("function fn{}(): number {{ return {}; }}\n", i, i));
    }
    for i in 0..200 {
        code.push_str(&format!("const VAR{} = {};\n", i, i));
    }
    let out = check(&code);
    assert!(!has_smell(&out, "God Class"));
}

#[test]
fn god_method_triggers_god_class() {
    let mut code = String::from("function monster(): void {\n");
    for i in 0..10 {
        code.push_str(&format!("    if (x === {}) {{}}\n", i));
    }
    for i in 0..40 {
        code.push_str(&format!("    const y{} = {};\n", i, i));
    }
    code.push_str("}\n\n");
    for i in 0..21 {
        code.push_str(&format!("function fn{}(): number {{ return {}; }}\n", i, i));
    }
    for i in 0..350 {
        code.push_str(&format!("const V{} = {};\n", i, i));
    }
    let out = check(&code);
    assert!(has_smell(&out, "God Method"));
    assert!(has_smell(&out, "God Class"));
}

// ===========================================================================
// Overall Function Size
// ===========================================================================

#[test]
fn overall_function_size_not_triggered_below_threshold() {
    let mut code = String::new();
    for i in 0..2 {
        code.push_str(&format!("function lg{}(): void {{\n", i));
        for j in 0..45 {
            code.push_str(&format!("    const x{} = {};\n", j, j));
        }
        code.push_str("}\n\n");
    }
    let out = check(&code);
    assert!(!has_smell(&out, "Overall Function Size"));
}

#[test]
fn overall_function_size_triggered_at_threshold() {
    let mut code = String::new();
    for i in 0..3 {
        code.push_str(&format!("function lg{}(): void {{\n", i));
        for j in 0..45 {
            code.push_str(&format!("    const x{} = {};\n", j, j));
        }
        code.push_str("}\n\n");
    }
    let out = check(&code);
    assert!(has_smell(&out, "Overall Function Size"));
}

// ===========================================================================
// Declarations
// ===========================================================================

#[test]
fn declarations_below_threshold() {
    let mut code = String::new();
    for i in 0..10 {
        code.push_str(&format!("class T{} {{}}\n", i));
    }
    let out = check(&code);
    assert!(!has_smell(&out, "Declarations"));
}

#[test]
fn declarations_above_threshold() {
    let mut code = String::new();
    for i in 0..25 {
        code.push_str(&format!("class T{} {{}}\n", i));
    }
    let out = check(&code);
    assert!(has_smell(&out, "Declarations"));
}

// ===========================================================================
// Deep global nesting
// ===========================================================================

#[test]
fn shallow_global_if_not_flagged() {
    let out = check("if (true) {\n    const x = 1;\n}\n");
    assert!(!has_smell(&out, "Deep Global Nesting"));
}

#[test]
fn global_nesting_depth_3_flagged() {
    let out = check("if (a) {\n    if (b) {\n        if (c) {\n            const x = 1;\n        }\n    }\n}\n");
    assert!(has_smell(&out, "Deep Global Nesting"));
}

// ===========================================================================
// Constructor vs excess args
// ===========================================================================

#[test]
fn constructor_reports_over_injection() {
    let out = check("class S {\n    constructor(a: any, b: any, c: any, d: any, e: any, f: any) {}\n}\n");
    assert!(has_smell(&out, "Constructor Over-Injection"));
    let lines: Vec<&str> = out.lines().filter(|l| l.contains("constructor")).collect();
    assert!(!lines.iter().any(|l| l.contains("Excess Arguments")));
}

#[test]
fn regular_function_reports_excess_args() {
    let out = check("function f(a: any, b: any, c: any, d: any, e: any, f: any, g: any): void {}\n");
    assert!(has_smell(&out, "Excess Arguments"));
    assert!(!has_smell(&out, "Constructor Over-Injection"));
}

// ===========================================================================
// Multiple smells
// ===========================================================================

#[test]
fn function_can_have_multiple_smells() {
    let out = check(concat!(
        "function terrible(a: any, b: any, c: any, d: any, e: any, f: any, g: any, h: any): void {\n",
        "    const q = `\n",
        "        SELECT u.id,\n",
        "               u.first_name,\n",
        "               u.last_name,\n",
        "               u.email,\n",
        "               u.phone,\n",
        "               u.created_at,\n",
        "               u.updated_at,\n",
        "               u.status,\n",
        "               u.role,\n",
        "               u.department,\n",
        "               u.manager_id,\n",
        "               u.location,\n",
        "               u.timezone,\n",
        "               u.language\n",
        "        FROM users u\n",
        "        WHERE u.active = true\n",
        "    `;\n",
        "    for (const x of a) {\n",
        "        if (x) {\n",
        "            for (const y of x) {\n",
        "                if (y) {\n",
        "                    for (const z of y) {\n",
        "                        if (z) { process(z); }\n",
        "                    }\n",
        "                }\n",
        "            }\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Excess Arguments"));
    assert!(has_smell(&out, "Large Embedded Block"));
    assert!(has_smell(&out, "Deep Nested"));
}

// ===========================================================================
// Hook edge cases
// ===========================================================================

#[test]
fn hook_missing_tool_input() {
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["--hook"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.take().unwrap().write_all(b"{\"other\": 1}").unwrap();
            child.wait_with_output()
        })
        .expect("failed to run");
    assert!(out.stdout.is_empty());
}

#[test]
fn hook_empty_stdin() {
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["--hook"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            drop(child.stdin.take());
            child.wait_with_output()
        })
        .expect("failed to run");
    assert!(out.stdout.is_empty());
}

// ===========================================================================
// Performance
// ===========================================================================

#[test]
fn performance_1000_loc() {
    let mut code = String::new();
    for i in 0..50 {
        code.push_str(&format!(
            "function func{}(data: any): any {{\n    const r: any = {{}};\n", i
        ));
        for j in 0..18 {
            code.push_str(&format!("    r.f{} = data.f{};\n", j, j));
        }
        code.push_str("    return r;\n}\n\n");
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("big.ts");
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
fn performance_class_hierarchy() {
    let mut code = String::new();
    for i in 0..10 {
        code.push_str(&format!("class Service{} {{\n    private data{}: any[] = [];\n", i, i));
        for j in 0..5 {
            code.push_str(&format!("    method{}() {{ return this.data{}; }}\n", j, i));
        }
        code.push_str("}\n\n");
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("classes.ts");
    std::fs::write(&path, &code).unwrap();
    let start = std::time::Instant::now();
    let _ = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let elapsed = start.elapsed();
    assert!(elapsed.as_millis() < 500, "took: {}ms", elapsed.as_millis());
}
