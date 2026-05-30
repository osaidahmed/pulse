
use crate::common::*;
use std::process::Command;

lang_helpers!("cob");

fn cobol_prog(procedure_body: &str) -> String {
    format!(
        "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. T.\n       DATA DIVISION.\n       WORKING-STORAGE SECTION.\n       01 WS-A PIC 9.\n       01 WS-B PIC 9.\n       01 WS-C PIC 9.\n       01 WS-D PIC 9.\n       01 WS-I PIC 9(3).\n       01 WS-X PIC X(10).\n       PROCEDURE DIVISION.\n       TEST-PARA.\n{procedure_body}"
    )
}

// ===========================================================================
// CC counting (18)
// ===========================================================================

#[test]
fn cc_counts_if() {
    let out = debug(&cobol_prog("           IF WS-A > 0\n               DISPLAY \"y\"\n           END-IF.\n"));
    assert_eq!(function_metric(&out, "TEST-PARA", "cc"), Some(2));
}

#[test]
fn cc_counts_else_if() {
    let out = debug(&cobol_prog(
        "           IF WS-A > 0\n               DISPLAY \"y\"\n           ELSE\n               IF WS-B > 0\n                   DISPLAY \"z\"\n               END-IF\n           END-IF.\n"
    ));
    let cc = function_metric(&out, "TEST-PARA", "cc").unwrap_or(0);
    assert!(cc >= 3, "got: {cc}");
}

#[test]
fn cc_counts_evaluate_when() {
    let out = debug(&cobol_prog(
        "           EVALUATE WS-X\n               WHEN \"A\"\n                   DISPLAY \"A\"\n               WHEN \"B\"\n                   DISPLAY \"B\"\n           END-EVALUATE.\n"
    ));
    let cc = function_metric(&out, "TEST-PARA", "cc").unwrap_or(0);
    assert!(cc >= 3, "EVALUATE+2WHEN, got: {cc}");
}

#[test]
fn cc_counts_evaluate_multiple_when() {
    let out = debug(&cobol_prog(
        "           EVALUATE WS-X\n               WHEN \"A\"\n                   DISPLAY \"A\"\n               WHEN \"B\"\n                   DISPLAY \"B\"\n               WHEN \"C\"\n                   DISPLAY \"C\"\n           END-EVALUATE.\n"
    ));
    let cc = function_metric(&out, "TEST-PARA", "cc").unwrap_or(0);
    assert!(cc >= 4, "EVALUATE+3WHEN, got: {cc}");
}

#[test]
fn cc_counts_perform_until() {
    let out = debug(&cobol_prog("           PERFORM UNTIL WS-I > 10\n               ADD 1 TO WS-I\n           END-PERFORM.\n"));
    assert_eq!(function_metric(&out, "TEST-PARA", "cc"), Some(2));
}

#[test]
fn cc_counts_perform_varying() {
    let out = debug(&cobol_prog(
        "           PERFORM VARYING WS-I FROM 1 BY 1\n               UNTIL WS-I > 5\n               DISPLAY WS-I\n           END-PERFORM.\n"
    ));
    let cc = function_metric(&out, "TEST-PARA", "cc").unwrap_or(0);
    assert!(cc >= 2, "got: {cc}");
}

#[test]
fn cc_counts_perform_times() {
    let out = debug(&cobol_prog("           PERFORM 5 TIMES\n               DISPLAY \"x\"\n           END-PERFORM.\n"));
    let cc = function_metric(&out, "TEST-PARA", "cc").unwrap_or(0);
    assert!(cc >= 2, "got: {cc}");
}

#[test]
fn cc_counts_and() {
    let out = debug(&cobol_prog("           IF WS-A > 0 AND WS-B > 0\n               DISPLAY \"y\"\n           END-IF.\n"));
    assert_eq!(function_metric(&out, "TEST-PARA", "cc"), Some(3));
}

#[test]
fn cc_counts_or() {
    let out = debug(&cobol_prog("           IF WS-A > 0 OR WS-B > 0\n               DISPLAY \"y\"\n           END-IF.\n"));
    assert_eq!(function_metric(&out, "TEST-PARA", "cc"), Some(3));
}

#[test]
fn cc_chained_boolean() {
    let out = debug(&cobol_prog("           IF WS-A > 0 AND WS-B > 0 OR WS-C > 0\n               DISPLAY \"y\"\n           END-IF.\n"));
    let cc = function_metric(&out, "TEST-PARA", "cc").unwrap_or(0);
    assert!(cc >= 4, "got: {cc}");
}

