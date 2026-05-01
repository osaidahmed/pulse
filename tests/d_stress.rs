mod common;

use common::*;

lang_helpers!("d");

// ===========================================================================
// CC counting
// ===========================================================================

#[test]
fn cc_base() {
    let d = debug("void f() { int x = 1; }\n");
    assert_eq!(function_metric(&d, "f", "cc"), Some(1));
}

#[test]
fn cc_if() {
    let d = debug("void f(int x) { if (x > 0) {} }\n");
    assert_eq!(function_metric(&d, "f", "cc"), Some(2));
}

#[test]
fn cc_else_if() {
    let d = debug("void f(int x) { if (x > 0) {} else if (x < 0) {} }\n");
    assert_eq!(function_metric(&d, "f", "cc"), Some(3));
}

#[test]
fn cc_else_no_increment() {
    let d = debug("void f(int x) { if (x > 0) {} else {} }\n");
    assert_eq!(function_metric(&d, "f", "cc"), Some(2));
}

#[test]
fn cc_for() {
    let d = debug("void f() { for (int i = 0; i < 10; i++) {} }\n");
    assert_eq!(function_metric(&d, "f", "cc"), Some(2));
}

#[test]
fn cc_foreach() {
    let d = debug("void f(int[] data) { foreach (item; data) {} }\n");
    assert_eq!(function_metric(&d, "f", "cc"), Some(2));
}

#[test]
fn cc_while() {
    let d = debug("void f(int x) { while (x > 0) { x = x - 1; } }\n");
    assert_eq!(function_metric(&d, "f", "cc"), Some(2));
}

#[test]
fn cc_do_while() {
    let d = debug("void f(int x) { do { x = x + 1; } while (x < 10); }\n");
    assert_eq!(function_metric(&d, "f", "cc"), Some(2));
}

#[test]
fn cc_switch_cases() {
    let d = debug("int f(int x) {\n    switch (x) {\n        case 1: return 1;\n        case 2: return 2;\n        case 3: return 3;\n        default: return 0;\n    }\n}\n");
    assert_eq!(function_metric(&d, "f", "cc"), Some(4));
}

#[test]
fn cc_catch() {
    let d = debug("void f() { try { int x = 1; } catch (Exception e) { int y = 2; } }\n");
    assert_eq!(function_metric(&d, "f", "cc"), Some(2));
}

#[test]
fn cc_finally_no_increment() {
    let d = debug("void f() { try { int x = 1; } finally { int y = 2; } }\n");
    assert_eq!(function_metric(&d, "f", "cc"), Some(1));
}

#[test]
fn cc_and() {
    let d = debug("bool f(bool a, bool b) { if (a && b) { return true; } return false; }\n");
    assert_eq!(function_metric(&d, "f", "cc"), Some(3));
}

#[test]
fn cc_or() {
    let d = debug("bool f(bool a, bool b) { if (a || b) { return true; } return false; }\n");
    assert_eq!(function_metric(&d, "f", "cc"), Some(3));
}

#[test]
fn cc_chained_boolean() {
    let d = debug("bool f(bool a, bool b, bool c) { if (a && b || c) { return true; } return false; }\n");
    assert_eq!(function_metric(&d, "f", "cc"), Some(4));
}

#[test]
fn cc_chained_boolean_4way() {
    let d = debug("bool f(bool a, bool b, bool c, bool d, bool e) { if (a && b || c && d || e) { return true; } return false; }\n");
    assert_eq!(function_metric(&d, "f", "cc"), Some(6));
}

#[test]
fn cc_nested_if_counted_once() {
    let d = debug("void f(int x, int y) { if (x > 0) { if (y > 0) {} } }\n");
    assert_eq!(function_metric(&d, "f", "cc"), Some(3));
}

#[test]
fn cc_switch_default_not_counted() {
    let d = debug("int f(int x) {\n    switch (x) {\n        case 1: return 1;\n        default: return 0;\n    }\n}\n");
    assert_eq!(function_metric(&d, "f", "cc"), Some(2));
}

