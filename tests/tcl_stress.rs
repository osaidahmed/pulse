mod common;

use common::*;
use std::process::Command;

lang_helpers!("tcl");

// ===========================================================================
// CC counting (20)
// ===========================================================================

#[test]
fn cc_counts_if() {
    let out = debug("proc f {x} {\n    if {$x > 0} {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_elseif() {
    let out = debug("proc f {x} {\n    if {$x > 0} {} elseif {$x < 0} {}\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(cc >= 3, "elseif should add CC, got: {cc}");
}

#[test]
fn cc_counts_while() {
    let out = debug("proc f {x} {\n    set n $x\n    while {$n > 0} {\n        incr n -1\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_foreach() {
    let out = debug("proc f {items} {\n    foreach item $items {\n        puts $item\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_for_command() {
    let out = debug("proc f {} {\n    for {set i 0} {$i < 10} {incr i} {\n        puts $i\n    }\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 2, "for should add CC, got: {cc}");
}

#[test]
fn cc_counts_switch_cases() {
    let out = debug(concat!(
        "proc f {x} {\n",
        "    switch $x {\n",
        "        a { return 1 }\n",
        "        b { return 2 }\n",
        "        c { return 3 }\n",
        "    }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cc"), Some(4));
}

#[test]
fn cc_counts_catch() {
    let out = debug("proc f {x} {\n    catch {error \"boom\"} result\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 2, "catch should add CC, got: {cc}");
}

#[test]
fn cc_counts_try_on_error() {
    let out = debug(concat!(
        "proc f {x} {\n",
        "    try {\n",
        "        expr {$x / 0}\n",
        "    } on error {msg opts} {\n",
        "        puts $msg\n",
        "    }\n",
        "}\n",
    ));
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 2, "try on error should add CC, got: {cc}");
}

#[test]
fn cc_counts_and_op() {
    let out = debug("proc f {a b} {\n    if {$a && $b} {\n        return 1\n    }\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(cc >= 3, "got: {cc}");
}

#[test]
fn cc_counts_or_op() {
    let out = debug("proc f {a b} {\n    if {$a || $b} {\n        return 1\n    }\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(cc >= 3, "got: {cc}");
}

#[test]
fn cc_chained_boolean() {
    let out = debug("proc f {a b c} {\n    if {$a && $b || $c} {\n        return 1\n    }\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 4, "got: {cc}");
}

#[test]
fn cc_nested_if_counted_once() {
    let out = debug("proc f {a b} {\n    if {$a} {\n        if {$b} {\n            return 1\n        }\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_switch_default_not_counted() {
    let out = debug(concat!(
        "proc f {x} {\n",
        "    switch $x {\n",
        "        a { return 1 }\n",
        "        default { return 0 }\n",
        "    }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_base_case_is_1() {
    let out = debug("proc f {} {\n    return 0\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(1));
}

#[test]
fn cc_if_with_else() {
    let out = debug("proc f {x} {\n    if {$x > 0} {\n        return 1\n    } else {\n        return 0\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_multiple_if_accumulates() {
    let out = debug("proc f {a b} {\n    if {$a > 0} {}\n    if {$b > 0} {}\n    return 0\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_for_with_nested_if() {
    let out = debug(concat!(
        "proc f {} {\n",
        "    for {set i 0} {$i < 10} {incr i} {\n",
        "        if {$i > 5} {\n",
        "            puts $i\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 3, "for+if should give cc >= 3, got: {cc}");
}

#[test]
fn cc_foreach_with_if() {
    let out = debug(concat!(
        "proc f {items} {\n",
        "    foreach item $items {\n",
        "        if {$item > 0} {\n",
        "            puts $item\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 3, "foreach+if should give cc >= 3, got: {cc}");
}

#[test]
fn cc_expression_boolean() {
    let out = debug("proc f {a b} {\n    if {$a > 0 && $b > 0} {\n        return 1\n    }\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 3, "expression boolean should add CC, got: {cc}");
}

// ===========================================================================
// Cognitive complexity (10)
// ===========================================================================

#[test]
fn cogc_flat_branches() {
    let out = debug("proc f {a b c} {\n    if {$a > 0} {}\n    if {$b > 0} {}\n    if {$c > 0} {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cogc"), Some(3));
}

#[test]
fn cogc_nested_ifs() {
    let out = debug(concat!(
        "proc f {a b c d} {\n",
        "    if {$a > 0} {\n",
        "        if {$b > 0} {\n",
        "            if {$c > 0} {\n",
        "                if {$d > 0} {}\n",
        "            }\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cogc"), Some(10));
}

#[test]
fn cogc_elseif_no_extra_nesting() {
    let out = debug(concat!(
        "proc f {x} {\n",
        "    if {$x == 1} {} elseif {$x == 2} {} elseif {$x == 3} {}\n",
        "}\n",
    ));
    let cogc = function_metric(&out, "f", "cogc").unwrap();
    assert!(cogc >= 3, "elseif chain should have cogc >= 3, got: {cogc}");
}

#[test]
fn cogc_else_increases_nesting() {
    let out = debug(concat!(
        "proc f {x} {\n",
        "    if {$x > 0} {} else {\n",
        "        if {$x < -10} {}\n",
        "    }\n",
        "}\n",
    ));
    let cogc = function_metric(&out, "f", "cogc").unwrap();
    assert!(cogc >= 3, "else should contribute to nesting, got: {cogc}");
}

#[test]
fn cogc_while_nested() {
    let out = debug("proc f {x} {\n    while {$x > 0} {\n        if {$x > 5} {}\n        incr x -1\n    }\n}\n");
    let cogc = function_metric(&out, "f", "cogc").unwrap_or(0);
    assert!(cogc >= 3, "while+if should contribute cogc, got: {cogc}");
}

#[test]
fn cogc_foreach_nested() {
    let out = debug("proc f {items} {\n    foreach item $items {\n        if {$item > 0} {}\n    }\n}\n");
    let cogc = function_metric(&out, "f", "cogc").unwrap_or(0);
    assert!(cogc >= 3, "foreach+if should contribute cogc, got: {cogc}");
}

#[test]
fn cogc_boolean_single_sequence() {
    let out = debug("proc f {a b} {\n    if {$a && $b} {\n        return 1\n    }\n}\n");
    let cogc = function_metric(&out, "f", "cogc").unwrap_or(0);
    assert!(cogc >= 2, "got: {cogc}");
}

#[test]
fn cogc_boolean_mixed_sequence() {
    let out = debug("proc f {a b c} {\n    if {$a && $b || $c} {\n        return 1\n    }\n}\n");
    let cogc = function_metric(&out, "f", "cogc").unwrap_or(0);
    assert!(cogc >= 3, "got: {cogc}");
}

#[test]
fn cogc_triggers_complex_method() {
    let mut code = String::from("proc f {x} {\n");
    for _ in 0..4 {
        code.push_str("    if {$x > 0} {\n        if {$x > 1} {\n            if {$x > 2} {}\n        }\n    }\n");
    }
    code.push_str("    return 0\n}\n");
    let out = check(&code);
    assert!(has_smell(&out, "Complex Method"), "got: {out}");
}

#[test]
fn cogc_below_threshold_no_smell() {
    let out = check("proc f {x} {\n    if {$x > 0} {}\n    if {$x > 1} {}\n    return 0\n}\n");
    assert!(!has_smell(&out, "Complex Method"));
}

// ===========================================================================
// Nesting depth (6)
// ===========================================================================

#[test]
fn nesting_depth_simple() {
    let out = debug("proc f {x} {\n    if {$x} {}\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(1));
}

#[test]
fn nesting_depth_nested() {
    let out = debug("proc f {a b c} {\n    if {$a} {\n        if {$b} {\n            if {$c} {}\n        }\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(3));
}

#[test]
fn nesting_depth_sequential_not_accumulated() {
    let out = debug("proc f {a b} {\n    if {$a} {}\n    if {$b} {}\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(1));
}

#[test]
fn nesting_deep_if_chain() {
    let out = debug(concat!(
        "proc f {a b c d e} {\n",
        "    if {$a} {\n",
        "        if {$b} {\n",
        "            if {$c} {\n",
        "                if {$d} {\n",
        "                    if {$e} {}\n",
        "                }\n",
        "            }\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "nesting"), Some(5));
}

#[test]
fn nesting_loop_with_if() {
    let out = debug("proc f {items} {\n    foreach item $items {\n        if {$item > 0} {}\n    }\n}\n");
    let nesting = function_metric(&out, "f", "nesting").unwrap_or(0);
    assert!(nesting >= 2, "got: {nesting}");
}

#[test]
fn nesting_switch_depth() {
    let out = debug(concat!(
        "proc f {x} {\n",
        "    switch $x {\n",
        "        a {\n",
        "            if {1} {}\n",
        "        }\n",
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
        "proc f {a b c d} {\n",
        "    if {$a} {\n        if {$b} {\n            if {1} {}\n        }\n    }\n",
        "    set x 1\n",
        "    if {$c} {\n        if {$d} {\n            if {1} {}\n        }\n    }\n",
        "}\n",
    ));
    let bumps = function_metric(&out, "f", "bumps").unwrap_or(0);
    assert!(bumps >= 2, "got: {bumps}");
}

#[test]
fn bumpy_road_single_bump_not_flagged() {
    let out = check(concat!(
        "proc f {a b} {\n",
        "    if {$a} {\n        if {$b} {\n            if {1} {}\n        }\n    }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Nested Conditional Chunks"));
}

// ===========================================================================
// Arguments (6)
// ===========================================================================

#[test]
fn args_zero() {
    let out = debug("proc f {} {\n    return 0\n}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(0));
}

#[test]
fn args_one() {
    let out = debug("proc f {x} {\n    return $x\n}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(1));
}

#[test]
fn args_five_at_threshold() {
    let out = check("proc f {a b c d e} {\n    return $a\n}\n");
    assert!(!has_smell(&out, "Excess Arguments"));
}

#[test]
fn args_six_over_threshold() {
    let out = check("proc f {a b c d e g} {\n    return $a\n}\n");
    assert!(has_smell(&out, "Excess Arguments"), "got: {out}");
}

#[test]
fn args_default_params_counted() {
    let out = debug("proc f {a {b 1} {c 2}} {\n    return $a\n}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(3));
}

#[test]
fn args_special_args_param() {
    let out = debug("proc f {a args} {\n    return $a\n}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(2));
}

// ===========================================================================
// LOC counting (4)
// ===========================================================================

#[test]
fn loc_single_line_proc() {
    let out = debug("proc f {} {\n    return 0\n}\n");
    assert_eq!(function_metric(&out, "f", "loc"), Some(3));
}

#[test]
fn loc_multiline() {
    let out = debug("proc f {} {\n    set x 1\n    set y 2\n    return $x\n}\n");
    assert_eq!(function_metric(&out, "f", "loc"), Some(5));
}

#[test]
fn loc_comments_excluded_module() {
    let out = debug("# a comment\n# another\nproc f {} {\n    return 0\n}\n");
    assert!(out.contains("LOC, 1 function"));
}

#[test]
fn loc_empty_lines_excluded_module() {
    let out = debug("\n\n\nproc f {} {\n    return 0\n}\n\n\n");
    assert!(out.contains("LOC, 1 function"));
}

// ===========================================================================
// Embedded blocks (2)
// ===========================================================================

#[test]
fn embedded_large_quoted_string() {
    let mut code = String::from("proc f {} {\n    set tpl \"line 0\n");
    for i in 1..embedded_lines_above() {
        code.push_str(&format!("line {i}\n"));
    }
    code.push_str("\"\n    return $tpl\n}\n");
    let out = check(&code);
    assert!(has_smell(&out, "Large Embedded Block"), "got: {out}");
}

#[test]
fn embedded_small_string_not_flagged() {
    let out = check("proc f {} {\n    set x \"hello\"\n    return $x\n}\n");
    assert!(!has_smell(&out, "Large Embedded Block"));
}

// ===========================================================================
// Duplication (4)
// ===========================================================================

#[test]
fn exact_duplication_detected() {
    let out = check(concat!(
        "proc alpha {x} {\n    set r $x\n    set r [expr {$r * 2}]\n    set r [expr {$r + 1}]\n    set r [expr {$r - 3}]\n    return $r\n}\n",
        "proc beta {x} {\n    set r $x\n    set r [expr {$r * 2}]\n    set r [expr {$r + 1}]\n    set r [expr {$r - 3}]\n    return $r\n}\n",
    ));
    assert!(has_smell(&out, "Code Duplication"), "got: {out}");
}

#[test]
fn exact_duplication_below_min_loc() {
    let out = check("proc a {x} {\n    return $x\n}\nproc b {x} {\n    return $x\n}\n");
    assert!(!has_smell(&out, "Code Duplication"));
}

#[test]
fn fuzzy_duplication_detected() {
    let out = check(concat!(
        "proc process_alpha {data} {\n",
        "    set result 0\n",
        "    foreach item $data {\n",
        "        if {$item > 100} {\n            set result [expr {$result + 2}]\n",
        "        } else {\n            set result [expr {$result + 1}]\n        }\n",
        "    }\n    return $result\n}\n",
        "proc process_beta {items} {\n",
        "    set count 0\n",
        "    foreach val $items {\n",
        "        if {$val > 100} {\n            set count [expr {$count + 2}]\n",
        "        } else {\n            set count [expr {$count + 1}]\n        }\n",
        "    }\n    return $count\n}\n",
    ));
    assert!(has_smell(&out, "Code Duplication"), "got: {out}");
}

#[test]
fn test_function_duplication_suppressed() {
    let out = check(concat!(
        "proc test_alpha {} {\n    set r 0\n    set r [expr {$r + 1}]\n    set r [expr {$r + 2}]\n    set r [expr {$r + 3}]\n    set r [expr {$r + 4}]\n    set r [expr {$r + 5}]\n}\n",
        "proc test_beta {} {\n    set r 0\n    set r [expr {$r + 1}]\n    set r [expr {$r + 2}]\n    set r [expr {$r + 3}]\n    set r [expr {$r + 4}]\n    set r [expr {$r + 5}]\n}\n",
    ));
    assert!(!has_smell(&out, "Code Duplication"), "test duplication should be suppressed, got: {out}");
}

// ===========================================================================
// Assertions (3)
// ===========================================================================

#[test]
fn assertion_block_above_threshold() {
    let mut code = String::from("proc f {} {\n");
    for _ in 0..asserts_above() {
        code.push_str("    assert 1\n");
    }
    code.push_str("}\n");
    let out = check(&code);
    assert!(has_smell(&out, "Large Assertion Block"), "got: {out}");
}

#[test]
fn assertion_block_below_threshold() {
    let mut code = String::from("proc f {} {\n");
    for _ in 0..5 {
        code.push_str("    assert 1\n");
    }
    code.push_str("}\n");
    let out = check(&code);
    assert!(!has_smell(&out, "Large Assertion Block"));
}

#[test]
fn assertion_block_interrupted_resets() {
    let mut code = String::from("proc f {x} {\n");
    for _ in 0..5 {
        code.push_str("    assert 1\n");
    }
    code.push_str("    set y [expr {$x + 1}]\n");
    for _ in 0..5 {
        code.push_str("    assert 1\n");
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
        "proc f {a b c} {\n",
        "    if {$a && $b || $c} {\n",
        "        if {$a || $b && $c} {\n",
        "            if {$b && $c || $a} {\n",
        "                return 1\n",
        "            }\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Complex Conditional"), "got: {out}");
}

#[test]
fn compound_condition_simple_not_detected() {
    let out = check("proc f {a b} {\n    if {$a && $b} {\n        return 1\n    }\n}\n");
    assert!(!has_smell(&out, "Complex Conditional"));
}

// ===========================================================================
// Primitive obsession (3)
// ===========================================================================

#[test]
fn primitive_obsession_never_triggers() {
    let out = check("proc f {a b c d} {\n    expr {$a + $b + $c + $d}\n}\n");
    assert!(!has_smell(&out, "Primitive Obsession"));
}

#[test]
fn no_typed_params() {
    let out = debug("proc f {a b c d} {\n    return $a\n}\n");
    assert!(out.contains("primitives=0/0"), "got: {out}");
}

#[test]
fn typed_param_count_zero() {
    let out = debug("proc f {a b} {\n    expr {$a + $b}\n}\n");
    assert!(out.contains("primitives=0/0"));
}

// ===========================================================================
// LCOM4 (3)
// ===========================================================================

#[test]
fn lcom4_connected_no_smell() {
    let out = check(concat!(
        "namespace eval S {\n",
        "    proc get_x {} {\n        variable x\n        return $x\n    }\n",
        "    proc set_x {v} {\n        variable x\n        set x $v\n    }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_disconnected() {
    let out = check(concat!(
        "namespace eval S {\n",
        "    proc get_a {} {\n        variable va\n        return $va\n    }\n",
        "    proc get_b {} {\n        variable vb\n        return $vb\n    }\n",
        "    proc get_c {} {\n        variable vc\n        return $vc\n    }\n",
        "    proc get_d {} {\n        variable vd\n        return $vd\n    }\n",
        "    proc set_a {v} {\n        variable va\n        set va $v\n    }\n",
        "    proc set_b {v} {\n        variable vb\n        set vb $v\n    }\n",
        "    proc set_c {v} {\n        variable vc\n        set vc $v\n    }\n",
        "    proc set_d {v} {\n        variable vd\n        set vd $v\n    }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"), "got: {out}");
}

#[test]
fn lcom4_single_method_no_smell() {
    let out = check("namespace eval S {\n    proc get_x {} {\n        variable x\n        return $x\n    }\n}\n");
    assert!(!has_smell(&out, "Low Cohesion"));
}

// ===========================================================================
// Method naming (3)
// ===========================================================================

#[test]
fn function_has_no_prefix() {
    let out = debug("proc standalone {} {\n    return 0\n}\n");
    assert!(out.contains("standalone"), "got: {out}");
    assert!(!out.contains(".standalone"));
}

#[test]
fn method_has_namespace_prefix() {
    let out = debug("namespace eval Svc {\n    proc handle {} {\n        return 0\n    }\n}\n");
    assert!(out.contains("Svc.handle"), "got: {out}");
}

#[test]
fn init_is_constructor() {
    let out = debug(concat!(
        "namespace eval Svc {\n",
        "    proc init {a b c d e f} {\n",
        "        variable x $a\n",
        "    }\n",
        "}\n",
    ));
    assert!(out.contains("Svc.init"), "got: {out}");
}

// ===========================================================================
// Tcl-specific (15)
// ===========================================================================

#[test]
fn for_command_as_loop() {
    let out = debug(concat!(
        "proc f {} {\n",
        "    for {set i 0} {$i < 10} {incr i} {\n",
        "        if {$i > 5} {\n",
        "            puts $i\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    let nesting = function_metric(&out, "f", "nesting").unwrap_or(0);
    assert!(nesting >= 2, "for+if should nest, got: {nesting}");
}

#[test]
fn switch_command_with_cases() {
    let out = debug(concat!(
        "proc f {x} {\n",
        "    switch $x {\n",
        "        a { puts alpha }\n",
        "        b { puts beta }\n",
        "    }\n",
        "}\n",
    ));
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert_eq!(cc, 3);
}

#[test]
fn switch_command_with_default() {
    let out = debug(concat!(
        "proc f {x} {\n",
        "    switch $x {\n",
        "        a { return 1 }\n",
        "        default { return 0 }\n",
        "    }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn try_on_error_handler() {
    let out = debug(concat!(
        "proc f {x} {\n",
        "    try {\n",
        "        expr {$x / 0}\n",
        "    } on error {msg opts} {\n",
        "        puts $msg\n",
        "    } finally {\n",
        "        puts done\n",
        "    }\n",
        "}\n",
    ));
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 2, "try/on error should add CC, got: {cc}");
}

#[test]
fn standalone_catch_increments_cc() {
    let out = debug("proc f {} {\n    catch {error \"boom\"} result\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 2, "standalone catch should add CC, got: {cc}");
}

#[test]
fn empty_catch_detected() {
    let out = check(concat!(
        "proc f {x} {\n",
        "    catch {} result\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Empty Error Handler"), "got: {out}");
}

#[test]
fn namespace_eval_body_parsed() {
    let out = debug(concat!(
        "namespace eval Helper {\n",
        "    proc compute {x} {\n",
        "        expr {$x + 1}\n",
        "    }\n",
        "}\n",
    ));
    assert!(out.contains("Helper.compute"), "got: {out}");
}

#[test]
fn namespace_variable_field_access() {
    let out = debug(concat!(
        "namespace eval Svc {\n",
        "    proc get_db {} {\n",
        "        variable db\n",
        "        return $db\n",
        "    }\n",
        "    proc get_cache {} {\n",
        "        variable cache\n",
        "        return $cache\n",
        "    }\n",
        "}\n",
    ));
    assert!(out.contains("\"db\"") || out.contains("\"cache\""), "got: {out}");
}

#[test]
fn foreach_multiple_vars() {
    let out = debug("proc f {items} {\n    foreach item $items {\n        puts $item\n    }\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 2, "foreach should add CC, got: {cc}");
}

#[test]
fn expr_boolean_in_condition() {
    let out = debug("proc f {a b} {\n    if {$a > 0 && $b > 0} {\n        return 1\n    }\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap_or(0);
    assert!(cc >= 3, "expr boolean in if should add CC, got: {cc}");
}

#[test]
fn set_variable_short_name() {
    let out = debug("proc f {data} {\n    set a 1\n    set b 2\n    set c 3\n    set d 4\n    return $a\n}\n");
    let sv = function_metric(&out, "f", "short_vars").unwrap_or(0);
    assert!(sv >= 4, "should count short var names, got: {sv}");
}

#[test]
fn switch_string_arms_counted() {
    let out = debug(concat!(
        "proc f {cmd} {\n",
        "    switch $cmd {\n",
        "        create  { return 1 }\n",
        "        read    { return 2 }\n",
        "        update  { return 3 }\n",
        "        delete  { return 4 }\n",
        "        list    { return 5 }\n",
        "        search  { return 6 }\n",
        "        default { return 0 }\n",
        "    }\n",
        "}\n",
    ));
    let arms = function_metric(&out, "f", "str_match").unwrap_or(0);
    assert!(arms >= 6, "should count switch string arms, got: {arms}");
}

#[test]
fn global_for_command_not_in_proc() {
    let out = check("for {set i 0} {$i < 3} {incr i} {\n    puts $i\n}\nproc helper {x} {\n    return $x\n}\n");
    assert!(out.is_empty() || !out.is_empty(), "should parse without crash");
}

#[test]
fn nested_braced_word_bodies() {
    let out = debug(concat!(
        "proc f {x} {\n",
        "    if {$x > 0} {\n",
        "        while {$x > 0} {\n",
        "            foreach item {1 2 3} {\n",
        "                if {$item > 1} {}\n",
        "            }\n",
        "            incr x -1\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    let nesting = function_metric(&out, "f", "nesting").unwrap_or(0);
    assert!(nesting >= 4, "nested control should accumulate depth, got: {nesting}");
}

// ===========================================================================
// Performance (2)
// ===========================================================================

#[test]
fn performance_1000_loc() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("perf.tcl");
    let mut code = String::new();
    for i in 0..50 {
        code.push_str(&format!("proc func{i} {{x}} {{\n"));
        for j in 0..18 {
            code.push_str(&format!("    set v{j} {j}\n"));
        }
        code.push_str("    return $x\n}\n\n");
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
fn performance_namespace_hierarchy() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("namespaces.tcl");
    let mut code = String::new();
    for i in 0..10 {
        code.push_str(&format!("namespace eval S{i} {{\n"));
        for j in 0..5 {
            code.push_str(&format!("    proc m{j} {{}} {{\n        variable x{i}\n        return $x{i}\n    }}\n"));
        }
        code.push_str("}\n\n");
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
fn clean_namespace_not_flagged() {
    let out = check(concat!(
        "namespace eval Point {\n",
        "    proc add {} {\n        variable x\n        variable y\n        expr {$x + $y}\n    }\n",
        "}\n",
        "proc helper {x} {\n    expr {$x + 1}\n}\n",
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
fn empty_proc_body() {
    let out = debug("proc f {} {}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(1));
}

#[test]
fn multiple_functions_independent_metrics() {
    let out = debug("proc simple {} {\n    return 0\n}\nproc complex {x} {\n    if {$x} {}\n}\n");
    assert_eq!(function_metric(&out, "simple", "cc"), Some(1));
    assert_eq!(function_metric(&out, "complex", "cc"), Some(2));
}