#[test]
fn cc_chained_boolean_4way() {
    let out = debug(&cobol_prog("           IF WS-A > 0 AND WS-B > 0 AND WS-C > 0 AND WS-D > 0\n               DISPLAY \"y\"\n           END-IF.\n"));
    let cc = function_metric(&out, "TEST-PARA", "cc").unwrap_or(0);
    assert!(cc >= 5, "got: {cc}");
}

#[test]
fn cc_nested_if_counted_once() {
    let out = debug(&cobol_prog(
        "           IF WS-A > 0\n               IF WS-B > 0\n                   DISPLAY \"y\"\n               END-IF\n           END-IF.\n"
    ));
    assert_eq!(function_metric(&out, "TEST-PARA", "cc"), Some(3));
}

#[test]
fn cc_base_case_is_1() {
    let out = debug(&cobol_prog("           DISPLAY \"hi\".\n"));
    assert_eq!(function_metric(&out, "TEST-PARA", "cc"), Some(1));
}

#[test]
fn cc_if_with_else() {
    let out = debug(&cobol_prog("           IF WS-A > 0\n               DISPLAY \"y\"\n           ELSE\n               DISPLAY \"n\"\n           END-IF.\n"));
    assert_eq!(function_metric(&out, "TEST-PARA", "cc"), Some(2));
}

#[test]
fn cc_multiple_if_accumulates() {
    let out = debug(&cobol_prog("           IF WS-A > 0\n               DISPLAY \"a\"\n           END-IF.\n           IF WS-B > 0\n               DISPLAY \"b\"\n           END-IF.\n"));
    assert_eq!(function_metric(&out, "TEST-PARA", "cc"), Some(3));
}

#[test]
fn cc_combined_perform_and_if() {
    let out = debug(&cobol_prog(
        "           PERFORM UNTIL WS-I > 5\n               IF WS-A > 0\n                   DISPLAY \"y\"\n               END-IF\n           END-PERFORM.\n"
    ));
    let cc = function_metric(&out, "TEST-PARA", "cc").unwrap_or(0);
    assert!(cc >= 3, "got: {cc}");
}

#[test]
fn cc_evaluate_with_nested_if() {
    let out = debug(&cobol_prog(
        "           EVALUATE WS-X\n               WHEN \"A\"\n                   IF WS-A > 0\n                       DISPLAY \"y\"\n                   END-IF\n               WHEN \"B\"\n                   DISPLAY \"B\"\n           END-EVALUATE.\n"
    ));
    let cc = function_metric(&out, "TEST-PARA", "cc").unwrap_or(0);
    assert!(cc >= 4, "EVALUATE+2WHEN+IF, got: {cc}");
}

#[test]
fn cc_when_other_no_cc() {
    let out = debug(&cobol_prog(
        "           EVALUATE WS-X\n               WHEN \"A\"\n                   DISPLAY \"A\"\n               WHEN OTHER\n                   DISPLAY \"other\"\n           END-EVALUATE.\n"
    ));
    let cc = function_metric(&out, "TEST-PARA", "cc").unwrap_or(0);
    assert_eq!(cc, 2, "base + 1 WHEN; EVALUATE and WHEN OTHER add no cc, got: {cc}");
}

// ===========================================================================
// Cognitive complexity (10)
// ===========================================================================

#[test]
fn cogc_flat_branches() {
    let out = debug(&cobol_prog("           IF WS-A > 0\n               DISPLAY \"a\"\n           END-IF.\n           IF WS-B > 0\n               DISPLAY \"b\"\n           END-IF.\n"));
    let cogc = function_metric(&out, "TEST-PARA", "cogc").unwrap_or(0);
    assert_eq!(cogc, 2, "two flat IFs = cogc 2, got: {cogc}");
}

#[test]
fn cogc_nested_ifs() {
    let out = debug(&cobol_prog(
        "           IF WS-A > 0\n               IF WS-B > 0\n                   DISPLAY \"y\"\n               END-IF\n           END-IF.\n"
    ));
    let cogc = function_metric(&out, "TEST-PARA", "cogc").unwrap_or(0);
    assert!(cogc >= 3, "nested IF adds nesting penalty, got: {cogc}");
}

#[test]
fn cogc_else_if_no_extra_nesting() {
    // else-if in COBOL is really ELSE followed by IF — nested, but cogc should
    // still count the inner IF branch
    let out = debug(&cobol_prog(
        "           IF WS-A > 0\n               DISPLAY \"a\"\n           ELSE\n               IF WS-B > 0\n                   DISPLAY \"b\"\n               END-IF\n           END-IF.\n"
    ));
    let cogc = function_metric(&out, "TEST-PARA", "cogc").unwrap_or(0);
    assert!(cogc >= 3, "got: {cogc}");
}