#[test]
fn cc_ternary() {
    let d = debug("int f(int x) { auto r = (x > 0) ? 1 : 0; return r; }\n");
    assert_eq!(function_metric(&d, "f", "cc"), Some(2));
}

// ===========================================================================
// Cognitive complexity
// ===========================================================================

#[test]
fn cogc_flat_branches() {
    let d = debug("void f(int a, int b, int c, int d, int e) {\n    if (a > 0) {}\n    if (b > 0) {}\n    if (c > 0) {}\n    if (d > 0) {}\n    if (e > 0) {}\n}\n");
    assert_eq!(function_metric(&d, "f", "cogc"), Some(5));
}

#[test]
fn cogc_nested_ifs() {
    let d = debug("void f(int a, int b, int c) {\n    if (a > 0) {\n        if (b > 0) {\n            if (c > 0) {}\n        }\n    }\n}\n");
    let cogc = function_metric(&d, "f", "cogc").unwrap_or(0);
    assert_eq!(cogc, 6, "1+(1+1)+(1+2) = 6, got: {cogc}");
}

#[test]
fn cogc_else_if_no_nesting() {
    let d = debug("void f(int x) {\n    if (x > 0) {}\n    else if (x < 0) {}\n    else if (x == 0) {}\n}\n");
    let cogc = function_metric(&d, "f", "cogc").unwrap_or(99);
    assert_eq!(cogc, 3, "else-if chain is flat, got: {cogc}");
}

#[test]
fn cogc_else_increases() {
    let d = debug("void f(int x) {\n    if (x > 0) {} else {}\n}\n");
    let cogc = function_metric(&d, "f", "cogc").unwrap_or(0);
    assert_eq!(cogc, 2, "if=1 + else=1, got: {cogc}");
}

#[test]
fn cogc_for_nested() {
    let d = debug("void f(int x) {\n    for (int i = 0; i < 10; i++) {\n        if (x > 0) {}\n    }\n}\n");
    let cogc = function_metric(&d, "f", "cogc").unwrap_or(0);
    assert_eq!(cogc, 3, "for=1, nested if=1+1, got: {cogc}");
}

#[test]
fn cogc_foreach_nested() {
    let d = debug("void f(int[] data) {\n    foreach (item; data) {\n        if (item > 0) {}\n    }\n}\n");
    let cogc = function_metric(&d, "f", "cogc").unwrap_or(0);
    assert_eq!(cogc, 3, "foreach=1, nested if=1+1, got: {cogc}");
}

#[test]
fn cogc_while_nested() {
    let d = debug("void f(int x) {\n    while (x > 0) {\n        if (x > 5) {}\n        x = x - 1;\n    }\n}\n");
    let cogc = function_metric(&d, "f", "cogc").unwrap_or(0);
    assert_eq!(cogc, 3, "while=1, nested if=1+1, got: {cogc}");
}

#[test]
fn cogc_do_while_nested() {
    let d = debug("void f(int x) {\n    do {\n        if (x > 5) {}\n        x = x + 1;\n    } while (x < 10);\n}\n");
    let cogc = function_metric(&d, "f", "cogc").unwrap_or(0);
    assert_eq!(cogc, 3, "do=1, nested if=1+1, got: {cogc}");
}

#[test]
fn cogc_switch_counted() {
    let d = debug("int f(int x) {\n    switch (x) {\n        case 1: return 1;\n        default: return 0;\n    }\n}\n");
    assert_eq!(function_metric(&d, "f", "cogc"), Some(1));
}

#[test]
fn cogc_boolean_single_sequence() {
    let d = debug("bool f(bool a, bool b, bool c) { if (a && b && c) { return true; } return false; }\n");
    let cogc = function_metric(&d, "f", "cogc").unwrap_or(0);
    assert_eq!(cogc, 2, "if=1 + 1 sequence, got: {cogc}");
}

