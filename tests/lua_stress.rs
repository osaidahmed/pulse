mod common;

use common::*;
use std::process::Command;

lang_helpers!("lua");

// ===========================================================================
// CC counting (16)
// ===========================================================================

#[test]
fn cc_counts_if() {
    let out = debug("function f(x)\n    if x > 0 then end\n    return x\nend\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_elseif() {
    let out = debug(
        "function f(x)\n    if x > 0 then\n    elseif x < 0 then\n    end\n    return x\nend\n",
    );
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_counts_for_in() {
    let out = debug("function f(t)\n    for _, v in ipairs(t) do end\nend\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_for_numeric() {
    let out = debug("function f()\n    for i = 1, 10 do end\nend\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_while() {
    let out = debug("function f(x)\n    while x > 0 do\n        x = x - 1\n    end\nend\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_repeat_until() {
    let out = debug("function f(x)\n    repeat\n        x = x + 1\n    until x > 10\nend\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_and() {
    let out = debug("function f(a, b)\n    if a and b then\n        return true\n    end\n    return false\nend\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_counts_or() {
    let out = debug("function f(a, b)\n    if a or b then\n        return true\n    end\n    return false\nend\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_chained_boolean() {
    let out = debug("function f(a, b, c)\n    if a and b or c then\n        return true\n    end\n    return false\nend\n");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 4, "got: {}", cc);
}

#[test]
fn cc_chained_boolean_4way() {
    let out = debug("function f(a, b, c, d)\n    if a and b and c and d then\n        return true\n    end\n    return false\nend\n");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 5, "got: {}", cc);
}

#[test]
fn cc_nested_if_counted_once() {
    let out = debug("function f(a, b)\n    if a then\n        if b then\n            return true\n        end\n    end\n    return false\nend\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_base_case_is_1() {
    let out = debug("function f()\nend\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(1));
}

#[test]
fn cc_if_with_else() {
    let out = debug("function f(x)\n    if x then\n        return 1\n    else\n        return 0\n    end\nend\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_multiple_if_accumulates() {
    let out = debug("function f(a, b)\n    if a then end\n    if b then end\n    return 0\nend\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_combined_for_and_if() {
    let out = debug("function f(t)\n    for _, v in ipairs(t) do\n        if v > 0 then end\n    end\nend\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_repeat_with_boolean_condition() {
    let out = debug("function f(x, y)\n    repeat\n        x = x + 1\n    until x > 10 or y\nend\n");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 3, "got: {}", cc);
}

// ===========================================================================
// Cognitive complexity (10)
// ===========================================================================

#[test]
fn cogc_flat_branches() {
    let out = debug("function f(a, b)\n    if a then end\n    if b then end\nend\n");
    assert_eq!(function_metric(&out, "f", "cogc"), Some(2));
}

#[test]
fn cogc_nested_ifs() {
    let out = debug("function f(a, b)\n    if a then\n        if b then end\n    end\nend\n");
    let cogc = function_metric(&out, "f", "cogc").unwrap_or(0);
    assert!(cogc >= 3, "nested ifs should have cogc >= 3, got: {}", cogc);
}

#[test]
fn cogc_elseif_no_extra_nesting() {
    let out = debug("function f(x)\n    if x > 0 then\n    elseif x < 0 then\n    end\nend\n");
    let cogc = function_metric(&out, "f", "cogc").unwrap_or(99);
    assert!(cogc <= 3, "elseif should not add excessive nesting, got: {}", cogc);
}

#[test]
fn cogc_else_flat() {
    let out = debug("function f(x)\n    if x then\n    else\n    end\nend\n");
    let cogc = function_metric(&out, "f", "cogc").unwrap_or(0);
    assert!(cogc >= 2, "else should add flat increment, got: {}", cogc);
}

#[test]
fn cogc_for_loop_nested() {
    let out = debug("function f(t)\n    for _, v in ipairs(t) do\n        if v > 0 then end\n    end\nend\n");
    let cogc = function_metric(&out, "f", "cogc").unwrap_or(0);
    assert!(cogc >= 3, "for+nested if should have cogc >= 3, got: {}", cogc);
}

#[test]
fn cogc_boolean_single_sequence() {
    let out = debug("function f(a, b)\n    if a and b then end\nend\n");
    let cogc = function_metric(&out, "f", "cogc").unwrap_or(0);
    assert!(cogc >= 2, "boolean op should add cogc, got: {}", cogc);
}

#[test]
fn cogc_boolean_mixed_sequence() {
    let out = debug("function f(a, b, c)\n    if a and b or c then end\nend\n");
    let cogc = function_metric(&out, "f", "cogc").unwrap_or(0);
    assert!(cogc >= 3, "mixed boolean ops should add more cogc, got: {}", cogc);
}

#[test]
fn cogc_triggers_complex_method() {
    let mut code = String::from("function f(x)\n");
    for i in 0..8 {
        code.push_str(&format!("    if x > {} then\n        if x < {} then end\n    end\n", i, i + 100));
    }
    code.push_str("end\n");
    let out = check(&code);
    assert!(has_smell(&out, "Complex Method"), "cogc should trigger, got: {}", out);
}

#[test]
fn cogc_below_threshold_no_smell() {
    let out = check("function f(a, b)\n    if a then end\n    if b then end\nend\n");
    assert!(!has_smell(&out, "Complex Method"));
}

#[test]
fn cogc_repeat_until_nested() {
    let out = debug("function f(x)\n    repeat\n        if x > 0 then end\n        x = x + 1\n    until x > 10\nend\n");
    let cogc = function_metric(&out, "f", "cogc").unwrap_or(0);
    assert!(cogc >= 3, "repeat with nested if should have cogc >= 3, got: {}", cogc);
}

// ===========================================================================
// Nesting depth (6)
// ===========================================================================

#[test]
fn nesting_depth_simple() {
    let out = debug("function f(x)\n    if x then end\nend\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(1));
}

#[test]
fn nesting_depth_nested() {
    let out = debug("function f(a, b, c)\n    if a then\n        if b then\n            if c then end\n        end\n    end\nend\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(3));
}

#[test]
fn nesting_depth_sequential_not_accumulated() {
    let out = debug("function f(a, b)\n    if a then end\n    if b then end\nend\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(1));
}

#[test]
fn nesting_deep_if_chain() {
    let out = debug(concat!(
        "function f(a, b, c, d, e)\n",
        "    if a then\n",
        "        if b then\n",
        "            if c then\n",
        "                if d then\n",
        "                    if e then end\n",
        "                end\n",
        "            end\n",
        "        end\n",
        "    end\n",
        "end\n",
    ));
    assert_eq!(function_metric(&out, "f", "nesting"), Some(5));
}

#[test]
fn nesting_for_with_if() {
    let out = debug("function f(t)\n    for _, v in ipairs(t) do\n        if v > 0 then end\n    end\nend\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(2));
}

#[test]
fn nesting_repeat_depth() {
    let out = debug("function f(x)\n    repeat\n        if x > 0 then end\n        x = x + 1\n    until x > 10\nend\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(2));
}

// ===========================================================================
// Bump counting (2)
// ===========================================================================

#[test]
fn bumpy_road_two_bumps() {
    let out = check(concat!(
        "function f(a, b, c, d, e, g)\n",
        "    if a then\n        if b then\n            if c then end\n        end\n    end\n",
        "    local x = 1\n",
        "    if d then\n        if e then\n            if g then end\n        end\n    end\n",
        "    return x\n",
        "end\n",
    ));
    assert!(
        has_smell(&out, "Nested Conditional Chunks") || has_smell(&out, "Complex Method"),
        "got: {}",
        out
    );
}

#[test]
fn bumpy_road_single_bump_not_flagged() {
    let out = check(concat!(
        "function f(a, b, c)\n",
        "    if a then\n        if b then\n            if c then end\n        end\n    end\n",
        "    return 0\n",
        "end\n",
    ));
    assert!(!has_smell(&out, "Nested Conditional Chunks"));
}

// ===========================================================================
// Arguments (6)
// ===========================================================================

#[test]
fn args_zero() {
    let out = debug("function f()\nend\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(0));
}

#[test]
fn args_one() {
    let out = debug("function f(x)\nend\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(1));
}

#[test]
fn args_five_at_threshold() {
    let out = check("function f(a, b, c, d, e)\n    return 0\nend\n");
    assert!(!has_smell(&out, "Excess Arguments"));
}

#[test]
fn args_six_over_threshold() {
    let out = check("function f(a, b, c, d, e, g)\n    return 0\nend\n");
    assert!(has_smell(&out, "Excess Arguments"), "got: {}", out);
}

#[test]
fn args_method_self_not_counted() {
    let out = debug("local M = {}\nfunction M:doWork(a, b)\nend\n");
    assert_eq!(function_metric(&out, "M.doWork", "args"), Some(2));
}

#[test]
fn args_vararg_counted() {
    let out = debug("function f(a, ...)\nend\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(2));
}

// ===========================================================================
// LOC counting (4)
// ===========================================================================

#[test]
fn loc_single_line_function() {
    let out = debug("function f()\nend\n");
    assert_eq!(function_metric(&out, "f", "loc"), Some(2));
}

#[test]
fn loc_multiline() {
    let out = debug("function f()\n    local a = 1\n    local b = 2\n    return a + b\nend\n");
    assert_eq!(function_metric(&out, "f", "loc"), Some(5));
}

#[test]
fn loc_comments_excluded_module() {
    let out = debug("-- comment\nfunction f()\n    return 1\nend\n");
    assert!(out.contains("3 LOC"), "comments should be excluded, got: {}", out);
}

#[test]
fn loc_empty_lines_excluded_module() {
    let out = debug("\n\nfunction f()\n    return 1\nend\n\n");
    assert!(out.contains("3 LOC"), "empty lines should be excluded, got: {}", out);
}

// ===========================================================================
// Embedded blocks (2)
// ===========================================================================

#[test]
fn embedded_large_long_string() {
    let mut code = String::from("function f()\n    return [[\n");
    for i in 0..embedded_lines_above() {
        code.push_str(&format!("        line {}\n", i));
    }
    code.push_str("    ]]\nend\n");
    let out = check(&code);
    assert!(has_smell(&out, "Large Embedded Block"), "got: {}", out);
}

#[test]
fn embedded_small_string_not_flagged() {
    let out = check("function f()\n    return \"hello\"\nend\n");
    assert!(!has_smell(&out, "Large Embedded Block"));
}

// ===========================================================================
// Duplication (4)
// ===========================================================================

#[test]
fn exact_duplication_detected() {
    let out = check(concat!(
        "function a(d)\n    local r = 0\n    for _, v in ipairs(d) do\n        r = r + v\n    end\n    r = r * 2\n    return r\nend\n\n",
        "function b(d)\n    local r = 0\n    for _, v in ipairs(d) do\n        r = r + v\n    end\n    r = r * 2\n    return r\nend\n",
    ));
    assert!(has_smell(&out, "Code Duplication"), "got: {}", out);
}

#[test]
fn exact_duplication_below_min_loc() {
    let out = check(concat!(
        "function a(x)\n    return x + 1\nend\n\n",
        "function b(x)\n    return x + 1\nend\n",
    ));
    assert!(!has_smell(&out, "Code Duplication"));
}

#[test]
fn fuzzy_duplication_detected() {
    let out = check(concat!(
        "function a(d)\n    local r = 0\n    for _, v in ipairs(d) do\n        r = r + v\n    end\n    r = r * 2\n    return r\nend\n\n",
        "function b(d)\n    local r = 0\n    for _, v in ipairs(d) do\n        r = r + v\n    end\n    r = r * 3\n    return r\nend\n",
    ));
    assert!(has_smell(&out, "Code Duplication"), "got: {}", out);
}

#[test]
fn test_function_duplication_suppressed() {
    let out = check(concat!(
        "function test_a(d)\n    local r = 0\n    for _, v in ipairs(d) do\n        r = r + v\n    end\n    r = r * 2\n    return r\nend\n\n",
        "function test_b(d)\n    local r = 0\n    for _, v in ipairs(d) do\n        r = r + v\n    end\n    r = r * 2\n    return r\nend\n",
    ));
    assert!(!has_smell(&out, "Code Duplication"));
}

// ===========================================================================
// Assertions (3)
// ===========================================================================

#[test]
fn assertion_block_above_threshold() {
    let mut code = String::from("function test_many()\n");
    for i in 0..asserts_above() {
        code.push_str(&format!("    assert({} > 0)\n", i));
    }
    code.push_str("end\n");
    let out = check(&code);
    assert!(has_smell(&out, "Large Assertion Block"), "got: {}", out);
}

#[test]
fn assertion_block_below_threshold() {
    let mut code = String::from("function test_few()\n");
    for i in 0..3 {
        code.push_str(&format!("    assert({} > 0)\n", i));
    }
    code.push_str("end\n");
    let out = check(&code);
    assert!(!has_smell(&out, "Large Assertion Block"));
}

#[test]
fn assertion_block_interrupted_resets() {
    let mut code = String::from("function test_split()\n");
    for i in 0..5 {
        code.push_str(&format!("    assert({} > 0)\n", i));
    }
    code.push_str("    local x = 1\n");
    for i in 0..5 {
        code.push_str(&format!("    assert({} > 0)\n", i + 10));
    }
    code.push_str("end\n");
    let out = check(&code);
    assert!(!has_smell(&out, "Large Assertion Block"));
}

// ===========================================================================
// Compound conditions (2)
// ===========================================================================

#[test]
fn compound_condition_detected() {
    let out = debug("function f(a, b, c)\n    if a and b or c then end\n    return 0\nend\n");
    let cond = function_metric(&out, "f", "conditions").unwrap_or(0);
    assert!(cond >= 1, "compound condition should be detected, got: {}", cond);
}

#[test]
fn compound_condition_simple_not_detected() {
    let out = debug("function f(a, b)\n    if a and b then end\nend\n");
    let cond = function_metric(&out, "f", "conditions").unwrap_or(99);
    assert_eq!(cond, 0);
}

// ===========================================================================
// Primitive obsession (2)
// ===========================================================================

#[test]
fn primitive_obsession_not_triggered() {
    let out = check("function f(a, b, c, d, e, g)\n    return 0\nend\n");
    assert!(!has_smell(&out, "Primitive Obsession"));
}

#[test]
fn typed_param_count_always_zero() {
    let out = debug("function f(a, b, c)\n    return 0\nend\n");
    assert!(out.contains("primitives=0/0"), "lua should have no types, got: {}", out);
}

// ===========================================================================
// LCOM4 (3)
// ===========================================================================

#[test]
fn lcom4_connected_no_smell() {
    let out = check(concat!(
        "local M = {}\n",
        "function M:getA()\n    return self.x\nend\n",
        "function M:getB()\n    return self.x\nend\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_disconnected() {
    let mut code = String::from("local M = {}\n");
    for i in 0..8 {
        code.push_str(&format!(
            "function M:get{}()\n    return self.field{}\nend\n",
            i, i
        ));
    }
    let out = check(&code);
    assert!(has_smell(&out, "Low Cohesion"), "got: {}", out);
}

#[test]
fn lcom4_single_method_no_smell() {
    let out = check(concat!(
        "local M = {}\n",
        "function M:getA()\n    return self.x\nend\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"));
}

// ===========================================================================
// Method naming (3)
// ===========================================================================

#[test]
fn function_has_no_prefix() {
    let out = debug("function doWork()\nend\n");
    assert!(out.contains("doWork"), "got: {}", out);
    assert!(!out.contains(".doWork"), "standalone should have no class prefix, got: {}", out);
}

#[test]
fn method_has_class_prefix() {
    let out = debug("local M = {}\nfunction M:doWork()\nend\n");
    assert!(out.contains("M.doWork"), "got: {}", out);
}

#[test]
fn init_is_constructor() {
    let out = debug("local M = {}\nfunction M:init(a, b, c, d, e, g)\nend\n");
    assert!(
        out.contains("M.init"),
        "constructor should be detected, got: {}",
        out
    );
}

// ===========================================================================
// Lua-specific (12)
// ===========================================================================

#[test]
fn repeat_until_cc() {
    let out = debug("function f(x)\n    repeat\n        x = x + 1\n    until x > 10\nend\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn repeat_until_boolean() {
    let out = debug("function f(x, y)\n    repeat\n        x = x + 1\n    until x > 10 and y\nend\n");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 3, "repeat with and should have cc >= 3, got: {}", cc);
}

#[test]
fn for_in_pairs_cc() {
    let out = debug("function f(t)\n    for k, v in pairs(t) do end\nend\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn for_numeric_cc() {
    let out = debug("function f()\n    for i = 1, 100 do end\nend\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn colon_method_detected() {
    let out = debug("local C = {}\nfunction C:process(x)\n    self.x = x\nend\n");
    assert!(out.contains("C.process"), "got: {}", out);
}

#[test]
fn colon_self_excluded_from_args() {
    let out = debug("local C = {}\nfunction C:process(a, b, c)\nend\n");
    assert_eq!(function_metric(&out, "C.process", "args"), Some(3));
}

#[test]
fn dot_method_no_self_exclusion() {
    let out = debug("local C = {}\nfunction C.process(self, a, b)\nend\n");
    assert_eq!(function_metric(&out, "C.process", "args"), Some(3));
}

#[test]
fn local_function_analyzed() {
    let out = debug("local function helper(x)\n    return x + 1\nend\n");
    assert!(out.contains("helper"), "got: {}", out);
    assert_eq!(function_metric(&out, "helper", "args"), Some(1));
}

#[test]
fn anonymous_function_in_variable() {
    let out = debug("local f = function(a, b)\n    return a + b\nend\n");
    assert!(out.contains(" f "), "got: {}", out);
    assert_eq!(function_metric(&out, "f", "args"), Some(2));
}

#[test]
fn long_string_tracked() {
    let mut code = String::from("function f()\n    return [[\n");
    for i in 0..embedded_lines_above() {
        code.push_str(&format!("        line {}\n", i));
    }
    code.push_str("    ]]\nend\n");
    let out = check(&code);
    assert!(has_smell(&out, "Large Embedded Block"), "got: {}", out);
}

#[test]
fn multiline_comment_first_line_excluded() {
    let out = debug("-- single line comment\nfunction f()\n    return 1\nend\n");
    assert!(out.contains("3 LOC"), "single-line comments should be excluded, got: {}", out);
}

#[test]
fn nested_functions_outer_only() {
    let out = debug(concat!(
        "function outer(x)\n",
        "    local inner = function(y)\n",
        "        return y + 1\n",
        "    end\n",
        "    return inner(x)\n",
        "end\n",
    ));
    assert!(out.contains("1 functions"), "only outer should be top-level, got: {}", out);
}

// ===========================================================================
// Performance (2)
// ===========================================================================

#[test]
fn performance_1000_loc() {
    let mut code = String::new();
    for i in 0..20 {
        code.push_str(&format!("function fn{}(x)\n", i));
        for j in 0..48 {
            code.push_str(&format!("    local v{} = {}\n", j, j));
        }
        code.push_str("    return 0\nend\n\n");
    }
    let start = std::time::Instant::now();
    let _ = check(&code);
    let elapsed = start.elapsed();
    assert!(elapsed.as_millis() < 500, "took: {}ms", elapsed.as_millis());
}

#[test]
fn performance_module_hierarchy() {
    let mut code = String::from("local M = {}\n");
    for i in 0..15 {
        code.push_str(&format!("function M:method{}(x)\n    self.f{} = x\nend\n", i, i));
    }
    let start = std::time::Instant::now();
    let _ = check(&code);
    let elapsed = start.elapsed();
    assert!(elapsed.as_millis() < 500, "took: {}ms", elapsed.as_millis());
}

// ===========================================================================
// Edge cases (5)
// ===========================================================================

#[test]
fn clean_lua_module_not_flagged() {
    let out = check(concat!(
        "local M = {}\n",
        "function M:init()\n    self.x = 0\nend\n",
        "function M:get()\n    return self.x\nend\n",
        "return M\n",
    ));
    assert!(out.is_empty(), "got: {}", out);
}

#[test]
fn comments_only_no_output() {
    let out = check("-- only comments\n-- nothing else\n");
    assert!(out.is_empty());
}

#[test]
fn empty_file_no_crash() {
    let out = check("");
    assert!(out.is_empty());
}

#[test]
fn empty_function_body() {
    let out = debug("function f()\nend\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(1));
    assert_eq!(function_metric(&out, "f", "loc"), Some(2));
}

#[test]
fn multiple_functions_independent_metrics() {
    let out = debug(concat!(
        "function alpha(x)\n    if x then end\nend\n",
        "function zeta()\nend\n",
    ));
    assert_eq!(function_metric(&out, "alpha", "cc"), Some(2));
    assert_eq!(function_metric(&out, "zeta", "cc"), Some(1));
}
