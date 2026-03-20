mod common;

use common::*;
use std::process::Command;

fn check(code: &str) -> String { pulse_check_code(code, "cpp") }
fn debug(code: &str) -> String { pulse_debug_code(code, "cpp") }

// CC precision
#[test]
fn cc_counts_if() {
    let out = debug("void f() {\n    if (x) {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_else_if() {
    let out = debug("void f(int x) {\n    if (x == 1) {} else if (x == 2) {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_counts_for() {
    let out = debug("void f() {\n    for (int i = 0; i < 10; i++) {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_range_for() {
    let out = debug("#include <vector>\nvoid f() {\n    std::vector<int> v;\n    for (auto x : v) {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_while() {
    let out = debug("void f() {\n    while (x) {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_do_while() {
    let out = debug("void f() {\n    do {} while (x);\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_catch() {
    let out = debug("void f() {\n    try {} catch (...) {}\n}\n");
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
fn cc_chained_boolean() {
    let out = debug("void f(int a, int b, int c, int d) {\n    if (a && b && c && d) {}\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(cc >= 4, "got: {}", cc);
}

#[test]
fn cc_nested_if_in_for() {
    let out = debug("void f() {\n    for (int i = 0; i < 10; i++) {\n        if (i > 5) {}\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

// Nesting precision
#[test]
fn nesting_0_flat() {
    let out = debug("int f() {\n    return 1;\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(0));
}

#[test]
fn nesting_1_single_if() {
    let out = debug("void f() {\n    if (x) {}\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(1));
}

#[test]
fn nesting_2_nested_if() {
    let out = debug("void f() {\n    if (x) {\n        if (y) {}\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(2));
}

#[test]
fn nesting_3_for_if_for() {
    let out = debug("void f() {\n    if (x) {\n        for (int i = 0; i < n; i++) {\n            if (z) {}\n        }\n    }\n}\n");
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
    let out = debug("void f() {}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(0));
}

#[test]
fn args_reference_params() {
    let out = debug("#include <string>\nvoid f(const std::string& a, int& b) {}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(2));
}

// LCOM4
#[test]
fn lcom4_cohesive_not_flagged() {
    let out = check(concat!(
        "#include <vector>\n",
        "class C {\n",
        "public:\n",
        "    void add(int x) { this->data_.push_back(x); }\n",
        "    int get() { return this->data_.size(); }\n",
        "    void clear() { this->data_.clear(); }\n",
        "private:\n",
        "    std::vector<int> data_;\n",
        "};\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_transitive_connection() {
    let out = check(concat!(
        "class C {\n",
        "public:\n",
        "    int m1() { return this->a_; }\n",
        "    int m2() { return this->a_ + this->b_; }\n",
        "    int m3() { return this->b_; }\n",
        "private:\n",
        "    int a_; int b_;\n",
        "};\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"));
}

// Duplication
#[test]
fn duplication_detected() {
    let out = check(concat!(
        "int rpt_a(int* d, int n) {\n    int r = 0;\n    for (int i = 0; i < n; i++) {\n        r += d[i];\n    }\n    return r;\n}\n\n",
        "int rpt_b(int* d, int n) {\n    int r = 0;\n    for (int i = 0; i < n; i++) {\n        r += d[i];\n    }\n    return r;\n}\n",
    ));
    assert!(has_smell(&out, "Code Duplication"));
}

// Declarations
#[test]
fn declarations_above_threshold() {
    let mut code = String::new();
    for i in 0..25 {
        code.push_str(&format!("struct T{} {{}};\n", i));
    }
    let out = check(&code);
    assert!(has_smell(&out, "Declarations"));
}

// Constructor vs excess
#[test]
fn regular_function_reports_excess_args() {
    let out = check("void f(int a, int b, int c, int d, int e, int f, int g) {}\n");
    assert!(has_smell(&out, "Excess Arguments"));
    assert!(!has_smell(&out, "Constructor Over-Injection"));
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
    let path = dir.path().join("big.cpp");
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
    let mut code = String::from("#include <vector>\n");
    for i in 0..10 {
        code.push_str(&format!("class S{} {{\npublic:\n", i));
        for j in 0..5 {
            code.push_str(&format!("    int m{}() {{ return this->d_; }}\n", j));
        }
        code.push_str("private:\n    int d_;\n};\n\n");
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("classes.cpp");
    std::fs::write(&path, &code).unwrap();
    let start = std::time::Instant::now();
    let _ = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let elapsed = start.elapsed();
    assert!(elapsed.as_millis() < 500, "took: {}ms", elapsed.as_millis());
}
