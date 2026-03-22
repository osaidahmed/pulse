mod common;

use common::*;
use std::process::Command;

fn check(code: &str) -> String {
    pulse_check_code(code, "java")
}
fn debug(code: &str) -> String {
    pulse_debug_code(code, "java")
}

// CC precision
#[test]
fn cc_counts_if() {
    let out = debug("class T {\n    void f() {\n        if (true) {}\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_else_if() {
    let out = debug(
        "class T {\n    void f(int x) {\n        if (x == 1) {} else if (x == 2) {}\n    }\n}\n",
    );
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_counts_for() {
    let out =
        debug("class T {\n    void f() {\n        for (int i = 0; i < 10; i++) {}\n    }\n}\n");
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
    let out =
        debug("class T {\n    void f(boolean a, boolean b) {\n        if (a && b) {}\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_counts_or() {
    let out =
        debug("class T {\n    void f(boolean a, boolean b) {\n        if (a || b) {}\n    }\n}\n");
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
    let out =
        check("class T {\n    void f(int a, int b, int c, int d, int e, int f, int g) {}\n}\n");
    assert!(has_smell(&out, "Excess Arguments"));
    assert!(!has_smell(&out, "Constructor Over-Injection"));
}

// Multiple smells
#[test]
fn multiple_smells_same_function() {
    let mut code = String::from(
        "class T {\n    void bad(int a, int b, int c, int d, int e, int f, int g, int h) {\n",
    );
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

// ===========================================================================
// CC: not/! operator
// ===========================================================================

#[test]
fn cc_counts_not_operator() {
    let out = debug("class T {\n    void f(boolean a) {\n        if (!a) {}\n    }\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(cc >= 2, "if(!a) should have cc >= 2, got: {}", cc);
}

// ===========================================================================
// CC: switch many cases
// ===========================================================================

#[test]
fn cc_switch_many_cases() {
    let out = debug(concat!(
        "class T {\n",
        "    String f(int x) {\n",
        "        switch (x) {\n",
        "            case 1: return \"a\";\n",
        "            case 2: return \"b\";\n",
        "            case 3: return \"c\";\n",
        "            case 4: return \"d\";\n",
        "            case 5: return \"e\";\n",
        "            case 6: return \"f\";\n",
        "            case 7: return \"g\";\n",
        "            case 8: return \"h\";\n",
        "            default: return \"?\";\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(cc >= 9, "8 cases + base >= 9, got: {}", cc);
}

// ===========================================================================
// CC: multiple catch
// ===========================================================================

#[test]
fn cc_multiple_catch() {
    let out = debug("class T {\n    void f() {\n        try {} catch (RuntimeException e) {} catch (Exception e) {} catch (Throwable e) {}\n    }\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(cc >= 4, "base + 3 catch = 4, got: {}", cc);
}

// ===========================================================================
// Nesting: try-catch depth
// ===========================================================================

#[test]
fn nesting_try_catch_counts_depth() {
    let out = debug("class T {\n    void f() {\n        try {\n            if (true) {}\n        } catch (Exception e) {}\n    }\n}\n");
    let depth = function_metric(&out, "f", "nesting").unwrap_or(0);
    assert!(
        depth >= 1,
        "try-catch should contribute nesting, got: {}",
        depth
    );
}

// ===========================================================================
// Args: typed with defaults (N/A in Java, but typed params)
// ===========================================================================

#[test]
fn args_typed_params() {
    let out = debug("class T {\n    void f(String a, int b, boolean c) {}\n}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(3));
}

// ===========================================================================
// Primitive obsession: below min typed
// ===========================================================================

#[test]
fn primitive_obsession_below_min_typed() {
    let out = check("class T {\n    void f(int a, int b, int c) {}\n}\n");
    assert!(!has_smell(&out, "Primitive Obsession"));
}

#[test]
fn primitive_obsession_recognizes_long_float() {
    let out = check("class T {\n    void f(long a, float b, double c, int d) {}\n}\n");
    assert!(has_smell(&out, "Primitive Obsession"));
}

// ===========================================================================
// LCOM4: single method not flagged
// ===========================================================================

#[test]
fn lcom4_single_method_not_flagged() {
    let out = check("class T {\n    private int x;\n    int get() { return this.x; }\n}\n");
    assert!(!has_smell(&out, "Low Cohesion"));
}

// ===========================================================================
// LCOM4: constructor excluded
// ===========================================================================

#[test]
fn lcom4_constructor_excluded() {
    let out = check(concat!(
        "class Init {\n",
        "    private int a; private int b; private int c;\n",
        "    Init() { this.a = 1; this.b = 2; this.c = 3; }\n",
        "    int useA() { return this.a; }\n",
        "    int useB() { return this.b; }\n",
        "    int useC() { return this.c; }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"));
}

// ===========================================================================
// LCOM4: 2 groups not flagged
// ===========================================================================

#[test]
fn lcom4_two_groups_not_flagged() {
    let out = check(concat!(
        "class Split {\n",
        "    private int a; private int b;\n",
        "    int aWork() { return this.a; }\n",
        "    int aRead() { return this.a + 1; }\n",
        "    int bWork() { return this.b; }\n",
        "    int bRead() { return this.b + 1; }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"));
}

// ===========================================================================
// LCOM4: transitive connection
// ===========================================================================

#[test]
fn lcom4_transitive_connection() {
    let out = check(concat!(
        "class C {\n",
        "    private int a; private int b;\n",
        "    int m1() { return this.a; }\n",
        "    int m2() { return this.a + this.b; }\n",
        "    int m3() { return this.b; }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"));
}

// ===========================================================================
// Duplication: test suppressed
// ===========================================================================

#[test]
fn duplication_test_suppressed() {
    let out = check(concat!(
        "class T {\n",
        "    void test_a() {\n        int r = compute();\n        assert(r == 1);\n        assert(r != 0);\n        assert(r < 10);\n        assert(r > 0);\n    }\n",
        "    void test_b() {\n        int r = compute();\n        assert(r == 1);\n        assert(r != 0);\n        assert(r < 10);\n        assert(r > 0);\n    }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Code Duplication"));
}

// ===========================================================================
// Duplication: mixed test+prod flagged
// ===========================================================================

#[test]
fn duplication_mixed_test_and_prod_flagged() {
    let out = check(concat!(
        "class T {\n",
        "    void test_compute() {\n        int r = 0;\n        for (int i = 0; i < 10; i++) { r += i; }\n        r = r * 2;\n        System.out.println(r);\n    }\n",
        "    void processData() {\n        int r = 0;\n        for (int i = 0; i < 10; i++) { r += i; }\n        r = r * 2;\n        System.out.println(r);\n    }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Code Duplication"));
}

// ===========================================================================
// God class requires god method
// ===========================================================================

#[test]
fn god_class_requires_god_method() {
    let mut code = String::from("class Big {\n");
    for i in 0..25 {
        code.push_str(&format!("    int fn{}() {{ return {}; }}\n", i, i));
    }
    code.push_str("}\n");
    for i in 0..200 {
        code.push_str(&format!("// padding {}\n", i));
    }
    let out = check(&code);
    assert!(!has_smell(&out, "God Class"));
}

// ===========================================================================
// God class triggers
// ===========================================================================

#[test]
fn god_class_triggers_with_god_method() {
    let mut code = String::from("class Monster {\n    void monster() {\n");
    for i in 0..10 {
        code.push_str(&format!("        if ({} > 0) {{}}\n", i));
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("        int y{} = {};\n", i, i));
    }
    code.push_str("    }\n");
    for i in 0..21 {
        code.push_str(&format!("    int fn{}() {{ return {}; }}\n", i, i));
    }
    for i in 0..file_padding() {
        code.push_str(&format!("    static final int V{} = {};\n", i, i));
    }
    code.push_str("}\n");
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
        "class T {\n",
        "    void testInter() {\n",
        "        assert(true);\n",
        "        assert(true);\n",
        "        assert(true);\n",
        "        doSomething();\n",
        "        assert(true);\n",
        "    }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Large Assertion Block"));
}

#[test]
fn assertion_block_at_threshold() {
    let mut code = String::from("class T {\n    void testExact() {\n");
    for i in 0..10 {
        code.push_str(&format!("        assert(x{} == {});\n", i, i));
    }
    code.push_str("    }\n}\n");
    let out = check(&code);
    assert!(!has_smell(&out, "Large Assertion Block"));
}

#[test]
fn assertion_block_above_threshold() {
    let mut code = String::from("class T {\n    void testBig() {\n");
    for i in 0..15 {
        code.push_str(&format!("        assertEquals(x{}, {});\n", i, i));
    }
    code.push_str("    }\n}\n");
    let out = check(&code);
    assert!(has_smell(&out, "Large Assertion Block"));
}

// ===========================================================================
// Overall function size
// ===========================================================================

#[test]
fn overall_function_size_below_threshold() {
    let mut code = String::from("class T {\n");
    for i in 0..2 {
        code.push_str(&format!("    void lg{}() {{\n", i));
        for j in 0..55 {
            code.push_str(&format!("        int x{} = {};\n", j, j));
        }
        code.push_str("    }\n");
    }
    code.push_str("}\n");
    let out = check(&code);
    assert!(!has_smell(&out, "Overall Function Size"));
}

#[test]
fn overall_function_size_at_threshold() {
    let mut code = String::from("class T {\n");
    for i in 0..3 {
        code.push_str(&format!("    void lg{}() {{\n", i));
        for j in 0..55 {
            code.push_str(&format!("        int x{} = {};\n", j, j));
        }
        code.push_str("    }\n");
    }
    code.push_str("}\n");
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
// Embedded: small not flagged, multiline flagged
// ===========================================================================

#[test]
fn small_string_not_flagged() {
    let out = check("class T {\n    String f() {\n        return \"hello world\";\n    }\n}\n");
    assert!(!has_smell(&out, "Large Embedded Block"));
}

#[test]
fn multiline_string_flagged() {
    let mut code = String::from("class T {\n    String query() {\n        String q = \"\"\"\n");
    for i in 0..20 {
        code.push_str(&format!(
            "            SELECT field_{} FROM table_{}\n",
            i, i
        ));
    }
    code.push_str("            \"\"\";\n        return q;\n    }\n}\n");
    let out = check(&code);
    assert!(has_smell(&out, "Large Embedded Block"));
}

// ===========================================================================
// Deep global nesting
// ===========================================================================

#[test]
fn shallow_global_not_flagged() {
    let out = check("class T {\n    void f() {}\n}\n");
    assert!(!has_smell(&out, "Deep Global Nesting"));
}

// ===========================================================================
// Constructor vs excess
// ===========================================================================

#[test]
fn constructor_reports_injection_not_excess() {
    let out = check("class S {\n    S(int a, int b, int c, int d, int e, int f) {}\n}\n");
    assert!(has_smell(&out, "Constructor Over-Injection"));
    let lines: Vec<&str> = out
        .lines()
        .filter(|l| l.contains("S(") || l.contains(".S"))
        .collect();
    assert!(!lines.iter().any(|l| l.contains("Excess Arguments")));
}

// ===========================================================================
// Multiple smells
// ===========================================================================

#[test]
fn function_can_have_multiple_smells() {
    let mut code = String::from(
        "class T {\n    void bad(int a, int b, int c, int d, int e, int f, int g, int h) {\n",
    );
    code.push_str("        String q = \"\"\"\n");
    for i in 0..20 {
        code.push_str(&format!(
            "            SELECT field_{} FROM table_{}\n",
            i, i
        ));
    }
    code.push_str("            \"\"\";\n");
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
    assert!(has_smell(&out, "Large Embedded Block"));
    assert!(has_smell(&out, "Deep Nested"));
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
// Real-world: clean Java
// ===========================================================================

#[test]
fn clean_java_module_not_flagged() {
    let out = check(concat!(
        "class Config {\n",
        "    private String host;\n",
        "    private int port;\n",
        "    Config(String host, int port) {\n",
        "        this.host = host;\n",
        "        this.port = port;\n",
        "    }\n",
        "    String address() {\n",
        "        return this.host + \":\" + this.port;\n",
        "    }\n",
        "}\n",
    ));
    assert!(
        out.is_empty(),
        "clean Java code should not be flagged, got: {}",
        out
    );
}

// ===========================================================================
// Comments only
// ===========================================================================

#[test]
fn comments_only() {
    let out = check("// just comments\n// only\n");
    assert!(out.is_empty());
}

// ===========================================================================
// Empty file
// ===========================================================================

#[test]
fn empty_file() {
    let out = check("");
    assert!(out.is_empty());
}

// ===========================================================================
// Performance: class hierarchy
// ===========================================================================

#[test]
fn performance_class_hierarchy() {
    let mut code = String::new();
    for i in 0..10 {
        code.push_str(&format!("class S{} {{\n    private int d{};\n", i, i));
        for j in 0..5 {
            code.push_str(&format!("    int m{}() {{ return this.d{}; }}\n", j, i));
        }
        code.push_str("}\n\n");
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Classes.java");
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
// Nested conditional chunks
// ===========================================================================

#[test]
fn nested_conditional_chunks_detected() {
    let out = check(concat!(
        "class T {\n",
        "    void validate(int[] data) {\n",
        "        if (data.length > 0) {\n",
        "            if (data[0] > 0) {\n",
        "                if (data[0] > 10) {\n",
        "                    int x = 1;\n",
        "                }\n",
        "            }\n",
        "        }\n",
        "        int gap = 1;\n",
        "        if (data.length > 5) {\n",
        "            if (data[5] > 0) {\n",
        "                if (data[5] > 10) {\n",
        "                    int y = 2;\n",
        "                }\n",
        "            }\n",
        "        }\n",
        "        int gap2 = 2;\n",
        "        if (data.length > 10) {\n",
        "            if (data[10] > 0) {\n",
        "                if (data[10] > 10) {\n",
        "                    int z = 3;\n",
        "                }\n",
        "            }\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    assert!(
        has_smell(&out, "Nested Conditional Chunks") || has_smell(&out, "Complex Method"),
        "got: {}",
        out
    );
}

// ===========================================================================
// CC: else-if chain
// ===========================================================================

#[test]
fn cc_else_if_chain() {
    let out = debug("class T {\n    void f(int x) {\n        if (x == 1) {} else if (x == 2) {} else if (x == 3) {}\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(4));
}

// ===========================================================================
// Nesting: deep for-if-for
// ===========================================================================

#[test]
fn nesting_deep_for_if_for() {
    let out = debug("class T {\n    void f() {\n        if (true) {\n            for (int i = 0; i < 1; i++) {\n                if (true) {\n                    for (int j = 0; j < 1; j++) {}\n                }\n            }\n        }\n    }\n}\n");
    let depth = function_metric(&out, "f", "nesting").unwrap_or(0);
    assert!(depth >= 4, "got: {}", depth);
}

// ===========================================================================
// Output: pulse prefix
// ===========================================================================

#[test]
fn output_starts_with_pulse() {
    let out =
        check("class T {\n    void f(int a, int b, int c, int d, int e, int f, int g) {}\n}\n");
    assert!(out.starts_with("pulse:"));
}

// ===========================================================================
// Output: line numbers
// ===========================================================================

#[test]
fn output_has_line_numbers() {
    let out =
        check("class T {\n    void f(int a, int b, int c, int d, int e, int f, int g) {}\n}\n");
    let has_loc = out.lines().any(|l| l.contains("(L") && l.contains("): "));
    assert!(has_loc);
}

// ===========================================================================
// Excess args count
// ===========================================================================

#[test]
fn excess_args_count_verified() {
    let out = debug(
        "class T {\n    void f(int a, int b, int c, int d, int e, int f, int g, int h) {}\n}\n",
    );
    assert_eq!(function_metric(&out, "f", "args"), Some(8));
}

// ===========================================================================
// Hook: unsupported extension
// ===========================================================================

#[test]
fn hook_unsupported_extension() {
    let output = run_hook("/some/file.xyz");
    assert!(output.is_empty());
}

// ===========================================================================
// Deep nesting with switch
// ===========================================================================

#[test]
fn deep_nesting_with_switch() {
    let out = check(concat!(
        "class T {\n",
        "    void deep(int x) {\n",
        "        for (int i = 0; i < x; i++) {\n",
        "            if (i > 0) {\n",
        "                switch (i) {\n",
        "                    case 1:\n",
        "                        for (int j = 0; j < i; j++) {\n",
        "                            if (j > 0) {}\n",
        "                        }\n",
        "                        break;\n",
        "                }\n",
        "            }\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Deep Nested"));
}

// ===========================================================================
// CC: ternary
// ===========================================================================

#[test]
fn cc_ternary_inline() {
    let out = debug("class T {\n    int f(boolean a) {\n        return a ? 1 : 0;\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

// ===========================================================================
// LCOM4: all methods share field
// ===========================================================================

#[test]
fn lcom4_all_methods_share_field() {
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

// ===========================================================================
// Duplication: two functions is minimum
// ===========================================================================

#[test]
fn duplication_two_is_minimum() {
    let out = check(concat!(
        "class T {\n",
        "    int rptA(int[] d) {\n        int r = 0;\n        for (int v : d) { r += v; }\n        r = r * 2;\n        return r;\n    }\n",
        "    int rptB(int[] d) {\n        int r = 0;\n        for (int v : d) { r += v; }\n        r = r * 2;\n        return r;\n    }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Code Duplication"));
}

// ===========================================================================
// Module prefix
// ===========================================================================

#[test]
fn output_has_module_prefix() {
    let mut code = String::from("class Big {\n");
    for i in 0..25 {
        code.push_str(&format!("    int fn{}() {{ return {}; }}\n", i, i));
    }
    code.push_str("}\n");
    for i in 0..200 {
        code.push_str(&format!("// line {}\n", i));
    }
    let out = check(&code);
    assert!(out.contains("Module:"));
}

// ===========================================================================
// Issue count matches
// ===========================================================================

#[test]
fn issue_count_matches() {
    let out =
        check("class T {\n    void f(int a, int b, int c, int d, int e, int f, int g) {}\n}\n");
    let first = out.lines().next().unwrap_or("");
    let findings = out.lines().filter(|l| l.starts_with("  ")).count();
    assert!(first.contains(&format!("{} issue", findings)));
}

// ===========================================================================
// CC: do-while
// ===========================================================================

#[test]
fn cc_do_while() {
    let out = debug("class T {\n    void f() {\n        do {} while (true);\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

// ===========================================================================
// Nesting: switch counts depth
// ===========================================================================

#[test]
fn nesting_switch_counts_depth() {
    let out = debug("class T {\n    void f(int x) {\n        switch (x) {\n            case 1:\n                if (x > 0) {}\n                break;\n        }\n    }\n}\n");
    let depth = function_metric(&out, "f", "nesting").unwrap_or(0);
    assert!(depth >= 2, "switch+if >= 2, got: {}", depth);
}

// ===========================================================================
// Primitive obsession: untyped not applicable in Java
// ===========================================================================

#[test]
fn primitive_obsession_complex_types_not_flagged() {
    let out = check("import java.util.List;\nclass T {\n    void f(List<Integer> a, String b, Object c, Object d) {}\n}\n");
    assert!(!has_smell(&out, "Primitive Obsession"));
}

// ===========================================================================
// Cognitive Complexity (CogC)
// ===========================================================================

#[test]
fn cogc_flat_branches() {
    let out = debug(concat!(
        "class Test {\n",
        "    void f(int x) {\n",
        "        if (x == 1) {}\n",
        "        if (x == 2) {}\n",
        "        if (x == 3) {}\n",
        "        if (x == 4) {}\n",
        "        if (x == 5) {}\n",
        "    }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cogc"), Some(5));
}

#[test]
fn cogc_nested_ifs() {
    let out = debug(concat!(
        "class Test {\n",
        "    void f(int x) {\n",
        "        if (x > 0) {\n",
        "            if (x > 1) {\n",
        "                if (x > 2) {\n",
        "                    if (x > 3) {}\n",
        "                }\n",
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
        "class Test {\n",
        "    void f(int x) {\n",
        "        if (x == 1) {\n",
        "        } else if (x == 2) {\n",
        "        } else if (x == 3) {\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cogc"), Some(3));
}

#[test]
fn cogc_else_increases_nesting() {
    let out = debug(concat!(
        "class Test {\n",
        "    void f(int x) {\n",
        "        if (x > 0) {\n",
        "        } else {\n",
        "            if (x < -10) {}\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cogc"), Some(4));
}

#[test]
fn cogc_switch_counted() {
    let out = debug(concat!(
        "class Test {\n",
        "    void f(int x) {\n",
        "        switch (x) {\n",
        "            case 1: break;\n",
        "            default: break;\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cogc"), Some(1));
}

#[test]
fn cogc_enhanced_for_nested() {
    let out = debug(concat!(
        "import java.util.List;\n",
        "class Test {\n",
        "    void f(boolean x, List<Integer> items) {\n",
        "        if (x) {\n",
        "            for (var item : items) {}\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cogc"), Some(3));
}

#[test]
fn cogc_triggers_complex_method() {
    let code = concat!(
        "class Test {\n",
        "    void f(int x) {\n",
        "        if (x > 0) {\n",
        "            if (x > 1) {\n",
        "                if (x > 2) {\n",
        "                    if (x > 3) {\n",
        "                        if (x > 4) {}\n",
        "                    }\n",
        "                }\n",
        "            }\n",
        "        }\n",
        "    }\n",
        "}\n",
    );
    let out = check(code);
    let d = debug(code);
    let cogc = function_metric(&d, "f", "cogc").unwrap();
    let cc = function_metric(&d, "f", "cc").unwrap();
    assert!(cogc >= 15, "cogc should be >= 15, got: {}", cogc);
    assert!(cc < 9, "cc should be < 9, got: {}", cc);
    assert!(has_smell(&out, "Complex Method"));
}

// ===========================================================================
// Empty Error Handler
// ===========================================================================

#[test]
fn empty_catch_detected() {
    let out = check("class Test {\n    void f() {\n        try { risky(); } catch (Exception e) {}\n    }\n}\n");
    assert!(has_smell(&out, "Empty Error Handler"));
}

#[test]
fn non_empty_catch_not_detected() {
    let out = check(concat!(
        "class Test {\n",
        "    void f() {\n",
        "        try {\n",
        "            risky();\n",
        "        } catch (Exception e) {\n",
        "            System.out.println(e);\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Empty Error Handler"));
}

#[test]
fn multiple_empty_catches() {
    let out = check(concat!(
        "class Test {\n",
        "    void f() {\n",
        "        try { risky(); } catch (Exception e) {}\n",
        "        try { risky(); } catch (Exception e) {}\n",
        "    }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Empty Error Handler"));
    assert!(out.contains("2 empty catch blocks"));
}

#[test]
fn no_try_catch_no_smell() {
    let out = check("class Test {\n    void f() {\n        int x = 1;\n    }\n}\n");
    assert!(!has_smell(&out, "Empty Error Handler"));
}

// ===========================================================================
// Coverage: interfaces, abstract methods
// ===========================================================================

#[test]
fn java_interface_default_method() {
    let code = "interface Foo {\n  default void bar() {\n    if (true) {}\n  }\n}\n";
    let out = debug(code);
    assert!(out.contains("bar"), "interface default method should be found: {}", out);
}

#[test]
fn java_abstract_method_skipped() {
    let code = "abstract class Foo {\n  abstract void bar();\n  void baz() { if (true) {} }\n}\n";
    let out = debug(code);
    assert!(out.contains("baz"), "concrete method should be found: {}", out);
}
