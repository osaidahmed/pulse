use pulse::parse::{parse_and_walk, Language};

fn analyze(source: &str) -> pulse::walk::FileMetrics {
    parse_and_walk(source, Language::R).expect("parse R")
}

#[test]
fn bare_top_level_function_definition_collected() {
    let source = "function(x) {\n    if (x > 0) print(x)\n    x + 1\n}\n";
    let metrics = analyze(source);
    assert!(
        metrics.functions.iter().any(|f| f.name == "<anonymous>"),
        "bare top-level function_definition should be collected as <anonymous>"
    );
}

#[test]
fn top_level_right_arrow_operators_are_handled() {
    // tree-sitter-r parses `function() {...} -> name` with a greedy function body (the `->` is
    // absorbed into the body), so a right-arrow never *names* a function. What is reachable is the
    // ->/->> arm of handle_assignment via a top-level non-function right-assignment, which then
    // early-returns because the lhs is not a function_definition.
    let metrics = analyze("f <- function(a) a + 1\n5 -> x\n6 ->> y\n");
    assert!(metrics.functions.iter().any(|f| f.name == "f"), "the <- function is named: {:?}", metrics.functions);
    assert_eq!(metrics.functions.len(), 1, "non-function right-assignments add no functions: {:?}", metrics.functions);
}

#[test]
fn non_assignment_binary_operator_at_top_level_ignored() {
    let source = "1 + 2\nx * y\n";
    let metrics = analyze(source);
    assert!(metrics.functions.is_empty(), "top-level arithmetic binary_operator should not yield functions");
}

#[test]
fn dollar_extraction_lhs_names_function_via_rsplit() {
    let source = "obj$method <- function(x) {\n    x + 1\n}\n";
    let metrics = analyze(source);
    assert!(
        metrics.functions.iter().any(|f| f.name == "method"),
        "dollar-extraction lhs should resolve name via rsplit on $"
    );
}

#[test]
fn s4_slot_extraction_lhs_names_function_via_rsplit() {
    let source = "obj@handler <- function() {\n    NULL\n}\n";
    let metrics = analyze(source);
    assert!(
        metrics.functions.iter().any(|f| f.name == "handler"),
        "slot-extraction lhs should resolve name via rsplit on @"
    );
}

#[test]
fn nested_extraction_lhs_names_function_via_rsplit() {
    let source = "pkg$mod$run <- function(a, b) {\n    a + b\n}\n";
    let metrics = analyze(source);
    assert!(
        metrics.functions.iter().any(|f| f.name == "run"),
        "nested extraction lhs should resolve to the final segment via rsplit"
    );
}

#[test]
fn trycatch_named_handler_non_function_value_walked() {
    let source = "safe <- function(a, b) {\n    tryCatch(\n        a / b,\n        finally = cleanup(a, b)\n    )\n}\n";
    let metrics = analyze(source);
    assert!(
        metrics.functions.iter().any(|f| f.name == "safe"),
        "tryCatch with a non-function-definition handler value should still analyze the enclosing function"
    );
}

#[test]
fn trycatch_call_valued_handler_walked() {
    let source =
        "guarded <- function(x) {\n    tryCatch(\n        compute(x),\n        warning = log_warning(x)\n    )\n}\n";
    let _ = analyze(source);
}

#[test]
fn if_condition_with_compound_boolean_operators() {
    let source = "decide <- function(a, b, c) {\n    if (a && b || c) {\n        do_thing()\n    }\n}\n";
    let metrics = analyze(source);
    let max_cc = metrics.functions.iter().map(|f| f.cc).max().unwrap_or(0);
    assert!(max_cc >= 2, "compound boolean condition should raise cyclomatic complexity");
}

#[test]
fn if_condition_with_mixed_logical_and_comparison() {
    let source = "branchy <- function(x, y) {\n    if (x == 0 && y != 0) {\n        1\n    } else if (x > y || y > x) {\n        2\n    } else {\n        3\n    }\n}\n";
    let _ = analyze(source);
}

#[test]
fn malformed_leading_assignment_no_lhs_no_panic() {
    let source = "<- function(x) {\n    x + 1\n}\n";
    let _ = analyze(source);
}

#[test]
fn malformed_trailing_right_arrow_no_panic() {
    let source = "function(x) {\n    x + 1\n} ->\n";
    let _ = analyze(source);
}

#[test]
fn malformed_switch_missing_arguments_no_panic() {
    let source = "pick <- function(x) {\n    switch(\n}\n";
    let _ = analyze(source);
}

#[test]
fn switch_then_named_function_assignment_combined() {
    let source = "router <- function(x) {\n    switch(x,\n        \"a\" = 1,\n        \"b\" = 2,\n        0)\n}\nfunction(y) {\n    y - 1\n} -> decrement\n";
    let metrics = analyze(source);
    assert!(metrics.functions.iter().any(|f| f.name == "router"));
    // `function(y) {...} -> decrement` parses with a greedy body, so the second function is
    // collected anonymously rather than named `decrement`.
    assert!(metrics.functions.iter().any(|f| f.name == "<anonymous>"));
}