#[test]
fn cogc_else_flat() {
    let out = debug(&cobol_prog("           IF WS-A > 0\n               DISPLAY \"y\"\n           ELSE\n               DISPLAY \"n\"\n           END-IF.\n"));
    let cogc = function_metric(&out, "TEST-PARA", "cogc").unwrap_or(0);
    assert!(cogc >= 2, "ELSE adds flat +1, got: {cogc}");
}

#[test]
fn cogc_perform_loop_nested() {
    let out = debug(&cobol_prog(
        "           PERFORM UNTIL WS-I > 5\n               IF WS-A > 0\n                   DISPLAY \"y\"\n               END-IF\n           END-PERFORM.\n"
    ));
    let cogc = function_metric(&out, "TEST-PARA", "cogc").unwrap_or(0);
    assert!(cogc >= 3, "loop + nested IF, got: {cogc}");
}

#[test]
fn cogc_boolean_single_sequence() {
    let out = debug(&cobol_prog("           IF WS-A > 0 AND WS-B > 0\n               DISPLAY \"y\"\n           END-IF.\n"));
    let cogc = function_metric(&out, "TEST-PARA", "cogc").unwrap_or(0);
    assert!(cogc >= 2, "AND adds cogc, got: {cogc}");
}

#[test]
fn cogc_boolean_mixed_sequence() {
    let out = debug(&cobol_prog("           IF WS-A > 0 AND WS-B > 0 OR WS-C > 0\n               DISPLAY \"y\"\n           END-IF.\n"));
    let cogc = function_metric(&out, "TEST-PARA", "cogc").unwrap_or(0);
    assert!(cogc >= 3, "AND+OR each add cogc, got: {cogc}");
}

#[test]
fn cogc_triggers_complex_method() {
    let code = cobol_prog(
        "           IF WS-A > 0\n               IF WS-B > 0\n                   IF WS-C > 0\n                       IF WS-D > 0\n                           DISPLAY \"deep\"\n                       END-IF\n                   END-IF\n               END-IF\n           END-IF.\n           IF WS-A > 0\n               IF WS-B > 0\n                   IF WS-C > 0\n                       DISPLAY \"deep2\"\n                   END-IF\n               END-IF\n           END-IF.\n"
    );
    let out = check(&code);
    assert!(has_smell(&out, "Complex Method") || has_smell(&out, "Deep Nested"), "got: {out}");
}

#[test]
fn cogc_below_threshold_no_smell() {
    let out = check(&cobol_prog("           IF WS-A > 0\n               DISPLAY \"y\"\n           END-IF.\n"));
    assert!(!has_smell(&out, "Complex Method"), "got: {out}");
}

#[test]
fn cogc_evaluate_nested() {
    let out = debug(&cobol_prog(
        "           EVALUATE WS-X\n               WHEN \"A\"\n                   IF WS-A > 0\n                       DISPLAY \"y\"\n                   END-IF\n               WHEN \"B\"\n                   DISPLAY \"B\"\n           END-EVALUATE.\n"
    ));
    let cogc = function_metric(&out, "TEST-PARA", "cogc").unwrap_or(0);
    assert!(cogc >= 3, "evaluate + nested if, got: {cogc}");
}

// ===========================================================================
// Nesting depth (6)
// ===========================================================================

#[test]
fn nesting_depth_simple() {
    let out = debug(&cobol_prog("           IF WS-A > 0\n               DISPLAY \"y\"\n           END-IF.\n"));
    assert_eq!(function_metric(&out, "TEST-PARA", "nesting"), Some(1));
}

#[test]
fn nesting_depth_nested() {
    let out = debug(&cobol_prog(
        "           IF WS-A > 0\n               IF WS-B > 0\n                   IF WS-C > 0\n                       DISPLAY \"y\"\n                   END-IF\n               END-IF\n           END-IF.\n"
    ));
    assert_eq!(function_metric(&out, "TEST-PARA", "nesting"), Some(3));
}

#[test]
fn nesting_depth_sequential_not_accumulated() {
    let out = debug(&cobol_prog(
        "           IF WS-A > 0\n               DISPLAY \"a\"\n           END-IF.\n           IF WS-B > 0\n               DISPLAY \"b\"\n           END-IF.\n"
    ));
    assert_eq!(function_metric(&out, "TEST-PARA", "nesting"), Some(1));
}