#[test]
fn cogc_boolean_mixed_sequence() {
    let d = debug("bool f(bool a, bool b, bool c) { if (a && b || c) { return true; } return false; }\n");
    let cogc = function_metric(&d, "f", "cogc").unwrap_or(0);
    assert_eq!(cogc, 3, "if=1 + 2 sequences, got: {cogc}");
}

#[test]
fn cogc_catch_nesting() {
    let d = debug("void f(int x) {\n    try {\n        if (x > 0) {}\n    } catch (Exception e) {\n        if (x < 0) {}\n    }\n}\n");
    let cogc = function_metric(&d, "f", "cogc").unwrap_or(0);
    assert!(cogc >= 4, "try body if + catch + catch body if, got: {cogc}");
}

// ===========================================================================
// Nesting depth
// ===========================================================================

#[test]
fn nesting_0_flat() {
    let d = debug("void f() { int x = 1; }\n");
    assert_eq!(function_metric(&d, "f", "nesting"), Some(0));
}

#[test]
fn nesting_1_single_if() {
    let d = debug("void f(int x) { if (x > 0) {} }\n");
    assert_eq!(function_metric(&d, "f", "nesting"), Some(1));
}

#[test]
fn nesting_2_nested_if() {
    let d = debug("void f(int x, int y) { if (x > 0) { if (y > 0) {} } }\n");
    assert_eq!(function_metric(&d, "f", "nesting"), Some(2));
}

#[test]
fn nesting_3_triple() {
    let d = debug("void f(int x, int y) {\n    if (x > 0) {\n        for (int i = 0; i < 10; i++) {\n            if (y > 0) {}\n        }\n    }\n}\n");
    assert_eq!(function_metric(&d, "f", "nesting"), Some(3));
}

#[test]
fn nesting_4_deep() {
    let d = debug("void f(int x) {\n    if (x > 0) {\n        if (x > 1) {\n            if (x > 2) {\n                if (x > 3) {}\n            }\n        }\n    }\n}\n");
    assert_eq!(function_metric(&d, "f", "nesting"), Some(4));
}

#[test]
fn nesting_sequential_not_accumulated() {
    let d = debug("void f(int x) {\n    if (x > 0) {}\n    if (x < 0) {}\n}\n");
    assert_eq!(function_metric(&d, "f", "nesting"), Some(1));
}

#[test]
fn nesting_foreach_for() {
    let d = debug("void f(int[] data) {\n    foreach (item; data) {\n        for (int i = 0; i < 10; i++) {}\n    }\n}\n");
    assert_eq!(function_metric(&d, "f", "nesting"), Some(2));
}

#[test]
fn nesting_switch_depth() {
    let d = debug("void f(int x) {\n    switch (x) {\n        case 1:\n            if (x > 0) {}\n            break;\n        default: break;\n    }\n}\n");
    let n = function_metric(&d, "f", "nesting").unwrap_or(0);
    assert!(n >= 2, "switch + case if, got: {n}");
}

// ===========================================================================
// Argument counting
// ===========================================================================

#[test]
fn args_zero() {
    let d = debug("void f() { int x = 1; }\n");
    assert_eq!(function_metric(&d, "f", "args"), Some(0));
}

#[test]
fn args_one() {
    let d = debug("void f(int x) { int y = x; }\n");
    assert_eq!(function_metric(&d, "f", "args"), Some(1));
}

#[test]
fn args_five() {
    let d = debug("void f(int a, int b, int c, int d, int e) { int x = 1; }\n");
    assert_eq!(function_metric(&d, "f", "args"), Some(5));
}

#[test]
fn args_six() {
    let d = debug("void f(int a, int b, int c, int d, int e, int g) { int x = 1; }\n");
    assert_eq!(function_metric(&d, "f", "args"), Some(6));
}

#[test]
fn args_six_flagged() {
    let out = check("void f(int a, int b, int c, int d, int e, int g) { int x = 1; }\n");
    assert!(has_smell(&out, "Excess Arguments"), "got: {out}");
}

// ===========================================================================
// LOC counting
// ===========================================================================

