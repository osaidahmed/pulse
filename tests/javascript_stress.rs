mod common;

use common::*;
use std::process::Command;

fn check(code: &str) -> String {
    pulse_check_code(code, "js")
}
fn debug(code: &str) -> String {
    pulse_debug_code(code, "js")
}

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
        for j in 0..45 {
            code.push_str(&format!("    const x{} = {};\n", j, j));
        }
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
fn declarations_above_threshold() {
    let mut code = String::new();
    for i in 0..25 {
        code.push_str(&format!("class T{} {{}}\n", i));
    }
    let out = check(&code);
    assert!(has_smell(&out, "Declarations"));
}

// ===========================================================================
// Global nesting
// ===========================================================================

#[test]
fn global_nesting_3_flagged() {
    let out = check(
        "if (a) {\n    if (b) {\n        if (c) {\n            const x = 1;\n        }\n    }\n}\n",
    );
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
        code.push_str(&format!(
            "function func{}(data) {{\n    const r = {{}};\n",
            i
        ));
        for j in 0..18 {
            code.push_str(&format!("    r.f{} = data.f{};\n", j, j));
        }
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

// ===========================================================================
// CC: not/! operator
// ===========================================================================

#[test]
fn cc_counts_not_operator() {
    let out = debug("function f() {\n    if (!a) {}\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(cc >= 2, "if(!a) should have cc >= 2, got: {}", cc);
}

// ===========================================================================
// CC: chained boolean
// ===========================================================================

#[test]
fn cc_chained_boolean() {
    let out = debug("function f() {\n    if (a && b && c && d) {}\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(cc >= 4, "got: {}", cc);
}

// ===========================================================================
// CC: switch cases
// ===========================================================================

#[test]
fn cc_switch_many_cases() {
    let out = debug(concat!(
        "function f(x) {\n",
        "    switch (x) {\n",
        "        case 'a': return 'A';\n",
        "        case 'b': return 'B';\n",
        "        case 'c': return 'C';\n",
        "        case 'd': return 'D';\n",
        "        case 'e': return 'E';\n",
        "        case 'f': return 'F';\n",
        "        case 'g': return 'G';\n",
        "        case 'h': return 'H';\n",
        "        default: return '?';\n",
        "    }\n",
        "}\n",
    ));
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(cc >= 9, "8 cases + base should give cc >= 9, got: {}", cc);
}

// ===========================================================================
// Nesting: try-catch depth
// ===========================================================================

#[test]
fn nesting_try_catch_counts_depth() {
    let out = debug("function f() {\n    try {\n        if (x) {}\n    } catch (e) {}\n}\n");
    let depth = function_metric(&out, "f", "nesting").unwrap_or(0);
    assert!(
        depth >= 1,
        "try-catch should contribute nesting, got: {}",
        depth
    );
}

// ===========================================================================
// LCOM4: single method not flagged
// ===========================================================================

#[test]
fn lcom4_single_method_not_flagged() {
    let out =
        check("class T {\n    constructor() { this.x = 1; }\n    get() { return this.x; }\n}\n");
    assert!(!has_smell(&out, "Low Cohesion"));
}

// ===========================================================================
// LCOM4: constructor excluded
// ===========================================================================

#[test]
fn lcom4_constructor_excluded() {
    let out = check(concat!(
        "class Init {\n",
        "    constructor() {\n",
        "        this.a = 1;\n",
        "        this.b = 2;\n",
        "        this.c = 3;\n",
        "    }\n",
        "    useA() { return this.a; }\n",
        "    useB() { return this.b; }\n",
        "    useC() { return this.c; }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"));
}

// ===========================================================================
// LCOM4: 2 disconnected groups (below threshold)
// ===========================================================================

#[test]
fn lcom4_two_disconnected_groups_not_flagged() {
    let out = check(concat!(
        "class Split {\n",
        "    constructor() { this.fieldA = 1; this.fieldB = 2; }\n",
        "    aWork() { return this.fieldA; }\n",
        "    aRead() { return this.fieldA + 1; }\n",
        "    bWork() { return this.fieldB; }\n",
        "    bRead() { return this.fieldB + 1; }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"));
}

// ===========================================================================
// Duplication: decorators don't affect hash
// ===========================================================================

#[test]
fn duplication_decorators_dont_affect() {
    let out = check(concat!(
        "function funcA(data) {\n",
        "    const r = {};\n",
        "    r.id = data.id;\n",
        "    r.name = data.name;\n",
        "    r.val = data.val;\n",
        "    r.active = data.active;\n",
        "    return r;\n",
        "}\n\n",
        "function funcB(data) {\n",
        "    const r = {};\n",
        "    r.id = data.id;\n",
        "    r.name = data.name;\n",
        "    r.val = data.val;\n",
        "    r.active = data.active;\n",
        "    return r;\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Code Duplication"));
}

// ===========================================================================
// Duplication: async same body
// ===========================================================================

#[test]
fn duplication_async_same_body() {
    let out = check(concat!(
        "async function fetchA(url) {\n",
        "    const r = {};\n",
        "    r.data = await get(url);\n",
        "    r.status = 'ok';\n",
        "    r.ts = now();\n",
        "    r.src = url;\n",
        "    return r;\n",
        "}\n\n",
        "async function fetchB(url) {\n",
        "    const r = {};\n",
        "    r.data = await get(url);\n",
        "    r.status = 'ok';\n",
        "    r.ts = now();\n",
        "    r.src = url;\n",
        "    return r;\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Code Duplication"));
}

// ===========================================================================
// Duplication: mixed test+prod
// ===========================================================================

#[test]
fn duplication_mixed_test_and_prod_flagged() {
    let out = check(concat!(
        "function test_something(data) {\n",
        "    const r = {};\n",
        "    r.id = data.id;\n",
        "    r.name = data.name;\n",
        "    r.val = data.val;\n",
        "    r.extra = data.extra;\n",
        "    return r;\n",
        "}\n\n",
        "function processData(data) {\n",
        "    const r = {};\n",
        "    r.id = data.id;\n",
        "    r.name = data.name;\n",
        "    r.val = data.val;\n",
        "    r.extra = data.extra;\n",
        "    return r;\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Code Duplication"));
}

// ===========================================================================
// God Method triggers God Class
// ===========================================================================

#[test]
fn god_method_triggers_god_class() {
    let mut code = String::from("function monster() {\n");
    for i in 0..10 {
        code.push_str(&format!("    if (x === {}) {{}}\n", i));
    }
    for i in 0..40 {
        code.push_str(&format!("    const y{} = {};\n", i, i));
    }
    code.push_str("}\n\n");
    for i in 0..21 {
        code.push_str(&format!("function fn{}() {{ return {}; }}\n", i, i));
    }
    for i in 0..350 {
        code.push_str(&format!("const V{} = {};\n", i, i));
    }
    let out = check(&code);
    assert!(has_smell(&out, "God Method"));
    assert!(has_smell(&out, "God Class"));
}

// ===========================================================================
// Assertion block edge cases
// ===========================================================================

#[test]
fn assertion_block_interrupted_resets() {
    let out = check(concat!(
        "function testInterleaved() {\n",
        "    expect(x).toBe(1);\n",
        "    expect(y).toBe(2);\n",
        "    expect(z).toBe(3);\n",
        "    doSomething();\n",
        "    expect(a).toBe(4);\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Large Assertion Block"));
}

#[test]
fn assertion_block_at_threshold_not_flagged() {
    let mut code = String::from("function testExact() {\n");
    for i in 0..10 {
        code.push_str(&format!("    expect(x{}).toBe({});\n", i, i));
    }
    code.push_str("}\n");
    let out = check(&code);
    assert!(!has_smell(&out, "Large Assertion Block"));
}

#[test]
fn assertion_block_above_threshold() {
    let mut code = String::from("function testBig() {\n");
    for i in 0..15 {
        code.push_str(&format!("    expect(x{}).toBe({});\n", i, i));
    }
    code.push_str("}\n");
    let out = check(&code);
    assert!(has_smell(&out, "Large Assertion Block"));
}

// ===========================================================================
// Embedded block: small not flagged, multiline flagged
// ===========================================================================

#[test]
fn small_string_not_flagged_as_embedded() {
    let out = check("function f() {\n    const x = 'hello world';\n    return x;\n}\n");
    assert!(!has_smell(&out, "Large Embedded Block"));
}

#[test]
fn multiline_template_flagged_as_embedded() {
    let mut code = String::from("function f() {\n    const x = `\n");
    for i in 0..20 {
        code.push_str(&format!("        line {} of template\n", i));
    }
    code.push_str("    `;\n    return x;\n}\n");
    let out = check(&code);
    assert!(has_smell(&out, "Large Embedded Block"));
}

// ===========================================================================
// Deep global nesting
// ===========================================================================

#[test]
fn shallow_global_nesting_not_flagged() {
    let out = check("if (true) {\n    const x = 1;\n}\n");
    assert!(!has_smell(&out, "Deep Global Nesting"));
}

// ===========================================================================
// Declarations: below threshold
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

// ===========================================================================
// Real-world patterns: Express.js
// ===========================================================================

#[test]
fn clean_express_handler_not_flagged() {
    let out = check(concat!(
        "function getUsers(req, res) {\n",
        "    const users = db.find();\n",
        "    res.json(users);\n",
        "}\n",
    ));
    assert!(
        out.is_empty(),
        "clean Express handler should not be flagged, got: {}",
        out
    );
}

// ===========================================================================
// Hook JSON edge cases
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
            child
                .stdin
                .take()
                .unwrap()
                .write_all(b"{\"other\": 1}")
                .unwrap();
            child.wait_with_output()
        })
        .expect("failed to run");
    assert!(out.stdout.is_empty());
}

