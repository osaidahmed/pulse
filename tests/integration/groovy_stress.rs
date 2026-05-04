
use crate::common::*;

lang_helpers!("groovy");

// ===========================================================================
// CC counting
// ===========================================================================

#[test]
fn cc_base() {
    let d = debug("class T { void f() { int x = 1 } }\n");
    assert_eq!(function_metric(&d, "f", "cc"), Some(1));
}

#[test]
fn cc_if() {
    let d = debug("class T { void f(int x) { if (x > 0) {} } }\n");
    assert_eq!(function_metric(&d, "f", "cc"), Some(2));
}

#[test]
fn cc_else_if() {
    let d = debug("class T { void f(int x) { if (x > 0) {} else if (x < 0) {} } }\n");
    assert_eq!(function_metric(&d, "f", "cc"), Some(3));
}

#[test]
fn cc_else_no_increment() {
    let d = debug("class T { void f(int x) { if (x > 0) {} else {} } }\n");
    assert_eq!(function_metric(&d, "f", "cc"), Some(2));
}

#[test]
fn cc_for() {
    let d = debug("class T { void f() { for (int i = 0; i < 10; i++) {} } }\n");
    assert_eq!(function_metric(&d, "f", "cc"), Some(2));
}

#[test]
fn cc_enhanced_for() {
    let d = debug("class T { void f(int[] data) { for (int item : data) {} } }\n");
    assert_eq!(function_metric(&d, "f", "cc"), Some(2));
}

#[test]
fn cc_while() {
    let d = debug("class T { void f(int x) { while (x > 0) { x = x - 1 } } }\n");
    assert_eq!(function_metric(&d, "f", "cc"), Some(2));
}

#[test]
fn cc_do_while() {
    let d = debug("class T { void f(int x) { do { x = x + 1 } while (x < 10) } }\n");
    assert_eq!(function_metric(&d, "f", "cc"), Some(2));
}

#[test]
fn cc_switch_cases() {
    let d = debug("class T { int f(int x) { switch (x) { case 1: return 1; case 2: return 2; case 3: return 3; default: return 0 } } }\n");
    assert_eq!(function_metric(&d, "f", "cc"), Some(4));
}

#[test]
fn cc_catch() {
    let d = debug("class T { void f() { try { int x = 1 } catch (Exception e) { int y = 2 } } }\n");
    assert_eq!(function_metric(&d, "f", "cc"), Some(2));
}

#[test]
fn cc_finally_no_increment() {
    let d = debug("class T { void f() { try { int x = 1 } finally { int y = 2 } } }\n");
    assert_eq!(function_metric(&d, "f", "cc"), Some(1));
}

#[test]
fn cc_and() {
    let d = debug("class T { boolean f(boolean a, boolean b) { if (a && b) { return true } return false } }\n");
    assert_eq!(function_metric(&d, "f", "cc"), Some(3));
}

#[test]
fn cc_or() {
    let d = debug("class T { boolean f(boolean a, boolean b) { if (a || b) { return true } return false } }\n");
    assert_eq!(function_metric(&d, "f", "cc"), Some(3));
}

#[test]
fn cc_chained_boolean() {
    let d = debug("class T { boolean f(boolean a, boolean b, boolean c) { if (a && b || c) { return true } return false } }\n");
    assert_eq!(function_metric(&d, "f", "cc"), Some(4));
}

#[test]
fn cc_chained_boolean_4way() {
    let d = debug("class T { boolean f(boolean a, boolean b, boolean c, boolean d, boolean e) { if (a && b || c && d || e) { return true } return false } }\n");
    assert_eq!(function_metric(&d, "f", "cc"), Some(6));
}

#[test]
fn cc_nested_if_counted_once() {
    let d = debug("class T { void f(int x, int y) { if (x > 0) { if (y > 0) {} } } }\n");
    assert_eq!(function_metric(&d, "f", "cc"), Some(3));
}

#[test]
fn cc_switch_default_not_counted() {
    let d = debug("class T { int f(int x) { switch (x) { case 1: return 1; default: return 0 } } }\n");
    assert_eq!(function_metric(&d, "f", "cc"), Some(2));
}

#[test]
fn cc_ternary() {
    let d = debug("class T { int f(int x) { int r = (x > 0) ? 1 : 0; return r } }\n");
    assert_eq!(function_metric(&d, "f", "cc"), Some(2));
}

// ===========================================================================
// CC precision
// ===========================================================================

