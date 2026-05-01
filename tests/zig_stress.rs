mod common;

use common::*;
use std::process::Command;

lang_helpers!("zig");

// ===========================================================================
// CC counting (18)
// ===========================================================================

#[test]
fn cc_counts_if() {
    let out = debug("fn f(x: i32) i32 {\n    if (x > 0) {}\n    return x;\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_else_if() {
    let out = debug(
        "fn f(x: i32) i32 {\n    if (x > 0) {\n    } else if (x < 0) {\n    }\n    return x;\n}\n",
    );
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_counts_for() {
    let out = debug("fn f(items: []const u8) void {\n    for (items) |_| {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_while() {
    let out = debug("fn f(x: i32) void {\n    var n = x;\n    while (n > 0) {\n        n -= 1;\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_switch_cases() {
    let out = debug(concat!(
        "fn f(x: u8) u8 {\n",
        "    return switch (x) {\n",
        "        1 => 10,\n",
        "        2 => 20,\n",
        "        3 => 30,\n",
        "        else => 0,\n",
        "    };\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cc"), Some(4));
}

#[test]
fn cc_counts_catch() {
    let out = debug("fn f(val: anyerror!i32) i32 {\n    return val catch 0;\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_orelse() {
    let out = debug("fn f(opt: ?i32) i32 {\n    return opt orelse 0;\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_and() {
    let out = debug("fn f(a: bool, b: bool) bool {\n    if (a and b) {\n        return true;\n    }\n    return false;\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_counts_or() {
    let out = debug("fn f(a: bool, b: bool) bool {\n    if (a or b) {\n        return true;\n    }\n    return false;\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_chained_boolean() {
    let out = debug("fn f(a: bool, b: bool, c: bool) bool {\n    if (a and b or c) {\n        return true;\n    }\n    return false;\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 4, "got: {cc}");
}

#[test]
fn cc_chained_boolean_4way() {
    let out = debug("fn f(a: bool, b: bool, c: bool, d: bool) bool {\n    if (a and b and c and d) {\n        return true;\n    }\n    return false;\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 5, "got: {cc}");
}

#[test]
fn cc_nested_if_counted_once() {
    let out = debug("fn f(a: bool, b: bool) bool {\n    if (a) {\n        if (b) {\n            return true;\n        }\n    }\n    return false;\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_switch_default_not_counted() {
    let out = debug(concat!(
        "fn f(x: u8) u8 {\n",
        "    return switch (x) {\n",
        "        1 => 10,\n",
        "        else => 0,\n",
        "    };\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_for_with_capture() {
    let out = debug("fn f(items: []const u8) u32 {\n    var s: u32 = 0;\n    for (items) |item| {\n        s += item;\n    }\n    return s;\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_base_case_is_1() {
    let out = debug("fn f() void {}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(1));
}

#[test]
fn cc_combined_catch_and_orelse() {
    let out = debug("fn f(e: anyerror!i32, o: ?i32) i32 {\n    const a = e catch 0;\n    const b = o orelse 0;\n    return a + b;\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_if_with_else() {
    let out = debug("fn f(x: i32) i32 {\n    if (x > 0) {\n        return 1;\n    } else {\n        return 0;\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_multiple_if_accumulates() {
    let out = debug(concat!(
        "fn f(a: i32, b: i32) i32 {\n",
        "    if (a > 0) {}\n",
        "    if (b > 0) {}\n",
        "    return 0;\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

// ===========================================================================
// Cognitive complexity (10)
// ===========================================================================

#[test]
fn cogc_flat_branches() {
    let out = debug("fn f(a: bool, b: bool) void {\n    if (a) {}\n    if (b) {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cogc"), Some(2));
}

#[test]
fn cogc_nested_ifs() {
    let out = debug("fn f(a: bool, b: bool) void {\n    if (a) {\n        if (b) {}\n    }\n}\n");
    let cogc = function_metric(&out, "f", "cogc").unwrap_or(0);
    assert!(cogc >= 3, "got: {cogc}");
}

#[test]
fn cogc_else_if_no_extra_nesting() {
    let out = debug("fn f(a: bool, b: bool) void {\n    if (a) {\n    } else if (b) {\n    }\n}\n");
    let cogc = function_metric(&out, "f", "cogc").unwrap_or(0);
    assert!(cogc >= 2, "got: {cogc}");
}

#[test]
fn cogc_else_flat() {
    let out = debug("fn f(a: bool) void {\n    if (a) {\n    } else {\n    }\n}\n");
    let cogc = function_metric(&out, "f", "cogc").unwrap_or(0);
    assert!(cogc >= 2, "got: {cogc}");
}

#[test]
fn cogc_for_loop_nested() {
    let out = debug("fn f(items: []const u8) void {\n    for (items) |_| {\n        if (true) {}\n    }\n}\n");
    let cogc = function_metric(&out, "f", "cogc").unwrap_or(0);
    assert!(cogc >= 3, "for+nested if should have high cogc, got: {cogc}");
}

#[test]
fn cogc_switch_counted() {
    let out = debug(concat!(
        "fn f(x: u8) u8 {\n",
        "    return switch (x) {\n",
        "        1 => 10,\n",
        "        else => 0,\n",
        "    };\n",
        "}\n",
    ));
    let cogc = function_metric(&out, "f", "cogc").unwrap_or(0);
    assert!(cogc >= 1, "switch should add cogc, got: {cogc}");
}

#[test]
fn cogc_boolean_single_sequence() {
    let out = debug("fn f(a: bool, b: bool) bool {\n    if (a and b) {\n        return true;\n    }\n    return false;\n}\n");
    let cogc = function_metric(&out, "f", "cogc").unwrap_or(0);
    assert!(cogc >= 2, "got: {cogc}");
}

#[test]
fn cogc_boolean_mixed_sequence() {
    let out = debug("fn f(a: bool, b: bool, c: bool) bool {\n    if (a and b or c) {\n        return true;\n    }\n    return false;\n}\n");
    let cogc = function_metric(&out, "f", "cogc").unwrap_or(0);
    assert!(cogc >= 3, "got: {cogc}");
}

#[test]
fn cogc_triggers_complex_method() {
    let mut code = String::from("fn f(x: i32) i32 {\n");
    for _ in 0..4 {
        code.push_str("    if (x > 0) {\n        if (x > 1) {\n            if (x > 2) {}\n        }\n    }\n");
    }
    code.push_str("    return 0;\n}\n");
    let out = check(&code);
    assert!(has_smell(&out, "Complex Method"), "got: {out}");
}

#[test]
fn cogc_below_threshold_no_smell() {
    let out = check("fn f(x: i32) i32 {\n    if (x > 0) {}\n    if (x > 1) {}\n    return 0;\n}\n");
    assert!(!has_smell(&out, "Complex Method"));
}

// ===========================================================================
// Nesting depth (6)
// ===========================================================================

#[test]
fn nesting_depth_simple() {
    let out = debug("fn f(x: bool) void {\n    if (x) {}\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(1));
}

#[test]
fn nesting_depth_nested() {
    let out = debug("fn f(a: bool, b: bool, c: bool) void {\n    if (a) {\n        if (b) {\n            if (c) {}\n        }\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(3));
}

#[test]
fn nesting_depth_sequential_not_accumulated() {
    let out = debug("fn f(a: bool, b: bool) void {\n    if (a) {}\n    if (b) {}\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(1));
}

#[test]
fn nesting_deep_if_chain() {
    let out = debug(concat!(
        "fn f(a: bool, b: bool, c: bool, d: bool, e: bool) void {\n",
        "    if (a) {\n",
        "        if (b) {\n",
        "            if (c) {\n",
        "                if (d) {\n",
        "                    if (e) {}\n",
        "                }\n",
        "            }\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "nesting"), Some(5));
}

#[test]
fn nesting_for_with_if() {
    let out = debug("fn f(items: []const u8) void {\n    for (items) |_| {\n        if (true) {}\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(2));
}

#[test]
fn nesting_switch_depth() {
    let out = debug(concat!(
        "fn f(x: u8) void {\n",
        "    switch (x) {\n",
        "        1 => {\n",
        "            if (true) {}\n",
        "        },\n",
        "        else => {},\n",
        "    }\n",
        "}\n",
    ));
    let nesting = function_metric(&out, "f", "nesting").unwrap_or(0);
    assert!(nesting >= 2, "got: {nesting}");
}

// ===========================================================================
// Bump counting (2)
// ===========================================================================

#[test]
fn bumpy_road_two_bumps() {
    let out = debug(concat!(
        "fn f(a: bool, b: bool, c: bool, d: bool) void {\n",
        "    if (a) {\n        if (b) {\n            if (true) {}\n        }\n    }\n",
        "    const x = 1;\n    _ = x;\n",
        "    if (c) {\n        if (d) {\n            if (true) {}\n        }\n    }\n",
        "}\n",
    ));
    let bumps = function_metric(&out, "f", "bumps").unwrap_or(0);
    assert!(bumps >= 2, "got: {bumps}");
}

#[test]
fn bumpy_road_single_bump_not_flagged() {
    let out = check(concat!(
        "fn f(a: bool, b: bool) void {\n",
        "    if (a) {\n        if (b) {\n            if (true) {}\n        }\n    }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Nested Conditional Chunks"));
}

// ===========================================================================
// Arguments (6)
// ===========================================================================

#[test]
fn args_zero() {
    let out = debug("fn f() void {}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(0));
}

#[test]
fn args_one() {
    let out = debug("fn f(x: i32) void { _ = x; }\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(1));
}

#[test]
fn args_five_at_threshold() {
    let out = check("fn f(a: i32, b: i32, c: i32, d: i32, e: i32) void { _ = a; _ = b; _ = c; _ = d; _ = e; }\n");
    assert!(!has_smell(&out, "Excess Arguments"));
}

#[test]
fn args_six_over_threshold() {
    let out = check("fn f(a: i32, b: i32, c: i32, d: i32, e: i32, g: i32) void { _ = a; _ = b; _ = c; _ = d; _ = e; _ = g; }\n");
    assert!(has_smell(&out, "Excess Arguments"), "got: {out}");
}

#[test]
fn args_method_self_not_counted() {
    let out = debug(concat!(
        "const S = struct {\n",
        "    pub fn f(self: S, a: i32) void {\n",
        "        _ = self;\n",
        "        _ = a;\n",
        "    }\n",
        "};\n",
    ));
    assert_eq!(function_metric(&out, "S.f", "args"), Some(1));
}

#[test]
fn args_comptime_param() {
    let out = debug("fn f(comptime T: type, val: T) T {\n    return val;\n}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(2));
}

// ===========================================================================
// LOC counting (4)
// ===========================================================================

#[test]
fn loc_single_line() {
    let out = debug("fn f() void {}\n");
    assert_eq!(function_metric(&out, "f", "loc"), Some(1));
}

#[test]
fn loc_multiline() {
    let out = debug("fn f() i32 {\n    const x = 1;\n    return x;\n}\n");
    assert_eq!(function_metric(&out, "f", "loc"), Some(4));
}

#[test]
fn loc_comments_excluded_module() {
    let out = debug("// a comment\n// another\nfn f() void {}\n");
    assert!(out.contains("LOC, 1 function"));
}

#[test]
fn loc_empty_lines_excluded_module() {
    let out = debug("\n\n\nfn f() void {}\n\n\n");
    assert!(out.contains("LOC, 1 function"));
}

// ===========================================================================
// Embedded blocks (2)
// ===========================================================================

#[test]
fn embedded_large_multiline_string() {
    let mut code = String::from("fn f() []const u8 {\n    return\n");
    for i in 0..embedded_lines_above() {
        code.push_str(&format!("        \\\\line {i}\n"));
    }
    code.push_str("    ;\n}\n");
    let out = check(&code);
    assert!(has_smell(&out, "Large Embedded Block"), "got: {out}");
}

#[test]
fn embedded_small_string_not_flagged() {
    let out = check("fn f() []const u8 {\n    return \"hello\";\n}\n");
    assert!(!has_smell(&out, "Large Embedded Block"));
}

// ===========================================================================
// Duplication (4)
// ===========================================================================

#[test]
fn exact_duplication_detected() {
    let out = check(concat!(
        "fn alpha(x: i32) i32 {\n",
        "    var r = x;\n",
        "    r = r * 2;\n",
        "    r = r + 1;\n",
        "    r = r - 3;\n",
        "    return r;\n",
        "}\n",
        "fn beta(x: i32) i32 {\n",
        "    var r = x;\n",
        "    r = r * 2;\n",
        "    r = r + 1;\n",
        "    r = r - 3;\n",
        "    return r;\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Code Duplication"), "got: {out}");
}

#[test]
fn exact_duplication_below_min_loc() {
    let out = check(concat!(
        "fn a(x: i32) i32 {\n    return x;\n}\n",
        "fn b(x: i32) i32 {\n    return x;\n}\n",
    ));
    assert!(!has_smell(&out, "Code Duplication"));
}

#[test]
fn fuzzy_duplication_detected() {
    let out = check(concat!(
        "fn processAlpha(data: []const u8) u32 {\n",
        "    var result: u32 = 0;\n",
        "    for (data) |item| {\n",
        "        if (item > 100) {\n",
        "            result += 2;\n",
        "        } else {\n",
        "            result += 1;\n",
        "        }\n",
        "    }\n",
        "    return result;\n",
        "}\n",
        "fn processBeta(items: []const u8) u32 {\n",
        "    var count: u32 = 0;\n",
        "    for (items) |val| {\n",
        "        if (val > 100) {\n",
        "            count += 2;\n",
        "        } else {\n",
        "            count += 1;\n",
        "        }\n",
        "    }\n",
        "    return count;\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Code Duplication"), "got: {out}");
}

#[test]
fn test_function_duplication_suppressed() {
    let out = check(concat!(
        "const std = @import(\"std\");\n",
        "test \"alpha\" {\n",
        "    var r: i32 = 0;\n",
        "    r += 1;\n",
        "    r += 2;\n",
        "    r += 3;\n",
        "    r += 4;\n",
        "    r += 5;\n",
        "}\n",
        "test \"beta\" {\n",
        "    var r: i32 = 0;\n",
        "    r += 1;\n",
        "    r += 2;\n",
        "    r += 3;\n",
        "    r += 4;\n",
        "    r += 5;\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Code Duplication"), "test duplication should be suppressed, got: {out}");
}

// ===========================================================================
// Assertions (3)
// ===========================================================================

#[test]
fn assertion_block_above_threshold() {
    let mut code = String::from("const std = @import(\"std\");\nfn f() void {\n");
    for _ in 0..asserts_above() {
        code.push_str("    std.debug.assert(true);\n");
    }
    code.push_str("}\n");
    let out = check(&code);
    assert!(has_smell(&out, "Large Assertion Block"), "got: {out}");
}

#[test]
fn assertion_block_below_threshold() {
    let mut code = String::from("const std = @import(\"std\");\nfn f() void {\n");
    for _ in 0..5 {
        code.push_str("    std.debug.assert(true);\n");
    }
    code.push_str("}\n");
    let out = check(&code);
    assert!(!has_smell(&out, "Large Assertion Block"));
}

#[test]
fn assertion_block_interrupted_resets() {
    let mut code = String::from("const std = @import(\"std\");\nfn f(x: i32) void {\n");
    for _ in 0..5 {
        code.push_str("    std.debug.assert(true);\n");
    }
    code.push_str("    const y = x + 1;\n    _ = y;\n");
    for _ in 0..5 {
        code.push_str("    std.debug.assert(true);\n");
    }
    code.push_str("}\n");
    let out = check(&code);
    assert!(!has_smell(&out, "Large Assertion Block"));
}

// ===========================================================================
// Compound conditions (2)
// ===========================================================================

#[test]
fn compound_condition_detected() {
    let out = check(concat!(
        "fn f(a: bool, b: bool, c: bool) bool {\n",
        "    if (a and b or c) {\n",
        "        if (a or b and c) {\n",
        "            if (b and c or a) {\n",
        "                return true;\n",
        "            }\n",
        "        }\n",
        "    }\n",
        "    return false;\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Complex Conditional"), "got: {out}");
}

#[test]
fn compound_condition_simple_not_detected() {
    let out = check(concat!(
        "fn f(a: bool, b: bool) bool {\n",
        "    if (a and b) {\n",
        "        return true;\n",
        "    }\n",
        "    return false;\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Complex Conditional"));
}

// ===========================================================================
// Primitive obsession (3)
// ===========================================================================

#[test]
fn primitive_obsession_all_primitives() {
    let out = check("fn f(a: i32, b: i32, c: i32, d: i32) void { _ = a; _ = b; _ = c; _ = d; }\n");
    assert!(has_smell(&out, "Primitive Obsession"), "got: {out}");
}

#[test]
fn primitive_obsession_below_min_typed() {
    let out = check("fn f(a: i32, b: i32) void { _ = a; _ = b; }\n");
    assert!(!has_smell(&out, "Primitive Obsession"));
}

#[test]
fn primitive_obsession_mixed_no_smell() {
    let out = check(concat!(
        "const Allocator = struct { x: i32 };\n",
        "fn f(a: Allocator, b: Allocator, c: i32, d: i32) void { _ = a; _ = b; _ = c; _ = d; }\n",
    ));
    assert!(!has_smell(&out, "Primitive Obsession"));
}

// ===========================================================================
// LCOM4 (3)
// ===========================================================================

#[test]
fn lcom4_connected_no_smell() {
    let out = check(concat!(
        "const S = struct {\n",
        "    x: i32,\n",
        "    pub fn getX(self: S) i32 { return self.x; }\n",
        "    pub fn setX(self: *S, v: i32) void { self.x = v; }\n",
        "};\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_disconnected() {
    let out = check(concat!(
        "const S = struct {\n",
        "    a: i32,\n",
        "    b: i32,\n",
        "    c: i32,\n",
        "    d: i32,\n",
        "    pub fn getA(self: S) i32 { return self.a; }\n",
        "    pub fn getB(self: S) i32 { return self.b; }\n",
        "    pub fn getC(self: S) i32 { return self.c; }\n",
        "    pub fn getD(self: S) i32 { return self.d; }\n",
        "    pub fn setA(self: *S, v: i32) void { self.a = v; }\n",
        "    pub fn setB(self: *S, v: i32) void { self.b = v; }\n",
        "    pub fn setC(self: *S, v: i32) void { self.c = v; }\n",
        "    pub fn setD(self: *S, v: i32) void { self.d = v; }\n",
        "};\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"), "got: {out}");
}

#[test]
fn lcom4_single_method_no_smell() {
    let out = check(concat!(
        "const S = struct {\n",
        "    x: i32,\n",
        "    pub fn getX(self: S) i32 { return self.x; }\n",
        "};\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_methods_connected_by_call() {
    let out = check(concat!(
        "const Coord = struct {\n",
        "    state: i32 = 0,\n",
        "    pub fn process(self: *Coord, e: i32) bool { return self.validate(e) and self.dispatch(e); }\n",
        "    pub fn validate(self: *Coord, e: i32) bool { _ = self; return e > 0; }\n",
        "    pub fn dispatch(self: *Coord, e: i32) bool { return self.send(e); }\n",
        "    pub fn send(self: *Coord, e: i32) bool { _ = self; _ = e; return true; }\n",
        "};\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"), "got: {out}");
}

#[test]
fn lcom4_mixed_field_and_call_connection() {
    let out = check(concat!(
        "const Mixed = struct {\n",
        "    x: i32 = 0,\n",
        "    pub fn a(self: *Mixed) i32 { return self.x; }\n",
        "    pub fn b(self: *Mixed) i32 { self.x = 1; return self.c(); }\n",
        "    pub fn c(self: *Mixed) i32 { _ = self; return 42; }\n",
        "};\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_god_struct_still_fires() {
    let out = check(concat!(
        "const Svc = struct {\n",
        "    db: i32 = 0, cache: i32 = 0, mailer: i32 = 0, events: i32 = 0, audit: i32 = 0,\n",
        "    pub fn getUser(self: *Svc) i32 { return self.db; }\n",
        "    pub fn cacheUser(self: *Svc) i32 { return self.cache; }\n",
        "    pub fn sendWelcome(self: *Svc) i32 { return self.mailer; }\n",
        "    pub fn publish(self: *Svc) i32 { return self.events; }\n",
        "    pub fn auditLog(self: *Svc) i32 { return self.audit; }\n",
        "};\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"), "got: {out}");
}

#[test]
fn lcom4_dependency_method_calls_dont_falsely_connect() {
    let out = check(concat!(
        "const Svc = struct {\n",
        "    db: i32 = 0, cache: i32 = 0, log: i32 = 0,\n",
        "    pub fn a(self: *Svc) i32 { return self.db; }\n",
        "    pub fn b(self: *Svc) i32 { return self.cache; }\n",
        "    pub fn c(self: *Svc) i32 { return self.log; }\n",
        "};\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"));
}

// ===========================================================================
// Method naming (3)
// ===========================================================================

#[test]
fn function_has_no_prefix() {
    let out = debug("fn standalone() void {}\n");
    assert!(out.contains("standalone"), "got: {out}");
    assert!(!out.contains(".standalone"));
}

#[test]
fn method_has_struct_type_prefix() {
    let out = debug(concat!(
        "const Svc = struct {\n",
        "    pub fn handle(self: Svc) void { _ = self; }\n",
        "};\n",
    ));
    assert!(out.contains("Svc.handle"), "got: {out}");
}

#[test]
fn init_is_constructor() {
    let out = debug(concat!(
        "const Svc = struct {\n",
        "    x: i32,\n",
        "    pub fn init(x: i32) Svc {\n",
        "        return .{ .x = x };\n",
        "    }\n",
        "};\n",
    ));
    assert!(out.contains("Svc.init"), "got: {out}");
}

// ===========================================================================
// Zig-specific (12)
// ===========================================================================

#[test]
fn catch_increments_cc() {
    let out = debug("fn f(val: anyerror!i32) i32 {\n    return val catch 0;\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert_eq!(cc, 2);
}

#[test]
fn orelse_increments_cc() {
    let out = debug("fn f(opt: ?i32) i32 {\n    return opt orelse 0;\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert_eq!(cc, 2);
}

#[test]
fn test_decl_analyzed() {
    let out = debug(concat!(
        "const std = @import(\"std\");\n",
        "test \"my test\" {\n",
        "    if (true) {}\n",
        "}\n",
    ));
    assert!(out.contains("test_my_test"), "got: {out}");
}

#[test]
fn test_decl_name_has_test_prefix() {
    let out = debug(concat!(
        "const std = @import(\"std\");\n",
        "test \"some feature\" {\n",
        "    if (true) {}\n",
        "}\n",
    ));
    assert!(out.contains("test_some_feature"), "got: {out}");
}

#[test]
fn comptime_block_skipped() {
    let out = debug(concat!(
        "fn f(comptime T: type) T {\n",
        "    comptime {\n",
        "        if (true) {}\n",
        "    }\n",
        "    return undefined;\n",
        "}\n",
    ));
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert_eq!(cc, 1, "comptime block should not add CC, got: {cc}");
}

#[test]
fn defer_does_not_increment_cc() {
    let out = debug("fn f() void {\n    defer {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(1));
}

#[test]
fn errdefer_does_not_increment_cc() {
    let out = debug("fn f() void {\n    errdefer {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(1));
}

#[test]
fn labeled_block_no_cc() {
    let out = debug("fn f() i32 {\n    const val = blk: {\n        break :blk 42;\n    };\n    return val;\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(1));
}

#[test]
fn struct_method_detected() {
    let out = debug(concat!(
        "const Point = struct {\n",
        "    x: i32,\n",
        "    y: i32,\n",
        "    pub fn sum(self: Point) i32 {\n",
        "        return self.x + self.y;\n",
        "    }\n",
        "};\n",
    ));
    assert!(out.contains("Point.sum"), "got: {out}");
}

#[test]
fn self_param_excluded() {
    let out = debug(concat!(
        "const P = struct {\n",
        "    x: i32,\n",
        "    pub fn getX(self: P) i32 {\n",
        "        return self.x;\n",
        "    }\n",
        "};\n",
    ));
    assert_eq!(function_metric(&out, "P.getX", "args"), Some(0));
}

#[test]
fn for_range_captures() {
    let out = debug("fn f(items: []const u8) u32 {\n    var s: u32 = 0;\n    for (items) |item| {\n        s += item;\n    }\n    return s;\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert_eq!(cc, 2);
}

#[test]
fn payload_capture_no_extra_cc() {
    let out = debug("fn f(items: []const u8) u32 {\n    var s: u32 = 0;\n    for (items) |item| {\n        s += item;\n    }\n    return s;\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert_eq!(cc, 2, "payload capture should not add extra CC");
}

// ===========================================================================
// Performance (2)
// ===========================================================================

#[test]
fn performance_1000_loc() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("perf.zig");
    let mut code = String::new();
    for i in 0..50 {
        code.push_str(&format!("fn func{i}(x: i32) i32 {{\n"));
        for j in 0..18 {
            code.push_str(&format!("    const v{j} = {j};\n"));
        }
        code.push_str("    return x;\n}\n\n");
    }
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
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("structs.zig");
    let mut code = String::new();
    for i in 0..10 {
        code.push_str(&format!("const S{i} = struct {{\n    x: i32,\n"));
        for j in 0..5 {
            code.push_str(&format!(
                "    pub fn m{j}(self: S{i}) i32 {{ return self.x + {j}; }}\n"
            ));
        }
        code.push_str("};\n\n");
    }
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
// Edge cases (5)
// ===========================================================================

#[test]
fn clean_zig_module_not_flagged() {
    let out = check(concat!(
        "const Point = struct {\n",
        "    x: i32,\n",
        "    y: i32,\n",
        "    pub fn add(self: Point) i32 {\n",
        "        return self.x + self.y;\n",
        "    }\n",
        "};\n",
        "pub fn helper(x: i32) i32 {\n",
        "    return x + 1;\n",
        "}\n",
    ));
    assert!(out.is_empty(), "got: {out}");
}

#[test]
fn comments_only_no_output() {
    let out = check("// this is a comment\n// another comment\n");
    assert!(out.is_empty());
}

#[test]
fn empty_file_no_crash() {
    let out = check("");
    assert!(out.is_empty());
}

#[test]
fn empty_function_body() {
    let out = debug("fn f() void {}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(1));
}

#[test]
fn multiple_functions_independent_metrics() {
    let out = debug(concat!(
        "fn simple() void {}\n",
        "fn complex(x: bool) void {\n",
        "    if (x) {}\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "simple", "cc"), Some(1));
    assert_eq!(function_metric(&out, "complex", "cc"), Some(2));
}
