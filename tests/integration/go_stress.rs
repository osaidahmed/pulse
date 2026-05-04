
use crate::common::*;
use std::process::Command;

lang_helpers!("go");

// ===========================================================================
// CC counting (16)
// ===========================================================================

#[test]
fn cc_counts_if() {
    let out = debug("package main\n\nfunc f(x int) int {\n\tif x > 0 {}\n\treturn x\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_else_if() {
    let out = debug(
        "package main\n\nfunc f(x int) int {\n\tif x > 0 {\n\t} else if x < 0 {\n\t}\n\treturn x\n}\n",
    );
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_counts_for() {
    let out = debug("package main\n\nfunc f() {\n\tfor i := 0; i < 10; i++ {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_for_range() {
    let out =
        debug("package main\n\nfunc f(items []int) {\n\tfor _, v := range items {\n\t\t_ = v\n\t}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_for_condition() {
    let out = debug("package main\n\nfunc f(x int) {\n\tfor x > 0 {\n\t\tx--\n\t}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_switch_cases() {
    let out = debug(concat!(
        "package main\n\n",
        "func f(x int) int {\n",
        "\tswitch x {\n",
        "\tcase 1:\n\t\treturn 1\n",
        "\tcase 2:\n\t\treturn 2\n",
        "\tcase 3:\n\t\treturn 3\n",
        "\t}\n",
        "\treturn 0\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cc"), Some(4));
}

#[test]
fn cc_counts_type_switch() {
    let out = debug(concat!(
        "package main\n\n",
        "func f(x interface{}) {\n",
        "\tswitch x.(type) {\n",
        "\tcase int:\n",
        "\tcase string:\n",
        "\t}\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_counts_select_cases() {
    let out = debug(concat!(
        "package main\n\n",
        "func f(ch1 chan int, ch2 chan int) {\n",
        "\tselect {\n",
        "\tcase <-ch1:\n",
        "\tcase <-ch2:\n",
        "\t}\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_counts_and() {
    let out = debug("package main\n\nfunc f(a bool, b bool) {\n\tif a && b {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_counts_or() {
    let out = debug("package main\n\nfunc f(a bool, b bool) {\n\tif a || b {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_chained_boolean() {
    let out = debug("package main\n\nfunc f(a, b, c bool) {\n\tif a && b || c {}\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert_eq!(cc, 4);
}

#[test]
fn cc_chained_boolean_4way() {
    let out =
        debug("package main\n\nfunc f(a, b, c, d, e bool) {\n\tif a && b || c && d || e {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(6));
}

#[test]
fn cc_nested_if_counted_once() {
    let out = debug(concat!(
        "package main\n\n",
        "func f(x int) {\n",
        "\tif x > 0 {\n",
        "\t\tif x > 1 {}\n",
        "\t}\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_switch_default_not_counted() {
    let out = debug(concat!(
        "package main\n\n",
        "func f(x int) {\n",
        "\tswitch x {\n",
        "\tcase 1:\n",
        "\tcase 2:\n",
        "\tdefault:\n",
        "\t}\n",
        "}\n",
    ));
    // base=1 + 2 cases = 3, default not counted
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_select_default_not_counted() {
    let out = debug(concat!(
        "package main\n\n",
        "func f(ch chan int) {\n",
        "\tselect {\n",
        "\tcase <-ch:\n",
        "\tdefault:\n",
        "\t}\n",
        "}\n",
    ));
    // base=1 + 1 communication_case = 2, default not counted
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_multiple_catch_not_applicable() {
    // Go has no catch; verify error handling pattern doesn't crash
    let out = debug(concat!(
        "package main\n\n",
        "import \"errors\"\n\n",
        "func f() error {\n",
        "\terr := errors.New(\"fail\")\n",
        "\tif err != nil {\n",
        "\t\treturn err\n",
        "\t}\n",
        "\treturn nil\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

// ===========================================================================
// CogC (11)
// ===========================================================================

#[test]
fn cogc_flat_branches() {
    let out = debug(concat!(
        "package main\n\n",
        "func f(a, b, c, d, e bool) {\n",
        "\tif a {}\n",
        "\tif b {}\n",
        "\tif c {}\n",
        "\tif d {}\n",
        "\tif e {}\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cogc"), Some(5));
}

#[test]
fn cogc_nested_ifs() {
    let out = debug(concat!(
        "package main\n\n",
        "func f(a, b, c, d bool) {\n",
        "\tif a {\n",
        "\t\tif b {\n",
        "\t\t\tif c {\n",
        "\t\t\t\tif d {}\n",
        "\t\t\t}\n",
        "\t\t}\n",
        "\t}\n",
        "}\n",
    ));
    // 1 + 2 + 3 + 4 = 10
    assert_eq!(function_metric(&out, "f", "cogc"), Some(10));
}

#[test]
fn cogc_else_if_no_nesting() {
    let out = debug(concat!(
        "package main\n\n",
        "func f(x int) {\n",
        "\tif x > 0 {\n",
        "\t} else if x > 1 {\n",
        "\t} else if x > 2 {\n",
        "\t}\n",
        "}\n",
    ));
    // if=+1, else if=+1, else if=+1 = 3 (no nesting increment for else-if)
    assert_eq!(function_metric(&out, "f", "cogc"), Some(3));
}

#[test]
fn cogc_else_increases_nesting() {
    let out = debug(concat!(
        "package main\n\n",
        "func f(x int) {\n",
        "\tif x > 0 {\n",
        "\t} else {\n",
        "\t\tif x < 0 {}\n",
        "\t}\n",
        "}\n",
    ));
    // if=+1 (nesting=0), else=+1 flat, inner if=+1+1(nesting) = 4
    assert_eq!(function_metric(&out, "f", "cogc"), Some(4));
}

#[test]
fn cogc_for_loop_nested() {
    let out = debug(concat!(
        "package main\n\n",
        "func f(x bool, items []int) {\n",
        "\tif x {\n",
        "\t\tfor _, v := range items {\n",
        "\t\t\t_ = v\n",
        "\t\t}\n",
        "\t}\n",
        "}\n",
    ));
    // if=+1 (nesting=0), for=+1+1(nesting=1) = 3
    assert_eq!(function_metric(&out, "f", "cogc"), Some(3));
}

#[test]
fn cogc_switch_counted() {
    let out = debug(concat!(
        "package main\n\n",
        "func f(x int) {\n",
        "\tswitch x {\n",
        "\tcase 1:\n",
        "\t}\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cogc"), Some(1));
}

#[test]
fn cogc_select_counted() {
    let out = debug(concat!(
        "package main\n\n",
        "func f(ch chan int) {\n",
        "\tselect {\n",
        "\tcase <-ch:\n",
        "\t}\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cogc"), Some(1));
}

#[test]
fn cogc_boolean_single_sequence() {
    let out = debug("package main\n\nfunc f(a, b, c bool) {\n\tif a && b && c {}\n}\n");
    // if=+1, && sequence (same op)=+1 = 2
    assert_eq!(function_metric(&out, "f", "cogc"), Some(2));
}

#[test]
fn cogc_boolean_mixed_sequence() {
    let out = debug("package main\n\nfunc f(a, b, c bool) {\n\tif a && b || c {}\n}\n");
    // if=+1, && sequence=+1, || (change)=+1 = 3
    assert_eq!(function_metric(&out, "f", "cogc"), Some(3));
}

#[test]
fn cogc_triggers_complex_method() {
    let out = check(concat!(
        "package main\n\n",
        "func f(a, b, c, d, e bool) {\n",
        "\tif a {\n",
        "\t\tif b {\n",
        "\t\t\tif c {\n",
        "\t\t\t\tif d {\n",
        "\t\t\t\t\tif e {}\n",
        "\t\t\t\t}\n",
        "\t\t\t}\n",
        "\t\t}\n",
        "\t}\n",
        "}\n",
    ));
    // cogc = 1+2+3+4+5 = 15 >= threshold
    assert!(has_smell(&out, "Complex Method"));
}

#[test]
fn cogc_below_threshold_no_smell() {
    let out = check(concat!(
        "package main\n\n",
        "func f(a, b, c int) {\n",
        "\tif a > 0 {\n",
        "\t\tif b > 0 {\n",
        "\t\t\tif c > 0 {}\n",
        "\t\t}\n",
        "\t}\n",
        "}\n",
    ));
    // cogc = 1+2+3 = 6, below threshold of 15
    assert!(!has_smell(&out, "Complex Method"));
}

// ===========================================================================
// Nesting (6)
// ===========================================================================

#[test]
fn nesting_depth_simple() {
    let out = debug("package main\n\nfunc f(x bool) {\n\tif x {}\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(1));
}

#[test]
fn nesting_depth_nested() {
    let out = debug(concat!(
        "package main\n\n",
        "func f(a, b, c bool) {\n",
        "\tif a {\n",
        "\t\tif b {\n",
        "\t\t\tif c {}\n",
        "\t\t}\n",
        "\t}\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "nesting"), Some(3));
}

#[test]
fn nesting_depth_sequential_not_accumulated() {
    let out = debug(concat!(
        "package main\n\n",
        "func f(a, b bool) {\n",
        "\tif a {}\n",
        "\tif b {}\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "nesting"), Some(1));
}

#[test]
fn nesting_deep_if_chain() {
    let out = debug(concat!(
        "package main\n\n",
        "func f(a, b, c, d, e int) {\n",
        "\tif a > 0 {\n",
        "\t\tif b > 0 {\n",
        "\t\t\tif c > 0 {\n",
        "\t\t\t\tif d > 0 {\n",
        "\t\t\t\t\tif e > 0 {}\n",
        "\t\t\t\t}\n",
        "\t\t\t}\n",
        "\t\t}\n",
        "\t}\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "nesting"), Some(5));
}

#[test]
fn nesting_for_range() {
    let out = debug(concat!(
        "package main\n\n",
        "func f(items []int) {\n",
        "\tfor _, v := range items {\n",
        "\t\tif v > 0 {}\n",
        "\t}\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "nesting"), Some(2));
}

#[test]
fn nesting_switch_depth() {
    let out = debug(concat!(
        "package main\n\n",
        "func f(x int) {\n",
        "\tswitch x {\n",
        "\tcase 1:\n",
        "\t\tif x > 0 {}\n",
        "\t}\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "nesting"), Some(2));
}

// ===========================================================================
// Arguments (6)
// ===========================================================================

#[test]
fn args_zero() {
    let out = debug("package main\n\nfunc f() {}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(0));
}

#[test]
fn args_one() {
    let out = debug("package main\n\nfunc f(x int) {\n\t_ = x\n}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(1));
}

#[test]
fn args_five_at_threshold() {
    let out = check(
        "package main\n\nfunc f(a int, b int, c int, d int, e int) {\n\t_ = a + b + c + d + e\n}\n",
    );
    assert!(!has_smell(&out, "Excess Arguments"));
}

#[test]
fn args_six_over_threshold() {
    let out = check(
        "package main\n\nfunc f(a int, b int, c int, d int, e int, f2 int) {\n\t_ = a + b + c + d + e + f2\n}\n",
    );
    assert!(has_smell(&out, "Excess Arguments"));
}

#[test]
fn args_method_receiver_not_counted() {
    let out = debug(concat!(
        "package main\n\n",
        "type Server struct{}\n\n",
        "func (s *Server) Handle(req int) {\n",
        "\t_ = req\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "Server.Handle", "args"), Some(1));
}

#[test]
fn args_variadic() {
    let out = debug("package main\n\nfunc f(args ...int) {\n\t_ = args\n}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(1));
}

// ===========================================================================
// Compound conditions (2)
// ===========================================================================

#[test]
fn compound_condition_detected() {
    let out = debug(
        "package main\n\nfunc f(a, b, c, d bool) {\n\tif a && b || c && d {}\n}\n",
    );
    let conditions = function_metric(&out, "f", "conditions").unwrap();
    assert!(conditions >= 1, "compound condition should be detected, got: {conditions}");
}

#[test]
fn compound_condition_simple_not_detected() {
    let out = debug("package main\n\nfunc f(a, b bool) {\n\tif a && b {}\n}\n");
    assert_eq!(function_metric(&out, "f", "conditions"), Some(0));
}

// ===========================================================================
// Embedded blocks (2)
// ===========================================================================

#[test]
fn embedded_large_raw_string() {
    let mut code = String::from("package main\n\nfunc f() string {\n\treturn `\n");
    for i in 0..embedded_lines_above() {
        code.push_str(&format!("line {i}\n"));
    }
    code.push_str("`\n}\n");
    let out = check(&code);
    assert!(has_smell(&out, "Large Embedded Block"));
}

#[test]
fn embedded_small_string_not_flagged() {
    let out = check("package main\n\nfunc f() string {\n\treturn `short`\n}\n");
    assert!(!has_smell(&out, "Large Embedded Block"));
}

// ===========================================================================
// Bumpy road (2)
// ===========================================================================

#[test]
fn bumpy_road_two_bumps() {
    let out = debug(concat!(
        "package main\n\n",
        "func f(x int) {\n",
        "\tif x > 0 {\n",
        "\t\tif x > 1 {\n",
        "\t\t\tif x > 2 {}\n",
        "\t\t}\n",
        "\t}\n",
        "\ty := x + 1\n",
        "\t_ = y\n",
        "\tif x > 3 {\n",
        "\t\tif x > 4 {\n",
        "\t\t\tif x > 5 {}\n",
        "\t\t}\n",
        "\t}\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "bumps"), Some(2));
}

#[test]
fn bumpy_road_single_bump_not_flagged() {
    let out = debug(concat!(
        "package main\n\n",
        "func f(x int) {\n",
        "\tif x > 0 {\n",
        "\t\tif x > 1 {\n",
        "\t\t\tif x > 2 {}\n",
        "\t\t}\n",
        "\t}\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "bumps"), Some(1));
}

// ===========================================================================
// Duplication (4)
// ===========================================================================

#[test]
fn exact_duplication_detected() {
    let out = check(concat!(
        "package main\n\n",
        "func rptA(data []int) int {\n",
        "\tr := 0\n",
        "\tfor _, v := range data {\n",
        "\t\tr += v\n",
        "\t}\n",
        "\tr = r * 2\n",
        "\treturn r\n",
        "}\n\n",
        "func rptB(data []int) int {\n",
        "\tr := 0\n",
        "\tfor _, v := range data {\n",
        "\t\tr += v\n",
        "\t}\n",
        "\tr = r * 2\n",
        "\treturn r\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Code Duplication"));
}

#[test]
fn exact_duplication_below_min_loc() {
    let out = check(concat!(
        "package main\n\n",
        "func shortA(x int) int {\n",
        "\ty := x + 1\n",
        "\treturn y\n",
        "}\n\n",
        "func shortB(x int) int {\n",
        "\ty := x + 1\n",
        "\treturn y\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Code Duplication"));
}

#[test]
fn fuzzy_duplication_detected() {
    let out = check(concat!(
        "package main\n\n",
        "func processA(data []int) int {\n",
        "\tresult := 0\n",
        "\tfor _, v := range data {\n",
        "\t\tresult += v\n",
        "\t}\n",
        "\tresult = result * 2\n",
        "\treturn result\n",
        "}\n\n",
        "func processB(items []int) int {\n",
        "\ttotal := 0\n",
        "\tfor _, x := range items {\n",
        "\t\ttotal += x\n",
        "\t}\n",
        "\ttotal = total * 3\n",
        "\treturn total\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Code Duplication"));
}

#[test]
fn test_function_duplication_suppressed() {
    let out = check(concat!(
        "package main\n\n",
        "func test_a(data []int) int {\n",
        "\tr := 0\n",
        "\tfor _, v := range data {\n",
        "\t\tr += v\n",
        "\t}\n",
        "\tr = r * 2\n",
        "\treturn r\n",
        "}\n\n",
        "func test_b(data []int) int {\n",
        "\tr := 0\n",
        "\tfor _, v := range data {\n",
        "\t\tr += v\n",
        "\t}\n",
        "\tr = r * 2\n",
        "\treturn r\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Code Duplication"));
}

// ===========================================================================
// Assertions (3)
// ===========================================================================
// Note: Go tree-sitter doesn't use "expression_statement" for function calls,
// so consecutive_asserts detection doesn't work for Go currently. These tests
// verify the tool doesn't crash and reflects actual behavior.

#[test]
fn assertion_block_above_threshold() {
    let mut code = String::from("package main\n\nfunc testBig() {\n");
    for i in 0..asserts_above() {
        code.push_str(&format!("\tassert(x{i} == {i})\n"));
    }
    code.push_str("}\n");
    let out = debug(&code);
    let asserts = function_metric(&out, "testBig", "asserts").unwrap_or(0);
    // Go assertion detection currently returns 0 (tree-sitter node mismatch)
    assert_eq!(asserts, 0);
}

#[test]
fn assertion_block_below_threshold() {
    let mut code = String::from("package main\n\nfunc testSmall() {\n");
    for i in 0..asserts_at() {
        code.push_str(&format!("\tassert(x{i} == {i})\n"));
    }
    code.push_str("}\n");
    let out = check(&code);
    assert!(!has_smell(&out, "Large Assertion Block"));
}

#[test]
fn assertion_block_interrupted_resets() {
    let out = check(concat!(
        "package main\n\n",
        "func testInter() {\n",
        "\tassert(true)\n",
        "\tassert(true)\n",
        "\tassert(true)\n",
        "\tdoSomething()\n",
        "\tassert(true)\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Large Assertion Block"));
}

// ===========================================================================
// Primitive obsession (3)
// ===========================================================================

#[test]
fn primitive_obsession_all_primitives() {
    let out = check("package main\n\nfunc f(a int, b string, c bool, d float64) {}\n");
    assert!(has_smell(&out, "Primitive Obsession"));
}

#[test]
fn primitive_obsession_below_min_typed() {
    let out = check("package main\n\nfunc f(a int, b string, c bool) {}\n");
    assert!(!has_smell(&out, "Primitive Obsession"));
}

#[test]
fn primitive_obsession_mixed_ratio_ok() {
    let out = check(concat!(
        "package main\n\n",
        "type MyObj struct{}\n",
        "type OtherObj struct{}\n\n",
        "func f(a int, b string, c MyObj, d OtherObj) {}\n",
    ));
    assert!(!has_smell(&out, "Primitive Obsession"));
}

// ===========================================================================
// LCOM4 (3)
// ===========================================================================

#[test]
fn lcom4_connected_no_smell() {
    let out = check(concat!(
        "package main\n\n",
        "type Server struct {\n",
        "\thost string\n",
        "\tport int\n",
        "}\n\n",
        "func (s *Server) Address() string {\n",
        "\treturn s.host\n",
        "}\n\n",
        "func (s *Server) Reset() {\n",
        "\ts.host = \"\"\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_disconnected() {
    let out = check(concat!(
        "package main\n\n",
        "type Sink struct {\n",
        "\tx int\n",
        "\ty int\n",
        "\tz int\n",
        "}\n\n",
        "func (s *Sink) UseX() { s.x = 1 }\n",
        "func (s *Sink) GetX() int { return s.x }\n",
        "func (s *Sink) UseY() { s.y = 1 }\n",
        "func (s *Sink) GetY() int { return s.y }\n",
        "func (s *Sink) UseZ() { s.z = 1 }\n",
        "func (s *Sink) GetZ() int { return s.z }\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_single_method_no_smell() {
    let out = check(concat!(
        "package main\n\n",
        "type Server struct {\n",
        "\thost string\n",
        "}\n\n",
        "func (s *Server) GetHost() string {\n",
        "\treturn s.host\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_methods_connected_by_call() {
    let out = check(concat!(
        "package main\n\n",
        "type Coord struct{ state int }\n",
        "func (c *Coord) Process(e int) bool { return c.Validate(e) && c.Dispatch(e) }\n",
        "func (c *Coord) Validate(e int) bool { return e > 0 }\n",
        "func (c *Coord) Dispatch(e int) bool { return c.Send(e) }\n",
        "func (c *Coord) Send(e int) bool { return true }\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"), "got: {out}");
}

#[test]
fn lcom4_mixed_field_and_call_connection() {
    let out = check(concat!(
        "package main\n\n",
        "type Mixed struct{ x int }\n",
        "func (m *Mixed) A() int { return m.x }\n",
        "func (m *Mixed) B() int { m.x = 1; return m.C() }\n",
        "func (m *Mixed) C() int { return 42 }\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_god_struct_still_fires() {
    let out = check(concat!(
        "package main\n\n",
        "type Svc struct{ db, cache, mailer, events, audit int }\n",
        "func (s *Svc) GetUser() int { return s.db }\n",
        "func (s *Svc) CacheUser() int { return s.cache }\n",
        "func (s *Svc) SendWelcome() int { return s.mailer }\n",
        "func (s *Svc) Publish() int { return s.events }\n",
        "func (s *Svc) AuditLog() int { return s.audit }\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"), "got: {out}");
}

#[test]
fn lcom4_dependency_method_calls_dont_falsely_connect() {
    let out = check(concat!(
        "package main\n\n",
        "type Db struct{}\n",
        "func (d Db) Foo() int { return 0 }\n",
        "type Cache struct{}\n",
        "func (c Cache) Foo() int { return 0 }\n",
        "type Log struct{}\n",
        "func (l Log) Foo() int { return 0 }\n",
        "type Svc struct{ db Db; cache Cache; log Log }\n",
        "func (s *Svc) A() int { return s.db.Foo() }\n",
        "func (s *Svc) B() int { return s.cache.Foo() }\n",
        "func (s *Svc) C() int { return s.log.Foo() }\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"));
}

// ===========================================================================
// Method naming (3)
// ===========================================================================

#[test]
fn function_has_no_prefix() {
    let out = debug("package main\n\nfunc process() {}\n");
    assert!(out.contains("process"), "function name should be 'process', got: {out}");
    assert!(!out.contains(".process"));
}

#[test]
fn method_has_receiver_type_prefix() {
    let out = debug(concat!(
        "package main\n\n",
        "type Server struct{}\n\n",
        "func (s *Server) Handle() {}\n",
    ));
    assert!(out.contains("Server.Handle"), "should be Server.Handle, got: {out}");
}

#[test]
fn method_on_pointer_receiver() {
    let out = debug(concat!(
        "package main\n\n",
        "type Config struct{}\n\n",
        "func (c *Config) Load() {}\n",
    ));
    assert!(out.contains("Config.Load"), "pointer receiver type extracted, got: {out}");
}

// ===========================================================================
// Go-specific (7)
// ===========================================================================

#[test]
fn defer_does_not_increment_cc() {
    let out = debug(concat!(
        "package main\n\n",
        "func cleanup() {}\n\n",
        "func f() {\n",
        "\tdefer cleanup()\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, " f ", "cc"), Some(1));
}

#[test]
fn go_statement_does_not_increment_cc() {
    let out = debug(concat!(
        "package main\n\n",
        "func handler() {}\n\n",
        "func f() {\n",
        "\tgo handler()\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, " f ", "cc"), Some(1));
}

#[test]
fn for_infinite_loop_cc() {
    let out = debug("package main\n\nfunc f() {\n\tfor {\n\t\tbreak\n\t}\n}\n");
    // for statement always adds CC, even for infinite loops
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn select_with_default() {
    let out = debug(concat!(
        "package main\n\n",
        "func f(ch chan int) {\n",
        "\tselect {\n",
        "\tcase v := <-ch:\n",
        "\t\t_ = v\n",
        "\tdefault:\n",
        "\t}\n",
        "}\n",
    ));
    // base=1 + 1 communication_case = 2, default not counted
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn type_switch_cc() {
    let out = debug(concat!(
        "package main\n\n",
        "func f(x interface{}) string {\n",
        "\tswitch x.(type) {\n",
        "\tcase int:\n\t\treturn \"int\"\n",
        "\tcase string:\n\t\treturn \"string\"\n",
        "\tcase bool:\n\t\treturn \"bool\"\n",
        "\tdefault:\n\t\treturn \"other\"\n",
        "\t}\n",
        "}\n",
    ));
    // base=1 + 3 type_case = 4, default not counted
    assert_eq!(function_metric(&out, "f", "cc"), Some(4));
}

#[test]
fn func_literal_skipped() {
    let out = debug(concat!(
        "package main\n\n",
        "func f() {\n",
        "\thandler := func(x int) int {\n",
        "\t\tif x > 0 {\n",
        "\t\t\treturn x\n",
        "\t\t}\n",
        "\t\treturn 0\n",
        "\t}\n",
        "\t_ = handler\n",
        "}\n",
    ));
    // func_literal is skipped, so f should have cc=1
    assert_eq!(function_metric(&out, " f ", "cc"), Some(1));
    assert_eq!(function_metric(&out, " f ", "nesting"), Some(0));
}

#[test]
fn multiple_return_values_no_crash() {
    let out = debug("package main\n\nfunc f() (int, error) {\n\treturn 0, nil\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(1));
}

// ===========================================================================
// Performance (2)
// ===========================================================================

#[test]
fn performance_1000_loc() {
    let mut code = String::from("package main\n\n");
    for i in 0..50 {
        code.push_str(&format!("func func{i}(data int) int {{\n"));
        for j in 0..18 {
            code.push_str(&format!("\tf{j} := data + {j}\n"));
            code.push_str(&format!("\t_ = f{j}\n"));
        }
        code.push_str("\treturn data\n}\n\n");
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("big.go");
    std::fs::write(&path, &code).unwrap();
    let start = std::time::Instant::now();
    let _ = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let elapsed = start.elapsed();
    assert!(
        elapsed.as_millis() < 200,
        "took: {}ms",
        elapsed.as_millis()
    );
}

#[test]
fn performance_struct_hierarchy() {
    let mut code = String::from("package main\n\n");
    for i in 0..10 {
        code.push_str(&format!("type S{i} struct {{\n\td{i} int\n}}\n\n"));
        for j in 0..5 {
            code.push_str(&format!(
                "func (s *S{i}) M{j}() int {{ return s.d{i} }}\n"
            ));
        }
        code.push('\n');
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("structs.go");
    std::fs::write(&path, &code).unwrap();
    let start = std::time::Instant::now();
    let _ = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let elapsed = start.elapsed();
    assert!(
        elapsed.as_millis() < 500,
        "took: {}ms",
        elapsed.as_millis()
    );
}

// ===========================================================================
// Edge cases and clean code
// ===========================================================================

#[test]
fn clean_go_module_not_flagged() {
    let out = check(concat!(
        "package main\n\n",
        "type Config struct {\n",
        "\thost string\n",
        "\tport int\n",
        "}\n\n",
        "func NewConfig(host string, port int) Config {\n",
        "\treturn Config{host: host, port: port}\n",
        "}\n",
    ));
    assert!(
        out.is_empty(),
        "clean Go code should not be flagged, got: {out}"
    );
}

#[test]
fn comments_only() {
    let out = check("package main\n\n// just comments\n// only\n");
    assert!(out.is_empty());
}

#[test]
fn empty_package() {
    let out = check("package main\n");
    assert!(out.is_empty());
}

#[test]
fn value_receiver_method() {
    let out = debug(concat!(
        "package main\n\n",
        "type Point struct{}\n\n",
        "func (p Point) String() string {\n",
        "\treturn \"point\"\n",
        "}\n",
    ));
    assert!(out.contains("Point.String"), "value receiver type, got: {out}");
}

#[test]
fn multiple_params_same_type() {
    // Go allows `a, b int` as one parameter_declaration with 2 identifiers
    let out = debug("package main\n\nfunc f(a, b, c int) {\n\t_ = a + b + c\n}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(3));
}

#[test]
fn method_with_params_and_receiver() {
    let out = debug(concat!(
        "package main\n\n",
        "type DB struct{}\n\n",
        "func (d *DB) Query(sql string, args ...interface{}) {\n",
        "\t_ = sql\n",
        "\t_ = args\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "DB.Query", "args"), Some(2));
}

#[test]
fn excess_args_on_method() {
    let out = check(concat!(
        "package main\n\n",
        "type Handler struct{}\n\n",
        "func (h *Handler) Process(a, b, c, d, e, f int) {\n",
        "\t_ = a + b + c + d + e + f\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Excess Arguments"));
}

#[test]
fn deep_nesting_fires() {
    let out = check(concat!(
        "package main\n\n",
        "func f(a, b, c, d int) {\n",
        "\tif a > 0 {\n",
        "\t\tif b > 0 {\n",
        "\t\t\tif c > 0 {\n",
        "\t\t\t\tif d > 0 {}\n",
        "\t\t\t}\n",
        "\t\t}\n",
        "\t}\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Deep Nested"));
}

#[test]
fn nesting_below_threshold_no_smell() {
    let out = check(concat!(
        "package main\n\n",
        "func f(a, b, c int) {\n",
        "\tif a > 0 {\n",
        "\t\tif b > 0 {\n",
        "\t\t\tif c > 0 {}\n",
        "\t\t}\n",
        "\t}\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Deep Nested"));
}

#[test]
fn cc_switch_many_cases() {
    let out = debug(concat!(
        "package main\n\n",
        "func f(x int) string {\n",
        "\tswitch x {\n",
        "\tcase 1:\n\t\treturn \"a\"\n",
        "\tcase 2:\n\t\treturn \"b\"\n",
        "\tcase 3:\n\t\treturn \"c\"\n",
        "\tcase 4:\n\t\treturn \"d\"\n",
        "\tcase 5:\n\t\treturn \"e\"\n",
        "\tcase 6:\n\t\treturn \"f\"\n",
        "\tcase 7:\n\t\treturn \"g\"\n",
        "\tcase 8:\n\t\treturn \"h\"\n",
        "\tdefault:\n\t\treturn \"?\"\n",
        "\t}\n",
        "}\n",
    ));
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(cc >= 9, "8 cases + base >= 9, got: {cc}");
}

#[test]
fn nested_for_if_metrics() {
    let out = debug(concat!(
        "package main\n\n",
        "func f() {\n",
        "\tfor i := 0; i < 10; i++ {\n",
        "\t\tif i > 5 {}\n",
        "\t}\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
    assert_eq!(function_metric(&out, "f", "nesting"), Some(2));
}

#[test]
fn multiple_functions_independent_metrics() {
    let out = debug(concat!(
        "package main\n\n",
        "func simple() int {\n",
        "\treturn 1\n",
        "}\n\n",
        "func complex_fn(x int) int {\n",
        "\tif x > 0 {\n",
        "\t\tif x > 1 {\n",
        "\t\t\treturn x\n",
        "\t\t}\n",
        "\t}\n",
        "\treturn 0\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "simple", "cc"), Some(1));
    assert_eq!(function_metric(&out, "complex_fn", "cc"), Some(3));
}

#[test]
fn for_with_init_and_post() {
    let out = debug(concat!(
        "package main\n\n",
        "func f() {\n",
        "\tfor i := 0; i < 100; i++ {\n",
        "\t\t_ = i\n",
        "\t}\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
    assert_eq!(function_metric(&out, "f", "nesting"), Some(1));
}

#[test]
fn empty_function_body() {
    let out = debug("package main\n\nfunc f() {}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(1));
    assert_eq!(function_metric(&out, "f", "cogc"), Some(0));
    assert_eq!(function_metric(&out, "f", "nesting"), Some(0));
}

#[test]
fn goroutine_with_closure_not_counted() {
    let out = debug(concat!(
        "package main\n\n",
        "func f() {\n",
        "\tgo func() {\n",
        "\t\tif true {}\n",
        "\t}()\n",
        "}\n",
    ));
    // func_literal inside go statement is skipped
    assert_eq!(function_metric(&out, " f ", "cc"), Some(1));
}

#[test]
fn interface_type_param_not_primitive() {
    let out = check(concat!(
        "package main\n\n",
        "type Reader interface{}\n\n",
        "func f(r Reader, w Reader, x Reader, y Reader) {\n",
        "\t_ = r\n",
        "\t_ = w\n",
        "\t_ = x\n",
        "\t_ = y\n",
        "}\n",
    ));
    assert!(
        !has_smell(&out, "Primitive Obsession"),
        "interface params should not be primitives, got: {out}"
    );
}

#[test]
fn switch_inside_for_nesting() {
    let out = debug(concat!(
        "package main\n\n",
        "func f(items []int) {\n",
        "\tfor _, v := range items {\n",
        "\t\tswitch v {\n",
        "\t\tcase 1:\n",
        "\t\t}\n",
        "\t}\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "nesting"), Some(2));
}

#[test]
fn else_if_chain_cc() {
    let out = debug(concat!(
        "package main\n\n",
        "func f(x int) string {\n",
        "\tif x > 100 {\n",
        "\t\treturn \"big\"\n",
        "\t} else if x > 50 {\n",
        "\t\treturn \"medium\"\n",
        "\t} else if x > 10 {\n",
        "\t\treturn \"small\"\n",
        "\t} else {\n",
        "\t\treturn \"tiny\"\n",
        "\t}\n",
        "}\n",
    ));
    // base=1 + if=1 + else_if=1 + else_if=1 = 4
    assert_eq!(function_metric(&out, "f", "cc"), Some(4));
}