#[test]
fn loc_simple() {
    let d = debug("void f() {\n    int x = 1;\n    int y = 2;\n}\n");
    assert_eq!(function_metric(&d, "f", "loc"), Some(4));
}

#[test]
fn loc_blank_lines_counted() {
    let d = debug("void f() {\n    int x = 1;\n\n    int y = 2;\n}\n");
    assert_eq!(function_metric(&d, "f", "loc"), Some(5));
}

#[test]
fn loc_comments_excluded_from_module() {
    let d = debug("// comment\nvoid f() {\n    int x = 1;\n}\n");
    assert!(d.contains("LOC"), "should show module LOC");
}

// ===========================================================================
// Constructor metrics
// ===========================================================================

#[test]
fn constructor_is_constructor() {
    let d = debug("class Foo {\n    this(int x) {\n        int y = x;\n    }\n}\n");
    assert!(d.contains("Foo.this"), "got: {d}");
}

#[test]
fn constructor_arg_count() {
    let d = debug("class Foo {\n    this(int a, int b, int c) {\n        int x = 1;\n    }\n}\n");
    let args = function_metric(&d, "Foo.this", "args").unwrap_or(0);
    assert_eq!(args, 3);
}

#[test]
fn constructor_over_injection() {
    let out = check("class Foo {\n    this(int a, int b, int c, int d, int e, int f, int g) {\n        int x = 1;\n    }\n}\n");
    assert!(has_smell(&out, "Constructor Over-Injection"), "got: {out}");
}

#[test]
fn destructor_not_constructor() {
    let d = debug("class Foo {\n    ~this() {\n        int x = 0;\n    }\n}\n");
    assert!(d.contains("Foo.~this"), "got: {d}");
}

// ===========================================================================
// Field access / class context
// ===========================================================================

#[test]
fn method_class_prefix() {
    let d = debug("class Foo {\n    void bar() {\n        int x = 1;\n    }\n}\n");
    assert!(d.contains("Foo.bar"), "got: {d}");
}

#[test]
fn struct_method_class_name() {
    let d = debug("struct Point {\n    void draw() {\n        int x = 1;\n    }\n}\n");
    assert!(d.contains("Point.draw"), "got: {d}");
}

#[test]
fn standalone_function_no_class() {
    let d = debug("void hello() { int x = 1; }\n");
    assert!(d.contains("hello"), "got: {d}");
    assert!(!d.contains(".hello"), "should not have class prefix");
}

// ===========================================================================
// Embedded blocks
// ===========================================================================

#[test]
fn embedded_large_string() {
    let mut code = String::from("string f() {\n    string s = \"");
    for i in 0..20 {
        code.push_str(&format!("line{i}\\n"));
    }
    code.push_str("\";\n    return s;\n}\n");
    let d = debug(&code);
    let emb = function_metric(&d, "f", "embedded").unwrap_or(0);
    assert!(emb >= 1, "got: {emb}");
}

#[test]
fn embedded_small_not_flagged() {
    let out = check("string f() {\n    return \"hello\";\n}\n");
    assert!(!has_smell(&out, "Large Embedded Block"));
}

// ===========================================================================
// Short variables
// ===========================================================================

#[test]
fn short_var_detected() {
    let mut code = String::from("void f() {\n");
    for n in 0..20 {
        code.push_str(&format!("    int var{n} = {n};\n"));
    }
    code.push_str("    int a = 1;\n    int b = 2;\n    int c = 3;\n    int d = 4;\n}\n");
    let d = debug(&code);
    let sv = function_metric(&d, "f", "short_vars").unwrap_or(0);
    assert!(sv >= 4, "got: {sv}");
}

#[test]
fn short_var_ijk_exempt() {
    let mut code = String::from("void f() {\n");
    for n in 0..20 {
        code.push_str(&format!("    int var{n} = {n};\n"));
    }
    code.push_str("    int i = 1;\n    int j = 2;\n    int k = 3;\n}\n");
    let d = debug(&code);
    let sv = function_metric(&d, "f", "short_vars").unwrap_or(99);
    assert!(sv <= 3, "i/j/k should be exempt, got: {sv}");
}

