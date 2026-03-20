mod common;

use common::*;
use std::process::Command;

fn check(code: &str) -> String { pulse_check_code(code, "rs") }
fn debug(code: &str) -> String { pulse_debug_code(code, "rs") }

// ===========================================================================
// CC counting precision
// ===========================================================================

#[test]
fn cc_counts_if() {
    let out = debug("fn f() {\n    if true {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_else_if() {
    let out = debug("fn f(x: i32) {\n    if x == 1 {} else if x == 2 {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_counts_for() {
    let out = debug("fn f() {\n    for x in 0..10 {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_while() {
    let out = debug("fn f() {\n    while true {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_loop() {
    let out = debug("fn f() {\n    loop { break; }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_match_arms() {
    let out = debug("fn f(x: i32) {\n    match x {\n        1 => {},\n        2 => {},\n        3 => {},\n        _ => {},\n    }\n}\n");
    // base(1) + 3 non-wildcard arms = 4
    assert_eq!(function_metric(&out, "f", "cc"), Some(4));
}

#[test]
fn cc_counts_and_operator() {
    let out = debug("fn f(a: bool, b: bool) {\n    if a && b {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_counts_or_operator() {
    let out = debug("fn f(a: bool, b: bool) {\n    if a || b {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_nested_if_in_for() {
    let out = debug("fn f() {\n    for x in 0..10 {\n        if x > 5 {}\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_chained_boolean() {
    let out = debug("fn f(a: bool, b: bool, c: bool, d: bool) {\n    if a && b && c && d {}\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(cc >= 4, "chained boolean should increase cc, got: {}", cc);
}

// ===========================================================================
// Nesting depth precision
// ===========================================================================

#[test]
fn nesting_0_flat() {
    let out = debug("fn f() -> i32 {\n    42\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(0));
}

#[test]
fn nesting_1_single_if() {
    let out = debug("fn f() {\n    if true {}\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(1));
}

#[test]
fn nesting_2_nested_if() {
    let out = debug("fn f() {\n    if true {\n        if true {}\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(2));
}

#[test]
fn nesting_3_for_if_for() {
    let out = debug("fn f() {\n    if true {\n        for x in 0..1 {\n            if true {}\n        }\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(3));
}

// ===========================================================================
// Argument counting
// ===========================================================================

#[test]
fn args_counts_positional() {
    let out = debug("fn f(a: i32, b: i32, c: i32) {}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(3));
}

#[test]
fn args_zero() {
    let out = debug("fn f() {}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(0));
}

#[test]
fn args_self_excluded_in_method() {
    let out = debug("struct S;\nimpl S {\n    fn m(&self, a: i32, b: i32) {}\n}\n");
    assert_eq!(function_metric(&out, "S.m", "args"), Some(2));
}

#[test]
fn args_mut_self_excluded() {
    let out = debug("struct S;\nimpl S {\n    fn m(&mut self, a: i32) {}\n}\n");
    assert_eq!(function_metric(&out, "S.m", "args"), Some(1));
}

// ===========================================================================
// Primitive obsession
// ===========================================================================

#[test]
fn primitive_obsession_all_primitives() {
    let out = check("fn f(a: i32, b: u64, c: f32, d: bool) {}\n");
    assert!(has_smell(&out, "Primitive Obsession"));
}

#[test]
fn primitive_obsession_mixed_below_threshold() {
    let out = check("fn f(a: i32, b: MyStruct, c: OtherType, d: Vec<u8>) {}\n");
    assert!(!has_smell(&out, "Primitive Obsession"));
}

#[test]
fn primitive_obsession_reference_types() {
    let out = check("fn f(a: &str, b: &str, c: &str, d: &str) {}\n");
    assert!(has_smell(&out, "Primitive Obsession"));
}

// ===========================================================================
// LCOM4
// ===========================================================================

#[test]
fn lcom4_cohesive_struct_not_flagged() {
    let out = check("struct S { data: Vec<i32> }\nimpl S {\n    fn add(&mut self, x: i32) { self.data.push(x); }\n    fn get(&self) -> &[i32] { &self.data }\n    fn clear(&mut self) { self.data.clear(); }\n}\n");
    assert!(!has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_three_disconnected_groups() {
    let out = check(concat!(
        "struct M { x: i32, y: i32, z: i32 }\n",
        "impl M {\n",
        "    fn use_x(&self) -> i32 { self.x }\n",
        "    fn use_y(&self) -> i32 { self.y }\n",
        "    fn use_z(&self) -> i32 { self.z }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_transitive_connection() {
    let out = check(concat!(
        "struct C { a: i32, b: i32 }\n",
        "impl C {\n",
        "    fn m1(&self) -> i32 { self.a }\n",
        "    fn m2(&self) -> i32 { self.a + self.b }\n",
        "    fn m3(&self) -> i32 { self.b }\n",
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
        "fn report_a(data: &[Item]) -> Vec<Entry> {\n",
        "    let mut r = Vec::new();\n",
        "    for item in data {\n",
        "        let e = Entry { id: item.id, name: item.name.clone(), val: item.val };\n",
        "        r.push(e);\n",
        "    }\n",
        "    r\n",
        "}\n\n",
        "fn report_b(data: &[Item]) -> Vec<Entry> {\n",
        "    let mut r = Vec::new();\n",
        "    for item in data {\n",
        "        let e = Entry { id: item.id, name: item.name.clone(), val: item.val };\n",
        "        r.push(e);\n",
        "    }\n",
        "    r\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Code Duplication"));
}

#[test]
fn duplication_test_functions_suppressed() {
    let out = check(concat!(
        "fn test_a() {\n    let r = compute();\n    assert_eq!(r.id, 1);\n    assert_eq!(r.name, \"a\");\n    assert_eq!(r.val, 10);\n    assert!(r.ok);\n}\n\n",
        "fn test_b() {\n    let r = compute();\n    assert_eq!(r.id, 1);\n    assert_eq!(r.name, \"a\");\n    assert_eq!(r.val, 10);\n    assert!(r.ok);\n}\n",
    ));
    assert!(!has_smell(&out, "Code Duplication"));
}

// ===========================================================================
// Declarations
// ===========================================================================

#[test]
fn declarations_above_threshold() {
    let mut code = String::new();
    for i in 0..25 {
        code.push_str(&format!("struct T{} {{}}\n", i));
    }
    let out = check(&code);
    assert!(has_smell(&out, "Declarations"));
}

// ===========================================================================
// God Class / God Method
// ===========================================================================

#[test]
fn god_method_detected() {
    let mut code = String::from("fn monster() {\n");
    for i in 0..10 {
        code.push_str(&format!("    if {} > 0 {{}}\n", i));
    }
    for i in 0..40 {
        code.push_str(&format!("    let x{} = {};\n", i, i));
    }
    code.push_str("}\n");
    let out = check(&code);
    assert!(has_smell(&out, "God Method"));
}

// ===========================================================================
// Overall function size
// ===========================================================================

#[test]
fn overall_function_size_triggered() {
    let mut code = String::new();
    for i in 0..3 {
        code.push_str(&format!("fn lg{}() {{\n", i));
        for j in 0..45 {
            code.push_str(&format!("    let x{} = {};\n", j, j));
        }
        code.push_str("}\n\n");
    }
    let out = check(&code);
    assert!(has_smell(&out, "Overall Function Size"));
}

// ===========================================================================
// Deep nesting
// ===========================================================================

#[test]
fn deep_nesting_detected() {
    let out = check("fn deep() {\n    for x in 0..1 {\n        if true {\n            for y in 0..1 {\n                if true {\n                    for z in 0..1 {\n                        if true {}\n                    }\n                }\n            }\n        }\n    }\n}\n");
    assert!(has_smell(&out, "Deep Nested"));
}

// ===========================================================================
// Constructor vs excess args
// ===========================================================================

#[test]
fn constructor_reports_over_injection() {
    let out = check("struct S {}\nimpl S {\n    fn new(a: i32, b: i32, c: i32, d: i32, e: i32, f: i32) -> Self { S {} }\n}\n");
    assert!(has_smell(&out, "Constructor Over-Injection"));
}

#[test]
fn regular_function_reports_excess_args() {
    let out = check("fn f(a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32) {}\n");
    assert!(has_smell(&out, "Excess Arguments"));
    assert!(!has_smell(&out, "Constructor Over-Injection"));
}

// ===========================================================================
// Embedded block
// ===========================================================================

#[test]
fn embedded_block_detected() {
    let mut code = String::from("fn query() -> &'static str {\n    let q = r#\"\n");
    for i in 0..20 {
        code.push_str(&format!("        SELECT field_{} FROM table_{}\n", i, i));
    }
    code.push_str("    \"#;\n    q\n}\n");
    let out = check(&code);
    assert!(has_smell(&out, "Large Embedded Block"));
}

// ===========================================================================
// Global nesting
// ===========================================================================

#[test]
fn global_nesting_not_common_in_rust() {
    // Rust rarely has global conditionals — this is correct behavior
    let out = check("const X: i32 = 42;\nfn main() {}\n");
    assert!(!has_smell(&out, "Global Conditionals"));
    assert!(!has_smell(&out, "Deep Global Nesting"));
}

// ===========================================================================
// Performance
// ===========================================================================

#[test]
fn performance_1000_loc() {
    let mut code = String::new();
    for i in 0..50 {
        code.push_str(&format!("fn func{}(data: &Data) -> Result<(), Error> {{\n", i));
        for j in 0..18 {
            code.push_str(&format!("    let f{} = data.field{};\n", j, j));
        }
        code.push_str("    Ok(())\n}\n\n");
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("big.rs");
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
fn performance_impl_blocks() {
    let mut code = String::new();
    for i in 0..10 {
        code.push_str(&format!("struct S{} {{ data: Vec<i32> }}\nimpl S{} {{\n", i, i));
        for j in 0..5 {
            code.push_str(&format!("    fn m{}(&self) -> &[i32] {{ &self.data }}\n", j));
        }
        code.push_str("}\n\n");
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("impls.rs");
    std::fs::write(&path, &code).unwrap();
    let start = std::time::Instant::now();
    let _ = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let elapsed = start.elapsed();
    assert!(elapsed.as_millis() < 500, "took: {}ms", elapsed.as_millis());
}
