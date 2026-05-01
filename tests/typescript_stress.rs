mod common;

use common::*;
use std::process::Command;

lang_helpers!("ts");

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
    assert!(cc >= 4, "chained boolean should increase cc, got: {cc}");
}

#[test]
fn cc_counts_switch_cases() {
    let out = debug("function f(x: string): string {\n    switch (x) {\n        case 'a': return 'A';\n        case 'b': return 'B';\n        case 'c': return 'C';\n        default: return '?';\n    }\n}\n");
    // base(1) + 3 cases (default doesn't count) = 4
    assert_eq!(function_metric(&out, "f", "cc"), Some(4));
}

#[test]
fn cc_nested_if_in_for() {
    let out =
        debug("function f(): void {\n    for (const x of y) {\n        if (x) {}\n    }\n}\n");
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

#[test]
fn lcom4_methods_connected_by_call() {
    let out = check(concat!(
        "class Coord {\n",
        "    state = 0;\n",
        "    process(e: any) { return this.validate(e) && this.dispatch(e); }\n",
        "    validate(e: any) { return e.isValid(); }\n",
        "    dispatch(e: any) { return this.send(e); }\n",
        "    send(e: any) { return null; }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"), "got: {out}");
}

#[test]
fn lcom4_mixed_field_and_call_connection() {
    let out = check(concat!(
        "class Mixed {\n",
        "    x = 0;\n",
        "    a() { return this.x; }\n",
        "    b() { this.x = 1; return this.c(); }\n",
        "    c() { return 42; }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_god_class_still_fires() {
    let out = check(concat!(
        "class UserService {\n",
        "    constructor(private db: any, private cache: any, private mailer: any, private events: any, private audit: any) {}\n",
        "    getUser(uid: string) { return this.db.get(uid); }\n",
        "    cacheUser(u: any) { this.cache.set(u.id, u); }\n",
        "    sendWelcome(u: any) { this.mailer.send(u.email); }\n",
        "    publish(e: any) { this.events.emit(e); }\n",
        "    auditLog(msg: string) { this.audit.log(msg); }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"), "got: {out}");
}

#[test]
fn lcom4_dependency_method_calls_dont_falsely_connect() {
    let out = check(concat!(
        "class Service {\n",
        "    constructor(private db: any, private cache: any, private log: any) {}\n",
        "    a() { return this.db.foo(); }\n",
        "    b() { return this.cache.foo(); }\n",
        "    c() { return this.log.foo(); }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"));
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
    for i in 0..declarations_above() {
        code.push_str(&format!("function fn{i}(): number {{ return {i}; }}\n"));
    }
    for i in 0..file_padding() {
        code.push_str(&format!("const VAR{i} = {i};\n"));
    }
    let out = check(&code);
    assert!(!has_smell(&out, "God Class"));
}

#[test]
fn god_method_triggers_god_class() {
    let mut code = String::from("function monster(): void {\n");
    for i in 0..cc_branches() {
        code.push_str(&format!("    if (x === {i}) {{}}\n"));
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("    const y{i} = {i};\n"));
    }
    code.push_str("}\n\n");
    for i in 0..functions_above() {
        code.push_str(&format!("function fn{i}(): number {{ return {i}; }}\n"));
    }
    for i in 0..file_padding() {
        code.push_str(&format!("const V{i} = {i};\n"));
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
    for i in 0..(t().large_fn_count as usize - 1) {
        code.push_str(&format!("function lg{i}(): void {{\n"));
        for j in 0..large_fn_lines() {
            code.push_str(&format!("    const x{j} = {j};\n"));
        }
        code.push_str("}\n\n");
    }
    let out = check(&code);
    assert!(!has_smell(&out, "Overall Function Size"));
}

#[test]
fn overall_function_size_triggered_at_threshold() {
    let mut code = String::new();
    for i in 0..t().large_fn_count as usize {
        code.push_str(&format!("function lg{i}(): void {{\n"));
        for j in 0..large_fn_lines() {
            code.push_str(&format!("    const x{j} = {j};\n"));
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
        code.push_str(&format!("class T{i} {{}}\n"));
    }
    let out = check(&code);
    assert!(!has_smell(&out, "Declarations"));
}

#[test]
fn declarations_above_threshold() {
    let mut code = String::new();
    for i in 0..declarations_above() {
        code.push_str(&format!("class T{i} {{}}\n"));
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
    let out =
        check("class S {\n    constructor(a: any, b: any, c: any, d: any, e: any, f: any) {}\n}\n");
    assert!(has_smell(&out, "Constructor Over-Injection"));
    let lines: Vec<&str> = out.lines().filter(|l| l.contains("constructor")).collect();
    assert!(!lines.iter().any(|l| l.contains("Excess Arguments")));
}

#[test]
fn regular_function_reports_excess_args() {
    let out =
        check("function f(a: any, b: any, c: any, d: any, e: any, f: any, g: any): void {}\n");
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
            "function func{i}(data: any): any {{\n    const r: any = {{}};\n"
        ));
        for j in 0..18 {
            code.push_str(&format!("    r.f{j} = data.f{j};\n"));
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
        code.push_str(&format!(
            "class Service{i} {{\n    private data{i}: any[] = [];\n"
        ));
        for j in 0..5 {
            code.push_str(&format!("    method{j}() {{ return this.data{i}; }}\n"));
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

// ===========================================================================
// CC: not/! operator
// ===========================================================================

#[test]
fn cc_counts_not_operator() {
    let out = debug("function f(): void {\n    if (!a) {}\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(cc >= 2, "if(!a) should have cc >= 2, got: {cc}");
}

// ===========================================================================
// Nesting: try-catch depth
// ===========================================================================

#[test]
fn nesting_depth_with_try_catch() {
    let out = debug("function f(): void {\n    try {\n        if (x) {}\n    } catch (e) {}\n}\n");
    let depth = function_metric(&out, "f", "nesting").unwrap_or(0);
    assert!(
        depth >= 1,
        "try-catch should add nesting depth, got: {depth}"
    );
}

// ===========================================================================
// Args: typed params, typed+defaults
// ===========================================================================

#[test]
fn args_counts_typed_params() {
    let out = debug("function f(a: number, b: string, c: boolean): void {}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(3));
}

#[test]
fn args_counts_typed_with_defaults() {
    let out = debug("function f(a: number = 0, b: string = ''): void {}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(2));
}

#[test]
fn args_excludes_this_in_method() {
    let out = debug("class C {\n    m(a: number, b: number): void {}\n}\n");
    assert_eq!(function_metric(&out, "C.m", "args"), Some(2));
}

// ===========================================================================
// Primitive obsession: recognizes specific types
// ===========================================================================

#[test]
fn primitive_obsession_recognizes_string_symbol() {
    let out = check("function f(a: string, b: string, c: string, d: string): void {}\n");
    assert!(has_smell(&out, "Primitive Obsession"));
}

// ===========================================================================
// LCOM4: single method not flagged
// ===========================================================================

#[test]
fn lcom4_single_method_not_flagged() {
    let out = check("class T {\n    x = 1;\n    get() { return this.x; }\n}\n");
    assert!(!has_smell(&out, "Low Cohesion"));
}

// ===========================================================================
// LCOM4: constructor excluded
// ===========================================================================

#[test]
fn lcom4_constructor_excluded() {
    let out = check(concat!(
        "class Init {\n",
        "    a = 1; b = 2; c = 3;\n",
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
        "    fieldA = 1; fieldB = 2;\n",
        "    aWork() { return this.fieldA; }\n",
        "    aRead() { return this.fieldA + 1; }\n",
        "    bWork() { return this.fieldB; }\n",
        "    bRead() { return this.fieldB + 1; }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"));
}

// ===========================================================================
// Duplication: decorators
// ===========================================================================

#[test]
fn duplication_decorators_dont_affect() {
    let out = check(concat!(
        "function decA(f: any) { return f; }\n",
        "function decB(f: any) { return f; }\n\n",
        "function funcA(data: any): any {\n",
        "    const r: any = {};\n",
        "    r.id = data.id;\n",
        "    r.name = data.name;\n",
        "    r.val = data.val;\n",
        "    r.active = data.active;\n",
        "    return r;\n",
        "}\n\n",
        "function funcB(data: any): any {\n",
        "    const r: any = {};\n",
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
        "async function fetchA(url: string): Promise<any> {\n",
        "    const r: any = {};\n",
        "    r.data = await get(url);\n",
        "    r.status = 'ok';\n",
        "    r.ts = now();\n",
        "    r.src = url;\n",
        "    return r;\n",
        "}\n\n",
        "async function fetchB(url: string): Promise<any> {\n",
        "    const r: any = {};\n",
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
        "function test_something(data: any): any {\n",
        "    const r: any = {};\n",
        "    r.id = data.id;\n",
        "    r.name = data.name;\n",
        "    r.val = data.val;\n",
        "    r.extra = data.extra;\n",
        "    return r;\n",
        "}\n\n",
        "function processData(data: any): any {\n",
        "    const r: any = {};\n",
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
// Assertion block edge cases
// ===========================================================================

#[test]
fn assertion_block_interrupted_resets() {
    let out = check(concat!(
        "function testInterleaved(): void {\n",
        "    expect(x).toBe(1);\n",
        "    expect(y).toBe(2);\n",
        "    expect(z).toBe(3);\n",
        "    doSomething();\n",
        "    expect(a).toBe(4);\n",
        "    expect(b).toBe(5);\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Large Assertion Block"));
}

#[test]
fn assertion_block_above_threshold() {
    let mut code = String::from("function testBig(): void {\n");
    for i in 0..asserts_above() {
        code.push_str(&format!("    expect(x{i}).toBe({i});\n"));
    }
    code.push_str("}\n");
    let out = check(&code);
    assert!(has_smell(&out, "Large Assertion Block"));
}

// ===========================================================================
// Embedded block: small not flagged
// ===========================================================================

#[test]
fn small_string_not_flagged_as_embedded() {
    let out = check("function f(): string {\n    const x = 'hello world';\n    return x;\n}\n");
    assert!(!has_smell(&out, "Large Embedded Block"));
}

#[test]
fn multiline_template_flagged_as_embedded() {
    let mut code = String::from("function f(): string {\n    const x = `\n");
    for i in 0..embedded_lines_above() {
        code.push_str(&format!("        line {i} of template\n"));
    }
    code.push_str("    `;\n    return x;\n}\n");
    let out = check(&code);
    assert!(has_smell(&out, "Large Embedded Block"));
}

// ===========================================================================
// Real-world patterns: React component
// ===========================================================================

#[test]
fn clean_react_component_not_flagged() {
    let out = check(concat!(
        "interface Props {\n",
        "    title: string;\n",
        "    count: number;\n",
        "}\n\n",
        "function ItemList(props: Props): any {\n",
        "    const items = getItems();\n",
        "    return items;\n",
        "}\n",
    ));
    assert!(
        out.is_empty(),
        "clean React component should not be flagged, got: {out}"
    );
}

// ===========================================================================
// Hook JSON edge cases
// ===========================================================================

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

// ===========================================================================
// CC: multiple catch clauses
// ===========================================================================

#[test]
fn cc_multiple_catch_not_applicable_ts() {
    // TS/JS only supports one catch block, but the single catch should still count
    let out = debug("function f(): void {\n    try { } catch (e) { }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

// ===========================================================================
// CC: switch with many cases
// ===========================================================================

#[test]
fn cc_switch_many_cases() {
    let out = debug(concat!(
        "function f(x: string): string {\n",
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
    assert!(cc >= 9, "8 cases + base should give cc >= 9, got: {cc}");
}

// ===========================================================================
// Assertion block at threshold
// ===========================================================================

#[test]
fn assertion_block_at_threshold_not_flagged() {
    let mut code = String::from("function testExact(): void {\n");
    for i in 0..asserts_at() {
        code.push_str(&format!("    expect(x{i}).toBe({i});\n"));
    }
    code.push_str("}\n");
    let out = check(&code);
    assert!(!has_smell(&out, "Large Assertion Block"));
}

// ===========================================================================
// Declarations: decorated counted
// ===========================================================================

#[test]
fn decorated_classes_counted_as_declarations() {
    let mut code = String::from("function deco(cls: any) { return cls; }\n\n");
    for i in 0..declarations_above() {
        code.push_str(&format!("class T{i} {{}}\n"));
    }
    let out = check(&code);
    assert!(has_smell(&out, "Declarations"));
}

// ===========================================================================
// Global nesting: shallow not flagged
// ===========================================================================

#[test]
fn shallow_global_nesting_not_flagged() {
    let out = check("if (true) {\n    const x = 1;\n}\n");
    assert!(!has_smell(&out, "Deep Global Nesting"));
}

// ===========================================================================
// God Class: not triggered without god method
// ===========================================================================

#[test]
fn god_class_not_triggered_without_god_method() {
    let mut code = String::new();
    for i in 0..declarations_above() {
        code.push_str(&format!("function fn{i}(): number {{ return {i}; }}\n"));
    }
    for i in 0..file_padding() {
        code.push_str(&format!("const VAR{i} = {i};\n"));
    }
    let out = check(&code);
    assert!(!has_smell(&out, "God Class"));
}

// ===========================================================================
// God Class: triggers when large+many+god
// ===========================================================================

#[test]
fn god_class_triggers_when_large_with_god_method() {
    let mut code = String::from("function monster(): void {\n");
    for i in 0..cc_branches() {
        code.push_str(&format!("    if (x === {i}) {{}}\n"));
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("    const y{i} = {i};\n"));
    }
    code.push_str("}\n\n");
    for i in 0..functions_above() {
        code.push_str(&format!("function fn{i}(): number {{ return {i}; }}\n"));
    }
    for i in 0..file_padding() {
        code.push_str(&format!("const V{i} = {i};\n"));
    }
    let out = check(&code);
    assert!(has_smell(&out, "God Method"));
    assert!(has_smell(&out, "God Class"));
}

// ===========================================================================
// Overall function size: below threshold
// ===========================================================================

#[test]
fn overall_function_size_below_threshold_not_flagged() {
    let mut code = String::new();
    for i in 0..(t().large_fn_count as usize - 1) {
        code.push_str(&format!("function lg{i}(): void {{\n"));
        for j in 0..large_fn_lines() {
            code.push_str(&format!("    const x{j} = {j};\n"));
        }
        code.push_str("}\n\n");
    }
    let out = check(&code);
    assert!(!has_smell(&out, "Overall Function Size"));
}

// ===========================================================================
// Constructor: regular reports excess, not constructor injection
// ===========================================================================

#[test]
fn constructor_reports_injection_not_excess() {
    let out =
        check("class S {\n    constructor(a: any, b: any, c: any, d: any, e: any, f: any) {}\n}\n");
    assert!(has_smell(&out, "Constructor Over-Injection"));
    let lines: Vec<&str> = out.lines().filter(|l| l.contains("constructor")).collect();
    assert!(!lines.iter().any(|l| l.contains("Excess Arguments")));
}

// ===========================================================================
// Multiple smells: excess + embedded + deep nesting
// ===========================================================================

#[test]
fn function_with_excess_embedded_and_deep_nesting() {
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
        "    `;\n",
        "    for (const x of [a]) {\n",
        "        if (x) {\n",
        "            for (const y of [x]) {\n",
        "                if (y) {\n",
        "                    for (const z of [y]) {\n",
        "                        if (z) { console.log(z); }\n",
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
// Nesting: with try-catch as depth contributor
// ===========================================================================

#[test]
fn nesting_try_catch_counts_depth() {
    let out = debug("function f(): void {\n    try {\n        if (x) {\n            if (y) {}\n        }\n    } catch (e) {}\n}\n");
    let depth = function_metric(&out, "f", "nesting").unwrap_or(0);
    assert!(depth >= 2, "try+if+if should be >= 2, got: {depth}");
}

// ===========================================================================
// Cognitive Complexity (CogC) precision
// ===========================================================================

#[test]
fn cogc_flat_branches() {
    let out = debug(concat!(
        "function f(): void {\n",
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
        "function f(): void {\n",
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
        "function f(): void {\n",
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
        "function f(): void {\n",
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
        "function f(): void {\n",
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
        "function f(x: number): void {\n",
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
        "function f(): void {\n",
        "    try {\n",
        "        if (x) {}\n",
        "    } catch (e) {}\n",
        "}\n",
    ));
    let cogc = function_metric(&out, "f", "cogc").unwrap();
    assert!(cogc >= 3, "try/catch with nested if should have cogc >= 3, got: {cogc}");
}

#[test]
fn cogc_ternary_counted() {
    let out = debug(concat!(
        "function f(): number {\n",
        "    return a ? 1 : 0;\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cogc"), Some(1));
}

#[test]
fn cogc_boolean_single_sequence() {
    let out = debug(concat!(
        "function f(): void {\n",
        "    if (a && b && c) {}\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cogc"), Some(2));
}

#[test]
fn cogc_boolean_mixed_sequence() {
    let out = debug(concat!(
        "function f(): void {\n",
        "    if (a && b || c) {}\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cogc"), Some(3));
}

#[test]
fn cogc_triggers_complex_method() {
    let out = check(concat!(
        "function f(): void {\n",
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
        "function f(): void {\n",
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
    assert!(cogc >= 15, "cogc should be >= 15, got: {cogc}");
    assert!(cc < 9, "cc should be < 9, got: {cc}");
    assert!(out.contains("cogc="), "detail should contain cogc=");
}

#[test]
fn cogc_below_threshold_no_smell() {
    let out = check(concat!(
        "function f(): void {\n",
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
        "function f(): void {\n",
        "    try { risky(); } catch (e) {}\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Empty Error Handler"));
}

#[test]
fn non_empty_catch_not_detected() {
    let out = check(concat!(
        "function f(): void {\n",
        "    try { risky(); } catch (e) { console.log(e); }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Empty Error Handler"));
}

#[test]
fn multiple_empty_catches() {
    let out = check(concat!(
        "function f(): void {\n",
        "    try { a(); } catch (e) {}\n",
        "    try { b(); } catch (e) {}\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Empty Error Handler"));
    assert!(out.contains("2 empty catch blocks"), "should report count=2, got: {out}");
}

#[test]
fn no_try_catch_no_smell() {
    let out = check(concat!(
        "function f(): void {\n",
        "    const x = 1;\n",
        "    return;\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Empty Error Handler"));
}

#[test]
fn catch_with_only_comment_detected() {
    let out = check(concat!(
        "function f(): void {\n",
        "    try { risky(); } catch (e) { /* todo */ }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Empty Error Handler"));
}

// ===========================================================================
// Coverage: global nesting, edge cases
// ===========================================================================

#[test]
fn ts_global_for_deep_nesting() {
    let code = "for (let i = 0; i < 10; i++) {\n  if (true) {\n    if (true) {\n      if (true) {\n      }\n    }\n  }\n}\nfunction f() {}\n";
    let out = check(code);
    assert!(has_smell(&out, "Deep Global Nesting"), "global for nesting: {out}");
}

#[test]
fn ts_untyped_params_counted() {
    let out = debug("function f(a, b, c, d, e, f) {}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(6));
}

#[test]
fn ts_ambient_class_no_crash() {
    let out = debug("declare class Foo {}\nfunction f() { return 1; }\n");
    assert!(out.contains('f'));
}