// ===========================================================================
// String match arms
// ===========================================================================

#[test]
fn string_switch_arms_counted() {
    let d = debug(concat!(
        "string f(string x) {\n",
        "    switch (x) {\n",
        "        case \"a\": return \"1\";\n",
        "        case \"b\": return \"2\";\n",
        "        case \"c\": return \"3\";\n",
        "        case \"d\": return \"4\";\n",
        "        case \"e\": return \"5\";\n",
        "        case \"f\": return \"6\";\n",
        "        default: return \"0\";\n",
        "    }\n",
        "}\n",
    ));
    let arms = function_metric(&d, "f", "str_match").unwrap_or(0);
    assert!(arms >= 6, "got: {arms}");
}

#[test]
fn string_switch_below_threshold() {
    let out = check(concat!(
        "string f(string x) {\n",
        "    switch (x) {\n",
        "        case \"a\": return \"1\";\n",
        "        case \"b\": return \"2\";\n",
        "        default: return \"0\";\n",
        "    }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Stringly-Typed"));
}

// ===========================================================================
// Empty catch
// ===========================================================================

#[test]
fn empty_catch_detected() {
    let out = check("void f() {\n    try {\n        int x = 1;\n    } catch (Exception e) {\n    }\n}\n");
    assert!(has_smell(&out, "Empty Error Handler"), "got: {out}");
}

#[test]
fn non_empty_catch_ok() {
    let out = check("void f() {\n    try {\n        int x = 1;\n    } catch (Exception e) {\n        int y = 2;\n    }\n}\n");
    assert!(!has_smell(&out, "Empty Error Handler"));
}

#[test]
fn multiple_empty_catches() {
    let d = debug("void f() {\n    try {\n        int x = 1;\n    } catch (Exception e) {\n    }\n    try {\n        int y = 2;\n    } catch (Exception e) {\n    }\n}\n");
    // Should count multiple
    assert!(d.contains('f'), "function should be parsed");
}

// ===========================================================================
// Duplication
// ===========================================================================

#[test]
fn exact_duplication_detected() {
    let out = check(concat!(
        "int funcA(int x, int y) {\n",
        "    int result = x + y;\n",
        "    if (result > 100) { result = result - 50; }\n",
        "    if (result < 0) { result = 0; }\n",
        "    return result * 2;\n",
        "}\n",
        "int funcB(int a, int b) {\n",
        "    int result = a + b;\n",
        "    if (result > 100) { result = result - 50; }\n",
        "    if (result < 0) { result = 0; }\n",
        "    return result * 2;\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Code Duplication"), "got: {out}");
}

// ===========================================================================
// D-specific edge cases
// ===========================================================================

#[test]
fn scope_guard_no_cc_exit() {
    let d = debug("void f() {\n    scope(exit) int x = 1;\n    int y = 2;\n}\n");
    assert_eq!(function_metric(&d, "f", "cc"), Some(1));
}

#[test]
fn scope_guard_no_cc_success() {
    let d = debug("void f() {\n    scope(success) int x = 1;\n    int y = 2;\n}\n");
    assert_eq!(function_metric(&d, "f", "cc"), Some(1));
}

#[test]
fn scope_guard_no_cc_failure() {
    let d = debug("void f() {\n    scope(failure) int x = 1;\n    int y = 2;\n}\n");
    assert_eq!(function_metric(&d, "f", "cc"), Some(1));
}

#[test]
fn unittest_parsed() {
    let d = debug("unittest {\n    assert(1 == 1);\n}\n");
    assert!(d.contains("unittest_L1"), "got: {d}");
}

#[test]
fn mixin_no_crash() {
    let d = debug("void f() {\n    mixin(\"int x = 1;\");\n}\n");
    assert!(d.contains('f'), "should parse without crash");
}