#[test]
fn hook_missing_file_path_key() {
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["--hook"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child
                .stdin
                .take()
                .unwrap()
                .write_all(b"{\"tool_input\": {\"content\": \"hello\"}}")
                .unwrap();
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
// Performance: class hierarchy
// ===========================================================================

#[test]
fn performance_class_hierarchy() {
    let mut code = String::new();
    for i in 0..10 {
        code.push_str(&format!(
            "class Service{} {{\n    constructor() {{ this.data{} = []; }}\n",
            i, i
        ));
        for j in 0..5 {
            code.push_str(&format!("    method{}() {{ return this.data{}; }}\n", j, i));
        }
        code.push_str("}\n\n");
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("classes.js");
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
// Arrow function analyzed
// ===========================================================================

#[test]
fn arrow_function_with_excess_args_flagged() {
    let out = check("const handler = (a, b, c, d, e, f, g) => {\n    return a + b;\n};\n");
    assert!(has_smell(&out, "Excess Arguments"));
}

// ===========================================================================
// Overall function size: at threshold
// ===========================================================================

#[test]
fn overall_function_size_at_threshold() {
    let mut code = String::new();
    for i in 0..3 {
        code.push_str(&format!("function lg{}() {{\n", i));
        for j in 0..45 {
            code.push_str(&format!("    const x{} = {};\n", j, j));
        }
        code.push_str("}\n\n");
    }
    let out = check(&code);
    assert!(has_smell(&out, "Overall Function Size"));
}

// ===========================================================================
// CC: nullish coalescing chained
// ===========================================================================

#[test]
fn cc_nullish_chained() {
    let out = debug("function f() {\n    if ((a ?? b) || c) {}\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(cc >= 4, "nullish + or + if should increase cc, got: {}", cc);
}

// ===========================================================================
// CC: ternary in assignment
// ===========================================================================

#[test]
fn cc_ternary_in_assignment() {
    let out = debug("function f() {\n    const x = a ? 1 : 2;\n    const y = b ? 3 : 4;\n    return x + y;\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(cc >= 3, "two ternaries should give cc >= 3, got: {}", cc);
}

// ===========================================================================
// Args: with defaults
// ===========================================================================

#[test]
fn args_with_destructured_defaults() {
    let out = debug("function f(a, b = 1, c = null) {}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(3));
}

// ===========================================================================
// Nesting: deeper try-catch
// ===========================================================================

#[test]
fn nesting_deep_try_catch() {
    let out = debug("function f() {\n    try {\n        if (x) {\n            if (y) {}\n        }\n    } catch (e) {}\n}\n");
    let depth = function_metric(&out, "f", "nesting").unwrap_or(0);
    assert!(depth >= 2, "try+if+if should be >= 2, got: {}", depth);
}

// ===========================================================================
// Constructor vs excess args disambiguation
// ===========================================================================

#[test]
fn constructor_injection_not_excess() {
    let out = check("class S {\n    constructor(a, b, c, d, e, f) {}\n}\n");
    assert!(has_smell(&out, "Constructor Over-Injection"));
    let lines: Vec<&str> = out.lines().filter(|l| l.contains("constructor")).collect();
    assert!(!lines.iter().any(|l| l.contains("Excess Arguments")));
}

// ===========================================================================
// Declarations: decorated counted
// ===========================================================================

#[test]
fn decorated_classes_counted_as_declarations() {
    let mut code = String::new();
    for i in 0..25 {
        code.push_str(&format!("class T{} {{}}\n", i));
    }
    let out = check(&code);
    assert!(has_smell(&out, "Declarations"));
}

// ===========================================================================
// Global conditionals shallow not flagged
// ===========================================================================

#[test]
fn global_nesting_shallow_not_flagged() {
    let out = check("if (true) {\n    const x = 1;\n}\n");
    assert!(!has_smell(&out, "Deep Global Nesting"));
}

// ===========================================================================
// Primitive obsession: never triggers in JS (reminder)
// ===========================================================================

#[test]
fn primitive_obsession_never_triggers_with_many_args() {
    let out = check("function f(a, b, c, d, e, f, g, h, i) {\n    return a;\n}\n");
    assert!(!has_smell(&out, "Primitive Obsession"));
}

// ===========================================================================
// God class not triggered without god method
// ===========================================================================

#[test]
fn god_class_not_triggered_without_god_method() {
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
// Duplication: test functions suppressed
// ===========================================================================

#[test]
fn duplication_test_functions_suppressed() {
    let out = check(concat!(
        "function test_a(d) {\n    const r = {};\n    r.id = d.id;\n    r.name = d.name;\n    r.val = d.val;\n    r.ok = d.ok;\n    return r;\n}\n\n",
        "function test_b(d) {\n    const r = {};\n    r.id = d.id;\n    r.name = d.name;\n    r.val = d.val;\n    r.ok = d.ok;\n    return r;\n}\n",
    ));
    assert!(!has_smell(&out, "Code Duplication"));
}

// ===========================================================================
// Multiple smells: embedded + excess
// ===========================================================================

#[test]
fn function_with_embedded_and_excess_args() {
    let mut code = String::from("function bad(a, b, c, d, e, f, g, h) {\n    const q = `\n");
    for i in 0..20 {
        code.push_str(&format!("        SELECT field_{}\n", i));
    }
    code.push_str("    `;\n    return q;\n}\n");
    let out = check(&code);
    assert!(has_smell(&out, "Excess Arguments"));
    assert!(has_smell(&out, "Large Embedded Block"));
}

// ===========================================================================
// Empty file inline
// ===========================================================================

#[test]
fn empty_file_inline() {
    let out = check("");
    assert!(out.is_empty());
}

// ===========================================================================
// Cognitive Complexity (CogC) precision
// ===========================================================================

#[test]
fn cogc_flat_branches() {
    let out = debug(concat!(
        "function f() {\n",
        "    if (x === 1) {}\n",
        "    if (x === 2) {}\n",
        "    if (x === 3) {}\n",
        "    if (x === 4) {}\n",
        "    if (x === 5) {}\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cogc"), Some(5));
}

#[test]
fn cogc_nested_ifs() {
    let out = debug(concat!(
        "function f() {\n",
        "    if (a) {\n",
        "        if (b) {\n",
        "            if (c) {\n",
        "                if (d) {}\n",
        "            }\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cogc"), Some(10));
}

#[test]
fn cogc_else_if_no_nesting() {
    let out = debug(concat!(
        "function f() {\n",
        "    if (a) {}\n",
        "    else if (b) {}\n",
        "    else if (c) {}\n",
        "    else if (d) {}\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cogc"), Some(4));
}

#[test]
fn cogc_else_increases_nesting() {
    let out = debug(concat!(
        "function f() {\n",
        "    if (a) {}\n",
        "    else {\n",
        "        if (b) {}\n",
        "    }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cogc"), Some(4));
}

#[test]
fn cogc_for_loop_nested() {
    let out = debug(concat!(
        "function f() {\n",
        "    if (a) {\n",
        "        for (let i = 0; i < 10; i++) {}\n",
        "    }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cogc"), Some(3));
}

#[test]
fn cogc_switch_counted() {
    let out = debug(concat!(
        "function f(x) {\n",
        "    switch (x) {\n",
        "        case 1: break;\n",
        "        case 2: break;\n",
        "        case 3: break;\n",
        "    }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cogc"), Some(1));
}

#[test]
fn cogc_catch_penalized() {
    let out = debug(concat!(
        "function f() {\n",
        "    try {\n",
        "        if (x) {}\n",
        "    } catch (e) {}\n",
        "}\n",
    ));
    let cogc = function_metric(&out, "f", "cogc").unwrap();
    assert!(cogc >= 3, "try/catch with nested if should have cogc >= 3, got: {}", cogc);
}

#[test]
fn cogc_ternary_counted() {
    let out = debug(concat!(
        "function f() {\n",
        "    return a ? 1 : 0;\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cogc"), Some(1));
}

#[test]
fn cogc_boolean_single_sequence() {
    let out = debug(concat!(
        "function f() {\n",
        "    if (a && b && c) {}\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cogc"), Some(2));
}

#[test]
fn cogc_boolean_mixed_sequence() {
    let out = debug(concat!(
        "function f() {\n",
        "    if (a && b || c) {}\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cogc"), Some(3));
}

#[test]
fn cogc_triggers_complex_method() {
    let out = check(concat!(
        "function f() {\n",
        "    if (a) {\n",
        "        if (b) {\n",
        "            if (c) {\n",
        "                if (d) {\n",
        "                    if (e) {}\n",
        "                }\n",
        "            }\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Complex Method"), "high cogc should trigger Complex Method");
    let dbg = debug(concat!(
        "function f() {\n",
        "    if (a) {\n",
        "        if (b) {\n",
        "            if (c) {\n",
        "                if (d) {\n",
        "                    if (e) {}\n",
        "                }\n",
        "            }\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    let cogc = function_metric(&dbg, "f", "cogc").unwrap();
    let cc = function_metric(&dbg, "f", "cc").unwrap();
    assert!(cogc >= 15, "cogc should be >= 15, got: {}", cogc);
    assert!(cc < 9, "cc should be < 9, got: {}", cc);
    assert!(out.contains("cogc="), "detail should contain cogc=");
}

#[test]
fn cogc_below_threshold_no_smell() {
    let out = check(concat!(
        "function f() {\n",
        "    if (a) {\n",
        "        if (b) {\n",
        "            if (c) {\n",
        "                if (d) {}\n",
        "            }\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Complex Method"), "cogc=10 should not trigger Complex Method");
}

// ===========================================================================
// Empty Error Handler
// ===========================================================================

#[test]
fn empty_catch_detected() {
    let out = check(concat!(
        "function f() {\n",
        "    try { risky(); } catch (e) {}\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Empty Error Handler"));
}

#[test]
fn non_empty_catch_not_detected() {
    let out = check(concat!(
        "function f() {\n",
        "    try { risky(); } catch (e) { console.log(e); }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Empty Error Handler"));
}

#[test]
fn multiple_empty_catches() {
    let out = check(concat!(
        "function f() {\n",
        "    try { a(); } catch (e) {}\n",
        "    try { b(); } catch (e) {}\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Empty Error Handler"));
    assert!(out.contains("2 empty catch blocks"), "should report count=2, got: {}", out);
}

#[test]
fn no_try_catch_no_smell() {
    let out = check(concat!(
        "function f() {\n",
        "    const x = 1;\n",
        "    return;\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Empty Error Handler"));
}

#[test]
fn catch_with_only_comment_detected() {
    let out = check(concat!(
        "function f() {\n",
        "    try { risky(); } catch (e) { /* todo */ }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Empty Error Handler"));
}