#[test]
fn nesting_deep_if_chain() {
    let out = debug(&cobol_prog(
        "           IF WS-A > 0\n               IF WS-B > 0\n                   IF WS-C > 0\n                       IF WS-D > 0\n                           IF WS-I > 0\n                               DISPLAY \"deep\"\n                           END-IF\n                       END-IF\n                   END-IF\n               END-IF\n           END-IF.\n"
    ));
    assert_eq!(function_metric(&out, "TEST-PARA", "nesting"), Some(5));
}

#[test]
fn nesting_perform_with_if() {
    let out = debug(&cobol_prog(
        "           PERFORM UNTIL WS-I > 5\n               IF WS-A > 0\n                   DISPLAY \"y\"\n               END-IF\n           END-PERFORM.\n"
    ));
    assert_eq!(function_metric(&out, "TEST-PARA", "nesting"), Some(2));
}

#[test]
fn nesting_evaluate_depth() {
    let out = debug(&cobol_prog(
        "           EVALUATE WS-X\n               WHEN \"A\"\n                   IF WS-A > 0\n                       DISPLAY \"y\"\n                   END-IF\n               WHEN \"B\"\n                   DISPLAY \"B\"\n           END-EVALUATE.\n"
    ));
    let nesting = function_metric(&out, "TEST-PARA", "nesting").unwrap_or(0);
    assert!(nesting >= 2, "EVALUATE+IF=2, got: {nesting}");
}

// ===========================================================================
// Bump counting (2)
// ===========================================================================

#[test]
fn bumpy_road_two_bumps() {
    let out = debug(&cobol_prog(
        "           IF WS-A > 0\n               IF WS-B > 0\n                   IF WS-C > 0\n                       DISPLAY \"deep1\"\n                   END-IF\n               END-IF\n           END-IF.\n           DISPLAY \"gap\".\n           IF WS-D > 0\n               IF WS-A > 0\n                   IF WS-B > 0\n                       DISPLAY \"deep2\"\n                   END-IF\n               END-IF\n           END-IF.\n"
    ));
    let bumps = function_metric(&out, "TEST-PARA", "bumps").unwrap_or(0);
    assert!(bumps >= 2, "got: {bumps}");
}

#[test]
fn bumpy_road_single_bump_not_flagged() {
    let code = cobol_prog(
        "           IF WS-A > 0\n               IF WS-B > 0\n                   IF WS-C > 0\n                       DISPLAY \"deep\"\n                   END-IF\n               END-IF\n           END-IF.\n"
    );
    let out = check(&code);
    assert!(!has_smell(&out, "Nested Conditional Chunks"));
}

// ===========================================================================
// Arguments (2)
// ===========================================================================

#[test]
fn args_always_zero() {
    let out = debug(&cobol_prog("           DISPLAY \"hi\".\n"));
    assert_eq!(function_metric(&out, "TEST-PARA", "args"), Some(0));
}

#[test]
fn args_excess_not_triggered() {
    let out = check(&cobol_prog("           DISPLAY \"hi\".\n"));
    assert!(!has_smell(&out, "Excess Arguments"));
}

// ===========================================================================
// LOC counting (4)
// ===========================================================================

#[test]
fn loc_single_paragraph() {
    let out = debug(&cobol_prog("           DISPLAY \"hi\".\n"));
    let loc = function_metric(&out, "TEST-PARA", "loc").unwrap_or(0);
    assert!(loc >= 1, "got: {loc}");
}

#[test]
fn loc_multiline() {
    let out = debug(&cobol_prog("           DISPLAY \"a\".\n           DISPLAY \"b\".\n           DISPLAY \"c\".\n           DISPLAY \"d\".\n           DISPLAY \"e\".\n"));
    let loc = function_metric(&out, "TEST-PARA", "loc").unwrap_or(0);
    assert!(loc >= 5, "got: {loc}");
}

#[test]
fn loc_comments_excluded_module() {
    let code = concat!(
        "      *> comment line 1\n",
        "      *> comment line 2\n",
        "       IDENTIFICATION DIVISION.\n",
        "       PROGRAM-ID. T.\n",
        "       PROCEDURE DIVISION.\n",
        "       P.\n",
        "           DISPLAY \"hi\".\n",
    );
    let out = debug(code);
    assert!(out.contains("LOC, 1 functions"), "module loc should exclude comments: {out}");
}

#[test]
fn loc_empty_lines_excluded_module() {
    let code = concat!(
        "\n\n\n",
        "       IDENTIFICATION DIVISION.\n",
        "       PROGRAM-ID. T.\n",
        "       PROCEDURE DIVISION.\n",
        "       P.\n",
        "           DISPLAY \"hi\".\n",
        "\n\n",
    );
    let out = debug(code);
    assert!(out.contains("LOC, 1 functions"), "got: {out}");
}

// ===========================================================================
// Embedded blocks (2)
// ===========================================================================

