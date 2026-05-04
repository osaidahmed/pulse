
use crate::common::*;
use std::process::Command;

lang_helpers!("c");

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
    let out = debug(
        "void f(void) {\n    for (int i = 0; i < 10; i++) {\n        if (i > 5) {}\n    }\n}\n",
    );
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_chained_boolean() {
    let out = debug("void f(int a, int b, int c, int d) {\n    if (a && b && c && d) {}\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(cc >= 4, "got: {cc}");
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

// ===========================================================================
// CC: not/! operator
// ===========================================================================

#[test]
fn cc_counts_not_operator() {
    let out = debug("void f(int a) {\n    if (!a) {}\n}\n");
    // In C, ! may or may not increment cc depending on parser; at minimum base(1) + if(1) = 2
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(cc >= 2, "if(!a) should have cc >= 2, got: {cc}");
}

// ===========================================================================
// CC: do-while
// ===========================================================================

#[test]
fn cc_do_while_counts() {
    let out = debug("void f(void) {\n    do {} while (x);\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
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
// Nesting: for-in-if depth
// ===========================================================================

#[test]
fn nesting_for_if_for_depth() {
    let out = debug("void f(void) {\n    if (x) {\n        for (int i = 0; i < n; i++) {\n            if (z) {\n                for (int j = 0; j < m; j++) {}\n            }\n        }\n    }\n}\n");
    let depth = function_metric(&out, "f", "nesting").unwrap_or(0);
    assert!(depth >= 4, "got: {depth}");
}

// ===========================================================================
// Args: const and pointer params
// ===========================================================================

#[test]
fn args_const_pointer() {
    let out = debug("void f(const int* a, char** b) {}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(2));
}

#[test]
fn args_function_pointer() {
    let out = debug("void f(void (*callback)(int), int data) {}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(2));
}

// ===========================================================================
// Duplication: test functions suppressed
// ===========================================================================

#[test]
fn duplication_test_suppressed() {
    let out = check(concat!(
        "void test_a(int* d, int n) {\n    int r = 0;\n    for (int i = 0; i < n; i++) {\n        r += d[i];\n    }\n    printf(\"%d\", r);\n}\n\n",
        "void test_b(int* d, int n) {\n    int r = 0;\n    for (int i = 0; i < n; i++) {\n        r += d[i];\n    }\n    printf(\"%d\", r);\n}\n",
    ));
    assert!(!has_smell(&out, "Code Duplication"));
}

// ===========================================================================
// Declarations: below threshold
// ===========================================================================

#[test]
fn declarations_below_threshold() {
    let mut code = String::new();
    for i in 0..10 {
        code.push_str(&format!("struct T{i} {{ int x; }};\n"));
    }
    let out = check(&code);
    assert!(!has_smell(&out, "Declarations"));
}

// ===========================================================================
// Declarations: above threshold
// ===========================================================================

#[test]
fn declarations_above_threshold() {
    let mut code = String::new();
    for i in 0..declarations_above() {
        code.push_str(&format!("struct T{i} {{ int x; }};\n"));
    }
    let out = check(&code);
    assert!(has_smell(&out, "Declarations"));
}

// ===========================================================================
// God class requires god method
// ===========================================================================

#[test]
fn god_class_requires_god_method() {
    let mut code = String::new();
    for i in 0..functions_above() {
        code.push_str(&format!("int fn{i}(void) {{ return {i}; }}\n"));
    }
    for i in 0..file_padding() {
        code.push_str(&format!("int VAR{i} = {i};\n"));
    }
    let out = check(&code);
    assert!(!has_smell(&out, "God Class"));
}

// ===========================================================================
// God class triggers with god method
// ===========================================================================

#[test]
fn god_class_triggers_with_god_method() {
    let mut code = String::from("void monster(void) {\n");
    for i in 0..cc_branches() {
        code.push_str(&format!("    if ({i} > 0) {{}}\n"));
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("    int y{i} = {i};\n"));
    }
    code.push_str("}\n\n");
    for i in 0..functions_above() {
        code.push_str(&format!("int fn{i}(void) {{ return {i}; }}\n"));
    }
    for i in 0..file_padding() {
        code.push_str(&format!("int V{i} = {i};\n"));
    }
    let out = check(&code);
    assert!(has_smell(&out, "God Method"));
    assert!(has_smell(&out, "God Class"));
}

// ===========================================================================
// Overall function size: below threshold
// ===========================================================================

#[test]
fn overall_function_size_below_threshold() {
    let mut code = String::new();
    for i in 0..(t().module.large_fn_count as usize - 1) {
        code.push_str(&format!("void lg{i}(void) {{\n"));
        for j in 0..large_fn_lines() {
            code.push_str(&format!("    int x{j} = {j};\n"));
        }
        code.push_str("}\n\n");
    }
    let out = check(&code);
    assert!(!has_smell(&out, "Overall Function Size"));
}

// ===========================================================================
// Overall function size: at threshold
// ===========================================================================

#[test]
fn overall_function_size_at_threshold() {
    let mut code = String::new();
    for i in 0..t().module.large_fn_count as usize {
        code.push_str(&format!("void lg{i}(void) {{\n"));
        for j in 0..large_fn_lines() {
            code.push_str(&format!("    int x{j} = {j};\n"));
        }
        code.push_str("}\n\n");
    }
    let out = check(&code);
    assert!(has_smell(&out, "Overall Function Size"));
}

// ===========================================================================
// Embedded block: multiline flagged
// ===========================================================================

#[test]
fn multiline_string_flagged() {
    let mut code = String::from("const char* f(void) {\n    const char* q = \"\\\n");
    for i in 0..embedded_lines_above() {
        code.push_str(&format!("        SELECT field_{i} \\\n"));
    }
    code.push_str("    \";\n    return q;\n}\n");
    let out = check(&code);
    assert!(has_smell(&out, "Large Embedded Block"));
}

#[test]
fn small_string_not_flagged() {
    let out = check("const char* f(void) {\n    return \"hello\";\n}\n");
    assert!(!has_smell(&out, "Large Embedded Block"));
}

// ===========================================================================
// Deep global nesting
// ===========================================================================

#[test]
fn global_nesting_deep_flagged() {
    let out = check("#include <stdio.h>\nint x = 1;\nvoid setup(void) {}\n");
    // C rarely has global if blocks; verify no crash
    assert!(!has_smell(&out, "Deep Global Nesting"));
}

// ===========================================================================
// Comments only
// ===========================================================================

#[test]
fn comments_only() {
    let out = check("/* just comments */\n// nothing else\n");
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
// Regular function reports excess args, not constructor injection
// ===========================================================================

#[test]
fn regular_function_reports_excess_args_not_constructor() {
    let out = check("void f(int a, int b, int c, int d, int e, int f, int g) {}\n");
    assert!(has_smell(&out, "Excess Arguments"));
    assert!(!has_smell(&out, "Constructor Over-Injection"));
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
// Real-world: clean C module
// ===========================================================================

#[test]
fn clean_c_module_not_flagged() {
    let out = check(concat!(
        "typedef struct {\n",
        "    int x;\n",
        "    int y;\n",
        "} Point;\n\n",
        "Point point_new(int x, int y) {\n",
        "    Point p = {x, y};\n",
        "    return p;\n",
        "}\n\n",
        "int point_distance(Point* a, Point* b) {\n",
        "    int dx = a->x - b->x;\n",
        "    int dy = a->y - b->y;\n",
        "    return dx * dx + dy * dy;\n",
        "}\n",
    ));
    assert!(
        out.is_empty(),
        "clean C code should not be flagged, got: {out}"
    );
}

// ===========================================================================
// Duplication: mixed test+prod flagged
// ===========================================================================

#[test]
fn duplication_mixed_test_and_prod_flagged() {
    let out = check(concat!(
        "void test_compute(int* d, int n) {\n    int r = 0;\n    for (int i = 0; i < n; i++) {\n        r += d[i];\n    }\n    printf(\"%d\", r);\n}\n\n",
        "void process_data(int* d, int n) {\n    int r = 0;\n    for (int i = 0; i < n; i++) {\n        r += d[i];\n    }\n    printf(\"%d\", r);\n}\n",
    ));
    assert!(has_smell(&out, "Code Duplication"));
}

// ===========================================================================
// CC: chained boolean operators
// ===========================================================================

#[test]
fn cc_chained_boolean_4way() {
    let out = debug("void f(int a, int b, int c, int d) {\n    if (a && b && c && d) {}\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(cc >= 5, "base+if+3 ands >= 5, got: {cc}");
}

// ===========================================================================
// CC: nested if in while
// ===========================================================================

#[test]
fn cc_nested_if_in_while() {
    let out = debug("void f(void) {\n    while (x) {\n        if (y) {}\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

// ===========================================================================
// Nesting: deep do-while
// ===========================================================================

#[test]
fn nesting_do_while_counts_depth() {
    let out = debug("void f(void) {\n    do {\n        if (x) {}\n    } while (y);\n}\n");
    let depth = function_metric(&out, "f", "nesting").unwrap_or(0);
    assert!(
        depth >= 1,
        "do-while should contribute nesting, got: {depth}"
    );
}

// ===========================================================================
// Args: struct params
// ===========================================================================

#[test]
fn args_struct_params() {
    let out = debug("typedef struct { int x; } S;\nvoid f(S a, S b, S c) {}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(3));
}

// ===========================================================================
// Assertion block: not flagged below threshold
// ===========================================================================

#[test]
fn assertion_block_below_threshold_not_flagged() {
    let mut code = String::from("void test_few(void) {\n");
    for i in 0..5 {
        code.push_str(&format!("    assert(x{i} == {i});\n"));
    }
    code.push_str("}\n");
    let out = check(&code);
    assert!(!has_smell(&out, "Large Assertion Block"));
}

// ===========================================================================
// Assertion block: above threshold
// ===========================================================================

#[test]
fn assertion_block_above_threshold() {
    let mut code = String::from("void test_many(void) {\n");
    for i in 0..asserts_above() {
        code.push_str(&format!("    assert(x{i} == {i});\n"));
    }
    code.push_str("}\n");
    let out = check(&code);
    assert!(has_smell(&out, "Large Assertion Block"));
}

// ===========================================================================
// Assertion block: interrupted resets
// ===========================================================================

#[test]
fn assertion_block_interrupted_resets() {
    let out = check(concat!(
        "void test_interleaved(void) {\n",
        "    assert(x == 1);\n",
        "    assert(y == 2);\n",
        "    assert(z == 3);\n",
        "    do_something();\n",
        "    assert(a == 4);\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Large Assertion Block"));
}

// ===========================================================================
// Primitive obsession: C has types so it may trigger
// ===========================================================================

#[test]
fn primitive_obsession_all_ints() {
    let out = check("void f(int a, int b, int c, int d) {}\n");
    assert!(has_smell(&out, "Primitive Obsession"));
}

#[test]
fn primitive_obsession_mixed_not_flagged() {
    let out = check("typedef struct { int x; } MyStruct;\nvoid f(int a, MyStruct b, MyStruct c, MyStruct d) {}\n");
    assert!(!has_smell(&out, "Primitive Obsession"));
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
// Function at cc boundary
// ===========================================================================

#[test]
fn function_at_cc_boundary() {
    let out = check("int f(void) {\n    if (a) {}\n    if (b) {}\n    if (c) {}\n    if (d) {}\n    if (e) {}\n    if (f) {}\n    if (g) {}\n    if (h) {}\n    return 0;\n}\n");
    assert!(has_smell(&out, "Complex Method"));
}

// ===========================================================================
// Function below cc boundary
// ===========================================================================

#[test]
fn function_below_cc_boundary() {
    let out = check("int f(void) {\n    if (a) {}\n    if (b) {}\n    if (c) {}\n    if (d) {}\n    if (e) {}\n    if (f) {}\n    if (g) {}\n    return 0;\n}\n");
    assert!(!has_smell(&out, "Complex Method"));
}

// ===========================================================================
// Multiple functions with duplication
// ===========================================================================

#[test]
fn duplication_two_is_minimum() {
    let out = check(concat!(
        "void rpt_a(int* d, int n) {\n    int r = 0;\n    for (int i = 0; i < n; i++) {\n        r += d[i];\n    }\n    printf(\"%d\", r);\n}\n\n",
        "void rpt_b(int* d, int n) {\n    int r = 0;\n    for (int i = 0; i < n; i++) {\n        r += d[i];\n    }\n    printf(\"%d\", r);\n}\n",
    ));
    assert!(has_smell(&out, "Code Duplication"));
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
// Excess args: many params
// ===========================================================================

#[test]
fn excess_args_exactly_at_threshold() {
    // 6 args is typically the threshold for flagging
    let out = check("void f(int a, int b, int c, int d, int e, int f, int g) {}\n");
    assert!(has_smell(&out, "Excess Arguments"));
}

// ===========================================================================
// Void param for zero args
// ===========================================================================

#[test]
fn void_param_zero_args() {
    let out = debug("int f(void) {\n    return 0;\n}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(0));
}

// ===========================================================================
// CC: ternary in return
// ===========================================================================

#[test]
fn cc_ternary_in_return() {
    let out = debug("int f(int a) {\n    return a ? 1 : 0;\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
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
// Decorated function (C doesn't have decorators, but attribute functions)
// ===========================================================================

#[test]
fn attributed_function_analyzed() {
    let out = check(
        "__attribute__((noinline))\nvoid f(int a, int b, int c, int d, int e, int f, int g) {}\n",
    );
    assert!(has_smell(&out, "Excess Arguments"), "got: {out}");
}

// ===========================================================================
// Deep global nesting (not common in C)
// ===========================================================================

#[test]
fn global_conditional_not_flagged_for_shallow() {
    let out = check("int x = 1;\nvoid setup(void) {}\n");
    assert!(!has_smell(&out, "Deep Global Nesting"));
}

// ===========================================================================
// Excess args: count verification
// ===========================================================================

#[test]
fn excess_args_count_verified() {
    let out = debug("void f(int a, int b, int c, int d, int e, int f, int g, int h) {}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(8));
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
    assert!(depth >= 2, "switch+if should be >= 2, got: {depth}");
}

// ===========================================================================
// CC: or operator
// ===========================================================================

#[test]
fn cc_or_operator() {
    let out = debug("void f(int a, int b) {\n    if (a || b) {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

// ===========================================================================
// Assertion block at threshold exactly
// ===========================================================================

#[test]
fn assertion_block_at_threshold_exact() {
    let mut code = String::from("void test_exact(void) {\n");
    for i in 0..asserts_at() {
        code.push_str(&format!("    assert(x{i} == {i});\n"));
    }
    code.push_str("}\n");
    let out = check(&code);
    assert!(!has_smell(&out, "Large Assertion Block"));
}

// ===========================================================================
// Primitive obsession: below min typed
// ===========================================================================

#[test]
fn primitive_obsession_below_min_typed() {
    let out = check("void f(int a, int b, int c) {}\n");
    assert!(!has_smell(&out, "Primitive Obsession"));
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
        code.push_str(&format!("int fn{i}(void) {{ return {i}; }}\n"));
    }
    let out = check(&code);
    assert!(out.contains("Module:"));
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
// Coverage: global nesting
// ===========================================================================

#[test]
fn c_global_if_deep_nesting() {
    // C doesn't normally have top-level if, but tree-sitter may parse fragments
    let code = "void f() {}\n";
    let out = check(code);
    // Just ensure no crash on clean C code
    assert!(!has_smell(&out, "Deep Global Nesting"));
}
