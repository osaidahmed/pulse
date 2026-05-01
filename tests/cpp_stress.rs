mod common;

use common::*;
use std::process::Command;

lang_helpers!("cpp");

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
    let out = debug(
        "#include <vector>\nvoid f() {\n    std::vector<int> v;\n    for (auto x : v) {}\n}\n",
    );
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
    assert!(cc >= 4, "got: {cc}");
}

#[test]
fn cc_nested_if_in_for() {
    let out =
        debug("void f() {\n    for (int i = 0; i < 10; i++) {\n        if (i > 5) {}\n    }\n}\n");
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

#[test]
fn lcom4_methods_connected_by_call() {
    let out = check(concat!(
        "class Coord {\n",
        "    int state = 0;\n",
        "public:\n",
        "    bool process(int e) { return this->validate(e) && this->dispatch(e); }\n",
        "    bool validate(int e) { return e > 0; }\n",
        "    bool dispatch(int e) { return this->send(e); }\n",
        "    bool send(int e) { return true; }\n",
        "};\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"), "got: {out}");
}

#[test]
fn lcom4_mixed_field_and_call_connection() {
    let out = check(concat!(
        "class Mixed {\n",
        "    int x = 0;\n",
        "public:\n",
        "    int a() { return this->x; }\n",
        "    int b() { this->x = 1; return this->c(); }\n",
        "    int c() { return 42; }\n",
        "};\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_god_class_still_fires() {
    let out = check(concat!(
        "class Svc {\n",
        "    int db = 0; int cache = 0; int mailer = 0; int events = 0; int audit = 0;\n",
        "public:\n",
        "    int getUser() { return this->db; }\n",
        "    int cacheUser() { return this->cache; }\n",
        "    int sendWelcome() { return this->mailer; }\n",
        "    int publish() { return this->events; }\n",
        "    int auditLog() { return this->audit; }\n",
        "};\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"), "got: {out}");
}

#[test]
fn lcom4_dependency_method_calls_dont_falsely_connect() {
    let out = check(concat!(
        "class Svc {\n",
        "    int db = 0; int cache = 0; int log = 0;\n",
        "public:\n",
        "    int a() { return this->db; }\n",
        "    int b() { return this->cache; }\n",
        "    int c() { return this->log; }\n",
        "};\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"));
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
    for i in 0..declarations_above() {
        code.push_str(&format!("struct T{i} {{}};\n"));
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
    let mut code =
        String::from("void bad(int a, int b, int c, int d, int e, int f, int g, int h) {\n");
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
        code.push_str(&format!("int func{i}(int data) {{\n"));
        for j in 0..18 {
            code.push_str(&format!("    int f{j} = data + {j};\n"));
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
        code.push_str(&format!("class S{i} {{\npublic:\n"));
        for j in 0..5 {
            code.push_str(&format!("    int m{j}() {{ return this->d_; }}\n"));
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

// ===========================================================================
// CC: not/! operator
// ===========================================================================

#[test]
fn cc_counts_not_operator() {
    let out = debug("void f(int a) {\n    if (!a) {}\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(cc >= 2, "if(!a) should have cc >= 2, got: {cc}");
}

// ===========================================================================
// Nesting: try-catch depth
// ===========================================================================

#[test]
fn nesting_try_catch_counts_depth() {
    let out = debug("void f() {\n    try {\n        if (x) {}\n    } catch (...) {}\n}\n");
    let depth = function_metric(&out, "f", "nesting").unwrap_or(0);
    assert!(
        depth >= 1,
        "try-catch should contribute nesting, got: {depth}"
    );
}

// ===========================================================================
// Args: reference params
// ===========================================================================

#[test]
fn args_default_params() {
    let out = debug("void f(int a = 0, int b = 1) {}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(2));
}

// ===========================================================================
// LCOM4: single method not flagged
// ===========================================================================

#[test]
fn lcom4_single_method_not_flagged() {
    let out = check("class T {\npublic:\n    int x_;\n    int get() { return this->x_; }\n};\n");
    assert!(!has_smell(&out, "Low Cohesion"));
}

// ===========================================================================
// LCOM4: constructor excluded
// ===========================================================================

#[test]
fn lcom4_constructor_excluded() {
    let out = check(concat!(
        "class Init {\n",
        "public:\n",
        "    Init() : a_(1), b_(2), c_(3) {}\n",
        "    int useA() { return this->a_; }\n",
        "    int useB() { return this->b_; }\n",
        "    int useC() { return this->c_; }\n",
        "private:\n",
        "    int a_; int b_; int c_;\n",
        "};\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"));
}

// ===========================================================================
// LCOM4: 2 disconnected groups (below threshold)
// ===========================================================================

#[test]
fn lcom4_two_groups_not_flagged() {
    let out = check(concat!(
        "class Split {\n",
        "public:\n",
        "    int aWork() { return this->a_; }\n",
        "    int aRead() { return this->a_ + 1; }\n",
        "    int bWork() { return this->b_; }\n",
        "    int bRead() { return this->b_ + 1; }\n",
        "private:\n",
        "    int a_; int b_;\n",
        "};\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"));
}

// ===========================================================================
// LCOM4: three groups flagged
// ===========================================================================

#[test]
fn lcom4_three_groups_flagged() {
    let out = check(concat!(
        "class M {\n",
        "public:\n",
        "    int useX() { return this->x_; }\n",
        "    int useY() { return this->y_; }\n",
        "    int useZ() { return this->z_; }\n",
        "private:\n",
        "    int x_; int y_; int z_;\n",
        "};\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"));
}

// ===========================================================================
// Duplication: test suppressed
// ===========================================================================

#[test]
fn duplication_test_suppressed() {
    let out = check(concat!(
        "void test_a() {\n    int r = compute();\n    assert(r == 1);\n    assert(r != 0);\n    assert(r < 10);\n    assert(r > 0);\n}\n\n",
        "void test_b() {\n    int r = compute();\n    assert(r == 1);\n    assert(r != 0);\n    assert(r < 10);\n    assert(r > 0);\n}\n",
    ));
    assert!(!has_smell(&out, "Code Duplication"));
}

// ===========================================================================
// Duplication: mixed test+prod flagged
// ===========================================================================

#[test]
fn duplication_mixed_test_and_prod_flagged() {
    let out = check(concat!(
        "void test_compute() {\n    int r = 0;\n    for (int i = 0; i < 10; i++) {\n        r += i;\n    }\n    printf(\"%d\", r);\n}\n\n",
        "void process_data() {\n    int r = 0;\n    for (int i = 0; i < 10; i++) {\n        r += i;\n    }\n    printf(\"%d\", r);\n}\n",
    ));
    assert!(has_smell(&out, "Code Duplication"));
}

// ===========================================================================
// God class: requires god method
// ===========================================================================

#[test]
fn god_class_requires_god_method() {
    let mut code = String::new();
    for i in 0..functions_above() {
        code.push_str(&format!("int fn{i}() {{ return {i}; }}\n"));
    }
    for i in 0..file_padding() {
        code.push_str(&format!("int VAR{i} = {i};\n"));
    }
    let out = check(&code);
    assert!(!has_smell(&out, "God Class"));
}

// ===========================================================================
// God class: triggers
// ===========================================================================

#[test]
fn god_class_triggers_with_god_method() {
    let mut code = String::from("void monster() {\n");
    for i in 0..cc_branches() {
        code.push_str(&format!("    if ({i} > 0) {{}}\n"));
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("    int y{i} = {i};\n"));
    }
    code.push_str("}\n\n");
    for i in 0..functions_above() {
        code.push_str(&format!("int fn{i}() {{ return {i}; }}\n"));
    }
    for i in 0..file_padding() {
        code.push_str(&format!("int V{i} = {i};\n"));
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
        "void test_inter() {\n",
        "    assert(x == 1);\n",
        "    assert(y == 2);\n",
        "    assert(z == 3);\n",
        "    do_something();\n",
        "    assert(a == 4);\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Large Assertion Block"));
}

#[test]
fn assertion_block_at_threshold() {
    let mut code = String::from("void test_exact() {\n");
    for i in 0..asserts_at() {
        code.push_str(&format!("    assert(x{i} == {i});\n"));
    }
    code.push_str("}\n");
    let out = check(&code);
    assert!(!has_smell(&out, "Large Assertion Block"));
}

#[test]
fn assertion_block_above_threshold() {
    let mut code = String::from("void test_big() {\n");
    for i in 0..asserts_above() {
        code.push_str(&format!("    assert(x{i} == {i});\n"));
    }
    code.push_str("}\n");
    let out = check(&code);
    assert!(has_smell(&out, "Large Assertion Block"));
}

// ===========================================================================
// Overall function size
// ===========================================================================

#[test]
fn overall_function_size_below_threshold() {
    let mut code = String::new();
    for i in 0..(t().large_fn_count as usize - 1) {
        code.push_str(&format!("void lg{i}() {{\n"));
        for j in 0..large_fn_lines() {
            code.push_str(&format!("    int x{j} = {j};\n"));
        }
        code.push_str("}\n\n");
    }
    let out = check(&code);
    assert!(!has_smell(&out, "Overall Function Size"));
}

#[test]
fn overall_function_size_at_threshold() {
    let mut code = String::new();
    for i in 0..t().large_fn_count as usize {
        code.push_str(&format!("void lg{i}() {{\n"));
        for j in 0..large_fn_lines() {
            code.push_str(&format!("    int x{j} = {j};\n"));
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
        code.push_str(&format!("struct T{i} {{}};\n"));
    }
    let out = check(&code);
    assert!(!has_smell(&out, "Declarations"));
}

// ===========================================================================
// Embedded: small not flagged, multiline flagged
// ===========================================================================

#[test]
fn small_string_not_flagged_as_embedded() {
    let out = check("const char* f() {\n    return \"hello world\";\n}\n");
    assert!(!has_smell(&out, "Large Embedded Block"));
}

#[test]
fn multiline_raw_string_flagged() {
    let mut code = String::from("const char* f() {\n    const char* q = R\"(\n");
    for i in 0..embedded_lines_above() {
        code.push_str(&format!("        SELECT field_{i}\n"));
    }
    code.push_str("    )\";\n    return q;\n}\n");
    let out = check(&code);
    assert!(has_smell(&out, "Large Embedded Block"));
}

// ===========================================================================
// Deep global nesting
// ===========================================================================

#[test]
fn shallow_global_not_flagged() {
    let out = check("int x = 1;\nvoid setup() {}\n");
    assert!(!has_smell(&out, "Deep Global Nesting"));
}

// ===========================================================================
// Constructor vs excess args
// ===========================================================================

#[test]
fn constructor_reports_injection_not_excess() {
    let out = check("class S {\npublic:\n    S(int a, int b, int c, int d, int e, int f) {}\n};\n");
    assert!(has_smell(&out, "Constructor Over-Injection"));
    let lines: Vec<&str> = out
        .lines()
        .filter(|l| l.contains("S(") || l.contains("S.S"))
        .collect();
    assert!(!lines.iter().any(|l| l.contains("Excess Arguments")));
}

// ===========================================================================
// Regular function reports excess
// ===========================================================================

#[test]
fn regular_function_reports_excess() {
    let out = check("void f(int a, int b, int c, int d, int e, int f, int g) {}\n");
    assert!(has_smell(&out, "Excess Arguments"));
    assert!(!has_smell(&out, "Constructor Over-Injection"));
}

// ===========================================================================
// Multiple smells
// ===========================================================================

#[test]
fn function_can_have_multiple_smells() {
    let mut code =
        String::from("void bad(int a, int b, int c, int d, int e, int f, int g, int h) {\n");
    code.push_str("    const char* q = R\"(\n");
    for i in 0..embedded_lines_above() {
        code.push_str(&format!("        SELECT field_{i}\n"));
    }
    code.push_str("    )\";\n");
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
// Primitive obsession: mixed not flagged
// ===========================================================================

#[test]
fn primitive_obsession_mixed_not_flagged() {
    let out =
        check("#include <string>\nvoid f(int a, std::string b, std::string c, std::string d) {}\n");
    assert!(!has_smell(&out, "Primitive Obsession"));
}

// ===========================================================================
// Real-world: clean C++ module
// ===========================================================================

#[test]
fn clean_cpp_module_not_flagged() {
    let out = check(concat!(
        "#include <string>\n",
        "class Config {\n",
        "public:\n",
        "    Config(std::string host, int port) : host_(host), port_(port) {}\n",
        "    std::string address() { return this->host_ + \":\" + std::to_string(this->port_); }\n",
        "private:\n",
        "    std::string host_;\n",
        "    int port_;\n",
        "};\n",
    ));
    assert!(
        out.is_empty(),
        "clean C++ code should not be flagged, got: {out}"
    );
}

// ===========================================================================
// Comments only
// ===========================================================================

#[test]
fn comments_only() {
    let out = check("/* comments */\n// only\n");
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
// Nested conditional chunks
// ===========================================================================

#[test]
fn nested_conditional_chunks_detected() {
    let out = check(concat!(
        "void validate(int* data, int n) {\n",
        "    if (n > 0) {\n",
        "        if (data[0] > 0) {\n",
        "            if (data[0] > 10) {\n",
        "                int x = 1;\n",
        "            }\n",
        "        }\n",
        "    }\n",
        "    int gap = 1;\n",
        "    if (n > 5) {\n",
        "        if (data[5] > 0) {\n",
        "            if (data[5] > 10) {\n",
        "                int y = 2;\n",
        "            }\n",
        "        }\n",
        "    }\n",
        "    int gap2 = 2;\n",
        "    if (n > 10) {\n",
        "        if (data[10] > 0) {\n",
        "            if (data[10] > 10) {\n",
        "                int z = 3;\n",
        "            }\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    assert!(
        has_smell(&out, "Nested Conditional Chunks") || has_smell(&out, "Complex Method"),
        "got: {out}"
    );
}

// ===========================================================================
// CC: do-while
// ===========================================================================

#[test]
fn cc_do_while() {
    let out = debug("void f() {\n    do {} while (x);\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

// ===========================================================================
// CC: multiple catch
// ===========================================================================

#[test]
fn cc_multiple_catch() {
    let out =
        debug("void f() {\n    try {} catch (int e) {} catch (float e) {} catch (...) {}\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(cc >= 4, "base + 3 catch = 4, got: {cc}");
}

// ===========================================================================
// CC: switch many cases
// ===========================================================================

#[test]
fn cc_switch_many_cases() {
    let out = debug(concat!(
        "const char* f(int x) {\n",
        "    switch (x) {\n",
        "        case 1: return \"a\";\n",
        "        case 2: return \"b\";\n",
        "        case 3: return \"c\";\n",
        "        case 4: return \"d\";\n",
        "        case 5: return \"e\";\n",
        "        case 6: return \"f\";\n",
        "        case 7: return \"g\";\n",
        "        case 8: return \"h\";\n",
        "        default: return \"?\";\n",
        "    }\n",
        "}\n",
    ));
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(cc >= 9, "8 cases + base >= 9, got: {cc}");
}

// ===========================================================================
// Nesting: deep for-if-for
// ===========================================================================

#[test]
fn nesting_deep_for_if_for() {
    let out = debug("void f() {\n    if (x) {\n        for (int i = 0; i < n; i++) {\n            if (z) {\n                for (int j = 0; j < m; j++) {}\n            }\n        }\n    }\n}\n");
    let depth = function_metric(&out, "f", "nesting").unwrap_or(0);
    assert!(depth >= 4, "got: {depth}");
}

// ===========================================================================
// Args: template params
// ===========================================================================

#[test]
fn args_template_params() {
    let out = debug("#include <vector>\nvoid f(std::vector<int> a, int b) {}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(2));
}

// ===========================================================================
// Primitive obsession: all primitives
// ===========================================================================

#[test]
fn primitive_obsession_all_primitives() {
    let out = check("void f(int a, float b, double c, char d) {}\n");
    assert!(has_smell(&out, "Primitive Obsession"));
}

#[test]
fn primitive_obsession_below_min() {
    let out = check("void f(int a, int b, int c) {}\n");
    assert!(!has_smell(&out, "Primitive Obsession"));
}

// ===========================================================================
// Duplication: two functions is minimum
// ===========================================================================

#[test]
fn duplication_two_is_minimum() {
    let out = check(concat!(
        "int rpt_a(int* d, int n) {\n    int r = 0;\n    for (int i = 0; i < n; i++) {\n        r += d[i];\n    }\n    return r;\n}\n\n",
        "int rpt_b(int* d, int n) {\n    int r = 0;\n    for (int i = 0; i < n; i++) {\n        r += d[i];\n    }\n    return r;\n}\n",
    ));
    assert!(has_smell(&out, "Code Duplication"));
}

// ===========================================================================
// Decorated function (C++: attribute)
// ===========================================================================

#[test]
fn attributed_function_analyzed() {
    let out =
        check("[[nodiscard]]\nvoid f(int a, int b, int c, int d, int e, int f, int g, int h) {}\n");
    assert!(has_smell(&out, "Excess Arguments"), "got: {out}");
}

// ===========================================================================
// Deep nesting with switch
// ===========================================================================

#[test]
fn deep_nesting_with_switch() {
    let out = check(concat!(
        "void deep(int x) {\n",
        "    for (int i = 0; i < x; i++) {\n",
        "        if (i > 0) {\n",
        "            switch (i) {\n",
        "                case 1:\n",
        "                    for (int j = 0; j < i; j++) {\n",
        "                        if (j > 0) {}\n",
        "                    }\n",
        "                    break;\n",
        "            }\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Deep Nested"));
}

// ===========================================================================
// CC: else-if chain
// ===========================================================================

#[test]
fn cc_else_if_chain() {
    let out =
        debug("void f(int x) {\n    if (x == 1) {} else if (x == 2) {} else if (x == 3) {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(4));
}

// ===========================================================================
// Nesting: switch counts depth
// ===========================================================================

#[test]
fn nesting_switch_counts_depth() {
    let out = debug("void f(int x) {\n    switch (x) {\n        case 1:\n            if (x > 0) {}\n            break;\n    }\n}\n");
    let depth = function_metric(&out, "f", "nesting").unwrap_or(0);
    assert!(depth >= 2, "switch+if >= 2, got: {depth}");
}

// ===========================================================================
// Output format: pulse prefix
// ===========================================================================

#[test]
fn output_starts_with_pulse() {
    let out = check("void f(int a, int b, int c, int d, int e, int f, int g) {}\n");
    assert!(out.starts_with("pulse:"));
}

// ===========================================================================
// Output format: line numbers
// ===========================================================================

#[test]
fn output_has_line_numbers() {
    let out = check("void f(int a, int b, int c, int d, int e, int f, int g) {}\n");
    let has_loc = out.lines().any(|l| l.contains("(L") && l.contains("): "));
    assert!(has_loc);
}

// ===========================================================================
// Issue count matches
// ===========================================================================

#[test]
fn issue_count_matches() {
    let out = check("void f(int a, int b, int c, int d, int e, int f, int g) {}\n");
    let first = out.lines().next().unwrap_or("");
    let findings = out.lines().filter(|l| l.starts_with("  ")).count();
    assert!(first.contains(&format!("{findings} issue")));
}

// ===========================================================================
// Module prefix in output
// ===========================================================================

#[test]
fn output_has_module_prefix() {
    let mut code = String::new();
    for i in 0..functions_above() {
        code.push_str(&format!("int fn{i}() {{ return {i}; }}\n"));
    }
    let out = check(&code);
    assert!(out.contains("Module:"));
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
// Excess args count
// ===========================================================================

#[test]
fn excess_args_count_verified() {
    let out = debug("void f(int a, int b, int c, int d, int e, int f, int g, int h) {}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(8));
}

// ===========================================================================
// CC: ternary
// ===========================================================================

#[test]
fn cc_ternary_inline() {
    let out = debug("int f(int a) {\n    return a ? 1 : 0;\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

// ===========================================================================
// Cognitive Complexity (CogC)
// ===========================================================================

#[test]
fn cogc_flat_branches() {
    let out = debug(concat!(
        "void f(int x) {\n",
        "    if (x == 1) {}\n",
        "    if (x == 2) {}\n",
        "    if (x == 3) {}\n",
        "    if (x == 4) {}\n",
        "    if (x == 5) {}\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cogc"), Some(5));
}

#[test]
fn cogc_nested_ifs() {
    let out = debug(concat!(
        "void f(int x) {\n",
        "    if (x > 0) {\n",
        "        if (x > 1) {\n",
        "            if (x > 2) {\n",
        "                if (x > 3) {}\n",
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
        "void f(int x) {\n",
        "    if (x == 1) {\n",
        "    } else if (x == 2) {\n",
        "    } else if (x == 3) {\n",
        "    }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cogc"), Some(3));
}

#[test]
fn cogc_else_increases_nesting() {
    let out = debug(concat!(
        "void f(int x) {\n",
        "    if (x > 0) {\n",
        "    } else {\n",
        "        if (x < -10) {}\n",
        "    }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cogc"), Some(4));
}

#[test]
fn cogc_switch_counted() {
    let out = debug(concat!(
        "void f(int x) {\n",
        "    switch (x) {\n",
        "        case 1: break;\n",
        "        default: break;\n",
        "    }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cogc"), Some(1));
}

#[test]
fn cogc_catch_penalized() {
    let out = debug(concat!(
        "void f(int x) {\n",
        "    if (x > 0) {\n",
        "        try {\n",
        "        } catch (...) {}\n",
        "    }\n",
        "}\n",
    ));
    let cogc = function_metric(&out, "f", "cogc").unwrap();
    assert!(cogc >= 3, "catch in nested context should be penalized, got: {cogc}");
}

#[test]
fn cogc_triggers_complex_method() {
    let code = concat!(
        "void f(int x) {\n",
        "    if (x > 0) {\n",
        "        if (x > 1) {\n",
        "            if (x > 2) {\n",
        "                if (x > 3) {\n",
        "                    if (x > 4) {}\n",
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
    assert!(cogc >= 15, "cogc should be >= 15, got: {cogc}");
    assert!(cc < 9, "cc should be < 9, got: {cc}");
    assert!(has_smell(&out, "Complex Method"));
}

// ===========================================================================
// Empty Error Handler
// ===========================================================================

#[test]
fn empty_catch_detected() {
    let out = check("void f() {\n    try { risky(); } catch (...) {}\n}\n");
    assert!(has_smell(&out, "Empty Error Handler"));
}

#[test]
fn non_empty_catch_not_detected() {
    let out = check(concat!(
        "void f() {\n",
        "    try {\n",
        "        risky();\n",
        "    } catch (...) {\n",
        "        log(\"error\");\n",
        "    }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Empty Error Handler"));
}

#[test]
fn no_try_catch_no_smell() {
    let out = check("void f() {\n    int x = 1;\n}\n");
    assert!(!has_smell(&out, "Empty Error Handler"));
}

// ===========================================================================
// Coverage: namespaces, pure virtual, forward declarations
// ===========================================================================

#[test]
fn cpp_namespace_functions_analyzed() {
    let code = "namespace ns {\n  void f() {\n    if (true) {}\n  }\n}\n";
    let out = debug(code);
    assert!(out.contains('f'), "function in namespace should be found: {out}");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cpp_forward_declaration_no_crash() {
    let code = "class Foo;\nvoid f() {}\n";
    let out = debug(code);
    assert!(out.contains('f'));
}

#[test]
fn cpp_pure_virtual_method_skipped() {
    let code = "class Base {\npublic:\n  virtual void f() = 0;\n};\nvoid g() {}\n";
    let out = debug(code);
    assert!(out.contains('g'), "concrete function should be found: {out}");
}

#[test]
fn cpp_pointer_return_function_params() {
    let code = "int* f(int a, int b, int c, int d, int e, int g) { return 0; }\n";
    let out = debug(code);
    assert_eq!(function_metric(&out, "f", "args"), Some(6));
}
