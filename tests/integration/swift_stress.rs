use crate::common::*;

lang_helpers!("swift");

// ===========================================================================
// CC counting (16)
// ===========================================================================

#[test]
fn cc_counts_if() {
    let out = debug("func f(x: Int) -> Int {\n    if x > 0 {}\n    return x\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_else_if() {
    let out = debug("func f(x: Int) -> Int {\n    if x > 0 {\n    } else if x < 0 {\n    }\n    return x\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_counts_for_in() {
    let out = debug("func f(items: [Int]) {\n    for _ in items {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_while() {
    let out = debug("func f(x: Int) {\n    var n = x\n    while n > 0 {\n        n -= 1\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_repeat_while() {
    let out = debug("func f(x: Int) {\n    var n = x\n    repeat {\n        n -= 1\n    } while n > 0\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_guard() {
    let out = debug("func f(x: Int) -> Int {\n    guard x > 0 else { return 0 }\n    return x\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_switch_cases() {
    let out = debug(concat!(
        "func f(x: Int) -> Int {\n",
        "    switch x {\n",
        "    case 1:\n        return 1\n",
        "    case 2:\n        return 2\n",
        "    case 3:\n        return 3\n",
        "    default:\n        return 0\n",
        "    }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cc"), Some(4));
}

#[test]
fn cc_counts_ternary() {
    let out = debug("func f(x: Int) -> Int {\n    return x > 0 ? 1 : 0\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_logical_and() {
    let out =
        debug("func f(a: Bool, b: Bool) -> Bool {\n    if a && b {\n        return true\n    }\n    return false\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_counts_logical_or() {
    let out =
        debug("func f(a: Bool, b: Bool) -> Bool {\n    if a || b {\n        return true\n    }\n    return false\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_counts_mixed_boolean() {
    let out = debug("func f(a: Bool, b: Bool, c: Bool) -> Bool {\n    if a && b || c {\n        return true\n    }\n    return false\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 4, "got: {cc}");
}

#[test]
fn cc_multiple_if_accumulates() {
    let out = debug(concat!(
        "func f(a: Int, b: Int) -> Int {\n",
        "    if a > 0 {}\n",
        "    if b > 0 {}\n",
        "    return 0\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_nested_if_accumulates() {
    let out = debug(concat!(
        "func f(a: Int, b: Int) -> Int {\n",
        "    if a > 0 {\n",
        "        if b > 0 {}\n",
        "    }\n",
        "    return 0\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_catch_increments() {
    let out = debug(concat!(
        "func f() {\n",
        "    do {\n",
        "        try something()\n",
        "    } catch {\n",
        "        print(\"err\")\n",
        "    }\n",
        "}\n",
        "func something() throws {}\n",
    ));
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert_eq!(cc, 2);
}

#[test]
fn cc_guard_with_boolean_ops() {
    let out = debug("func f(a: Bool, b: Bool) -> Int {\n    guard a && b else { return 0 }\n    return 1\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert_eq!(cc, 3);
}

#[test]
fn cc_complex_combination() {
    let out = debug(concat!(
        "func f(x: Int, items: [Int]) -> Int {\n",
        "    guard x > 0 else { return 0 }\n",
        "    if x > 10 {\n",
        "        for _ in items {\n",
        "            if x > 100 {}\n",
        "        }\n",
        "    } else if x > 5 {\n",
        "        switch x {\n",
        "        case 6:\n            break\n",
        "        case 7:\n            break\n",
        "        default:\n            break\n",
        "        }\n",
        "    }\n",
        "    return x\n",
        "}\n",
    ));
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 7, "got: {cc}");
}

// ===========================================================================
// Cognitive complexity (12)
// ===========================================================================

#[test]
fn cogc_simple_if_is_1() {
    let out = debug("func f(x: Int) {\n    if x > 0 {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cogc"), Some(1));
}

#[test]
fn cogc_nested_if_weighted() {
    let out =
        debug(concat!("func f(x: Int, y: Int) {\n", "    if x > 0 {\n", "        if y > 0 {}\n", "    }\n", "}\n",));
    let cogc = function_metric(&out, "f", "cogc").unwrap_or(0);
    assert!(cogc >= 3, "nested if should get extra weight, got: {cogc}");
}

#[test]
fn cogc_else_if_flat() {
    let out = debug(concat!("func f(x: Int) {\n", "    if x > 0 {\n", "    } else if x < 0 {\n", "    }\n", "}\n",));
    let cogc = function_metric(&out, "f", "cogc").unwrap_or(0);
    assert!(cogc >= 2, "got: {cogc}");
}

#[test]
fn cogc_for_loop() {
    let out = debug("func f(items: [Int]) {\n    for _ in items {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cogc"), Some(1));
}

#[test]
fn cogc_nested_loop_weighted() {
    let out = debug(concat!(
        "func f(a: [Int], b: [Int]) {\n",
        "    for _ in a {\n",
        "        for _ in b {}\n",
        "    }\n",
        "}\n",
    ));
    let cogc = function_metric(&out, "f", "cogc").unwrap_or(0);
    assert!(cogc >= 3, "got: {cogc}");
}

#[test]
fn cogc_guard_increments() {
    let out = debug("func f(x: Int) -> Int {\n    guard x > 0 else { return 0 }\n    return x\n}\n");
    let cogc = function_metric(&out, "f", "cogc").unwrap_or(0);
    assert!(cogc >= 1, "got: {cogc}");
}

#[test]
fn cogc_switch_increments() {
    let out = debug(concat!(
        "func f(x: Int) {\n",
        "    switch x {\n",
        "    case 1:\n        break\n",
        "    default:\n        break\n",
        "    }\n",
        "}\n",
    ));
    let cogc = function_metric(&out, "f", "cogc").unwrap_or(0);
    assert!(cogc >= 1, "got: {cogc}");
}

#[test]
fn cogc_ternary_increments() {
    let out = debug("func f(x: Int) -> Int {\n    return x > 0 ? 1 : 0\n}\n");
    let cogc = function_metric(&out, "f", "cogc").unwrap_or(0);
    assert!(cogc >= 1, "got: {cogc}");
}

#[test]
fn cogc_boolean_sequence_change() {
    let out = debug("func f(a: Bool, b: Bool, c: Bool) -> Bool {\n    if a && b || c {\n        return true\n    }\n    return false\n}\n");
    let cogc = function_metric(&out, "f", "cogc").unwrap_or(0);
    assert!(cogc >= 3, "got: {cogc}");
}

#[test]
fn cogc_deep_nesting_costly() {
    let out = debug(concat!(
        "func f(a: Int, b: Int, c: Int) {\n",
        "    if a > 0 {\n",
        "        if b > 0 {\n",
        "            if c > 0 {}\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    let cogc = function_metric(&out, "f", "cogc").unwrap_or(0);
    assert!(cogc >= 6, "3 nested ifs should be costly, got: {cogc}");
}

#[test]
fn cogc_catch_increments() {
    let out = debug(concat!(
        "func f() {\n",
        "    do {\n",
        "        try something()\n",
        "    } catch {\n",
        "        print(\"err\")\n",
        "    }\n",
        "}\n",
        "func something() throws {}\n",
    ));
    let cogc = function_metric(&out, "f", "cogc").unwrap_or(0);
    assert!(cogc >= 1, "got: {cogc}");
}

#[test]
fn cogc_repeat_while() {
    let out = debug("func f() {\n    var x = 5\n    repeat {\n        x -= 1\n    } while x > 0\n}\n");
    let cogc = function_metric(&out, "f", "cogc").unwrap_or(0);
    assert!(cogc >= 1, "got: {cogc}");
}

// ===========================================================================
// Nesting depth (8)
// ===========================================================================

#[test]
fn nesting_if_is_1() {
    let out = debug("func f(x: Int) {\n    if x > 0 {}\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(1));
}

#[test]
fn nesting_nested_if_is_2() {
    let out =
        debug(concat!("func f(x: Int, y: Int) {\n", "    if x > 0 {\n", "        if y > 0 {}\n", "    }\n", "}\n",));
    assert_eq!(function_metric(&out, "f", "nesting"), Some(2));
}

#[test]
fn nesting_for_is_1() {
    let out = debug("func f(items: [Int]) {\n    for _ in items {}\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(1));
}

#[test]
fn nesting_switch_is_1() {
    let out = debug(concat!(
        "func f(x: Int) {\n",
        "    switch x {\n",
        "    case 1:\n        break\n",
        "    default:\n        break\n",
        "    }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "nesting"), Some(1));
}

#[test]
fn nesting_guard_is_1() {
    let out = debug("func f(x: Int) -> Int {\n    guard x > 0 else { return 0 }\n    return x\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(1));
}

#[test]
fn nesting_while_is_1() {
    let out = debug("func f(x: Int) {\n    var n = x\n    while n > 0 {\n        n -= 1\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(1));
}

#[test]
fn nesting_3_deep() {
    let out = debug(concat!(
        "func f(a: Int, b: Int, c: Int) {\n",
        "    if a > 0 {\n",
        "        if b > 0 {\n",
        "            if c > 0 {}\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "nesting"), Some(3));
}

#[test]
fn nesting_5_deep() {
    let out = debug(concat!(
        "func f(a: Int, b: Int, c: Int, d: Int, e: Int) {\n",
        "    if a > 0 {\n",
        "        for _ in [b] {\n",
        "            if c > 0 {\n",
        "                if d > 0 {\n",
        "                    if e > 0 {}\n",
        "                }\n",
        "            }\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "nesting"), Some(5));
}

// ===========================================================================
// Bumps (4)
// ===========================================================================

#[test]
fn bumps_zero_for_flat() {
    let out = debug("func f(x: Int) {\n    if x > 0 {}\n}\n");
    assert_eq!(function_metric(&out, "f", "bumps"), Some(0));
}

#[test]
fn bumps_counted_for_nested_chunks() {
    let out = debug(concat!(
        "func f(x: Int) {\n",
        "    if x > 0 {\n",
        "        if x > 5 {\n",
        "            if x > 10 {}\n",
        "        }\n",
        "    }\n",
        "    let gap = 0\n",
        "    _ = gap\n",
        "    if x > 20 {\n",
        "        if x > 25 {\n",
        "            if x > 30 {}\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    let bumps = function_metric(&out, "f", "bumps").unwrap_or(0);
    assert!(bumps >= 2, "got: {bumps}");
}

#[test]
fn bumps_three_chunks() {
    let out = debug(concat!(
        "func f(x: Int) {\n",
        "    if x > 0 {\n        if x > 1 {\n            if x > 2 {}\n        }\n    }\n",
        "    let a = 0\n    _ = a\n",
        "    if x > 3 {\n        if x > 4 {\n            if x > 5 {}\n        }\n    }\n",
        "    let b = 0\n    _ = b\n",
        "    if x > 6 {\n        if x > 7 {\n            if x > 8 {}\n        }\n    }\n",
        "}\n",
    ));
    let bumps = function_metric(&out, "f", "bumps").unwrap_or(0);
    assert!(bumps >= 3, "got: {bumps}");
}

#[test]
fn single_deep_nest_is_one_bump() {
    let out = debug(concat!(
        "func f(x: Int) {\n",
        "    if x > 0 {\n",
        "        if x > 10 {\n",
        "            if x > 100 {}\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    let bumps = function_metric(&out, "f", "bumps").unwrap_or(0);
    assert_eq!(bumps, 1);
}

// ===========================================================================
// Arguments (6)
// ===========================================================================

#[test]
fn args_simple_count() {
    let out = debug("func f(a: Int, b: String, c: Double) -> Int {\n    return 0\n}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(3));
}

#[test]
fn args_zero_for_no_params() {
    let out = debug("func f() -> Int {\n    return 0\n}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(0));
}

#[test]
fn args_one_param() {
    let out = debug("func f(x: Int) -> Int {\n    return x\n}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(1));
}

#[test]
fn args_six_triggers_excess() {
    let out =
        check(concat!("func f(a: Int, b: Int, c: Int, d: Int, e: Int, f2: Int) -> Int {\n", "    return 0\n", "}\n",));
    assert!(has_smell(&out, "Excess Arguments"), "got: {out}");
}

#[test]
fn args_five_no_excess() {
    let out = check(concat!("func f(a: Int, b: Int, c: Int, d: Int, e: Int) -> Int {\n", "    return 0\n", "}\n",));
    assert!(!has_smell(&out, "Excess Arguments"));
}

#[test]
fn args_init_count() {
    let out = debug(concat!(
        "class Svc {\n",
        "    var a: Int = 0\n",
        "    init(a: Int, b: Int, c: Int) {\n",
        "        self.a = a\n",
        "    }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "Svc.init", "args"), Some(3));
}

// ===========================================================================
// LOC (4)
// ===========================================================================

#[test]
fn loc_simple() {
    let out = debug("func f() -> Int {\n    return 0\n}\n");
    assert_eq!(function_metric(&out, "f", "loc"), Some(3));
}

#[test]
fn loc_multiline() {
    let out = debug(concat!(
        "func f() -> Int {\n",
        "    let a = 1\n",
        "    let b = 2\n",
        "    let c = 3\n",
        "    return a + b + c\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "loc"), Some(6));
}

#[test]
fn loc_one_liner() {
    let out = debug("func f() -> Int { return 0 }\n");
    assert_eq!(function_metric(&out, "f", "loc"), Some(1));
}

#[test]
fn loc_with_nesting() {
    let out = debug(concat!(
        "func f(x: Int) -> Int {\n",
        "    if x > 0 {\n",
        "        return 1\n",
        "    }\n",
        "    return 0\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "loc"), Some(6));
}

// ===========================================================================
// Embedded blocks (4)
// ===========================================================================

#[test]
fn embedded_short_string_no_flag() {
    let out = check("func f() -> String {\n    return \"hello\"\n}\n");
    assert!(!has_smell(&out, "Large Embedded Block"));
}

#[test]
fn embedded_counts_string_lines() {
    let out = debug("func f() -> String {\n    return \"hello\"\n}\n");
    let emb = function_metric(&out, "f", "embedded").unwrap_or(99);
    assert_eq!(emb, 1);
}

#[test]
fn embedded_zero_when_no_strings() {
    let out = debug("func f() -> Int {\n    return 42\n}\n");
    assert_eq!(function_metric(&out, "f", "embedded"), Some(0));
}

#[test]
fn embedded_large_triggers() {
    let mut code = String::from("func f() -> String {\n    let s = \"\"\"\n");
    for i in 0..embedded_lines_above() {
        code.push_str(&format!("    line {i}\n"));
    }
    code.push_str("    \"\"\"\n    return s\n}\n");
    let out = check(&code);
    assert!(has_smell(&out, "Large Embedded Block"), "got: {out}");
}

// ===========================================================================
// Duplication (4)
// ===========================================================================

#[test]
fn identical_functions_detected_as_duplicates() {
    let out = check(concat!(
        "func a(x: [Int]) -> Int {\n    var r = 0\n    for v in x {\n        r += v\n    }\n    r = r * 2\n    return r\n}\n",
        "func b(x: [Int]) -> Int {\n    var r = 0\n    for v in x {\n        r += v\n    }\n    r = r * 2\n    return r\n}\n",
    ));
    assert!(has_smell(&out, "Code Duplication"), "got: {out}");
}

#[test]
fn duplication_detected_inline() {
    let out = check(concat!(
        "func a(x: [Int]) -> Int {\n    var r = 0\n    for v in x {\n        r += v\n    }\n    r = r * 2\n    return r\n}\n",
        "func b(x: [Int]) -> Int {\n    var r = 0\n    for v in x {\n        r += v\n    }\n    r = r * 2\n    return r\n}\n",
    ));
    assert!(has_smell(&out, "Code Duplication"), "got: {out}");
}

#[test]
fn no_duplication_for_different_structure() {
    let out = check(concat!(
        "func a(x: Int) -> Int {\n    if x > 0 {\n        return 1\n    }\n    return 0\n}\n",
        "func b(x: Int) -> Int {\n    for _ in 0..<x {}\n    return x\n}\n",
    ));
    assert!(!has_smell(&out, "Code Duplication"));
}

#[test]
fn duplication_needs_min_loc() {
    let out = check(concat!("func a() -> Int { return 1 }\n", "func b() -> Int { return 1 }\n",));
    assert!(!has_smell(&out, "Code Duplication"));
}

// ===========================================================================
// Assert blocks (4)
// ===========================================================================

#[test]
fn assert_count_zero_for_no_calls() {
    let out = debug("func f() -> Int {\n    return 42\n}\n");
    assert_eq!(function_metric(&out, "f", "asserts"), Some(0));
}

#[test]
fn assert_count_for_calls() {
    let out = debug(concat!("func f() {\n", "    assert(true)\n", "    assert(true)\n", "    assert(true)\n", "}\n",));
    let asserts = function_metric(&out, "f", "asserts").unwrap_or(0);
    assert!(asserts >= 3, "got: {asserts}");
}

#[test]
fn assert_block_single_gap_tolerated() {
    let out = debug(concat!("func f() {\n", "    assert(true)\n", "    let x = 1\n", "    assert(true)\n", "}\n",));
    let asserts = function_metric(&out, "f", "asserts").unwrap_or(0);
    assert!(asserts >= 2, "single gap should be tolerated, got: {asserts}");
}

#[test]
fn assert_block_large_gap_resets() {
    let out = debug(concat!(
        "func f() {\n",
        "    assert(true)\n",
        "    let x = 1\n",
        "    let y = 2\n",
        "    assert(true)\n",
        "}\n",
    ));
    let asserts = function_metric(&out, "f", "asserts").unwrap_or(0);
    assert!(asserts <= 1, "gap beyond tolerance should reset, got: {asserts}");
}

#[test]
fn assert_block_threshold() {
    let mut code = String::from("func f() {\n");
    for _ in 0..asserts_above() {
        code.push_str("    assert(true)\n");
    }
    code.push_str("}\n");
    let out = check(&code);
    assert!(has_smell(&out, "Large Assertion Block"), "got: {out}");
}

// ===========================================================================
// Compound conditions (4)
// ===========================================================================

#[test]
fn compound_condition_two_ops() {
    let out = debug("func f(a: Bool, b: Bool, c: Bool) {\n    if a && b && c {}\n}\n");
    let cond = function_metric(&out, "f", "conditions").unwrap_or(0);
    assert!(cond >= 1, "got: {cond}");
}

#[test]
fn compound_condition_one_op_no_flag() {
    let out = debug("func f(a: Bool, b: Bool) {\n    if a && b {}\n}\n");
    let cond = function_metric(&out, "f", "conditions").unwrap_or(0);
    assert_eq!(cond, 0);
}

#[test]
fn compound_condition_mixed_ops() {
    let out = debug("func f(a: Bool, b: Bool, c: Bool) {\n    if a && b || c {}\n}\n");
    let cond = function_metric(&out, "f", "conditions").unwrap_or(0);
    assert!(cond >= 1, "got: {cond}");
}

#[test]
fn compound_condition_guard() {
    let out = debug("func f(a: Bool, b: Bool, c: Bool) {\n    guard a && b || c else { return }\n}\n");
    let cond = function_metric(&out, "f", "conditions").unwrap_or(0);
    assert!(cond >= 1, "got: {cond}");
}

// ===========================================================================
// Constructor over-injection (3)
// ===========================================================================

#[test]
fn init_over_injection_detected() {
    let out = check(concat!(
        "class Svc {\n",
        "    var a: Int = 0\n",
        "    init(a: Int, b: Int, c: Int, d: Int, e: Int, f: Int, g: Int, h: Int, i: Int) {\n",
        "        self.a = a\n",
        "    }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Constructor Over-Injection"), "got: {out}");
}

#[test]
fn init_few_params_no_flag() {
    let out = check(concat!(
        "class Svc {\n",
        "    var a: Int = 0\n",
        "    init(a: Int, b: Int) {\n",
        "        self.a = a\n",
        "    }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Constructor Over-Injection"));
}

#[test]
fn init_exactly_at_boundary() {
    let out = check(concat!(
        "class Svc {\n",
        "    var a: Int = 0\n",
        "    init(a: Int, b: Int, c: Int, d: Int, e: Int) {\n",
        "        self.a = a\n",
        "    }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Constructor Over-Injection"));
}

// ===========================================================================
// Primitive obsession (4)
// ===========================================================================

#[test]
fn primitive_obsession_high_ratio() {
    let out = check(concat!("func f(a: Int, b: Int, c: Int, d: Double) -> Int {\n", "    return a\n", "}\n",));
    assert!(has_smell(&out, "Primitive Obsession"), "4/4 primitives should trigger, got: {out}");
}

#[test]
fn primitive_obsession_low_count_no_flag() {
    let out = check("func f(a: Int, b: Int) -> Int {\n    return a + b\n}\n");
    assert!(!has_smell(&out, "Primitive Obsession"));
}

#[test]
fn primitive_obsession_non_primitive_reduces_ratio() {
    let out =
        check(concat!("func f(a: Int, b: MyType, c: OtherType, d: AnotherType) -> Int {\n", "    return a\n", "}\n",));
    assert!(!has_smell(&out, "Primitive Obsession"));
}

#[test]
fn primitive_obsession_all_custom_no_flag() {
    let out =
        check(concat!("func f(a: MyType, b: YourType, c: TheirType, d: OurType) -> Int {\n", "    return 0\n", "}\n",));
    assert!(!has_smell(&out, "Primitive Obsession"));
}

// ===========================================================================
// Empty catch (4)
// ===========================================================================

#[test]
fn empty_catch_detected() {
    let out = check(concat!(
        "func f() {\n",
        "    do {\n",
        "        try something()\n",
        "    } catch {\n",
        "    }\n",
        "}\n",
        "func something() throws {}\n",
    ));
    assert!(has_smell(&out, "Empty Error Handler"), "got: {out}");
}

#[test]
fn non_empty_catch_no_flag() {
    let out = check(concat!(
        "func f() {\n",
        "    do {\n",
        "        try something()\n",
        "    } catch {\n",
        "        print(\"error\")\n",
        "    }\n",
        "}\n",
        "func something() throws {}\n",
    ));
    assert!(!has_smell(&out, "Empty Error Handler"));
}

#[test]
fn empty_catch_count_detected() {
    let out = check(concat!(
        "func f() {\n",
        "    do {\n",
        "        try something()\n",
        "    } catch {\n",
        "    }\n",
        "}\n",
        "func something() throws {}\n",
    ));
    assert!(has_smell(&out, "Empty Error Handler"), "got: {out}");
}

#[test]
fn multiple_empty_catches_detected() {
    let out = check(concat!(
        "func f() {\n",
        "    do {\n        try a()\n    } catch {\n    }\n",
        "    do {\n        try b()\n    } catch {\n    }\n",
        "}\n",
        "func a() throws {}\n",
        "func b() throws {}\n",
    ));
    assert!(has_smell(&out, "Empty Error Handler"), "got: {out}");
}

// ===========================================================================
// Field access / cohesion (4)
// ===========================================================================

#[test]
fn field_access_self_dot() {
    let out = debug(concat!(
        "class Svc {\n",
        "    var count: Int = 0\n",
        "    func inc() {\n",
        "        self.count += 1\n",
        "    }\n",
        "}\n",
    ));
    assert!(out.contains(r#"fields=["count"]"#), "got: {out}");
}

#[test]
fn field_access_multiple_fields() {
    let out = debug(concat!(
        "class Svc {\n",
        "    var a: Int = 0\n",
        "    var b: Int = 0\n",
        "    func both() -> Int {\n",
        "        let x = self.a\n",
        "        let y = self.b\n",
        "        return x + y\n",
        "    }\n",
        "}\n",
    ));
    assert!(out.contains(r#""a""#) && out.contains(r#""b""#), "got: {out}");
}

#[test]
fn no_field_access_without_self() {
    let out = debug(concat!(
        "class Svc {\n",
        "    func compute(x: Int) -> Int {\n",
        "        return x + 1\n",
        "    }\n",
        "}\n",
    ));
    assert!(out.contains("fields=[]"), "got: {out}");
}

#[test]
fn low_cohesion_many_disconnected_methods() {
    let out = check(concat!(
        "class Big {\n",
        "    var a: Int = 0\n    var b: Int = 0\n    var c: Int = 0\n",
        "    var d: Int = 0\n    var e: Int = 0\n",
        "    func useA() -> Int { return self.a }\n",
        "    func useB() -> Int { return self.b }\n",
        "    func useC() -> Int { return self.c }\n",
        "    func useD() -> Int { return self.d }\n",
        "    func useE() -> Int { return self.e }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"), "got: {out}");
}

#[test]
fn lcom4_methods_connected_by_call() {
    let out = check(concat!(
        "class Coord {\n",
        "    var state: Int = 0\n",
        "    func process(_ e: Int) -> Bool { return self.validate(e) && self.dispatch(e) }\n",
        "    func validate(_ e: Int) -> Bool { return e > 0 }\n",
        "    func dispatch(_ e: Int) -> Bool { return self.send(e) }\n",
        "    func send(_ e: Int) -> Bool { return true }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"), "got: {out}");
}

#[test]
fn lcom4_mixed_field_and_call_connection() {
    let out = check(concat!(
        "class Mixed {\n",
        "    var x: Int = 0\n",
        "    func a() -> Int { return self.x }\n",
        "    func b() -> Int { self.x = 1; return self.c() }\n",
        "    func c() -> Int { return 42 }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_god_class_still_fires() {
    let out = check(concat!(
        "class UserService {\n",
        "    var db: Int = 0\n    var cache: Int = 0\n    var mailer: Int = 0\n",
        "    var events: Int = 0\n    var audit: Int = 0\n",
        "    func getUser() -> Int { return self.db }\n",
        "    func cacheUser() -> Int { return self.cache }\n",
        "    func sendWelcome() -> Int { return self.mailer }\n",
        "    func publish() -> Int { return self.events }\n",
        "    func auditLog() -> Int { return self.audit }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"), "got: {out}");
}

#[test]
fn lcom4_dependency_method_calls_dont_falsely_connect() {
    let out = check(concat!(
        "class Service {\n",
        "    var db: Int = 0\n    var cache: Int = 0\n    var log: Int = 0\n",
        "    func a() -> Int { return self.db }\n",
        "    func b() -> Int { return self.cache }\n",
        "    func c() -> Int { return self.log }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"));
}

// ===========================================================================
// Module metrics (4)
// ===========================================================================

#[test]
fn module_declaration_count() {
    let out = debug(concat!("class A {}\n", "class B {}\n", "struct C {\n    var x: Int\n}\n",));
    assert!(out.contains("declarations=3"), "got: {out}");
}

#[test]
fn module_function_count() {
    let out = debug(concat!("func a() {}\n", "func b() {}\n", "func c() {}\n",));
    assert!(out.contains("3 functions"), "got: {out}");
}

#[test]
fn module_loc() {
    let out = debug("func f() -> Int {\n    return 0\n}\n");
    assert!(out.contains("3 LOC"), "got: {out}");
}

#[test]
fn extension_not_counted_as_declaration() {
    let out = debug(concat!(
        "struct Point {\n    var x: Int\n}\n\n",
        "extension Point {\n",
        "    func magnitude() -> Int { return x }\n",
        "}\n",
    ));
    assert!(out.contains("declarations=1"), "extensions should not count, got: {out}");
}