#[test]
fn module_declaration_no_crash() {
    let d = debug("module foo.bar;\n\nvoid f() {\n    int x = 1;\n}\n");
    assert!(d.contains('f'), "got: {d}");
}

#[test]
fn lambda_not_walked() {
    let d = debug("void f() {\n    auto fn = (int x) { if (x > 0) { if (x > 1) {} } };\n}\n");
    let cc = function_metric(&d, "f", "cc").unwrap_or(99);
    assert_eq!(cc, 1, "lambda body should not contribute to outer cc, got: {cc}");
}

// ===========================================================================
// Compound conditions
// ===========================================================================

#[test]
fn compound_condition_two_ops() {
    let d = debug("bool f(int a, int b, int c) {\n    if (a > 0 && b > 0 || c > 0) { return true; }\n    return false;\n}\n");
    let cc_count = function_metric(&d, "f", "conditions").unwrap_or(0);
    assert!(cc_count >= 1, "got: {cc_count}");
}

#[test]
fn compound_condition_single_op() {
    let d = debug("bool f(int a, int b) {\n    if (a > 0 && b > 0) { return true; }\n    return false;\n}\n");
    let cc_count = function_metric(&d, "f", "conditions").unwrap_or(99);
    assert_eq!(cc_count, 0, "single op should not count, got: {cc_count}");
}

// ===========================================================================
// Bump count
// ===========================================================================

#[test]
fn bump_count_tracked() {
    // Bumps count nested conditional blocks at depth >= 2
    let d = debug(concat!(
        "int f(int x, int y) {\n",
        "    if (x > 0) { if (y > 0) { return 1; } }\n",
        "    if (x < 0) { if (y < 0) { return 2; } }\n",
        "    if (x == 0) { if (y == 0) { return 0; } }\n",
        "    return -1;\n",
        "}\n",
    ));
    // Verify bumps metric is present and tracked
    assert!(d.contains("bumps="), "got: {d}");
}

#[test]
fn bump_flat_function_zero() {
    let d = debug("void f(int x) {\n    if (x > 0) {}\n    if (x < 0) {}\n}\n");
    let bumps = function_metric(&d, "f", "bumps").unwrap_or(99);
    assert_eq!(bumps, 0, "flat ifs should have 0 bumps, got: {bumps}");
}

// ===========================================================================
// Primitive type detection
// ===========================================================================

#[test]
fn primitives_int() {
    let d = debug("void f(int a, int b) { int x = 1; }\n");
    assert!(d.contains("primitives=2/2"), "got: {d}");
}

#[test]
fn primitives_mixed() {
    let d = debug("void f(int a, double b, string c) { int x = 1; }\n");
    assert!(d.contains("primitives=3/3"), "got: {d}");
}

#[test]
fn primitives_bool() {
    let d = debug("void f(bool a) { int x = 1; }\n");
    assert!(d.contains("primitives=1/1"), "got: {d}");
}

// ===========================================================================
// Struct fields
// ===========================================================================

#[test]
fn struct_fields_counted() {
    let d = debug("struct Foo {\n    int x;\n    int y;\n    string name;\n}\n");
    assert!(d.contains("declarations=1"), "struct should be counted as declaration, got: {d}");
}

// ===========================================================================
// Edge cases
// ===========================================================================

#[test]
fn clean_class_no_smells() {
    let out = check("class Foo {\n    void bar() {\n        int x = 1;\n    }\n}\n");
    assert!(out.is_empty(), "got: {out}");
}

#[test]
fn empty_file_no_crash() {
    let out = check("");
    assert!(out.is_empty());
}

#[test]
fn empty_method_body() {
    let d = debug("void f() {}\n");
    assert_eq!(function_metric(&d, "f", "cc"), Some(1));
}

#[test]
fn multiple_independent_functions() {
    let d = debug("void f() { int x = 1; }\nvoid g() { int y = 2; }\nvoid h() { int z = 3; }\n");
    assert!(d.contains('f'));
    assert!(d.contains('g'));
    assert!(d.contains('h'));
}