#[test]
fn cc_nested_if_in_for() {
    let d = debug(concat!(
        "class T {\n",
        "  void f() {\n",
        "    for (int i = 0; i < 10; i++) {\n",
        "      if (i > 5) {}\n",
        "    }\n",
        "  }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&d, "f", "cc"), Some(3));
}

#[test]
fn cc_else_if_chain() {
    let d = debug(concat!(
        "class T {\n",
        "  void f(int x) {\n",
        "    if (x == 1) {} else if (x == 2) {} else if (x == 3) {}\n",
        "  }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&d, "f", "cc"), Some(4));
}

#[test]
fn cc_switch_many_cases() {
    let d = debug(concat!(
        "class T {\n",
        "  String f(int x) {\n",
        "    switch (x) {\n",
        "      case 1: return \"a\"\n",
        "      case 2: return \"b\"\n",
        "      case 3: return \"c\"\n",
        "      case 4: return \"d\"\n",
        "      case 5: return \"e\"\n",
        "      case 6: return \"f\"\n",
        "      case 7: return \"g\"\n",
        "      case 8: return \"h\"\n",
        "      default: return \"?\"\n",
        "    }\n",
        "  }\n",
        "}\n",
    ));
    let cc = function_metric(&d, "f", "cc").unwrap();
    assert!(cc >= 9, "8 cases + base >= 9, got: {cc}");
}

#[test]
fn cc_not_operator() {
    let d = debug(concat!(
        "class T {\n",
        "  void f(boolean a) {\n",
        "    if (!a) {}\n",
        "  }\n",
        "}\n",
    ));
    let cc = function_metric(&d, "f", "cc").unwrap();
    assert!(cc >= 2, "got: {cc}");
}

#[test]
fn cc_multiple_catch_blocks() {
    let d = debug("class T { void f() { try { int x = 1 } catch (RuntimeException e) { int y = 2 } catch (Exception e) { int z = 3 } } }\n");
    assert_eq!(function_metric(&d, "f", "cc"), Some(3));
}

// ===========================================================================
// Cognitive complexity
// ===========================================================================

#[test]
fn cogc_flat_branches() {
    let d = debug(concat!(
        "class T {\n",
        "  void f(int a, int b, int c, int d, int e) {\n",
        "    if (a > 0) {}\n",
        "    if (b > 0) {}\n",
        "    if (c > 0) {}\n",
        "    if (d > 0) {}\n",
        "    if (e > 0) {}\n",
        "  }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&d, "f", "cogc"), Some(5));
}

#[test]
fn cogc_nested_ifs() {
    let d = debug(concat!(
        "class T {\n",
        "  void f(int a, int b, int c) {\n",
        "    if (a > 0) {\n",
        "      if (b > 0) {\n",
        "        if (c > 0) {}\n",
        "      }\n",
        "    }\n",
        "  }\n",
        "}\n",
    ));
    let cogc = function_metric(&d, "f", "cogc").unwrap_or(0);
    assert_eq!(cogc, 6, "1+(1+1)+(1+2) = 6, got: {cogc}");
}

#[test]
fn cogc_else_if_no_nesting() {
    let d = debug(concat!(
        "class T {\n",
        "  void f(int x) {\n",
        "    if (x > 0) {}\n",
        "    else if (x < 0) {}\n",
        "    else if (x == 0) {}\n",
        "  }\n",
        "}\n",
    ));
    let cogc = function_metric(&d, "f", "cogc").unwrap_or(99);
    assert_eq!(cogc, 3, "else-if chain is flat, got: {cogc}");
}

#[test]
fn cogc_else_increases() {
    let d = debug("class T { void f(int x) { if (x > 0) {} else {} } }\n");
    let cogc = function_metric(&d, "f", "cogc").unwrap_or(0);
    assert_eq!(cogc, 2, "if=1 + else=1, got: {cogc}");
}

#[test]
fn cogc_for_nested() {
    let d = debug(concat!(
        "class T {\n",
        "  void f(int x) {\n",
        "    for (int i = 0; i < 10; i++) {\n",
        "      if (x > 0) {}\n",
        "    }\n",
        "  }\n",
        "}\n",
    ));
    let cogc = function_metric(&d, "f", "cogc").unwrap_or(0);
    assert_eq!(cogc, 3, "for=1, nested if=1+1, got: {cogc}");
}

#[test]
fn cogc_enhanced_for_nested() {
    let d = debug(concat!(
        "class T {\n",
        "  void f(int[] data) {\n",
        "    for (int item : data) {\n",
        "      if (item > 0) {}\n",
        "    }\n",
        "  }\n",
        "}\n",
    ));
    let cogc = function_metric(&d, "f", "cogc").unwrap_or(0);
    assert_eq!(cogc, 3, "for=1, nested if=1+1, got: {cogc}");
}

#[test]
fn cogc_while_nested() {
    let d = debug(concat!(
        "class T {\n",
        "  void f(int x) {\n",
        "    while (x > 0) {\n",
        "      if (x > 5) {}\n",
        "      x = x - 1\n",
        "    }\n",
        "  }\n",
        "}\n",
    ));
    let cogc = function_metric(&d, "f", "cogc").unwrap_or(0);
    assert_eq!(cogc, 3, "while=1, nested if=1+1, got: {cogc}");
}

#[test]
fn cogc_do_while_nested() {
    let d = debug(concat!(
        "class T {\n",
        "  void f(int x) {\n",
        "    do {\n",
        "      if (x > 5) {}\n",
        "      x = x + 1\n",
        "    } while (x < 10)\n",
        "  }\n",
        "}\n",
    ));
    let cogc = function_metric(&d, "f", "cogc").unwrap_or(0);
    assert_eq!(cogc, 3, "do=1, nested if=1+1, got: {cogc}");
}

#[test]
fn cogc_switch_counted() {
    let d = debug("class T { int f(int x) { switch (x) { case 1: return 1; default: return 0 } } }\n");
    assert_eq!(function_metric(&d, "f", "cogc"), Some(1));
}

#[test]
fn cogc_boolean_single_sequence() {
    let d = debug("class T { boolean f(boolean a, boolean b, boolean c) { if (a && b && c) { return true } return false } }\n");
    let cogc = function_metric(&d, "f", "cogc").unwrap_or(0);
    assert_eq!(cogc, 2, "if=1 + 1 sequence, got: {cogc}");
}

#[test]
fn cogc_boolean_mixed_sequence() {
    let d = debug("class T { boolean f(boolean a, boolean b, boolean c) { if (a && b || c) { return true } return false } }\n");
    let cogc = function_metric(&d, "f", "cogc").unwrap_or(0);
    assert_eq!(cogc, 3, "if=1 + 2 sequences, got: {cogc}");
}

#[test]
fn cogc_catch_nesting() {
    let d = debug(concat!(
        "class T {\n",
        "  void f(int x) {\n",
        "    try {\n",
        "      if (x > 0) {}\n",
        "    } catch (Exception e) {\n",
        "      if (x < 0) {}\n",
        "    }\n",
        "  }\n",
        "}\n",
    ));
    let cogc = function_metric(&d, "f", "cogc").unwrap_or(0);
    assert!(cogc >= 4, "try body if + catch + catch body if, got: {cogc}");
}

// ===========================================================================
// Nesting depth
// ===========================================================================

#[test]
fn nesting_0_flat() {
    let d = debug("class T { void f() { int x = 1 } }\n");
    assert_eq!(function_metric(&d, "f", "nesting"), Some(0));
}

#[test]
fn nesting_1_single_if() {
    let d = debug("class T { void f(int x) { if (x > 0) {} } }\n");
    assert_eq!(function_metric(&d, "f", "nesting"), Some(1));
}

#[test]
fn nesting_2_nested_if() {
    let d = debug("class T { void f(int x, int y) { if (x > 0) { if (y > 0) {} } } }\n");
    assert_eq!(function_metric(&d, "f", "nesting"), Some(2));
}

#[test]
fn nesting_3_triple() {
    let d = debug(concat!(
        "class T {\n",
        "  void f(int x, int y) {\n",
        "    if (x > 0) {\n",
        "      for (int i = 0; i < 10; i++) {\n",
        "        if (y > 0) {}\n",
        "      }\n",
        "    }\n",
        "  }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&d, "f", "nesting"), Some(3));
}

#[test]
fn nesting_4_deep() {
    let d = debug(concat!(
        "class T {\n",
        "  void f(int x) {\n",
        "    if (x > 0) {\n",
        "      if (x > 1) {\n",
        "        if (x > 2) {\n",
        "          if (x > 3) {}\n",
        "        }\n",
        "      }\n",
        "    }\n",
        "  }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&d, "f", "nesting"), Some(4));
}

#[test]
fn nesting_sequential_not_accumulated() {
    let d = debug("class T { void f(int x) { if (x > 0) {} if (x < 0) {} } }\n");
    assert_eq!(function_metric(&d, "f", "nesting"), Some(1));
}

#[test]
fn nesting_switch_depth() {
    let d = debug(concat!(
        "class T {\n",
        "  void f(int x) {\n",
        "    switch (x) {\n",
        "      case 1:\n",
        "        if (x > 0) {}\n",
        "        break\n",
        "      default: break\n",
        "    }\n",
        "  }\n",
        "}\n",
    ));
    let n = function_metric(&d, "f", "nesting").unwrap_or(0);
    assert!(n >= 2, "switch + case if, got: {n}");
}

#[test]
fn nesting_deep_for_if_for() {
    let d = debug(concat!(
        "class T {\n",
        "  void f() {\n",
        "    if (true) {\n",
        "      for (int i = 0; i < 1; i++) {\n",
        "        if (true) {\n",
        "          for (int j = 0; j < 1; j++) {}\n",
        "        }\n",
        "      }\n",
        "    }\n",
        "  }\n",
        "}\n",
    ));
    let depth = function_metric(&d, "f", "nesting").unwrap_or(0);
    assert!(depth >= 4, "got: {depth}");
}

// ===========================================================================
// Bumpy road
// ===========================================================================

#[test]
fn bump_count_tracked() {
    let d = debug(concat!(
        "class T {\n",
        "  int f(int x, int y) {\n",
        "    if (x > 0) { if (y > 0) { return 1 } }\n",
        "    if (x < 0) { if (y < 0) { return 2 } }\n",
        "    if (x == 0) { if (y == 0) { return 0 } }\n",
        "    return -1\n",
        "  }\n",
        "}\n",
    ));
    assert!(d.contains("bumps="), "got: {d}");
}

#[test]
fn bump_flat_function_zero() {
    let d = debug("class T { void f(int x) { if (x > 0) {} if (x < 0) {} } }\n");
    let bumps = function_metric(&d, "f", "bumps").unwrap_or(99);
    assert_eq!(bumps, 0, "flat ifs should have 0 bumps, got: {bumps}");
}

// ===========================================================================
// Argument counting
// ===========================================================================

#[test]
fn args_zero() {
    let d = debug("class T { void f() { int x = 1 } }\n");
    assert_eq!(function_metric(&d, "f", "args"), Some(0));
}

#[test]
fn args_one() {
    let d = debug("class T { void f(int x) { int y = x } }\n");
    assert_eq!(function_metric(&d, "f", "args"), Some(1));
}

#[test]
fn args_five() {
    let d = debug("class T { void f(int a, int b, int c, int d, int e) { int x = 1 } }\n");
    assert_eq!(function_metric(&d, "f", "args"), Some(5));
}

#[test]
fn args_six() {
    let d = debug("class T { void f(int a, int b, int c, int d, int e, int g) { int x = 1 } }\n");
    assert_eq!(function_metric(&d, "f", "args"), Some(6));
}

#[test]
fn args_six_flagged() {
    let out = check("class T { void f(int a, int b, int c, int d, int e, int g) { int x = 1 } }\n");
    assert!(has_smell(&out, "Excess Arguments"), "got: {out}");
}

// ===========================================================================
// Args precision
// ===========================================================================

#[test]
fn args_typed_params() {
    let d = debug("class T { void f(String a, int b, boolean c) {} }\n");
    assert_eq!(function_metric(&d, "f", "args"), Some(3));
}

#[test]
fn args_untyped_params() {
    let d = debug("class T { void f(def a, def b) { println(a) } }\n");
    let args = function_metric(&d, "f", "args").unwrap_or(0);
    assert!(args >= 2, "untyped params still counted, got: {args}");
}

// ===========================================================================
// LOC counting
// ===========================================================================

#[test]
fn loc_simple() {
    let d = debug("class T {\n  void f() {\n    int x = 1\n    int y = 2\n  }\n}\n");
    assert_eq!(function_metric(&d, "f", "loc"), Some(4));
}

#[test]
fn loc_blank_lines_counted() {
    let d = debug("class T {\n  void f() {\n    int x = 1\n\n    int y = 2\n  }\n}\n");
    assert_eq!(function_metric(&d, "f", "loc"), Some(5));
}

#[test]
fn loc_comments_excluded_from_module() {
    let d = debug("// comment\nclass T { void f() { int x = 1 } }\n");
    assert!(d.contains("LOC"), "should show module LOC");
}

// ===========================================================================
// Constructor metrics
// ===========================================================================

#[test]
fn constructor_is_constructor() {
    let d = debug("class Foo { Foo(int x) { int y = x } }\n");
    assert!(d.contains("Foo.Foo"), "got: {d}");
}

#[test]
fn constructor_arg_count() {
    let d = debug("class Foo { Foo(int a, int b, int c) { int x = 1 } }\n");
    let args = function_metric(&d, "Foo.Foo", "args").unwrap_or(0);
    assert_eq!(args, 3);
}

#[test]
fn constructor_over_injection() {
    let out = check("class Foo { Foo(int a, int b, int c, int d, int e, int f, int g) { int x = 1 } }\n");
    assert!(has_smell(&out, "Constructor Over-Injection"), "got: {out}");
}

#[test]
fn regular_method_reports_excess_not_injection() {
    let out = check("class T { void f(int a, int b, int c, int d, int e, int f, int g) { int x = 1 } }\n");
    assert!(has_smell(&out, "Excess Arguments"), "got: {out}");
    assert!(!has_smell(&out, "Constructor Over-Injection"));
}

// ===========================================================================
// Constructor vs excess args precision
// ===========================================================================

#[test]
fn constructor_reports_injection_not_excess() {
    let out = check("class S { S(int a, int b, int c, int d, int e, int f) {} }\n");
    assert!(has_smell(&out, "Constructor Over-Injection"), "got: {out}");
    let lines: Vec<&str> = out.lines().filter(|l| l.contains("S(") || l.contains(".S")).collect();
    assert!(!lines.iter().any(|l| l.contains("Excess Arguments")));
}

#[test]
fn regular_method_reports_excess_args() {
    let out = check("class T { void f(int a, int b, int c, int d, int e, int f, int g) {} }\n");
    assert!(has_smell(&out, "Excess Arguments"), "got: {out}");
    assert!(!has_smell(&out, "Constructor Over-Injection"), "got: {out}");
}

// ===========================================================================
// Field access / class context
// ===========================================================================

#[test]
fn method_class_prefix() {
    let d = debug("class Foo { void bar() { int x = 1 } }\n");
    assert!(d.contains("Foo.bar"), "got: {d}");
}

#[test]
fn standalone_function_no_class() {
    let d = debug("class T { void hello() { int x = 1 } }\n");
    assert!(d.contains("hello"), "got: {d}");
}

#[test]
fn nested_class_methods_stress() {
    let d = debug(concat!(
        "class Outer {\n",
        "  class Inner {\n",
        "    void work() { int x = 1 }\n",
        "  }\n",
        "  void outerWork() { int y = 2 }\n",
        "}\n",
    ));
    assert!(d.contains("Inner.work"), "got: {d}");
    assert!(d.contains("Outer.outerWork"), "got: {d}");
}

#[test]
fn interface_method_has_prefix() {
    let d = debug(concat!(
        "interface IService {\n",
        "  int process(int x)\n",
        "}\n",
    ));
    // Interfaces with no body won't produce metrics but should not crash
    assert!(!d.is_empty(), "got: {d}");
}

// ===========================================================================
// Embedded blocks
// ===========================================================================

#[test]
fn embedded_large_gstring() {
    let mut code = String::from("class T {\n  String f() {\n    String s = \"\"\"");
    for i in 0..20 {
        code.push_str(&format!("line{i}\\n"));
    }
    code.push_str("\"\"\"\n    return s\n  }\n}\n");
    let d = debug(&code);
    let emb = function_metric(&d, "f", "embedded").unwrap_or(0);
    assert!(emb >= 1, "got: {emb}");
}

#[test]
fn embedded_small_not_flagged() {
    let out = check("class T { String f() { return \"hello\" } }\n");
    assert!(!has_smell(&out, "Large Embedded Block"));
}

// ===========================================================================
// Short variables
// ===========================================================================

#[test]
fn short_var_detected() {
    let mut code = String::from("class T {\n  void f() {\n");
    for n in 0..20 {
        code.push_str(&format!("    int var{n} = {n}\n"));
    }
    code.push_str("    int a = 1\n    int b = 2\n    int c = 3\n    int d = 4\n  }\n}\n");
    let d = debug(&code);
    let sv = function_metric(&d, "f", "short_vars").unwrap_or(0);
    assert!(sv >= 4, "got: {sv}");
}

#[test]
fn short_var_ijk_exempt() {
    let mut code = String::from("class T {\n  void f() {\n");
    for n in 0..20 {
        code.push_str(&format!("    int var{n} = {n}\n"));
    }
    code.push_str("    int i = 1\n    int j = 2\n    int k = 3\n  }\n}\n");
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
        "class T {\n",
        "  String f(String x) {\n",
        "    switch (x) {\n",
        "      case \"a\": return \"1\"\n",
        "      case \"b\": return \"2\"\n",
        "      case \"c\": return \"3\"\n",
        "      case \"d\": return \"4\"\n",
        "      case \"e\": return \"5\"\n",
        "      case \"f\": return \"6\"\n",
        "      default: return \"0\"\n",
        "    }\n",
        "  }\n",
        "}\n",
    ));
    let arms = function_metric(&d, "f", "str_match").unwrap_or(0);
    assert!(arms >= 6, "got: {arms}");
}

#[test]
fn string_switch_below_threshold() {
    let out = check(concat!(
        "class T {\n",
        "  String f(String x) {\n",
        "    switch (x) {\n",
        "      case \"a\": return \"1\"\n",
        "      case \"b\": return \"2\"\n",
        "      default: return \"0\"\n",
        "    }\n",
        "  }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Stringly-Typed"));
}

// ===========================================================================
// Empty catch
// ===========================================================================

#[test]
fn empty_catch_detected() {
    let out = check("class T { void f() { try { int x = 1 } catch (Exception e) { } } }\n");
    assert!(has_smell(&out, "Empty Error Handler"), "got: {out}");
}

#[test]
fn non_empty_catch_ok() {
    let out = check("class T { void f() { try { int x = 1 } catch (Exception e) { int y = 2 } } }\n");
    assert!(!has_smell(&out, "Empty Error Handler"));
}

#[test]
fn multiple_empty_catches() {
    let d = debug("class T { void f() { try { int x = 1 } catch (Exception e) { } try { int y = 2 } catch (Exception e) { } } }\n");
    assert!(d.contains('f'), "function should be parsed");
}

// ===========================================================================
// Duplication
// ===========================================================================

#[test]
fn exact_duplication_detected() {
    let out = check(concat!(
        "class T {\n",
        "  int funcA(int x, int y) {\n",
        "    int result = x + y\n",
        "    if (result > 100) { result = result - 50 }\n",
        "    if (result < 0) { result = 0 }\n",
        "    return result * 2\n",
        "  }\n",
        "  int funcB(int a, int b) {\n",
        "    int result = a + b\n",
        "    if (result > 100) { result = result - 50 }\n",
        "    if (result < 0) { result = 0 }\n",
        "    return result * 2\n",
        "  }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Code Duplication"), "got: {out}");
}

#[test]
fn duplication_two_is_minimum() {
    let out = check(concat!(
        "class T {\n",
        "  int rptA(int[] d) {\n",
        "    int r = 0\n",
        "    for (int v : d) { r += v }\n",
        "    r = r * 2\n",
        "    return r\n",
        "  }\n",
        "  int rptB(int[] d) {\n",
        "    int r = 0\n",
        "    for (int v : d) { r += v }\n",
        "    r = r * 2\n",
        "    return r\n",
        "  }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Code Duplication"), "got: {out}");
}

#[test]
fn duplication_mixed_test_and_prod_flagged() {
    let out = check(concat!(
        "class T {\n",
        "  void test_compute() {\n",
        "    int r = 0\n",
        "    for (int i = 0; i < 10; i++) { r += i }\n",
        "    r = r * 2\n",
        "    println(r)\n",
        "  }\n",
        "  void processData() {\n",
        "    int r = 0\n",
        "    for (int i = 0; i < 10; i++) { r += i }\n",
        "    r = r * 2\n",
        "    println(r)\n",
        "  }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Code Duplication"), "got: {out}");
}

// ===========================================================================
// Compound conditions
// ===========================================================================

#[test]
fn compound_condition_two_ops() {
    let d = debug("class T { boolean f(int a, int b, int c) { if (a > 0 && b > 0 || c > 0) { return true } return false } }\n");
    let cc_count = function_metric(&d, "f", "conditions").unwrap_or(0);
    assert!(cc_count >= 1, "got: {cc_count}");
}

#[test]
fn compound_condition_single_op() {
    let d = debug("class T { boolean f(int a, int b) { if (a > 0 && b > 0) { return true } return false } }\n");
    let cc_count = function_metric(&d, "f", "conditions").unwrap_or(99);
    assert_eq!(cc_count, 0, "single op should not count, got: {cc_count}");
}

// ===========================================================================
// Primitive obsession precision
// ===========================================================================

#[test]
fn primitive_obsession_recognizes_all_types() {
    let out = check("class T { void f(long a, float b, double c, int d) {} }\n");
    assert!(has_smell(&out, "Primitive Obsession"), "got: {out}");
}

#[test]
fn primitive_obsession_complex_types_not_flagged() {
    let out = check("class T { void f(MyList a, String b, MyObj c, OtherObj d) {} }\n");
    assert!(!has_smell(&out, "Primitive Obsession"), "got: {out}");
}

// ===========================================================================
// LCOM4 precision
// ===========================================================================

#[test]
fn lcom4_single_method_not_flagged() {
    let out = check(concat!(
        "class T {\n",
        "  int x\n",
        "  int get() { return this.x }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"), "got: {out}");
}

#[test]
fn lcom4_three_groups_flagged() {
    let out = check(concat!(
        "class Split {\n",
        "  int a\n",
        "  int b\n",
        "  int c\n",
        "  int aWork() { return this.a }\n",
        "  int aRead() { return this.a + 1 }\n",
        "  int bWork() { return this.b }\n",
        "  int bRead() { return this.b + 1 }\n",
        "  int cWork() { return this.c }\n",
        "  int cRead() { return this.c + 1 }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"), "got: {out}");
}

#[test]
fn lcom4_transitive_connected_not_flagged() {
    let out = check(concat!(
        "class C {\n",
        "  int a\n",
        "  int b\n",
        "  int m1() { return this.a }\n",
        "  int m2() { return this.a + this.b }\n",
        "  int m3() { return this.b }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"), "M2 bridges a and b, got: {out}");
}

#[test]
fn lcom4_methods_connected_by_call() {
    let out = check(concat!(
        "class Coord {\n",
        "  int state = 0\n",
        "  boolean process(int e) { return this.validate(e) && this.dispatch(e) }\n",
        "  boolean validate(int e) { return e > 0 }\n",
        "  boolean dispatch(int e) { return this.send(e) }\n",
        "  boolean send(int e) { return true }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"), "got: {out}");
}

#[test]
fn lcom4_mixed_field_and_call_connection() {
    let out = check(concat!(
        "class Mixed {\n",
        "  int x = 0\n",
        "  int a() { return this.x }\n",
        "  int b() { this.x = 1; return this.c() }\n",
        "  int c() { return 42 }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_god_class_still_fires() {
    let out = check(concat!(
        "class Svc {\n",
        "  Object db; Object cache; Object mailer; Object events; Object audit\n",
        "  Object getUser() { return this.db }\n",
        "  Object cacheUser() { return this.cache }\n",
        "  Object sendWelcome() { return this.mailer }\n",
        "  Object publish() { return this.events }\n",
        "  Object auditLog() { return this.audit }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"), "got: {out}");
}

#[test]
fn lcom4_dependency_method_calls_dont_falsely_connect() {
    let out = check(concat!(
        "class Svc {\n",
        "  Object db; Object cache; Object log\n",
        "  Object a() { return this.db }\n",
        "  Object b() { return this.cache }\n",
        "  Object c() { return this.log }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"));
}

// ===========================================================================
// God class
// ===========================================================================

#[test]
fn god_class_requires_god_method() {
    let mut code = String::from("class Big {\n");
    for i in 0..declarations_above() {
        code.push_str(&format!("  int fn{i}() {{ return {i} }}\n"));
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
    let mut code = String::from("class Monster {\n  void doMonster() {\n");
    for i in 0..cc_branches() {
        code.push_str(&format!("    if ({i} > 0) {{}}\n"));
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("    int y{i} = {i}\n"));
    }
    code.push_str("  }\n");
    for i in 0..functions_above() {
        code.push_str(&format!("  int fn{i}() {{ return {i} }}\n"));
    }
    for i in 0..file_padding() {
        code.push_str(&format!("  int v{i} = {i}\n"));
    }
    code.push_str("}\n");
    let out = check(&code);
    assert!(has_smell(&out, "God Method"), "got: {out}");
    assert!(has_smell(&out, "God Class"), "got: {out}");
}

// ===========================================================================
// Overall function size
// ===========================================================================

#[test]
fn overall_function_size_below_threshold_stress() {
    let mut code = String::from("class T {\n");
    for i in 0..(t().module.large_fn_count as usize - 1) {
        code.push_str(&format!("  void lg{i}() {{\n"));
        for j in 0..large_fn_lines() {
            code.push_str(&format!("    int x{j} = {j}\n"));
        }
        code.push_str("  }\n");
    }
    code.push_str("}\n");
    let out = check(&code);
    assert!(!has_smell(&out, "Overall Function Size"), "got: {out}");
}

#[test]
fn overall_function_size_at_threshold_stress() {
    let mut code = String::from("class T {\n");
    for i in 0..t().module.large_fn_count as usize {
        code.push_str(&format!("  void lg{i}() {{\n"));
        for j in 0..large_fn_lines() {
            code.push_str(&format!("    int x{j} = {j}\n"));
        }
        code.push_str("  }\n");
    }
    code.push_str("}\n");
    let out = check(&code);
    assert!(has_smell(&out, "Overall Function Size"), "got: {out}");
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
    assert!(!has_smell(&out, "Declarations"), "got: {out}");
}

#[test]
fn declarations_above_threshold() {
    let mut code = String::new();
    for i in 0..declarations_above() {
        code.push_str(&format!("class T{i} {{}}\n"));
    }
    let out = check(&code);
    assert!(has_smell(&out, "Declarations"), "got: {out}");
}

// ===========================================================================
// Deep nesting with switch
// ===========================================================================

#[test]
fn deep_nesting_with_switch() {
    let out = check(concat!(
        "class T {\n",
        "  void deep(int x) {\n",
        "    for (int i = 0; i < x; i++) {\n",
        "      if (i > 0) {\n",
        "        switch (i) {\n",
        "          case 1:\n",
        "            for (int j = 0; j < i; j++) {\n",
        "              if (j > 0) {}\n",
        "            }\n",
        "            break\n",
        "        }\n",
        "      }\n",
        "    }\n",
        "  }\n",
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
        "class T {\n",
        "  void validate(int[] data) {\n",
        "    if (data.length > 0) {\n",
        "      if (data[0] > 0) {\n",
        "        if (data[0] > 10) { int x = 1 }\n",
        "      }\n",
        "    }\n",
        "    int gap = 1\n",
        "    if (data.length > 5) {\n",
        "      if (data[0] > 0) {\n",
        "        if (data[0] > 10) { int y = 2 }\n",
        "      }\n",
        "    }\n",
        "    int gap2 = 2\n",
        "    if (data.length > 10) {\n",
        "      if (data[0] > 0) {\n",
        "        if (data[0] > 10) { int z = 3 }\n",
        "      }\n",
        "    }\n",
        "  }\n",
        "}\n",
    ));
    assert!(
        has_smell(&out, "Nested Conditional Chunks") || has_smell(&out, "Complex Method"),
        "got: {out}"
    );
}

// ===========================================================================
// Multiple smells same function
// ===========================================================================

#[test]
fn multiple_smells_same_function() {
    let mut code = String::from(
        "class T {\n  void bad(int a, int b, int c, int d, int e, int f, int g, int h) {\n",
    );
    code.push_str("    for (int i = 0; i < a; i++) {\n");
    code.push_str("      if (i > 0) {\n");
    code.push_str("        for (int j = 0; j < b; j++) {\n");
    code.push_str("          if (j > 0) {\n");
    code.push_str("            for (int k = 0; k < c; k++) {\n");
    code.push_str("              if (k > 0) {}\n");
    code.push_str("            }\n");
    code.push_str("          }\n");
    code.push_str("        }\n");
    code.push_str("      }\n");
    code.push_str("    }\n");
    code.push_str("  }\n}\n");
    let out = check(&code);
    assert!(has_smell(&out, "Excess Arguments"), "got: {out}");
    assert!(has_smell(&out, "Deep Nested"), "got: {out}");
}

// ===========================================================================
// Groovy-specific edge cases
// ===========================================================================

#[test]
fn closure_not_walked() {
    let d = debug("class T { void f() { def fn = { x -> if (x > 0) { if (x > 1) {} } } } }\n");
    let cc = function_metric(&d, "f", "cc").unwrap_or(99);
    assert_eq!(cc, 1, "closure body should not contribute to outer cc, got: {cc}");
}

#[test]
fn closure_in_method_arg() {
    let d = debug("class T { void f() { [1,2,3].each { if (it > 0) {} } } }\n");
    let cc = function_metric(&d, "f", "cc").unwrap_or(99);
    assert_eq!(cc, 1, "closure arg should not contribute, got: {cc}");
}

#[test]
fn gstring_template_tracked() {
    let mut code = String::from("class T {\n  String f(String name) {\n    String s = \"\"\"");
    for i in 0..25 {
        code.push_str(&format!("line {i} ${{name}}\n"));
    }
    code.push_str("\"\"\"\n    return s\n  }\n}\n");
    let d = debug(&code);
    let emb = function_metric(&d, "f", "embedded").unwrap_or(0);
    assert!(emb >= 1, "GString should be tracked, got: {emb}");
}

#[test]
fn untyped_params_not_primitive() {
    let d = debug("class T { void f(def a, def b) { println(a) } }\n");
    assert!(d.contains("primitives=0/"), "untyped should not be primitive, got: {d}");
}

#[test]
fn typed_groovy_params() {
    let d = debug("class T { void f(int a, String b) { println(a) } }\n");
    assert!(d.contains("primitives=2/2"), "got: {d}");
}

#[test]
fn mixed_typed_untyped_params() {
    let d = debug("class T { void f(int a, def b) { println(a) } }\n");
    let args = function_metric(&d, "f", "args").unwrap_or(0);
    assert_eq!(args, 2, "both params counted, got: {args}");
}

// ===========================================================================
// Clean code and edge cases
// ===========================================================================

#[test]
fn clean_class_no_smells() {
    let out = check("class Foo { void bar() { int x = 1 } }\n");
    assert!(out.is_empty(), "got: {out}");
}

#[test]
fn empty_file_no_crash() {
    let out = check("");
    assert!(out.is_empty());
}

#[test]
fn empty_method_body() {
    let d = debug("class T { void f() {} }\n");
    assert_eq!(function_metric(&d, "f", "cc"), Some(1));
}

#[test]
fn multiple_independent_functions() {
    let d = debug("class T { void f() { int x = 1 } void g() { int y = 2 } void h() { int z = 3 } }\n");
    assert!(d.contains('f'));
    assert!(d.contains('g'));
    assert!(d.contains('h'));
}

// ===========================================================================
// Performance
// ===========================================================================

#[test]
fn performance_1000_loc() {
    let mut code = String::from("class Big {\n");
    for i in 0..50 {
        code.push_str(&format!("  int func{i}(int data) {{\n"));
        for j in 0..18 {
            code.push_str(&format!("    int f{j} = data + {j}\n"));
        }
        code.push_str("    return data\n  }\n\n");
    }
    code.push_str("}\n");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Big.groovy");
    std::fs::write(&path, &code).unwrap();
    let start = std::time::Instant::now();
    let _ = std::process::Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output().expect("failed to run");
    let elapsed = start.elapsed();
    assert!(elapsed.as_millis() < 500, "took: {}ms", elapsed.as_millis());
}

#[test]
fn performance_class_hierarchy() {
    let mut code = String::new();
    for i in 0..10 {
        code.push_str(&format!("class S{i} {{\n  int d{i}\n"));
        for j in 0..5 {
            code.push_str(&format!("  int m{j}() {{ return this.d{i} }}\n"));
        }
        code.push_str("}\n\n");
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Classes.groovy");
    std::fs::write(&path, &code).unwrap();
    let start = std::time::Instant::now();
    let _ = std::process::Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output().expect("failed to run");
    let elapsed = start.elapsed();
    assert!(elapsed.as_millis() < 500, "took: {}ms", elapsed.as_millis());
}

// ===========================================================================
// Global scope
// ===========================================================================

#[test]
fn shallow_global_not_flagged() {
    let out = check("class T { void f() {} }\n");
    assert!(!has_smell(&out, "Deep Global Nesting"), "got: {out}");
}

#[test]
fn global_conditional_detected() {
    let out = check(concat!(
        "if (true) {\n",
        "  if (true) {\n",
        "    if (true) {\n",
        "      if (true) {\n",
        "        int x = 1\n",
        "      }\n",
        "    }\n",
        "  }\n",
        "}\n",
    ));
    assert!(
        has_smell(&out, "Global Conditionals") || has_smell(&out, "Deep Global Nesting"),
        "got: {out}"
    );
}

// ===========================================================================
// Output format
// ===========================================================================

#[test]
fn output_starts_with_pulse_stress() {
    let out = check("class T { void f(int a, int b, int c, int d, int e, int f, int g) {} }\n");
    assert!(out.starts_with("pulse:"), "got: {out}");
}

#[test]
fn output_has_line_numbers_stress() {
    let out = check("class T { void f(int a, int b, int c, int d, int e, int f, int g) {} }\n");
    let has_loc = out.lines().any(|l| l.contains("(L") && l.contains("): "));
    assert!(has_loc, "got: {out}");
}

#[test]
fn excess_args_count_verified() {
    let d = debug("class T { void f(int a, int b, int c, int d, int e, int f, int g, int h) {} }\n");
    assert_eq!(function_metric(&d, "f", "args"), Some(8));
}
