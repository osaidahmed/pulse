mod common;

use common::*;
use std::process::Command;

fn check(code: &str) -> String { pulse_check_code(code, "js") }
fn debug(code: &str) -> String { pulse_debug_code(code, "js") }

// ===========================================================================
// CC counting precision
// ===========================================================================

#[test]
fn cc_counts_if() {
    let out = debug("function f() {\n    if (x) {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_else_if() {
    let out = debug("function f() {\n    if (x) {} else if (y) {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_counts_for_of() {
    let out = debug("function f() {\n    for (const x of y) {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_for_in() {
    let out = debug("function f() {\n    for (const x in y) {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_while() {
    let out = debug("function f() {\n    while (x) {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_catch() {
    let out = debug("function f() {\n    try {} catch (e) {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_and() {
    let out = debug("function f() {\n    if (a && b) {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_counts_or() {
    let out = debug("function f() {\n    if (a || b) {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_counts_nullish() {
    let out = debug("function f() {\n    if (a ?? b) {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_counts_ternary() {
    let out = debug("function f() {\n    const x = a ? 1 : 2;\n    return x;\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_switch_cases() {
    let out = debug("function f(x) {\n    switch (x) {\n        case 'a': return 1;\n        case 'b': return 2;\n        case 'c': return 3;\n        default: return 0;\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(4));
}

#[test]
fn cc_nested_if_in_for() {
    let out = debug("function f() {\n    for (const x of y) {\n        if (x) {}\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

// ===========================================================================
// Nesting depth precision
// ===========================================================================

#[test]
fn nesting_0_flat() {
    let out = debug("function f() {\n    return 1;\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(0));
}

#[test]
fn nesting_1_single_if() {
    let out = debug("function f() {\n    if (x) {}\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(1));
}

#[test]
fn nesting_2_nested_if() {
    let out = debug("function f() {\n    if (x) {\n        if (y) {}\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(2));
}

#[test]
fn nesting_3_for_if_for() {
    let out = debug("function f() {\n    if (x) {\n        for (const i of y) {\n            if (z) {}\n        }\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(3));
}

// ===========================================================================
// Argument counting
// ===========================================================================

#[test]
fn args_positional() {
    let out = debug("function f(a, b, c) {}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(3));
}

#[test]
fn args_with_defaults() {
    let out = debug("function f(a, b = 1, c = null) {}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(3));
}

#[test]
fn args_with_rest() {
    let out = debug("function f(a, ...rest) {}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(2));
}

#[test]
fn args_zero() {
    let out = debug("function f() {}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(0));
}

// ===========================================================================
// Primitive obsession never triggers in JS
// ===========================================================================

#[test]
fn primitive_obsession_never_in_js() {
    let out = check("function f(a, b, c, d, e, f, g, h, i) {\n    return a;\n}\n");
    assert!(!has_smell(&out, "Primitive Obsession"));
}

// ===========================================================================
// LCOM4
// ===========================================================================

#[test]
fn lcom4_three_groups_flagged() {
    let out = check(concat!(
        "class M {\n",
        "    constructor() { this.x = 1; this.y = 2; this.z = 3; }\n",
        "    useX() { return this.x; }\n",
        "    useY() { return this.y; }\n",
        "    useZ() { return this.z; }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_cohesive_not_flagged() {
    let out = check("class C {\n    constructor() { this.d = []; }\n    add(x) { this.d.push(x); }\n    get() { return this.d; }\n    clear() { this.d = []; }\n}\n");
    assert!(!has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_transitive_connection() {
    let out = check(concat!(
        "class C {\n",
        "    constructor() { this.a = 1; this.b = 2; }\n",
        "    m1() { return this.a; }\n",
        "    m2() { return this.a + this.b; }\n",
        "    m3() { return this.b; }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"));
}

// ===========================================================================
// Code duplication
// ===========================================================================

#[test]
fn duplication_detected() {
    let out = check(concat!(
        "function rptA(data) {\n    const r = {};\n    r.id = data.id;\n    r.name = data.name;\n    r.val = data.val;\n    r.status = 'active';\n    return r;\n}\n\n",
        "function rptB(data) {\n    const r = {};\n    r.id = data.id;\n    r.name = data.name;\n    r.val = data.val;\n    r.status = 'active';\n    return r;\n}\n",
    ));
    assert!(has_smell(&out, "Code Duplication"));
}

#[test]
fn duplication_test_suppressed() {
    let out = check(concat!(
        "function test_a(d) {\n    const r = {};\n    r.id = d.id;\n    r.name = d.name;\n    r.val = d.val;\n    r.ok = d.ok;\n    return r;\n}\n\n",
        "function test_b(d) {\n    const r = {};\n    r.id = d.id;\n    r.name = d.name;\n    r.val = d.val;\n    r.ok = d.ok;\n    return r;\n}\n",
    ));
    assert!(!has_smell(&out, "Code Duplication"));
}

// ===========================================================================
// God Class
// ===========================================================================

#[test]
fn god_class_requires_god_method() {
    let mut code = String::new();
    for i in 0..25 {
        code.push_str(&format!("function fn{}() {{ return {}; }}\n", i, i));
    }
    for i in 0..200 {
        code.push_str(&format!("const V{} = {};\n", i, i));
    }
    let out = check(&code);
    assert!(!has_smell(&out, "God Class"));
}

// ===========================================================================
// Overall function size
// ===========================================================================

#[test]
fn overall_size_below_threshold() {
    let mut code = String::new();
    for i in 0..2 {
        code.push_str(&format!("function lg{}() {{\n", i));
        for j in 0..45 { code.push_str(&format!("    const x{} = {};\n", j, j)); }
        code.push_str("}\n\n");
    }
    let out = check(&code);
    assert!(!has_smell(&out, "Overall Function Size"));
}

#[test]
fn overall_size_above_threshold() {
    let mut code = String::new();
    for i in 0..3 {
        code.push_str(&format!("function lg{}() {{\n", i));
        for j in 0..45 { code.push_str(&format!("    const x{} = {};\n", j, j)); }
        code.push_str("}\n\n");
    }
    let out = check(&code);
    assert!(has_smell(&out, "Overall Function Size"));
}

// ===========================================================================
// Declarations
// ===========================================================================

#[test]
fn declarations_above_threshold() {
    let mut code = String::new();
    for i in 0..25 { code.push_str(&format!("class T{} {{}}\n", i)); }
    let out = check(&code);
    assert!(has_smell(&out, "Declarations"));
}

// ===========================================================================
// Global nesting
// ===========================================================================

#[test]
fn global_nesting_3_flagged() {
    let out = check("if (a) {\n    if (b) {\n        if (c) {\n            const x = 1;\n        }\n    }\n}\n");
    assert!(has_smell(&out, "Deep Global Nesting"));
}

// ===========================================================================
// Constructor vs excess args
// ===========================================================================

#[test]
fn constructor_reports_over_injection() {
    let out = check("class S {\n    constructor(a, b, c, d, e, f) {}\n}\n");
    assert!(has_smell(&out, "Constructor Over-Injection"));
}

#[test]
fn regular_function_reports_excess_args() {
    let out = check("function f(a, b, c, d, e, f, g) {}\n");
    assert!(has_smell(&out, "Excess Arguments"));
    assert!(!has_smell(&out, "Constructor Over-Injection"));
}

// ===========================================================================
// Multiple smells
// ===========================================================================

#[test]
fn multiple_smells_same_function() {
    let out = check(concat!(
        "function bad(a, b, c, d, e, f, g, h) {\n",
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
// Performance
// ===========================================================================

#[test]
fn performance_1000_loc() {
    let mut code = String::new();
    for i in 0..50 {
        code.push_str(&format!("function func{}(data) {{\n    const r = {{}};\n", i));
        for j in 0..18 { code.push_str(&format!("    r.f{} = data.f{};\n", j, j)); }
        code.push_str("    return r;\n}\n\n");
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("big.js");
    std::fs::write(&path, &code).unwrap();
    let start = std::time::Instant::now();
    let _ = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let elapsed = start.elapsed();
    assert!(elapsed.as_millis() < 500, "took: {}ms", elapsed.as_millis());
}
