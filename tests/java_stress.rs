mod common;

use common::*;
use std::process::Command;

fn check(code: &str) -> String { pulse_check_code(code, "java") }
fn debug(code: &str) -> String { pulse_debug_code(code, "java") }

// CC precision
#[test]
fn cc_counts_if() {
    let out = debug("class T {\n    void f() {\n        if (true) {}\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_else_if() {
    let out = debug("class T {\n    void f(int x) {\n        if (x == 1) {} else if (x == 2) {}\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_counts_for() {
    let out = debug("class T {\n    void f() {\n        for (int i = 0; i < 10; i++) {}\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_enhanced_for() {
    let out = debug("import java.util.List;\nclass T {\n    void f(List<Integer> items) {\n        for (var item : items) {}\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_while() {
    let out = debug("class T {\n    void f() {\n        while (true) {}\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_do_while() {
    let out = debug("class T {\n    void f() {\n        do {} while (true);\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_catch() {
    let out = debug("class T {\n    void f() {\n        try {} catch (Exception e) {}\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_and() {
    let out = debug("class T {\n    void f(boolean a, boolean b) {\n        if (a && b) {}\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_counts_or() {
    let out = debug("class T {\n    void f(boolean a, boolean b) {\n        if (a || b) {}\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_counts_ternary() {
    let out = debug("class T {\n    int f(boolean a) {\n        return a ? 1 : 0;\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_chained_boolean() {
    let out = debug("class T {\n    void f(boolean a, boolean b, boolean c, boolean d) {\n        if (a && b && c && d) {}\n    }\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(cc >= 4, "got: {}", cc);
}

#[test]
fn cc_nested_if_in_for() {
    let out = debug("class T {\n    void f() {\n        for (int i = 0; i < 10; i++) {\n            if (i > 5) {}\n        }\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

// Nesting
#[test]
fn nesting_0_flat() {
    let out = debug("class T {\n    int f() {\n        return 1;\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(0));
}

#[test]
fn nesting_1_single_if() {
    let out = debug("class T {\n    void f() {\n        if (true) {}\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(1));
}

#[test]
fn nesting_2_nested_if() {
    let out = debug("class T {\n    void f() {\n        if (true) {\n            if (true) {}\n        }\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(2));
}

#[test]
fn nesting_3_for_if_for() {
    let out = debug("class T {\n    void f() {\n        if (true) {\n            for (int i = 0; i < 1; i++) {\n                if (true) {}\n            }\n        }\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(3));
}

// Args
#[test]
fn args_positional() {
    let out = debug("class T {\n    void f(int a, int b, int c) {}\n}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(3));
}

#[test]
fn args_zero() {
    let out = debug("class T {\n    void f() {}\n}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(0));
}

// Primitive obsession
#[test]
fn primitive_obsession_all_primitives() {
    let out = check("class T {\n    void f(int a, long b, double c, boolean d) {}\n}\n");
    assert!(has_smell(&out, "Primitive Obsession"));
}

#[test]
fn primitive_obsession_mixed_below_threshold() {
    let out = check("class T {\n    void f(int a, String b, MyObj c, OtherObj d) {}\n}\n");
    assert!(!has_smell(&out, "Primitive Obsession"));
}

// LCOM4
#[test]
fn lcom4_three_groups_flagged() {
    let out = check(concat!(
        "class Sink {\n",
        "    private int x; private int y; private int z;\n",
        "    void useX() { this.x = 1; }\n",
        "    int getX() { return this.x; }\n",
        "    void useY() { this.y = 1; }\n",
        "    int getY() { return this.y; }\n",
        "    void useZ() { this.z = 1; }\n",
        "    int getZ() { return this.z; }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"), "got: {}", out);
}

#[test]
fn lcom4_cohesive_not_flagged() {
    let out = check(concat!(
        "import java.util.List;\n",
        "import java.util.ArrayList;\n",
        "class C {\n",
        "    private List<Integer> data = new ArrayList<>();\n",
        "    void add(int x) { this.data.add(x); }\n",
        "    List<Integer> get() { return this.data; }\n",
        "    void clear() { this.data.clear(); }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"));
}

// Duplication
#[test]
fn duplication_detected() {
    let out = check(concat!(
        "class T {\n",
        "    int rptA(int[] d) {\n",
        "        int r = 0;\n",
        "        for (int v : d) {\n",
        "            r += v;\n",
        "        }\n",
        "        r = r * 2;\n",
        "        return r;\n",
        "    }\n",
        "    int rptB(int[] d) {\n",
        "        int r = 0;\n",
        "        for (int v : d) {\n",
        "            r += v;\n",
        "        }\n",
        "        r = r * 2;\n",
        "        return r;\n",
        "    }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Code Duplication"));
}

// Constructor vs excess
#[test]
fn constructor_reports_over_injection() {
    let out = check("class S {\n    S(int a, int b, int c, int d, int e, int f) {}\n}\n");
    assert!(has_smell(&out, "Constructor Over-Injection"));
}

#[test]
fn regular_method_reports_excess_args() {
    let out = check("class T {\n    void f(int a, int b, int c, int d, int e, int f, int g) {}\n}\n");
    assert!(has_smell(&out, "Excess Arguments"));
    assert!(!has_smell(&out, "Constructor Over-Injection"));
}

// Multiple smells
#[test]
fn multiple_smells_same_function() {
    let mut code = String::from("class T {\n    void bad(int a, int b, int c, int d, int e, int f, int g, int h) {\n");
    code.push_str("        for (int i = 0; i < a; i++) {\n");
    code.push_str("            if (i > 0) {\n");
    code.push_str("                for (int j = 0; j < b; j++) {\n");
    code.push_str("                    if (j > 0) {\n");
    code.push_str("                        for (int k = 0; k < c; k++) {\n");
    code.push_str("                            if (k > 0) {}\n");
    code.push_str("                        }\n");
    code.push_str("                    }\n");
    code.push_str("                }\n");
    code.push_str("            }\n");
    code.push_str("        }\n");
    code.push_str("    }\n}\n");
    let out = check(&code);
    assert!(has_smell(&out, "Excess Arguments"));
    assert!(has_smell(&out, "Deep Nested"));
}

// Performance
#[test]
fn performance_1000_loc() {
    let mut code = String::from("class Big {\n");
    for i in 0..50 {
        code.push_str(&format!("    int func{}(int data) {{\n", i));
        for j in 0..18 {
            code.push_str(&format!("        int f{} = data + {};\n", j, j));
        }
        code.push_str("        return data;\n    }\n\n");
    }
    code.push_str("}\n");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Big.java");
    std::fs::write(&path, &code).unwrap();
    let start = std::time::Instant::now();
    let _ = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let elapsed = start.elapsed();
    assert!(elapsed.as_millis() < 500, "took: {}ms", elapsed.as_millis());
}