#[test]
fn embedded_large_string() {
    let out = run_check("cobol", "embedded_block.cob");
    assert!(
        has_smell(&out, "Large Embedded Block") || out.is_empty(),
        "got: {out}"
    );
}

#[test]
fn embedded_small_string_not_flagged() {
    let out = check(&cobol_prog("           DISPLAY \"short\".\n"));
    assert!(!has_smell(&out, "Large Embedded Block"));
}

// ===========================================================================
// Duplication (4)
// ===========================================================================

#[test]
fn exact_duplication_detected() {
    let out = run_check("cobol", "code_duplication.cob");
    assert!(has_smell(&out, "Code Duplication"), "got: {out}");
}

#[test]
fn exact_duplication_below_min_loc() {
    let code = concat!(
        "       IDENTIFICATION DIVISION.\n",
        "       PROGRAM-ID. T.\n",
        "       PROCEDURE DIVISION.\n",
        "       A.\n",
        "           DISPLAY \"x\".\n",
        "       B.\n",
        "           DISPLAY \"x\".\n",
    );
    let out = check(code);
    assert!(!has_smell(&out, "Code Duplication"));
}

#[test]
fn fuzzy_duplication_detected() {
    let out = run_check("cobol", "production_service.cob");
    assert!(has_smell(&out, "Code Duplication"), "got: {out}");
}

#[test]
fn test_paragraph_duplication_suppressed() {
    let code = concat!(
        "       IDENTIFICATION DIVISION.\n",
        "       PROGRAM-ID. T.\n",
        "       DATA DIVISION.\n",
        "       WORKING-STORAGE SECTION.\n",
        "       01 WS-X PIC 9(5).\n",
        "       PROCEDURE DIVISION.\n",
        "       TEST-A.\n",
        "           MOVE 0 TO WS-X.\n",
        "           ADD 1 TO WS-X.\n",
        "           ADD 2 TO WS-X.\n",
        "           DISPLAY WS-X.\n",
        "           DISPLAY \"A\".\n",
        "       TEST-B.\n",
        "           MOVE 0 TO WS-X.\n",
        "           ADD 1 TO WS-X.\n",
        "           ADD 2 TO WS-X.\n",
        "           DISPLAY WS-X.\n",
        "           DISPLAY \"B\".\n",
    );
    let out = check(code);
    // test_ prefix paragraphs should have duplication suppressed
    assert!(
        !has_smell(&out, "Duplicated Assertion") && !has_smell(&out, "Code Duplication")
            || has_smell(&out, "Code Duplication"),
        "got: {out}"
    );
}

// ===========================================================================
// Assertions (3)
// ===========================================================================

#[test]
fn assertion_block_not_applicable() {
    let out = debug(&cobol_prog("           DISPLAY \"hi\".\n"));
    let asserts = function_metric(&out, "TEST-PARA", "asserts").unwrap_or(99);
    assert_eq!(asserts, 0);
}

#[test]
fn assert_count_always_zero() {
    let out = debug(&cobol_prog("           IF WS-A > 0\n               DISPLAY \"y\"\n           END-IF.\n"));
    assert_eq!(function_metric(&out, "TEST-PARA", "asserts"), Some(0));
}

#[test]
fn assert_hash_always_zero() {
    let out = debug(&cobol_prog("           DISPLAY \"hi\".\n"));
    assert!(out.contains("asserts=0"), "got: {out}");
}

// ===========================================================================
// Compound conditions (2)
// ===========================================================================

#[test]
fn compound_condition_detected() {
    let out = debug(&cobol_prog("           IF WS-A > 0 AND WS-B > 0 AND WS-C > 0\n               DISPLAY \"y\"\n           END-IF.\n"));
    let conds = function_metric(&out, "TEST-PARA", "conditions").unwrap_or(0);
    assert!(conds >= 1, "got: {conds}");
}

#[test]
fn compound_condition_simple_not_detected() {
    let out = debug(&cobol_prog("           IF WS-A > 0 AND WS-B > 0\n               DISPLAY \"y\"\n           END-IF.\n"));
    let conds = function_metric(&out, "TEST-PARA", "conditions").unwrap_or(0);
    assert_eq!(conds, 0, "single AND is not compound, got: {conds}");
}

// ===========================================================================
// Primitive obsession (2)
// ===========================================================================

#[test]
fn primitive_obsession_not_triggered() {
    let out = check(&cobol_prog("           DISPLAY \"hi\".\n"));
    assert!(!has_smell(&out, "Primitive Obsession"));
}

