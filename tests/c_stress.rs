mod common;

use common::*;
use std::process::Command;

fn check(code: &str) -> String { pulse_check_code(code, "c") }
fn debug(code: &str) -> String { pulse_debug_code(code, "c") }

// CC precision
#[test]
fn cc_counts_if() {
    let out = debug("void f(void) {\n    if (x) {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_else_if() {
    let out = debug("void f(int x) {\n    if (x == 1) {} else if (x == 2) {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_counts_for() {
    let out = debug("void f(void) {\n    for (int i = 0; i < 10; i++) {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_while() {
    let out = debug("void f(void) {\n    while (x) {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_do_while() {
    let out = debug("void f(void) {\n    do {} while (x);\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_switch_cases() {
    let out = debug("void f(int x) {\n    switch (x) {\n        case 1: break;\n        case 2: break;\n        case 3: break;\n        default: break;\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(4));
}

#[test]
fn cc_counts_and() {
    let out = debug("void f(int a, int b) {\n    if (a && b) {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_counts_or() {
    let out = debug("void f(int a, int b) {\n    if (a || b) {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_counts_ternary() {
    let out = debug("int f(int a) {\n    return a ? 1 : 0;\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_nested_if_in_for() {
    let out = debug("void f(void) {\n    for (int i = 0; i < 10; i++) {\n        if (i > 5) {}\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_chained_boolean() {
    let out = debug("void f(int a, int b, int c, int d) {\n    if (a && b && c && d) {}\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(cc >= 4, "got: {}", cc);
}

// Nesting precision
#[test]
fn nesting_0_flat() {
    let out = debug("int f(void) {\n    return 1;\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(0));
}

#[test]
fn nesting_1_single_if() {
    let out = debug("void f(void) {\n    if (x) {}\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(1));
}

#[test]
fn nesting_2_nested_if() {
    let out = debug("void f(void) {\n    if (x) {\n        if (y) {}\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(2));
}

#[test]
fn nesting_3_for_if_for() {
    let out = debug("void f(void) {\n    if (x) {\n        for (int i = 0; i < n; i++) {\n            if (z) {}\n        }\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(3));
}

// Arg precision
#[test]
fn args_positional() {
    let out = debug("void f(int a, int b, int c) {}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(3));
}

#[test]
fn args_zero() {
    let out = debug("void f(void) {}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(0));
}

#[test]
fn args_pointer_params() {
    let out = debug("void f(int* a, char* b) {}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(2));
}

// Duplication
#[test]
fn duplication_detected() {
    let out = check(concat!(
        "void rpt_a(int* d, int n) {\n    int r = 0;\n    for (int i = 0; i < n; i++) {\n        r += d[i];\n    }\n    printf(\"%d\", r);\n}\n\n",
        "void rpt_b(int* d, int n) {\n    int r = 0;\n    for (int i = 0; i < n; i++) {\n        r += d[i];\n    }\n    printf(\"%d\", r);\n}\n",
    ));
    assert!(has_smell(&out, "Code Duplication"));
}

// Multiple smells
#[test]
fn multiple_smells_same_function() {
    let mut code = String::from("void bad(int a, int b, int c, int d, int e, int f, int g, int h) {\n");
    code.push_str("    for (int i = 0; i < a; i++) {\n");
    code.push_str("        if (i > 0) {\n");
    code.push_str("            for (int j = 0; j < b; j++) {\n");
    code.push_str("                if (j > 0) {\n");
    code.push_str("                    for (int k = 0; k < c; k++) {\n");
    code.push_str("                        if (k > 0) {}\n");
    code.push_str("                    }\n");
    code.push_str("                }\n");
    code.push_str("            }\n");
    code.push_str("        }\n");
    code.push_str("    }\n");
    code.push_str("}\n");
    let out = check(&code);
    assert!(has_smell(&out, "Excess Arguments"));
    assert!(has_smell(&out, "Deep Nested"));
}

// Performance
#[test]
fn performance_1000_loc() {
    let mut code = String::new();
    for i in 0..50 {
        code.push_str(&format!("int func{}(int data) {{\n", i));
        for j in 0..18 {
            code.push_str(&format!("    int f{} = data + {};\n", j, j));
        }
        code.push_str("    return data;\n}\n\n");
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("big.c");
    std::fs::write(&path, &code).unwrap();
    let start = std::time::Instant::now();
    let _ = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let elapsed = start.elapsed();
    assert!(elapsed.as_millis() < 500, "took: {}ms", elapsed.as_millis());
}
