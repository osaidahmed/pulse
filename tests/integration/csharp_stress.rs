use crate::common::*;

lang_helpers!("cs");

// ===========================================================================
// CC counting
// ===========================================================================

#[test]
fn cc_counts_if() {
    let out = debug("public class T { static int f(int x) { if (x > 0) {} return x; } }");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_for() {
    let out = debug("public class T { static void f() { for (int i = 0; i < 10; i++) {} } }");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_foreach() {
    let out = debug("public class T { static void f(int[] items) { foreach (var x in items) {} } }");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_while() {
    let out = debug("public class T { static void f() { while (true) {} } }");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_do_while() {
    let out = debug("public class T { static void f() { do {} while (true); } }");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_switch_cases() {
    let out = debug(concat!(
        "public class T { static string f(int x) { ",
        "switch (x) { case 1: return \"a\"; case 2: return \"b\"; case 3: return \"c\"; } ",
        "return \"?\"; } }",
    ));
    assert_eq!(function_metric(&out, "f", "cc"), Some(4));
}

#[test]
fn cc_counts_catch() {
    let out = debug("public class T { static void f() { try {} catch (System.Exception e) {} } }");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_ternary() {
    let out = debug("public class T { static int f(bool a) { return a ? 1 : 0; } }");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_and() {
    let out = debug("public class T { static void f(bool a, bool b) { if (a && b) {} } }");
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(cc >= 2, "got: {cc}");
}

#[test]
fn cc_counts_or() {
    let out = debug("public class T { static void f(bool a, bool b) { if (a || b) {} } }");
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(cc >= 2, "got: {cc}");
}

#[test]
fn cc_counts_else_if() {
    let out = debug("public class T { static void f(int x) { if (x == 1) {} else if (x == 2) {} } }");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

// ===========================================================================
// Cognitive complexity
// ===========================================================================

#[test]
fn cogc_flat_branches() {
    let out = debug(concat!(
        "public class T {\n",
        "    static void f(int a, int b, int c, int d, int e) {\n",
        "        if (a > 0) {}\n",
        "        if (b > 0) {}\n",
        "        if (c > 0) {}\n",
        "        if (d > 0) {}\n",
        "        if (e > 0) {}\n",
        "    }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cogc"), Some(5));
}

#[test]
fn cogc_nested_ifs() {
    let out = debug(concat!(
        "public class T {\n",
        "    static void f(int a, int b, int c, int d) {\n",
        "        if (a > 0) {\n",
        "            if (b > 0) {\n",
        "                if (c > 0) {\n",
        "                    if (d > 0) {}\n",
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
        "public class T {\n",
        "    static void f(int x) {\n",
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
        "public class T {\n",
        "    static void f(int x) {\n",
        "        if (x > 0) {\n",
        "        } else {\n",
        "            if (x < -10) {}\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    let cogc = function_metric(&out, "f", "cogc").unwrap();
    assert!(cogc >= 3, "else should contribute to nesting, got: {cogc}");
}

#[test]
fn cogc_switch_counted() {
    let out = debug(concat!(
        "public class T {\n",
        "    static void f(int x) {\n",
        "        switch (x) {\n",
        "            case 1: break;\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cogc"), Some(1));
}

#[test]
fn cogc_triggers_complex_method() {
    let out = check(concat!(
        "public class T {\n",
        "    static void f(int a) {\n",
        "        if (a > 0) {\n",
        "            if (a > 1) {\n",
        "                if (a > 2) {\n",
        "                    if (a > 3) {\n",
        "                        if (a > 4) {}\n",
        "                    }\n",
        "                }\n",
        "            }\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Complex Method"), "got: {out}");
    let dbg = debug(concat!(
        "public class T {\n",
        "    static void f(int a) {\n",
        "        if (a > 0) {\n",
        "            if (a > 1) {\n",
        "                if (a > 2) {\n",
        "                    if (a > 3) {\n",
        "                        if (a > 4) {}\n",
        "                    }\n",
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
}

// ===========================================================================
// Other metrics
// ===========================================================================

#[test]
fn nesting_depth() {
    let out = debug(concat!(
        "public class T {\n",
        "    static void f(int a) {\n",
        "        if (a > 0) {\n",
        "            if (a > 1) {\n",
        "                if (a > 2) {\n",
        "                    if (a > 3) {\n",
        "                        if (a > 4) {}\n",
        "                    }\n",
        "                }\n",
        "            }\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "nesting"), Some(5));
}

#[test]
fn args_excess() {
    let out = debug("public class T { static void f(int a, int b, int c, int d, int e, int g) {} }");
    assert_eq!(function_metric(&out, "f", "args"), Some(6));
}

#[test]
fn constructor_detected() {
    let out = check(concat!(
        "public class T {\n",
        "    public T(object a, object b, object c, object d, object e, object f, object g, object h, object i) {}\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Constructor Over-Injection"), "got: {out}");
}

#[test]
fn class_method_naming() {
    let out = debug(concat!("public class MyClass {\n", "    public void DoWork() {}\n", "}\n",));
    assert!(out.contains("MyClass.DoWork"), "expected ClassName.MethodName format, got: {out}");
}

#[test]
fn namespace_methods_found() {
    let out = debug(concat!(
        "namespace MyApp {\n",
        "    public class Service {\n",
        "        public void Handle() {}\n",
        "    }\n",
        "}\n",
    ));
    assert!(out.contains("Service.Handle"), "method inside namespace should be found, got: {out}");
}

#[test]
fn empty_catch_detected() {
    let out = check(concat!(
        "public class T {\n",
        "    static void f() {\n",
        "        try { int x = 1; } catch (System.Exception e) {}\n",
        "    }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Empty Error Handler"), "got: {out}");
}

#[test]
fn non_empty_catch_not_detected() {
    let out = check(concat!(
        "public class T {\n",
        "    static void f() {\n",
        "        try { int x = 1; } catch (System.Exception e) { int y = 2; }\n",
        "    }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Empty Error Handler"), "non-empty catch should not be flagged, got: {out}");
}

#[test]
fn struct_methods_found() {
    let out = debug(concat!(
        "public struct Point {\n",
        "    public int X;\n",
        "    public int Y;\n",
        "    public int Sum() { return this.X + this.Y; }\n",
        "}\n",
    ));
    assert!(out.contains("Point.Sum"), "struct methods should be found, got: {out}");
}

// ===========================================================================
// CC precision tests
// ===========================================================================

#[test]
fn cc_nested_if_counted_once() {
    let out = debug(concat!(
        "public class T {\n",
        "    static void f(int x) {\n",
        "        if (x > 0) {\n",
        "            if (x > 1) {}\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_multiple_catch() {
    let out = debug(concat!(
        "public class T {\n",
        "    static void f() {\n",
        "        try {} catch (System.Exception e) {} catch (System.InvalidOperationException e) {} catch (System.ArgumentException e) {}\n",
        "    }\n",
        "}\n",
    ));
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(cc >= 4, "base + 3 catch = 4, got: {cc}");
}

#[test]
fn cc_chained_boolean() {
    let out = debug(concat!(
        "public class T {\n",
        "    static void f(bool a, bool b, bool c) {\n",
        "        if (a && b || c) {}\n",
        "    }\n",
        "}\n",
    ));
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(cc >= 4, "if + && + || = 4, got: {cc}");
}

#[test]
fn cc_chained_boolean_4way() {
    let out = debug(concat!(
        "public class T {\n",
        "    static void f(bool a, bool b, bool c, bool d) {\n",
        "        if (a && b || c && d) {}\n",
        "    }\n",
        "}\n",
    ));
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(cc >= 5, "if + 3 bool ops = 5, got: {cc}");
}

#[test]
fn cc_ternary_nested() {
    let out = debug(concat!(
        "public class T {\n",
        "    static int f(bool a, bool b) {\n",
        "        int x = a ? 1 : 0;\n",
        "        int y = b ? 1 : 0;\n",
        "        return x + y;\n",
        "    }\n",
        "}\n",
    ));
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(cc >= 3, "two ternaries = 3, got: {cc}");
}

#[test]
fn cc_switch_default_not_counted() {
    let out = debug(concat!(
        "public class T {\n",
        "    static string f(int x) {\n",
        "        switch (x) {\n",
        "            case 1: return \"a\";\n",
        "            case 2: return \"b\";\n",
        "            default: return \"?\";\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert_eq!(cc, 3, "base + 2 cases; default is fallthrough, not a decision point, got: {cc}");
}

#[test]
fn cc_counts_switch_expression_arms() {
    let out = debug(concat!(
        "public class T {\n",
        "    static string f(int x) {\n",
        "        return x switch {\n",
        "            1 => \"a\",\n",
        "            2 => \"b\",\n",
        "            _ => \"?\",\n",
        "        };\n",
        "    }\n",
        "}\n",
    ));
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert_eq!(cc, 3, "base + 2 arms; discard arm is not a decision point, got: {cc}");
}

// ===========================================================================
// Nesting tests
// ===========================================================================

#[test]
fn nesting_depth_simple() {
    let out =
        debug(concat!("public class T {\n", "    static void f() {\n", "        if (true) {}\n", "    }\n", "}\n",));
    assert_eq!(function_metric(&out, "f", "nesting"), Some(1));
}

#[test]
fn nesting_depth_nested() {
    let out = debug(concat!(
        "public class T {\n",
        "    static void f(int a) {\n",
        "        if (a > 0) {\n",
        "            if (a > 1) {\n",
        "                if (a > 2) {}\n",
        "            }\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "nesting"), Some(3));
}

#[test]
fn nesting_depth_sequential_not_accumulated() {
    let out = debug(concat!(
        "public class T {\n",
        "    static void f(int a) {\n",
        "        if (a > 0) {\n",
        "            if (a > 1) {}\n",
        "        }\n",
        "        if (a > 2) {\n",
        "            if (a > 3) {}\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "nesting"), Some(2));
}

#[test]
fn nesting_deep_if_chain() {
    let out = debug(concat!(
        "public class T {\n",
        "    static void f(int a) {\n",
        "        if (a > 0) {\n",
        "            if (a > 1) {\n",
        "                if (a > 2) {\n",
        "                    if (a > 3) {\n",
        "                        if (a > 4) {}\n",
        "                    }\n",
        "                }\n",
        "            }\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "nesting"), Some(5));
}

#[test]
fn nesting_try_catch_counts_depth() {
    let out = debug(concat!(
        "public class T {\n",
        "    static void f() {\n",
        "        try {\n",
        "            if (true) {}\n",
        "        } catch (System.Exception e) {}\n",
        "    }\n",
        "}\n",
    ));
    let depth = function_metric(&out, "f", "nesting").unwrap_or(0);
    assert!(depth >= 1, "try-catch should contribute nesting, got: {depth}");
}

// ===========================================================================
// Argument tests
// ===========================================================================

#[test]
fn args_zero() {
    let out = debug("public class T { static void f() {} }");
    assert_eq!(function_metric(&out, "f", "args"), Some(0));
}

#[test]
fn args_one() {
    let out = debug("public class T { static void f(int x) {} }");
    assert_eq!(function_metric(&out, "f", "args"), Some(1));
}

#[test]
fn args_five_at_threshold() {
    let out = check("public class T { static void f(int a, int b, int c, int d, int e) {} }");
    assert!(!has_smell(&out, "Excess Arguments"), "5 args should not fire, got: {out}");
}

#[test]
fn args_six_over_threshold() {
    let out = check("public class T { static void f(int a, int b, int c, int d, int e, int g) {} }");
    assert!(has_smell(&out, "Excess Arguments"), "6 args should fire, got: {out}");
}

#[test]
fn args_constructor_separate_threshold() {
    let out = check(concat!(
        "public class T {\n",
        "    public T(object a, object b, object c, object d, object e) {}\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Constructor Over-Injection"), "5 ctor args should not fire, got: {out}");
}

#[test]
fn args_constructor_over_threshold() {
    let out = check(concat!(
        "public class T {\n",
        "    public T(object a, object b, object c, object d, object e, object f, object g, object h, object i) {}\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Constructor Over-Injection"), "6 ctor args should fire, got: {out}");
}

// ===========================================================================
// Compound condition tests
// ===========================================================================

#[test]
fn compound_condition_detected() {
    let _out = check(concat!(
        "public class T {\n",
        "    static void f(bool a, bool b, bool c, bool d) {\n",
        "        if (a && b || c && d) {}\n",
        "    }\n",
        "}\n",
    ));
    let dbg = debug(concat!(
        "public class T {\n",
        "    static void f(bool a, bool b, bool c, bool d) {\n",
        "        if (a && b || c && d) {}\n",
        "    }\n",
        "}\n",
    ));
    let cc = function_metric(&dbg, "f", "cc").unwrap();
    assert!(cc >= 5, "if + 3 bool ops should give cc >= 5, got: {cc}");
}

#[test]
fn compound_condition_simple_not_detected() {
    let out = debug(concat!(
        "public class T {\n",
        "    static void f(bool a, bool b) {\n",
        "        if (a && b) {}\n",
        "    }\n",
        "}\n",
    ));
    let conditions = function_metric(&out, "f", "conditions").unwrap_or(0);
    assert_eq!(conditions, 0, "a && b should not be compound, got: {conditions}");
}

// ===========================================================================
// Embedded block tests
// ===========================================================================

#[test]
fn embedded_large_string_detected() {
    let mut code = String::from("public class T {\n    static string f() {\n        string q = @\"");
    for i in 0..embedded_lines_above() {
        code.push_str(&format!("\nSELECT field_{i} FROM table_{i}"));
    }
    code.push_str("\";\n        return q;\n    }\n}\n");
    let out = check(&code);
    assert!(has_smell(&out, "Large Embedded Block"), "16+ line string should fire, got: {out}");
}

#[test]
fn embedded_small_string_not_flagged() {
    let out = check(concat!(
        "public class T {\n",
        "    static string f() {\n",
        "        return \"hello world\";\n",
        "    }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Large Embedded Block"), "short string should not fire, got: {out}");
}

// ===========================================================================
// Bumpy road tests
// ===========================================================================

#[test]
fn bumpy_road_two_bumps() {
    let out = debug(concat!(
        "public class T {\n",
        "    static void f(int a, int b) {\n",
        "        if (a > 0) {\n",
        "            for (int i = 0; i < a; i++) {\n",
        "                if (i > 5) {}\n",
        "            }\n",
        "        }\n",
        "        int gap = 1;\n",
        "        if (b > 0) {\n",
        "            for (int j = 0; j < b; j++) {\n",
        "                if (j > 5) {}\n",
        "            }\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    let bumps = function_metric(&out, "f", "bumps").unwrap_or(0);
    assert!(bumps >= 2, "two nested chunks should give bumps >= 2, got: {bumps}");
}

#[test]
fn bumpy_road_single_bump_not_flagged() {
    let out = debug(concat!(
        "public class T {\n",
        "    static void f(int x) {\n",
        "        if (x > 0) {\n",
        "            if (x > 1) {}\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    let bumps = function_metric(&out, "f", "bumps").unwrap_or(0);
    assert!(bumps <= 1, "single bump should be <= 1, got: {bumps}");
}

// ===========================================================================
// Duplication tests
// ===========================================================================

#[test]
fn exact_duplication_detected() {
    let out = check(concat!(
        "public class T {\n",
        "    static int RptA(int[] d) {\n",
        "        int r = 0;\n",
        "        foreach (var v in d) {\n",
        "            r += v;\n",
        "        }\n",
        "        r = r * 2;\n",
        "        return r;\n",
        "    }\n",
        "    static int RptB(int[] d) {\n",
        "        int r = 0;\n",
        "        foreach (var v in d) {\n",
        "            r += v;\n",
        "        }\n",
        "        r = r * 2;\n",
        "        return r;\n",
        "    }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Code Duplication"), "got: {out}");
}

#[test]
fn exact_duplication_below_min_loc_not_detected() {
    let out = check(concat!(
        "public class T {\n",
        "    static int SmallA(int x) {\n",
        "        int r = x + 1;\n",
        "        r = r * 2;\n",
        "        return r;\n",
        "    }\n",
        "    static int SmallB(int x) {\n",
        "        int r = x + 1;\n",
        "        r = r * 2;\n",
        "        return r;\n",
        "    }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Code Duplication"), "5-line methods should not trigger, got: {out}");
}

#[test]
fn fuzzy_duplication_detected() {
    let out = check(concat!(
        "public class T {\n",
        "    static int ProcessA(int[] d) {\n",
        "        int r = 0;\n",
        "        foreach (var v in d) {\n",
        "            r += v;\n",
        "        }\n",
        "        r = r * 2;\n",
        "        return r;\n",
        "    }\n",
        "    static int ProcessB(int[] items) {\n",
        "        int result = 0;\n",
        "        foreach (var item in items) {\n",
        "            result += item;\n",
        "        }\n",
        "        result = result * 2;\n",
        "        return result;\n",
        "    }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Code Duplication"), "similar methods should trigger fuzzy, got: {out}");
}

#[test]
fn test_function_duplication_suppressed() {
    let out = check(concat!(
        "public class T {\n",
        "    static void test_a() {\n",
        "        int r = Compute();\n",
        "        System.Diagnostics.Debug.Assert(r == 1);\n",
        "        System.Diagnostics.Debug.Assert(r != 0);\n",
        "        System.Diagnostics.Debug.Assert(r < 10);\n",
        "        System.Diagnostics.Debug.Assert(r > 0);\n",
        "    }\n",
        "    static void test_b() {\n",
        "        int r = Compute();\n",
        "        System.Diagnostics.Debug.Assert(r == 1);\n",
        "        System.Diagnostics.Debug.Assert(r != 0);\n",
        "        System.Diagnostics.Debug.Assert(r < 10);\n",
        "        System.Diagnostics.Debug.Assert(r > 0);\n",
        "    }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Code Duplication"), "test_ prefixed should be suppressed, got: {out}");
}

// ===========================================================================
// Assertion tests
// ===========================================================================

#[test]
fn assertion_block_above_threshold() {
    let mut code = String::from("public class T {\n    static void TestBig() {\n");
    for i in 0..asserts_above() {
        code.push_str(&format!("        System.Diagnostics.Debug.Assert(x{i} == {i});\n"));
    }
    code.push_str("    }\n}\n");
    let out = check(&code);
    assert!(has_smell(&out, "Large Assertion Block"), "11+ asserts should fire, got: {out}");
}

#[test]
fn assertion_block_below_threshold_not_flagged() {
    let mut code = String::from("public class T {\n    static void TestSmall() {\n");
    for i in 0..asserts_at() {
        code.push_str(&format!("        System.Diagnostics.Debug.Assert(x{i} == {i});\n"));
    }
    code.push_str("    }\n}\n");
    let out = check(&code);
    assert!(!has_smell(&out, "Large Assertion Block"), "10 asserts should not fire, got: {out}");
}

#[test]
fn assertion_block_interrupted_resets() {
    let out = check(concat!(
        "public class T {\n",
        "    static void TestInter() {\n",
        "        System.Diagnostics.Debug.Assert(true);\n",
        "        System.Diagnostics.Debug.Assert(true);\n",
        "        System.Diagnostics.Debug.Assert(true);\n",
        "        DoSomething();\n",
        "        System.Diagnostics.Debug.Assert(true);\n",
        "    }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Large Assertion Block"), "interrupted asserts should reset, got: {out}");
}

// ===========================================================================
// Primitive obsession tests
// ===========================================================================

#[test]
fn primitive_obsession_all_primitives() {
    let out = check("public class T { static void f(int a, string b, bool c, double d, int e) {} }");
    assert!(has_smell(&out, "Primitive Obsession"), "all primitives with 4+ params should fire, got: {out}");
}

#[test]
fn primitive_obsession_below_min_typed() {
    let out = check("public class T { static void f(int a, int b, int c) {} }");
    assert!(!has_smell(&out, "Primitive Obsession"), "3 params below min_typed=4, got: {out}");
}

#[test]
fn primitive_obsession_mixed_ratio_ok() {
    let out = check(concat!(
        "public class T {\n",
        "    static void f(int a, int b, MyObj c, OtherObj d, ThirdObj e, FourthObj f) {}\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Primitive Obsession"), "2/6 = 0.33 ratio should not fire, got: {out}");
}

#[test]
fn primitive_obsession_untyped_not_counted() {
    let out = check("public class T { static void f(MyObj a, OtherObj b, ThirdObj c, FourthObj d) {} }");
    assert!(!has_smell(&out, "Primitive Obsession"), "no primitives should not fire, got: {out}");
}

// ===========================================================================
// LCOM4 tests
// ===========================================================================

#[test]
fn lcom4_connected_class_not_flagged() {
    let out = check(concat!(
        "public class C {\n",
        "    private int data;\n",
        "    public void Add(int x) { this.data = x; }\n",
        "    public int Get() { return this.data; }\n",
        "    public void Clear() { this.data = 0; }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"), "all methods share this.data so class is cohesive, got: {out}");
}

#[test]
fn lcom4_disconnected_class() {
    let out = check(concat!(
        "public class Sink {\n",
        "    private int x;\n",
        "    private int y;\n",
        "    private int z;\n",
        "    public void UseX() { this.x = 1; }\n",
        "    public int GetX() { return this.x; }\n",
        "    public void UseY() { this.y = 1; }\n",
        "    public int GetY() { return this.y; }\n",
        "    public void UseZ() { this.z = 1; }\n",
        "    public int GetZ() { return this.z; }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"), "6 disconnected groups should fire, got: {out}");
}

#[test]
fn lcom4_constructor_ignored() {
    let out = check(concat!(
        "public class Init {\n",
        "    private int a;\n",
        "    private int b;\n",
        "    private int c;\n",
        "    public Init() { this.a = 1; this.b = 2; this.c = 3; }\n",
        "    public int UseA() { return this.a; }\n",
        "    public int UseB() { return this.b; }\n",
        "    public int UseC() { return this.c; }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"), "constructor should not bridge groups, got: {out}");
}

#[test]
fn lcom4_methods_connected_by_call() {
    let out = check(concat!(
        "public class Coord {\n",
        "    private int state;\n",
        "    public bool Process(int e) { return this.Validate(e) && this.Dispatch(e); }\n",
        "    public bool Validate(int e) { return e > 0; }\n",
        "    public bool Dispatch(int e) { return this.Send(e); }\n",
        "    public bool Send(int e) { return true; }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"), "got: {out}");
}

#[test]
fn lcom4_mixed_field_and_call_connection() {
    let out = check(concat!(
        "public class Mixed {\n",
        "    private int x;\n",
        "    public int A() { return this.x; }\n",
        "    public int B() { this.x = 1; return this.C(); }\n",
        "    public int C() { return 42; }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_god_class_still_fires() {
    let out = check(concat!(
        "public class UserService {\n",
        "    private object db; private object cache; private object mailer;\n",
        "    private object events; private object audit;\n",
        "    public object GetUser() { return this.db; }\n",
        "    public object CacheUser() { return this.cache; }\n",
        "    public object SendWelcome() { return this.mailer; }\n",
        "    public object Publish() { return this.events; }\n",
        "    public object AuditLog() { return this.audit; }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"), "got: {out}");
}

#[test]
fn lcom4_dependency_method_calls_dont_falsely_connect() {
    let out = check(concat!(
        "public class Service {\n",
        "    private object db; private object cache; private object log;\n",
        "    public object A() { return this.db; }\n",
        "    public object B() { return this.cache; }\n",
        "    public object C() { return this.log; }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"));
}

// ===========================================================================
// Method naming tests
// ===========================================================================

#[test]
fn class_method_has_class_prefix() {
    let out =
        debug(concat!("public class Calculator {\n", "    public int Add(int a, int b) { return a + b; }\n", "}\n",));
    assert!(out.contains("Calculator.Add"), "method should be Calculator.Add, got: {out}");
}

#[test]
fn interface_method_has_prefix() {
    let out = debug(concat!("public interface IService {\n", "    int Process(int x) { return x; }\n", "}\n",));
    assert!(out.contains("IService.Process"), "method should be IService.Process, got: {out}");
}

#[test]
fn nested_class_methods() {
    let out = debug(concat!(
        "public class Outer {\n",
        "    public class Inner {\n",
        "        public void Work() {}\n",
        "    }\n",
        "    public void OuterWork() {}\n",
        "}\n",
    ));
    assert!(out.contains("Inner.Work"), "nested class method should be Inner.Work, got: {out}");
    assert!(out.contains("Outer.OuterWork"), "outer method should be Outer.OuterWork, got: {out}");
}

// ===========================================================================
// Performance test
// ===========================================================================

#[test]
fn performance_1000_loc() {
    let mut code = String::from("public class Big {\n");
    for i in 0..50 {
        code.push_str(&format!("    static int Func{i}(int data) {{\n"));
        for j in 0..18 {
            code.push_str(&format!("        int f{j} = data + {j};\n"));
        }
        code.push_str("        return data;\n    }\n\n");
    }
    code.push_str("}\n");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Big.cs");
    std::fs::write(&path, &code).unwrap();
    let start = std::time::Instant::now();
    let _ = std::process::Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let elapsed = start.elapsed();
    assert!(elapsed.as_millis() < 500, "took: {}ms", elapsed.as_millis());
}

// ===========================================================================
// Global scope tests
// ===========================================================================

#[test]
fn global_conditional_not_detected_in_top_level() {
    let out = check(concat!(
        "if (true) {\n",
        "    if (true) {\n",
        "        if (true) {\n",
        "            if (true) {\n",
        "                int x = 1;\n",
        "            }\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    assert!(
        !has_smell(&out, "Global Conditionals") && !has_smell(&out, "Deep Global Nesting"),
        "top-level statements in C# are wrapped in global_statement nodes, got: {out}"
    );
}

#[test]
fn shallow_global_if_not_flagged() {
    let out = check("if (true) { int x = 1; }\n");
    assert!(!has_smell(&out, "Deep Global Nesting"), "shallow global if should not fire, got: {out}");
}

// ===========================================================================
// CC: additional precision tests
// ===========================================================================

#[test]
fn cc_nested_if_in_for() {
    let out = debug(concat!(
        "public class T {\n",
        "    static void f() {\n",
        "        for (int i = 0; i < 10; i++) {\n",
        "            if (i > 5) {}\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_else_if_chain() {
    let out = debug(concat!(
        "public class T {\n",
        "    static void f(int x) {\n",
        "        if (x == 1) {} else if (x == 2) {} else if (x == 3) {}\n",
        "    }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cc"), Some(4));
}

#[test]
fn cc_switch_many_cases() {
    let out = debug(concat!(
        "public class T {\n",
        "    static string f(int x) {\n",
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
    assert!(cc >= 9, "8 cases + base >= 9, got: {cc}");
}

#[test]
fn cc_counts_not_operator() {
    let out =
        debug(
            concat!("public class T {\n", "    static void f(bool a) {\n", "        if (!a) {}\n", "    }\n", "}\n",),
        );
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(cc >= 2, "if(!a) should have cc >= 2, got: {cc}");
}

// ===========================================================================
// Nesting: additional precision tests
// ===========================================================================

#[test]
fn nesting_0_flat() {
    let out = debug("public class T { static int f() { return 1; } }");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(0));
}

#[test]
fn nesting_switch_counts_depth() {
    let out = debug(concat!(
        "public class T {\n",
        "    static void f(int x) {\n",
        "        switch (x) {\n",
        "            case 1:\n",
        "                if (x > 0) {}\n",
        "                break;\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    let depth = function_metric(&out, "f", "nesting").unwrap_or(0);
    assert!(depth >= 2, "switch+if >= 2, got: {depth}");
}

#[test]
fn nesting_deep_for_if_for() {
    let out = debug(concat!(
        "public class T {\n",
        "    static void f() {\n",
        "        if (true) {\n",
        "            for (int i = 0; i < 1; i++) {\n",
        "                if (true) {\n",
        "                    for (int j = 0; j < 1; j++) {}\n",
        "                }\n",
        "            }\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    let depth = function_metric(&out, "f", "nesting").unwrap_or(0);
    assert!(depth >= 4, "got: {depth}");
}

// ===========================================================================
// Args: additional precision tests
// ===========================================================================

#[test]
fn args_typed_params() {
    let out = debug("public class T { static void f(string a, int b, bool c) {} }");
    assert_eq!(function_metric(&out, "f", "args"), Some(3));
}

#[test]
fn args_positional() {
    let out = debug("public class T { static void f(int a, int b, int c) {} }");
    assert_eq!(function_metric(&out, "f", "args"), Some(3));
}

// ===========================================================================
// Primitive obsession: additional precision tests
// ===========================================================================

#[test]
fn primitive_obsession_recognizes_float_decimal() {
    let out = check("public class T { static void f(long a, float b, double c, int d, long e) {} }");
    assert!(has_smell(&out, "Primitive Obsession"), "got: {out}");
}

#[test]
fn primitive_obsession_complex_types_not_flagged() {
    let out = check("public class T { static void f(MyList a, string b, MyObj c, OtherObj d) {} }");
    assert!(!has_smell(&out, "Primitive Obsession"), "mostly non-primitive should not fire, got: {out}");
}

// ===========================================================================
// LCOM4: additional precision tests
// ===========================================================================

#[test]
fn lcom4_single_method_not_flagged() {
    let out = check(concat!(
        "public class T {\n",
        "    private int x;\n",
        "    public int Get() { return this.x; }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"), "single method should not fire, got: {out}");
}

#[test]
fn lcom4_three_groups_flagged_with_field_tracking() {
    let out = check(concat!(
        "public class Split {\n",
        "    private int a;\n",
        "    private int b;\n",
        "    private int c;\n",
        "    public int AWork() { return this.a; }\n",
        "    public int ARead() { return this.a + 1; }\n",
        "    public int BWork() { return this.b; }\n",
        "    public int BRead() { return this.b + 1; }\n",
        "    public int CWork() { return this.c; }\n",
        "    public int CRead() { return this.c + 1; }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"), "3 disjoint field groups: a, b, c, got: {out}");
}

#[test]
fn lcom4_transitive_connected_not_flagged() {
    let out = check(concat!(
        "public class C {\n",
        "    private int a;\n",
        "    private int b;\n",
        "    public int M1() { return this.a; }\n",
        "    public int M2() { return this.a + this.b; }\n",
        "    public int M3() { return this.b; }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"), "M2 bridges a and b transitively, got: {out}");
}

// ===========================================================================
// Constructor vs excess args precision
// ===========================================================================

#[test]
fn constructor_reports_injection_not_excess() {
    let out = check(concat!(
        "public class S {\n",
        "    public S(int a, int b, int c, int d, int e, int f, int g, int h, int i) {}\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Constructor Over-Injection"), "got: {out}");
    let lines: Vec<&str> = out.lines().filter(|l| l.contains("S(") || l.contains(".S")).collect();
    assert!(!lines.iter().any(|l| l.contains("Excess Arguments")));
}

#[test]
fn regular_method_reports_excess_args() {
    let out = check("public class T { static void f(int a, int b, int c, int d, int e, int f, int g) {} }");
    assert!(has_smell(&out, "Excess Arguments"), "got: {out}");
    assert!(!has_smell(&out, "Constructor Over-Injection"), "got: {out}");
}

// ===========================================================================
// Multiple smells same function
// ===========================================================================

#[test]
fn multiple_smells_same_function() {
    let mut code = String::from(
        "public class T {\n    static void Bad(int a, int b, int c, int d, int e, int f, int g, int h) {\n",
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
    assert!(has_smell(&out, "Excess Arguments"), "got: {out}");
    assert!(has_smell(&out, "Deep Nested"), "got: {out}");
}

// ===========================================================================
// Duplication: additional precision tests
// ===========================================================================

#[test]
fn duplication_two_is_minimum() {
    let out = check(concat!(
        "public class T {\n",
        "    static int RptA(int[] d) {\n",
        "        int r = 0;\n",
        "        foreach (var v in d) { r += v; }\n",
        "        r = r * 2;\n",
        "        return r;\n",
        "    }\n",
        "    static int RptB(int[] d) {\n",
        "        int r = 0;\n",
        "        foreach (var v in d) { r += v; }\n",
        "        r = r * 2;\n",
        "        return r;\n",
        "    }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Code Duplication"), "got: {out}");
}

#[test]
fn duplication_mixed_test_and_prod_flagged() {
    let out = check(concat!(
        "public class T {\n",
        "    static void test_compute() {\n",
        "        int r = 0;\n",
        "        for (int i = 0; i < 10; i++) { r += i; }\n",
        "        r = r * 2;\n",
        "        System.Console.WriteLine(r);\n",
        "    }\n",
        "    static void ProcessData() {\n",
        "        int r = 0;\n",
        "        for (int i = 0; i < 10; i++) { r += i; }\n",
        "        r = r * 2;\n",
        "        System.Console.WriteLine(r);\n",
        "    }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Code Duplication"), "mixed test+prod should be flagged, got: {out}");
}

// ===========================================================================
// God class tests
// ===========================================================================

#[test]
fn god_class_requires_god_method() {
    let mut code = String::from("public class Big {\n");
    for i in 0..declarations_above() {
        code.push_str(&format!("    static int Fn{i}() {{ return {i}; }}\n"));
    }
    code.push_str("}\n");
    for i in 0..file_padding() {
        code.push_str(&format!("// padding {i}\n"));
    }
    let out = check(&code);
    assert!(!has_smell(&out, "God Class"), "no god method, should not fire, got: {out}");
}

#[test]
fn god_class_triggers_with_god_method() {
    let mut code = String::from("public class Monster {\n    static void DoMonster() {\n");
    for i in 0..cc_branches() {
        code.push_str(&format!("        if ({i} > 0) {{}}\n"));
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("        int y{i} = {i};\n"));
    }
    code.push_str("    }\n");
    for i in 0..functions_above() {
        code.push_str(&format!("    static int Fn{i}() {{ return {i}; }}\n"));
    }
    for i in 0..file_padding() {
        code.push_str(&format!("    static readonly int V{i} = {i};\n"));
    }
    code.push_str("}\n");
    let out = check(&code);
    assert!(has_smell(&out, "God Method"), "got: {out}");
    assert!(has_smell(&out, "God Class"), "got: {out}");
}

// ===========================================================================
// Overall function size tests
// ===========================================================================

#[test]
fn overall_function_size_below_threshold() {
    let mut code = String::from("public class T {\n");
    for i in 0..(t().module.large_fn_count as usize - 1) {
        code.push_str(&format!("    static void Lg{i}() {{\n"));
        for j in 0..large_fn_lines() {
            code.push_str(&format!("        int x{j} = {j};\n"));
        }
        code.push_str("    }\n");
    }
    code.push_str("}\n");
    let out = check(&code);
    assert!(!has_smell(&out, "Overall Function Size"), "2 large fns below threshold=3, got: {out}");
}

#[test]
fn overall_function_size_at_threshold() {
    let mut code = String::from("public class T {\n");
    for i in 0..t().module.large_fn_count as usize {
        code.push_str(&format!("    static void Lg{i}() {{\n"));
        for j in 0..large_fn_lines() {
            code.push_str(&format!("        int x{j} = {j};\n"));
        }
        code.push_str("    }\n");
    }
    code.push_str("}\n");
    let out = check(&code);
    assert!(has_smell(&out, "Overall Function Size"), "3 large fns should fire, got: {out}");
}

// ===========================================================================
// Declarations tests
// ===========================================================================

#[test]
fn declarations_below_threshold() {
    let mut code = String::new();
    for i in 0..10 {
        code.push_str(&format!("public class T{i} {{}}\n"));
    }
    let out = check(&code);
    assert!(!has_smell(&out, "Declarations"), "10 below threshold=20, got: {out}");
}

#[test]
fn declarations_above_threshold() {
    let mut code = String::new();
    for i in 0..declarations_above() {
        code.push_str(&format!("public class T{i} {{}}\n"));
    }
    let out = check(&code);
    assert!(has_smell(&out, "Declarations"), "25 above threshold=20, got: {out}");
}

// ===========================================================================
// Deep nesting with switch
// ===========================================================================

#[test]
fn deep_nesting_with_switch() {
    let out = check(concat!(
        "public class T {\n",
        "    static void Deep(int x) {\n",
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
    assert!(has_smell(&out, "Deep Nested"), "got: {out}");
}

// ===========================================================================
// Nested conditional chunks
// ===========================================================================

#[test]
fn nested_conditional_chunks_detected() {
    let out = check(concat!(
        "public class T {\n",
        "    static void Validate(int[] data) {\n",
        "        if (data.Length > 0) {\n",
        "            if (data[0] > 0) {\n",
        "                if (data[0] > 10) {\n",
        "                    int x = 1;\n",
        "                }\n",
        "            }\n",
        "        }\n",
        "        int gap = 1;\n",
        "        if (data.Length > 5) {\n",
        "            if (data[0] > 0) {\n",
        "                if (data[0] > 10) {\n",
        "                    int y = 2;\n",
        "                }\n",
        "            }\n",
        "        }\n",
        "        int gap2 = 2;\n",
        "        if (data.Length > 10) {\n",
        "            if (data[0] > 0) {\n",
        "                if (data[0] > 10) {\n",
        "                    int z = 3;\n",
        "                }\n",
        "            }\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Nested Conditional Chunks") || has_smell(&out, "Complex Method"), "got: {out}");
}

// ===========================================================================
// Output format tests
// ===========================================================================

#[test]
fn output_starts_with_pulse() {
    let out = check("public class T { static void f(int a, int b, int c, int d, int e, int f, int g) {} }");
    assert!(out.starts_with("pulse:"), "got: {out}");
}

#[test]
fn output_has_line_numbers() {
    let out = check("public class T { static void f(int a, int b, int c, int d, int e, int f, int g) {} }");
    let has_loc = out.lines().any(|l| l.contains("(L") && l.contains("): "));
    assert!(has_loc, "got: {out}");
}

#[test]
fn excess_args_count_verified() {
    let out = debug("public class T { static void f(int a, int b, int c, int d, int e, int f, int g, int h) {} }");
    assert_eq!(function_metric(&out, "f", "args"), Some(8));
}

// ===========================================================================
// CogC: additional tests
// ===========================================================================

#[test]
fn cogc_foreach_nested() {
    let out = debug(concat!(
        "public class T {\n",
        "    static void f(bool x, int[] items) {\n",
        "        if (x) {\n",
        "            foreach (var item in items) {}\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cogc"), Some(3));
}

// ===========================================================================
// Empty error handler: additional tests
// ===========================================================================

#[test]
fn multiple_empty_catches() {
    let out = check(concat!(
        "public class T {\n",
        "    static void f() {\n",
        "        try { int x = 1; } catch (System.Exception e) {}\n",
        "        try { int y = 2; } catch (System.Exception e) {}\n",
        "    }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Empty Error Handler"), "got: {out}");
    assert!(out.contains("2 empty catch blocks"), "got: {out}");
}

#[test]
fn no_try_catch_no_smell() {
    let out = check("public class T { static void f() { int x = 1; } }");
    assert!(!has_smell(&out, "Empty Error Handler"), "got: {out}");
}

// ===========================================================================
// Clean code and edge cases
// ===========================================================================

#[test]
fn clean_csharp_module_not_flagged() {
    let out = check(concat!(
        "public class Config {\n",
        "    private string host;\n",
        "    private int port;\n",
        "    public Config(string host, int port) {\n",
        "        this.host = host;\n",
        "        this.port = port;\n",
        "    }\n",
        "    public string Address() {\n",
        "        return this.host + \":\" + this.port;\n",
        "    }\n",
        "}\n",
    ));
    assert!(out.is_empty(), "clean code should not be flagged, got: {out}");
}

#[test]
fn comments_only() {
    let out = check("// just comments\n// only\n");
    assert!(out.is_empty(), "got: {out}");
}

#[test]
fn empty_file() {
    let out = check("");
    assert!(out.is_empty(), "got: {out}");
}

// ===========================================================================
// Performance: class hierarchy
// ===========================================================================

#[test]
fn performance_class_hierarchy() {
    let mut code = String::new();
    for i in 0..10 {
        code.push_str(&format!("public class S{i} {{\n    private int d{i};\n"));
        for j in 0..5 {
            code.push_str(&format!("    public int M{j}() {{ return this.d{i}; }}\n"));
        }
        code.push_str("}\n\n");
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Classes.cs");
    std::fs::write(&path, &code).unwrap();
    let start = std::time::Instant::now();
    let _ = std::process::Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let elapsed = start.elapsed();
    assert!(elapsed.as_millis() < 500, "took: {}ms", elapsed.as_millis());
}

// ===========================================================================
// Issue count matches
// ===========================================================================

#[test]
fn issue_count_matches() {
    let out = check("public class T { static void f(int a, int b, int c, int d, int e, int f, int g) {} }");
    let first = out.lines().next().unwrap_or("");
    let findings = out.lines().filter(|l| l.starts_with("  ")).count();
    assert!(first.contains(&format!("{findings} issue")), "got: {out}");
}

// ===========================================================================
// Shallow global not flagged
// ===========================================================================

#[test]
fn shallow_global_not_flagged() {
    let out = check("public class T { static void f() {} }");
    assert!(!has_smell(&out, "Deep Global Nesting"), "got: {out}");
}

// ===========================================================================
// Module prefix
// ===========================================================================

#[test]
fn output_has_module_prefix() {
    let mut code = String::from("public class Big {\n");
    for i in 0..declarations_above() {
        code.push_str(&format!("    static int Fn{i}() {{ return {i}; }}\n"));
    }
    code.push_str("}\n");
    for i in 0..file_padding() {
        code.push_str(&format!("// line {i}\n"));
    }
    let out = check(&code);
    assert!(out.contains("Module:"), "got: {out}");
}
