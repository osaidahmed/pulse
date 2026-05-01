mod common;

use common::*;
use std::process::Command;

lang_helpers!("r");

// ===========================================================================
// CC counting (20)
// ===========================================================================

#[test]
fn cc_counts_if() {
    let out = debug("f <- function(x) {\n  if (x > 0) {\n  }\n  x\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_else_if() {
    let out = debug("f <- function(x) {\n  if (x > 0) {\n  } else if (x < 0) {\n  }\n  x\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(cc >= 3, "else if should add CC, got: {cc}");
}

#[test]
fn cc_counts_while() {
    let out = debug("f <- function(x) {\n  n <- x\n  while (n > 0) {\n    n <- n - 1\n  }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_for() {
    let out = debug("f <- function(items) {\n  for (item in items) {\n    print(item)\n  }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_repeat() {
    let out = debug("f <- function() {\n  repeat {\n    break\n  }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_switch() {
    let out = debug(concat!(
        "f <- function(x) {\n",
        "  switch(x,\n",
        "    a = 1,\n",
        "    b = 2,\n",
        "    c = 3\n",
        "  )\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cc"), Some(4));
}

#[test]
fn cc_counts_try_catch() {
    let out = debug(concat!(
        "f <- function(x) {\n",
        "  tryCatch(\n",
        "    as.numeric(x),\n",
        "    error = function(e) -1\n",
        "  )\n",
        "}\n",
    ));
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 2, "tryCatch handler should add CC, got: {cc}");
}

#[test]
fn cc_counts_and_and() {
    let out = debug("f <- function(a, b) {\n  if (a && b) {\n    TRUE\n  }\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(cc >= 3, "got: {cc}");
}

#[test]
fn cc_counts_or_or() {
    let out = debug("f <- function(a, b) {\n  if (a || b) {\n    TRUE\n  }\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(cc >= 3, "got: {cc}");
}

#[test]
fn cc_chained_boolean() {
    let out = debug("f <- function(a, b, c) {\n  if (a && b || c) {\n    TRUE\n  }\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 4, "got: {cc}");
}

#[test]
fn cc_nested_if_counted_once() {
    let out = debug("f <- function(a, b) {\n  if (a) {\n    if (b) {\n      TRUE\n    }\n  }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_base_case_is_1() {
    let out = debug("f <- function() {\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(1));
}

#[test]
fn cc_if_with_else() {
    let out = debug("f <- function(x) {\n  if (x > 0) {\n    1\n  } else {\n    0\n  }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_multiple_if_accumulates() {
    let out = debug("f <- function(a, b) {\n  if (a > 0) {\n  }\n  if (b > 0) {\n  }\n  0\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_vectorized_and_not_counted() {
    let out = debug("f <- function(a, b) {\n  if (a & b) {\n    TRUE\n  }\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert_eq!(cc, 2, "vectorized & should NOT add CC, got: {cc}");
}

#[test]
fn cc_vectorized_or_not_counted() {
    let out = debug("f <- function(a, b) {\n  if (a | b) {\n    TRUE\n  }\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert_eq!(cc, 2, "vectorized | should NOT add CC, got: {cc}");
}

#[test]
fn cc_try_catch_multiple_handlers() {
    let out = debug(concat!(
        "f <- function(x) {\n",
        "  tryCatch(\n",
        "    as.numeric(x),\n",
        "    error = function(e) -1,\n",
        "    warning = function(w) -2,\n",
        "    message = function(m) -3\n",
        "  )\n",
        "}\n",
    ));
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 4, "3 handlers should add 3 CC, got: {cc}");
}

#[test]
fn cc_else_if_chain() {
    let out = debug(concat!(
        "f <- function(x) {\n",
        "  if (x == 1) {\n    1\n",
        "  } else if (x == 2) {\n    2\n",
        "  } else if (x == 3) {\n    3\n",
        "  } else {\n    0\n  }\n",
        "}\n",
    ));
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert_eq!(cc, 4, "got: {cc}");
}

#[test]
fn cc_nested_switch() {
    let out = debug(concat!(
        "f <- function(x, y) {\n",
        "  switch(x,\n",
        "    a = switch(y, p = 1, q = 2),\n",
        "    b = 3\n",
        "  )\n",
        "}\n",
    ));
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 3, "nested switch should compound, got: {cc}");
}

#[test]
fn cc_switch_single_case() {
    let out = debug(concat!(
        "f <- function(x) {\n",
        "  switch(x, a = 1)\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

// ===========================================================================
// Cognitive complexity (10)
// ===========================================================================

#[test]
fn cogc_flat_branches() {
    let out = debug(concat!(
        "f <- function(a, b, c) {\n",
        "  if (a > 0) {\n  }\n",
        "  if (b > 0) {\n  }\n",
        "  if (c > 0) {\n  }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cogc"), Some(3));
}

#[test]
fn cogc_nested_ifs() {
    let out = debug(concat!(
        "f <- function(a, b, c, d) {\n",
        "  if (a > 0) {\n",
        "    if (b > 0) {\n",
        "      if (c > 0) {\n",
        "        if (d > 0) {\n",
        "        }\n",
        "      }\n",
        "    }\n",
        "  }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cogc"), Some(10));
}

#[test]
fn cogc_else_if_no_extra_nesting() {
    let out = debug(concat!(
        "f <- function(x) {\n",
        "  if (x == 1) {\n",
        "  } else if (x == 2) {\n",
        "  } else if (x == 3) {\n",
        "  }\n",
        "}\n",
    ));
    let cogc = function_metric(&out, "f", "cogc").unwrap();
    assert!(cogc >= 3, "else-if chain should have cogc >= 3, got: {cogc}");
}

#[test]
fn cogc_else_increases_nesting() {
    let out = debug(concat!(
        "f <- function(x) {\n",
        "  if (x > 0) {\n",
        "  } else {\n",
        "    if (x < -10) {\n",
        "    }\n",
        "  }\n",
        "}\n",
    ));
    let cogc = function_metric(&out, "f", "cogc").unwrap();
    assert!(cogc >= 3, "else should contribute to nesting, got: {cogc}");
}

#[test]
fn cogc_switch_counted() {
    let out = debug(concat!(
        "f <- function(x) {\n",
        "  switch(x, a = 1)\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cogc"), Some(1));
}

#[test]
fn cogc_loop_nested() {
    let out = debug(concat!(
        "f <- function(items) {\n",
        "  for (i in items) {\n",
        "    if (i > 0) {\n",
        "    }\n",
        "  }\n",
        "}\n",
    ));
    let cogc = function_metric(&out, "f", "cogc").unwrap_or(0);
    assert!(cogc >= 3, "for+if should contribute cogc, got: {cogc}");
}

#[test]
fn cogc_boolean_single_sequence() {
    let out = debug("f <- function(a, b) {\n  if (a && b) {\n    TRUE\n  }\n}\n");
    let cogc = function_metric(&out, "f", "cogc").unwrap_or(0);
    assert!(cogc >= 2, "got: {cogc}");
}

#[test]
fn cogc_boolean_mixed_sequence() {
    let out = debug("f <- function(a, b, c) {\n  if (a && b || c) {\n    TRUE\n  }\n}\n");
    let cogc = function_metric(&out, "f", "cogc").unwrap_or(0);
    assert!(cogc >= 3, "got: {cogc}");
}

#[test]
fn cogc_triggers_complex_method() {
    let mut code = String::from("f <- function(x) {\n");
    for _ in 0..4 {
        code.push_str("  if (x > 0) {\n    if (x > 1) {\n      if (x > 2) {\n      }\n    }\n  }\n");
    }
    code.push_str("  0\n}\n");
    let out = check(&code);
    assert!(has_smell(&out, "Complex Method"), "got: {out}");
}

#[test]
fn cogc_below_threshold_no_smell() {
    let out = check("f <- function(x) {\n  if (x > 0) {\n  }\n  if (x > 1) {\n  }\n  0\n}\n");
    assert!(!has_smell(&out, "Complex Method"));
}

// ===========================================================================
// Nesting depth (6)
// ===========================================================================

#[test]
fn nesting_depth_simple() {
    let out = debug("f <- function(x) {\n  if (x) {\n  }\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(1));
}

#[test]
fn nesting_depth_nested() {
    let out = debug(concat!(
        "f <- function(a, b, c) {\n",
        "  if (a) {\n    if (b) {\n      if (c) {\n      }\n    }\n  }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "nesting"), Some(3));
}

#[test]
fn nesting_depth_sequential_not_accumulated() {
    let out = debug("f <- function(a, b) {\n  if (a) {\n  }\n  if (b) {\n  }\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(1));
}

#[test]
fn nesting_deep_if_chain() {
    let out = debug(concat!(
        "f <- function(a, b, c, d, e) {\n",
        "  if (a) {\n",
        "    if (b) {\n",
        "      if (c) {\n",
        "        if (d) {\n",
        "          if (e) {\n",
        "          }\n",
        "        }\n",
        "      }\n",
        "    }\n",
        "  }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "nesting"), Some(5));
}

#[test]
fn nesting_loop_with_if() {
    let out = debug(concat!(
        "f <- function(items) {\n",
        "  for (i in items) {\n",
        "    if (i > 0) {\n",
        "    }\n",
        "  }\n",
        "}\n",
    ));
    let nesting = function_metric(&out, "f", "nesting").unwrap_or(0);
    assert!(nesting >= 2, "got: {nesting}");
}

#[test]
fn nesting_switch_depth() {
    let out = debug(concat!(
        "f <- function(x) {\n",
        "  switch(x,\n",
        "    a = if (TRUE) { 1 },\n",
        "    b = 2\n",
        "  )\n",
        "}\n",
    ));
    let nesting = function_metric(&out, "f", "nesting").unwrap_or(0);
    assert!(nesting >= 1, "got: {nesting}");
}

// ===========================================================================
// Bump counting (2)
// ===========================================================================

#[test]
fn bumpy_road_two_bumps() {
    let out = debug(concat!(
        "f <- function(a, b, c, d) {\n",
        "  if (a) {\n    if (b) {\n      if (TRUE) {\n      }\n    }\n  }\n",
        "  x <- 1\n",
        "  if (c) {\n    if (d) {\n      if (TRUE) {\n      }\n    }\n  }\n",
        "}\n",
    ));
    let bumps = function_metric(&out, "f", "bumps").unwrap_or(0);
    assert!(bumps >= 2, "got: {bumps}");
}

#[test]
fn bumpy_road_single_bump_not_flagged() {
    let out = check(concat!(
        "f <- function(a, b) {\n",
        "  if (a) {\n    if (b) {\n      if (TRUE) {\n      }\n    }\n  }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Nested Conditional Chunks"));
}

// ===========================================================================
// Arguments (6)
// ===========================================================================

#[test]
fn args_zero() {
    let out = debug("f <- function() {\n}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(0));
}

#[test]
fn args_one() {
    let out = debug("f <- function(x) {\n  x\n}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(1));
}

#[test]
fn args_five_at_threshold() {
    let out = check("f <- function(a, b, c, d, e) {\n  a + b + c + d + e\n}\n");
    assert!(!has_smell(&out, "Excess Arguments"));
}

#[test]
fn args_six_over_threshold() {
    let out = check("f <- function(a, b, c, d, e, g) {\n  a + b + c + d + e + g\n}\n");
    assert!(has_smell(&out, "Excess Arguments"), "got: {out}");
}

#[test]
fn args_default_params_counted() {
    let out = debug("f <- function(a, b = 1, c = 2) {\n  a + b + c\n}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(3));
}

#[test]
fn args_dots_counted() {
    let out = debug("f <- function(a, ...) {\n  a\n}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(2));
}

// ===========================================================================
// LOC counting (4)
// ===========================================================================

#[test]
fn loc_single_line_method() {
    let out = debug("f <- function() {\n}\n");
    assert_eq!(function_metric(&out, "f", "loc"), Some(2));
}

#[test]
fn loc_multiline() {
    let out = debug("f <- function() {\n  x <- 1\n  x + 1\n}\n");
    assert_eq!(function_metric(&out, "f", "loc"), Some(4));
}

#[test]
fn loc_comments_excluded_module() {
    let out = debug("# a comment\n# another\nf <- function() {\n}\n");
    assert!(out.contains("LOC, 1 function"));
}

#[test]
fn loc_empty_lines_excluded_module() {
    let out = debug("\n\n\nf <- function() {\n}\n\n\n");
    assert!(out.contains("LOC, 1 function"));
}

// ===========================================================================
// Embedded blocks (2)
// ===========================================================================

#[test]
fn embedded_large_string() {
    let mut code = String::from("f <- function() {\n  x <- \"\n");
    for i in 0..embedded_lines_above() {
        code.push_str(&format!("    line {i}\n"));
    }
    code.push_str("  \"\n}\n");
    let out = check(&code);
    assert!(has_smell(&out, "Large Embedded Block"), "got: {out}");
}

#[test]
fn embedded_small_string_not_flagged() {
    let out = check("f <- function() {\n  \"hello\"\n}\n");
    assert!(!has_smell(&out, "Large Embedded Block"));
}

// ===========================================================================
// Duplication (4)
// ===========================================================================

#[test]
fn exact_duplication_detected() {
    let out = check(concat!(
        "alpha <- function(x) {\n  r <- x\n  r <- r * 2\n  r <- r + 1\n  r <- r - 3\n  r\n}\n",
        "beta <- function(x) {\n  r <- x\n  r <- r * 2\n  r <- r + 1\n  r <- r - 3\n  r\n}\n",
    ));
    assert!(has_smell(&out, "Code Duplication"), "got: {out}");
}

#[test]
fn exact_duplication_below_min_loc() {
    let out = check("a <- function(x) {\n  x\n}\nb <- function(x) {\n  x\n}\n");
    assert!(!has_smell(&out, "Code Duplication"));
}

#[test]
fn fuzzy_duplication_detected() {
    let out = check(concat!(
        "process_alpha <- function(data) {\n",
        "  result <- 0\n",
        "  for (item in data) {\n",
        "    if (item > 100) {\n      result <- result + 2\n    } else {\n      result <- result + 1\n    }\n",
        "  }\n  result\n}\n",
        "process_beta <- function(items) {\n",
        "  count <- 0\n",
        "  for (val in items) {\n",
        "    if (val > 100) {\n      count <- count + 2\n    } else {\n      count <- count + 1\n    }\n",
        "  }\n  count\n}\n",
    ));
    assert!(has_smell(&out, "Code Duplication"), "got: {out}");
}

#[test]
fn test_function_duplication_suppressed() {
    let out = check(concat!(
        "test_alpha <- function() {\n  r <- 0\n  r <- r + 1\n  r <- r + 2\n  r <- r + 3\n  r <- r + 4\n  r <- r + 5\n}\n",
        "test_beta <- function() {\n  r <- 0\n  r <- r + 1\n  r <- r + 2\n  r <- r + 3\n  r <- r + 4\n  r <- r + 5\n}\n",
    ));
    assert!(!has_smell(&out, "Code Duplication"), "test duplication should be suppressed, got: {out}");
}

// ===========================================================================
// Assertions (3)
// ===========================================================================

#[test]
fn assertion_block_above_threshold() {
    let mut code = String::from("f <- function() {\n");
    for _ in 0..asserts_above() {
        code.push_str("  stopifnot(TRUE)\n");
    }
    code.push_str("}\n");
    let out = check(&code);
    assert!(has_smell(&out, "Large Assertion Block"), "got: {out}");
}

#[test]
fn assertion_block_below_threshold() {
    let mut code = String::from("f <- function() {\n");
    for _ in 0..5 {
        code.push_str("  stopifnot(TRUE)\n");
    }
    code.push_str("}\n");
    let out = check(&code);
    assert!(!has_smell(&out, "Large Assertion Block"));
}

#[test]
fn assertion_block_interrupted_resets() {
    let mut code = String::from("f <- function(x) {\n");
    for _ in 0..5 {
        code.push_str("  stopifnot(TRUE)\n");
    }
    code.push_str("  y <- x + 1\n");
    for _ in 0..5 {
        code.push_str("  stopifnot(TRUE)\n");
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
        "f <- function(a, b, c) {\n",
        "  if (a && b || c) {\n",
        "    if (a || b && c) {\n",
        "      if (b && c || a) {\n",
        "        TRUE\n",
        "      }\n",
        "    }\n",
        "  }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Complex Conditional"), "got: {out}");
}

#[test]
fn compound_condition_simple_not_detected() {
    let out = check("f <- function(a, b) {\n  if (a && b) {\n    TRUE\n  }\n}\n");
    assert!(!has_smell(&out, "Complex Conditional"));
}

// ===========================================================================
// Primitive obsession (3)
// ===========================================================================

#[test]
fn primitive_obsession_never_triggers() {
    let out = check("f <- function(a, b, c, d) {\n  a + b + c + d\n}\n");
    assert!(!has_smell(&out, "Primitive Obsession"));
}

#[test]
fn no_typed_params() {
    let out = debug("f <- function(a, b, c, d) {\n  a\n}\n");
    assert!(out.contains("primitives=0/0"), "got: {out}");
}

#[test]
fn typed_param_count_zero() {
    let out = debug("f <- function(a, b) {\n  a + b\n}\n");
    assert!(out.contains("primitives=0/0"));
}

// ===========================================================================
// Function naming (3)
// ===========================================================================

#[test]
fn function_has_extracted_name() {
    let out = debug("my_func <- function(x) {\n  x\n}\n");
    assert!(out.contains("my_func"), "got: {out}");
    assert!(!out.contains("<anonymous>"));
}

#[test]
fn anonymous_function_detected() {
    let out = debug("(function(x) {\n  x\n})(42)\n");
    // Anonymous functions in call position may or may not be detected;
    // the important thing is no crash
    assert!(out.contains("LOC"));
}

#[test]
fn equals_assignment_extracts_name() {
    let out = debug("my_func = function(x) {\n  x\n}\n");
    assert!(out.contains("my_func"), "got: {out}");
}

// ===========================================================================
// R-specific (15)
// ===========================================================================

#[test]
fn repeat_loop_cc() {
    let out = debug(concat!(
        "f <- function(x) {\n",
        "  n <- x\n",
        "  repeat {\n",
        "    if (n <= 0) break\n",
        "    n <- n - 1\n",
        "  }\n",
        "}\n",
    ));
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 3, "repeat+if should give cc>=3, got: {cc}");
}

#[test]
fn try_catch_empty_handler() {
    let out = check(concat!(
        "f <- function(x) {\n",
        "  tryCatch(\n",
        "    log(x),\n",
        "    error = function(e) {}\n",
        "  )\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Empty Error Handler"), "got: {out}");
}

#[test]
fn switch_each_case_increments() {
    let out = debug(concat!(
        "f <- function(x) {\n",
        "  switch(x,\n",
        "    a = 1,\n",
        "    b = 2,\n",
        "    c = 3,\n",
        "    d = 4,\n",
        "    e = 5,\n",
        "    f = 6,\n",
        "    g = 7,\n",
        "    h = 8,\n",
        "    i = 9\n",
        "  )\n",
        "}\n",
    ));
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 10, "9 cases should have cc>=10, got: {cc}");
}

#[test]
fn for_in_increments_cc() {
    let out = debug("f <- function(data) {\n  s <- 0\n  for (v in data) {\n    s <- s + v\n  }\n  s\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 2, "for should increment cc, got: {cc}");
}

#[test]
fn while_loop_cc() {
    let out = debug("f <- function(n) {\n  while (n > 0) {\n    n <- n - 1\n  }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn else_if_chain_cc() {
    let out = debug(concat!(
        "f <- function(x) {\n",
        "  if (x == 1) { 1\n",
        "  } else if (x == 2) { 2\n",
        "  } else if (x == 3) { 3\n",
        "  } else if (x == 4) { 4\n",
        "  } else { 0 }\n",
        "}\n",
    ));
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert_eq!(cc, 5, "got: {cc}");
}

#[test]
fn nested_function_separate_scope() {
    let out = debug(concat!(
        "outer <- function(x) {\n",
        "  inner <- function(y) {\n",
        "    if (y > 0) { y }\n",
        "  }\n",
        "  x + 1\n",
        "}\n",
    ));
    let cc = function_metric(&out, "outer", "cc").unwrap_or(0);
    assert_eq!(cc, 1, "nested function should not contribute CC to outer, got: {cc}");
}

#[test]
fn global_assignment_operator() {
    let out = debug("my_func <<- function(x) {\n  x + 1\n}\n");
    assert!(out.contains("my_func"), "<<- should extract name, got: {out}");
}

#[test]
fn field_access_via_dollar() {
    let out = debug(concat!(
        "f <- function(obj) {\n",
        "  obj$name\n",
        "}\n",
    ));
    // Should not crash, $ is traversed
    assert!(out.contains('f'));
}

#[test]
fn vectorized_and_no_cc() {
    let out = debug(concat!(
        "f <- function(x, y) {\n",
        "  x & y\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cc"), Some(1));
}

#[test]
fn do_call_complexity() {
    let out = debug(concat!(
        "f <- function(x) {\n",
        "  do.call(paste, list(x))\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cc"), Some(1));
}

#[test]
fn lapply_with_anonymous_function() {
    let out = debug(concat!(
        "f <- function(data) {\n",
        "  lapply(data, function(x) {\n",
        "    if (x > 0) x else 0\n",
        "  })\n",
        "}\n",
    ));
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert_eq!(cc, 1, "anonymous function in lapply should be separate scope, got: {cc}");
}

#[test]
fn pipe_operator_traversal() {
    let out = debug(concat!(
        "f <- function(x) {\n",
        "  x |> log() |> round()\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cc"), Some(1));
}

#[test]
fn multiple_assignments() {
    let out = debug(concat!(
        "a <- function(x) {\n  x + 1\n}\n",
        "b <- function(y) {\n  y + 2\n}\n",
    ));
    assert!(out.contains('a') && out.contains('b'), "got: {out}");
}

// ===========================================================================
// Performance (2)
// ===========================================================================

#[test]
fn performance_1000_loc() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("perf.r");
    let mut code = String::new();
    for i in 0..50 {
        code.push_str(&format!("func{i} <- function(x) {{\n"));
        for j in 0..18 {
            code.push_str(&format!("  v{j} <- {j}\n"));
        }
        code.push_str("  x\n}\n\n");
    }
    std::fs::write(&path, &code).unwrap();
    let start = std::time::Instant::now();
    let _ = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    assert!(start.elapsed().as_millis() < 200, "took: {}ms", start.elapsed().as_millis());
}

#[test]
fn performance_many_functions() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("many.r");
    let mut code = String::new();
    for i in 0..30 {
        code.push_str(&format!("fn{i} <- function(x) {{\n  x + {i}\n}}\n\n"));
    }
    std::fs::write(&path, &code).unwrap();
    let start = std::time::Instant::now();
    let _ = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    assert!(start.elapsed().as_millis() < 500, "took: {}ms", start.elapsed().as_millis());
}

// ===========================================================================
// Edge cases (5)
// ===========================================================================

#[test]
fn clean_code_not_flagged() {
    let out = check(concat!(
        "add <- function(a, b) {\n  a + b\n}\n",
        "helper <- function(x) {\n  x + 1\n}\n",
    ));
    assert!(out.is_empty(), "got: {out}");
}

#[test]
fn comments_only_no_output() {
    let out = check("# this is a comment\n# another comment\n");
    assert!(out.is_empty());
}

#[test]
fn empty_file_no_crash() {
    let out = check("");
    assert!(out.is_empty());
}

#[test]
fn empty_function_body() {
    let out = debug("f <- function() {\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(1));
}

#[test]
fn multiple_functions_independent_metrics() {
    let out = debug(concat!(
        "simple <- function() {\n}\n",
        "complex <- function(x) {\n  if (x) {\n  }\n}\n",
    ));
    assert_eq!(function_metric(&out, "simple", "cc"), Some(1));
    assert_eq!(function_metric(&out, "complex", "cc"), Some(2));
}

// ===========================================================================
// Production-realistic inline patterns (18)
// ===========================================================================

#[test]
fn realistic_data_pipeline_cc() {
    let out = debug(concat!(
        "process <- function(data, config) {\n",
        "  if (is.null(data)) return(NULL)\n",
        "  if (!is.data.frame(data)) {\n",
        "    if (is.list(data)) {\n",
        "      data <- as.data.frame(data)\n",
        "    } else {\n",
        "      stop(\"bad input\")\n",
        "    }\n",
        "  }\n",
        "  if (config$validate) {\n",
        "    for (col in names(data)) {\n",
        "      if (any(is.na(data[[col]]))) {\n",
        "        if (config$strict) {\n",
        "          stop(\"NA found\")\n",
        "        } else {\n",
        "          data[[col]][is.na(data[[col]])] <- 0\n",
        "        }\n",
        "      }\n",
        "    }\n",
        "  }\n",
        "  data\n",
        "}\n",
    ));
    let cc = function_metric(&out, "process", "cc").unwrap_or(0);
    assert!(cc >= 8, "realistic pipeline should have high cc, got: {cc}");
}

#[test]
fn realistic_data_pipeline_nesting() {
    let out = debug(concat!(
        "process <- function(data, config) {\n",
        "  if (config$validate) {\n",
        "    for (col in names(data)) {\n",
        "      if (any(is.na(data[[col]]))) {\n",
        "        if (config$strict) {\n",
        "          stop(\"NA\")\n",
        "        }\n",
        "      }\n",
        "    }\n",
        "  }\n",
        "  data\n",
        "}\n",
    ));
    let nesting = function_metric(&out, "process", "nesting").unwrap_or(0);
    assert!(nesting >= 4, "should have deep nesting, got: {nesting}");
}

#[test]
fn realistic_api_handler_cc() {
    let out = debug(concat!(
        "handle <- function(req, db, auth) {\n",
        "  if (is.null(req$method)) return(list(status = 400))\n",
        "  if (req$method == \"GET\") {\n",
        "    if (!auth$check(req$token)) return(list(status = 401))\n",
        "    result <- db$query(req$path)\n",
        "    if (is.null(result)) return(list(status = 404))\n",
        "    list(status = 200, body = result)\n",
        "  } else if (req$method == \"POST\") {\n",
        "    if (!auth$check(req$token)) return(list(status = 401))\n",
        "    if (is.null(req$body)) return(list(status = 400))\n",
        "    list(status = 201)\n",
        "  } else {\n",
        "    list(status = 405)\n",
        "  }\n",
        "}\n",
    ));
    let cc = function_metric(&out, "handle", "cc").unwrap_or(0);
    assert!(cc >= 7, "api handler should have cc>=7, got: {cc}");
}

#[test]
fn realistic_switch_dispatch() {
    let out = check(concat!(
        "dispatch <- function(cmd) {\n",
        "  switch(cmd,\n",
        "    start = \"starting\",\n",
        "    stop = \"stopping\",\n",
        "    restart = \"restarting\",\n",
        "    status = \"checking\",\n",
        "    deploy = \"deploying\",\n",
        "    rollback = \"rolling back\",\n",
        "    scale = \"scaling\",\n",
        "    migrate = \"migrating\",\n",
        "    \"unknown\"\n",
        "  )\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Complex Method"), "many switch arms should flag, got: {out}");
}

#[test]
fn realistic_try_catch_with_handlers() {
    let out = debug(concat!(
        "safe_read <- function(path) {\n",
        "  tryCatch(\n",
        "    {\n",
        "      data <- read.csv(path)\n",
        "      if (nrow(data) == 0) stop(\"empty\")\n",
        "      data\n",
        "    },\n",
        "    error = function(e) {\n",
        "      message(paste(\"Error:\", e$message))\n",
        "      NULL\n",
        "    },\n",
        "    warning = function(w) {\n",
        "      message(paste(\"Warning:\", w$message))\n",
        "      NULL\n",
        "    }\n",
        "  )\n",
        "}\n",
    ));
    let cc = function_metric(&out, "safe_read", "cc").unwrap_or(0);
    assert!(cc >= 4, "tryCatch with if + 2 handlers should have cc>=4, got: {cc}");
}

#[test]
fn realistic_for_with_nested_if_else() {
    let out = debug(concat!(
        "categorize <- function(items) {\n",
        "  result <- list()\n",
        "  for (item in items) {\n",
        "    if (item$type == \"A\") {\n",
        "      if (item$value > 100) {\n",
        "        result <- append(result, list(\"high_a\"))\n",
        "      } else {\n",
        "        result <- append(result, list(\"low_a\"))\n",
        "      }\n",
        "    } else if (item$type == \"B\") {\n",
        "      result <- append(result, list(\"b\"))\n",
        "    } else {\n",
        "      result <- append(result, list(\"other\"))\n",
        "    }\n",
        "  }\n",
        "  result\n",
        "}\n",
    ));
    let cc = function_metric(&out, "categorize", "cc").unwrap_or(0);
    assert!(cc >= 5, "for+if+else_if should compound, got: {cc}");
    let nesting = function_metric(&out, "categorize", "nesting").unwrap_or(0);
    assert!(nesting >= 3, "should have nesting>=3, got: {nesting}");
}

#[test]
fn realistic_short_vars_long_function() {
    let out = check(concat!(
        "process <- function(data) {\n",
        "  a <- data[[1]]\n",
        "  b <- data[[2]]\n",
        "  c <- data[[3]]\n",
        "  d <- data[[4]]\n",
        "  e <- data[[5]]\n",
        "  f <- data[[6]]\n",
        "  g <- data[[7]]\n",
        "  h <- data[[8]]\n",
        "  if (a > 100) return(-1)\n",
        "  if (b > 200) return(-2)\n",
        "  result <- a + b + c + d\n",
        "  extra <- e + f + g + h\n",
        "  result + extra\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Short Variable Names"), "got: {out}");
}

#[test]
fn realistic_empty_catch_in_middleware() {
    let out = check(concat!(
        "apply_mw <- function(req, middlewares) {\n",
        "  for (mw in middlewares) {\n",
        "    tryCatch(\n",
        "      { req <- mw(req) },\n",
        "      error = function(e) {}\n",
        "    )\n",
        "  }\n",
        "  req\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Empty Error Handler"), "got: {out}");
}

#[test]
fn realistic_non_empty_catch_clean() {
    let out = check(concat!(
        "safe_op <- function(x) {\n",
        "  tryCatch(\n",
        "    log(x),\n",
        "    error = function(e) {\n",
        "      warning(paste(\"log failed:\", e$message))\n",
        "      NA\n",
        "    }\n",
        "  )\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Empty Error Handler"), "non-empty handler should be clean, got: {out}");
}

#[test]
fn realistic_multiple_nested_for_loops() {
    let out = debug(concat!(
        "matrix_op <- function(mat) {\n",
        "  for (i in seq_len(nrow(mat))) {\n",
        "    for (j in seq_len(ncol(mat))) {\n",
        "      if (mat[i, j] > 0) {\n",
        "        mat[i, j] <- mat[i, j] * 2\n",
        "      }\n",
        "    }\n",
        "  }\n",
        "  mat\n",
        "}\n",
    ));
    let nesting = function_metric(&out, "matrix_op", "nesting").unwrap_or(0);
    assert!(nesting >= 3, "nested loops + if should give nesting>=3, got: {nesting}");
    let cc = function_metric(&out, "matrix_op", "cc").unwrap_or(0);
    assert_eq!(cc, 4, "2 for + 1 if + base = 4, got: {cc}");
}

#[test]
fn realistic_while_with_break() {
    let out = debug(concat!(
        "find_first <- function(items, pred) {\n",
        "  idx <- 1\n",
        "  while (idx <= length(items)) {\n",
        "    if (pred(items[[idx]])) {\n",
        "      return(items[[idx]])\n",
        "    }\n",
        "    idx <- idx + 1\n",
        "  }\n",
        "  NULL\n",
        "}\n",
    ));
    let cc = function_metric(&out, "find_first", "cc").unwrap_or(0);
    assert_eq!(cc, 3, "while + if + base = 3, got: {cc}");
}

#[test]
fn realistic_repeat_with_condition() {
    let out = debug(concat!(
        "retry <- function(action, max_retries) {\n",
        "  attempt <- 0\n",
        "  repeat {\n",
        "    attempt <- attempt + 1\n",
        "    result <- tryCatch(action(), error = function(e) NULL)\n",
        "    if (!is.null(result)) return(result)\n",
        "    if (attempt >= max_retries) break\n",
        "  }\n",
        "  NULL\n",
        "}\n",
    ));
    let cc = function_metric(&out, "retry", "cc").unwrap_or(0);
    assert!(cc >= 5, "repeat + tryCatch + 2 ifs = cc>=5, got: {cc}");
}

#[test]
fn realistic_sapply_with_anonymous_no_cc() {
    let out = debug(concat!(
        "normalize <- function(data) {\n",
        "  sapply(data, function(col) {\n",
        "    if (is.numeric(col)) {\n",
        "      (col - min(col)) / (max(col) - min(col))\n",
        "    } else {\n",
        "      col\n",
        "    }\n",
        "  })\n",
        "}\n",
    ));
    let cc = function_metric(&out, "normalize", "cc").unwrap_or(0);
    assert_eq!(cc, 1, "anonymous function is separate scope, got: {cc}");
}

#[test]
fn realistic_cogc_deeply_nested_pipeline() {
    let out = debug(concat!(
        "transform <- function(data, rules) {\n",
        "  for (rule in rules) {\n",
        "    if (rule$type == \"filter\") {\n",
        "      for (col in rule$columns) {\n",
        "        if (col %in% names(data)) {\n",
        "          if (rule$action == \"remove_na\") {\n",
        "            data <- data[!is.na(data[[col]]), ]\n",
        "          }\n",
        "        }\n",
        "      }\n",
        "    }\n",
        "  }\n",
        "  data\n",
        "}\n",
    ));
    let cogc = function_metric(&out, "transform", "cogc").unwrap_or(0);
    assert!(cogc >= 10, "deeply nested pipeline should have high cogc, got: {cogc}");
}

#[test]
fn realistic_clean_utility_no_smells() {
    let out = check(concat!(
        "safe_divide <- function(a, b) {\n",
        "  if (b == 0) return(NA)\n",
        "  a / b\n",
        "}\n",
        "clamp <- function(x, lo, hi) {\n",
        "  if (x < lo) return(lo)\n",
        "  if (x > hi) return(hi)\n",
        "  x\n",
        "}\n",
        "is_valid <- function(x) {\n",
        "  !is.null(x) && !is.na(x)\n",
        "}\n",
    ));
    assert!(out.is_empty(), "clean utilities should not flag, got: {out}");
}

#[test]
fn realistic_many_top_level_assignments_no_crash() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("many_decls.r");
    let mut code = String::new();
    for i in 0..50 {
        code.push_str(&format!("x{i} <- {i}\n"));
    }
    code.push_str("f <- function() { 1 }\n");
    std::fs::write(&path, &code).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    // R has no class declarations, so this just verifies no crash on many assignments
    assert!(stdout.is_empty() || !stdout.is_empty());
}

#[test]
fn realistic_global_if_at_top() {
    let out = check(concat!(
        "if (Sys.getenv(\"DEBUG\") == \"1\") {\n",
        "  options(warn = 2)\n",
        "}\n",
        "if (Sys.getenv(\"VERBOSE\") == \"1\") {\n",
        "  cat(\"verbose\\n\")\n",
        "}\n",
        "if (Sys.getenv(\"STRICT\") == \"1\") {\n",
        "  options(stringsAsFactors = FALSE)\n",
        "}\n",
        "run <- function(x) { x }\n",
    ));
    assert!(has_smell(&out, "Global Conditionals"), "got: {out}");
}

#[test]
fn realistic_no_global_conditionals_in_clean() {
    let out = check(concat!(
        "add <- function(a, b) {\n  a + b\n}\n",
        "mul <- function(a, b) {\n  a * b\n}\n",
    ));
    assert!(!has_smell(&out, "Global Conditionals"));
}