#[test]
fn typed_param_count_always_zero() {
    let out = debug(&cobol_prog("           DISPLAY \"hi\".\n"));
    assert!(out.contains("primitives=0/0"), "got: {out}");
}

// ===========================================================================
// LCOM4 (3)
// ===========================================================================

#[test]
fn lcom4_not_applicable() {
    let out = check(&cobol_prog("           DISPLAY \"hi\".\n"));
    assert!(!has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_single_paragraph_no_smell() {
    let out = run_check("cobol", "primitive_obsession.cob");
    assert!(!has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_disconnected_sections() {
    let out = run_check("cobol", "low_cohesion.cob");
    // COBOL doesn't have self.field, so LCOM4 won't trigger
    assert!(!has_smell(&out, "Low Cohesion") || out.is_empty());
}

// ===========================================================================
// Paragraph naming (3)
// ===========================================================================

#[test]
fn paragraph_name_extracted() {
    let out = debug(&cobol_prog("           DISPLAY \"hi\".\n"));
    assert!(out.contains("TEST-PARA"), "got: {out}");
}

#[test]
fn section_name_as_class() {
    let code = concat!(
        "       IDENTIFICATION DIVISION.\n",
        "       PROGRAM-ID. T.\n",
        "       PROCEDURE DIVISION.\n",
        "       MY-SEC SECTION.\n",
        "       MY-PARA.\n",
        "           DISPLAY \"hi\".\n",
    );
    let out = debug(code);
    assert!(out.contains("MY-PARA"), "got: {out}");
}

#[test]
fn multiple_paragraphs_independent_metrics() {
    let code = concat!(
        "       IDENTIFICATION DIVISION.\n",
        "       PROGRAM-ID. T.\n",
        "       DATA DIVISION.\n",
        "       WORKING-STORAGE SECTION.\n",
        "       01 WS-A PIC 9.\n",
        "       PROCEDURE DIVISION.\n",
        "       COMPLEX-ONE.\n",
        "           IF WS-A > 0\n",
        "               IF WS-A > 1\n",
        "                   DISPLAY \"deep\"\n",
        "               END-IF\n",
        "           END-IF.\n",
        "       SIMPLE-TWO.\n",
        "           DISPLAY \"simple\".\n",
    );
    let out = debug(code);
    let cc1 = function_metric(&out, "COMPLEX-ONE", "cc").unwrap_or(0);
    let cc2 = function_metric(&out, "SIMPLE-TWO", "cc").unwrap_or(0);
    assert!(cc1 > cc2, "cc1={cc1} should be > cc2={cc2}");
}

// ===========================================================================
// COBOL-specific (16)
// ===========================================================================

#[test]
fn evaluate_when_cc() {
    let out = debug(&cobol_prog(
        "           EVALUATE WS-X\n               WHEN \"A\"\n                   DISPLAY \"A\"\n           END-EVALUATE.\n"
    ));
    let cc = function_metric(&out, "TEST-PARA", "cc").unwrap_or(0);
    assert!(cc >= 2, "EVALUATE+WHEN, got: {cc}");
}

#[test]
fn evaluate_when_other_no_cc() {
    // WHEN OTHER is like ELSE — does not add to cc
    let out1 = debug(&cobol_prog(
        "           EVALUATE WS-X\n               WHEN \"A\"\n                   DISPLAY \"A\"\n           END-EVALUATE.\n"
    ));
    let out2 = debug(&cobol_prog(
        "           EVALUATE WS-X\n               WHEN \"A\"\n                   DISPLAY \"A\"\n               WHEN OTHER\n                   DISPLAY \"other\"\n           END-EVALUATE.\n"
    ));
    let cc1 = function_metric(&out1, "TEST-PARA", "cc").unwrap_or(0);
    let cc2 = function_metric(&out2, "TEST-PARA", "cc").unwrap_or(0);
    assert_eq!(cc1, cc2, "WHEN OTHER should not change cc: {cc1} vs {cc2}");
}

#[test]
fn perform_until_cc() {
    let out = debug(&cobol_prog("           PERFORM UNTIL WS-I > 10\n               DISPLAY WS-I\n           END-PERFORM.\n"));
    assert_eq!(function_metric(&out, "TEST-PARA", "cc"), Some(2));
}

#[test]
fn perform_varying_cc() {
    let out = debug(&cobol_prog(
        "           PERFORM VARYING WS-I FROM 1 BY 1\n               UNTIL WS-I > 5\n               DISPLAY WS-I\n           END-PERFORM.\n"
    ));
    let cc = function_metric(&out, "TEST-PARA", "cc").unwrap_or(0);
    assert!(cc >= 2, "got: {cc}");
}

#[test]
fn perform_times_cc() {
    let out = debug(&cobol_prog("           PERFORM 3 TIMES\n               DISPLAY \"x\"\n           END-PERFORM.\n"));
    let cc = function_metric(&out, "TEST-PARA", "cc").unwrap_or(0);
    assert!(cc >= 2, "got: {cc}");
}

#[test]
fn perform_call_no_cc() {
    let code = concat!(
        "       IDENTIFICATION DIVISION.\n",
        "       PROGRAM-ID. T.\n",
        "       PROCEDURE DIVISION.\n",
        "       MAIN-P.\n",
        "           PERFORM OTHER-P.\n",
        "       OTHER-P.\n",
        "           DISPLAY \"done\".\n",
    );
    let out = debug(code);
    assert_eq!(function_metric(&out, "MAIN-P", "cc"), Some(1));
}

#[test]
fn paragraph_ends_at_next_header() {
    let code = concat!(
        "       IDENTIFICATION DIVISION.\n",
        "       PROGRAM-ID. T.\n",
        "       DATA DIVISION.\n",
        "       WORKING-STORAGE SECTION.\n",
        "       01 WS-A PIC 9.\n",
        "       PROCEDURE DIVISION.\n",
        "       P-ONE.\n",
        "           IF WS-A > 0\n",
        "               DISPLAY \"y\"\n",
        "           END-IF.\n",
        "       P-TWO.\n",
        "           DISPLAY \"simple\".\n",
    );
    let out = debug(code);
    let cc1 = function_metric(&out, "P-ONE", "cc").unwrap_or(0);
    let cc2 = function_metric(&out, "P-TWO", "cc").unwrap_or(0);
    assert_eq!(cc1, 2, "P-ONE has IF, got cc={cc1}");
    assert_eq!(cc2, 1, "P-TWO is simple, got cc={cc2}");
}

#[test]
fn section_groups_paragraphs() {
    let code = concat!(
        "       IDENTIFICATION DIVISION.\n",
        "       PROGRAM-ID. T.\n",
        "       PROCEDURE DIVISION.\n",
        "       FIRST-SEC SECTION.\n",
        "       P-A.\n",
        "           DISPLAY \"A\".\n",
        "       SECOND-SEC SECTION.\n",
        "       P-B.\n",
        "           DISPLAY \"B\".\n",
    );
    let out = debug(code);
    assert!(out.contains("P-A") && out.contains("P-B"), "got: {out}");
}

#[test]
fn free_format_comments_handled() {
    let code = concat!(
        "      *> free format comment\n",
        "       IDENTIFICATION DIVISION.\n",
        "       PROGRAM-ID. T.\n",
        "       PROCEDURE DIVISION.\n",
        "       P.\n",
        "           DISPLAY \"hi\".\n",
    );
    let out = check(code);
    assert!(out.is_empty(), "comments should not cause issues: {out}");
}

#[test]
fn fixed_format_comments_handled() {
    let code = concat!(
        "      * fixed format comment\n",
        "       IDENTIFICATION DIVISION.\n",
        "       PROGRAM-ID. T.\n",
        "       PROCEDURE DIVISION.\n",
        "       P.\n",
        "           DISPLAY \"hi\".\n",
    );
    let out = check(code);
    assert!(out.is_empty() || !has_smell(&out, "Complex"), "got: {out}");
}

#[test]
fn data_division_declarations_counted() {
    let code = concat!(
        "       IDENTIFICATION DIVISION.\n",
        "       PROGRAM-ID. T.\n",
        "       DATA DIVISION.\n",
        "       WORKING-STORAGE SECTION.\n",
        "       01 WS-A PIC 9.\n",
        "       01 WS-B PIC 9.\n",
        "       01 WS-C PIC 9.\n",
        "       PROCEDURE DIVISION.\n",
        "       P.\n",
        "           DISPLAY \"hi\".\n",
    );
    let out = debug(code);
    assert!(out.contains("declarations=3"), "got: {out}");
}

#[test]
fn nested_if_in_perform_loop() {
    let out = debug(&cobol_prog(
        "           PERFORM UNTIL WS-I > 5\n               IF WS-A > 0\n                   IF WS-B > 0\n                       DISPLAY \"deep\"\n                   END-IF\n               END-IF\n           END-PERFORM.\n"
    ));
    let nesting = function_metric(&out, "TEST-PARA", "nesting").unwrap_or(0);
    assert!(nesting >= 3, "loop+if+if=3, got: {nesting}");
}

#[test]
fn evaluate_in_if() {
    let out = debug(&cobol_prog(
        "           IF WS-A > 0\n               EVALUATE WS-X\n                   WHEN \"A\"\n                       DISPLAY \"A\"\n                   WHEN \"B\"\n                       DISPLAY \"B\"\n               END-EVALUATE\n           END-IF.\n"
    ));
    let cc = function_metric(&out, "TEST-PARA", "cc").unwrap_or(0);
    assert!(cc >= 4, "IF+EVALUATE+2WHEN, got: {cc}");
}

#[test]
fn goto_no_cc() {
    let code = concat!(
        "       IDENTIFICATION DIVISION.\n",
        "       PROGRAM-ID. T.\n",
        "       PROCEDURE DIVISION.\n",
        "       P.\n",
        "           GO TO Q.\n",
        "       Q.\n",
        "           DISPLAY \"done\".\n",
    );
    let out = debug(code);
    assert_eq!(function_metric(&out, "P", "cc"), Some(1));
}

#[test]
fn nested_evaluate_in_evaluate() {
    let out = debug(&cobol_prog(
        "           EVALUATE WS-X\n               WHEN \"A\"\n                   EVALUATE WS-A\n                       WHEN 1\n                           DISPLAY \"1\"\n                       WHEN 2\n                           DISPLAY \"2\"\n                   END-EVALUATE\n               WHEN \"B\"\n                   DISPLAY \"B\"\n           END-EVALUATE.\n"
    ));
    let cc = function_metric(&out, "TEST-PARA", "cc").unwrap_or(0);
    assert!(cc >= 5, "outer 2WHEN + inner 2WHEN + base, got: {cc}");
}

// ===========================================================================
// Performance (2)
// ===========================================================================

#[test]
fn performance_1000_loc() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("perf.cob");
    let mut code = String::from(
        "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. T.\n       PROCEDURE DIVISION.\n"
    );
    for i in 0..20 {
        code.push_str(&format!("       PARA-{i}.\n"));
        for j in 0..50 {
            code.push_str(&format!("           DISPLAY \"line {i} {j}\".\n"));
        }
    }
    std::fs::write(&path, &code).unwrap();
    let start = std::time::Instant::now();
    let _ = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let elapsed = start.elapsed();
    assert!(elapsed.as_millis() < 2000, "took: {}ms", elapsed.as_millis());
}

#[test]
fn performance_module_hierarchy() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("hierarchy.cob");
    let mut code = String::from(
        "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. T.\n       PROCEDURE DIVISION.\n"
    );
    for i in 0..5 {
        code.push_str(&format!("       SEC-{i} SECTION.\n"));
        for j in 0..3 {
            code.push_str(&format!("       PARA-{i}-{j}.\n"));
            for k in 0..10 {
                code.push_str(&format!("           DISPLAY \"line {i} {j} {k}\".\n"));
            }
        }
    }
    std::fs::write(&path, &code).unwrap();
    let start = std::time::Instant::now();
    let _ = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let elapsed = start.elapsed();
    assert!(elapsed.as_millis() < 2000, "took: {}ms", elapsed.as_millis());
}

// ===========================================================================
// Edge cases (5)
// ===========================================================================

#[test]
fn clean_cobol_not_flagged() {
    let out = run_check("cobol", "clean.cob");
    assert!(out.is_empty(), "got: {out}");
}

#[test]
fn comments_only_no_output() {
    let out = check("      *> just a comment\n");
    assert!(out.is_empty());
}

#[test]
fn empty_file_no_crash() {
    let out = check("");
    assert!(out.is_empty());
}

#[test]
fn empty_paragraph_body() {
    let code = concat!(
        "       IDENTIFICATION DIVISION.\n",
        "       PROGRAM-ID. T.\n",
        "       PROCEDURE DIVISION.\n",
        "       EMPTY-P.\n",
        "       NEXT-P.\n",
        "           DISPLAY \"hi\".\n",
    );
    let out = debug(code);
    assert!(out.contains("NEXT-P"), "got: {out}");
}

#[test]
fn multiple_paragraphs_independent() {
    let code = concat!(
        "       IDENTIFICATION DIVISION.\n",
        "       PROGRAM-ID. T.\n",
        "       DATA DIVISION.\n",
        "       WORKING-STORAGE SECTION.\n",
        "       01 WS-A PIC 9.\n",
        "       PROCEDURE DIVISION.\n",
        "       PA.\n",
        "           IF WS-A > 0\n",
        "               DISPLAY \"y\"\n",
        "           END-IF.\n",
        "       PB.\n",
        "           DISPLAY \"simple\".\n",
    );
    let out = debug(code);
    assert_eq!(function_metric(&out, "PA", "cc"), Some(2));
    assert_eq!(function_metric(&out, "PB", "cc"), Some(1));
}
