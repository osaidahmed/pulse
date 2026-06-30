use pulse_syntax::parse::{parse_and_walk, Language};

fn analyze(source: &str) -> pulse_syntax::walk::FileMetrics {
    parse_and_walk(source, Language::R).expect("parse R")
}

#[test]
fn pipe_operator_handled() {
    let source = "process <- function(x) {\n    x |> sum()\n}\n";
    let _ = analyze(source);
}

#[test]
fn formula_handled() {
    let source = "fit_model <- function(data) {\n    lm(y ~ x, data = data)\n}\n";
    let _ = analyze(source);
}

#[test]
fn variadic_dots_parameter_handled() {
    let source = "merge_args <- function(...) {\n    list(...)\n}\n";
    let _ = analyze(source);
}

#[test]
fn dollar_extraction_handled() {
    let source = "get_value <- function(obj) {\n    obj$field\n}\n";
    let _ = analyze(source);
}

#[test]
fn at_extraction_for_s4_handled() {
    let source = "get_slot <- function(obj) {\n    obj@slot\n}\n";
    let _ = analyze(source);
}

#[test]
fn double_bracket_indexing_handled() {
    let source = "get_item <- function(obj, i) {\n    obj[[i]]\n}\n";
    let _ = analyze(source);
}

#[test]
fn nested_function_definitions_handled() {
    let source = "outer <- function(x) {\n    inner <- function(y) y * 2\n    inner(x)\n}\n";
    let _ = analyze(source);
}

#[test]
fn complex_control_flow_handled() {
    let source = "process <- function(x) {\n    if (x > 0) {\n        for (i in 1:x) {\n            if (i %% 2 == 0) print(i)\n            else next\n        }\n    } else {\n        while (x < 0) x <- x + 1\n    }\n    x\n}\n";
    let metrics = analyze(source);
    let max_cc = metrics.functions.iter().map(|f| f.cc).max().unwrap_or(0);
    assert!(max_cc >= 3);
}

#[test]
fn s4_method_definition_handled() {
    let source = "setMethod(\"show\", \"MyClass\", function(object) {\n    cat(\"showing\\n\")\n})\n";
    let _ = analyze(source);
}

#[test]
fn anonymous_function_handled() {
    let source = "result <- (function(x) x + 1)(5)\n";
    let _ = analyze(source);
}

#[test]
fn r_4_lambda_syntax_handled() {
    let source = "fn <- \\(x) x + 1\n";
    let _ = analyze(source);
}

#[test]
fn try_catch_handled() {
    let source = "safe_div <- function(a, b) {\n    tryCatch({\n        a / b\n    }, error = function(e) NA)\n}\n";
    let _ = analyze(source);
}

#[test]
fn repeat_loop_handled() {
    let source = "loop_until <- function(x) {\n    i <- 0\n    repeat {\n        i <- i + 1\n        if (i >= x) break\n    }\n    i\n}\n";
    let _ = analyze(source);
}

#[test]
fn list_construction_with_named_args_handled() {
    let source = "make_config <- function(host, port = 80) {\n    list(host = host, port = port)\n}\n";
    let _ = analyze(source);
}

#[test]
fn empty_r_file_no_panic() {
    let _ = analyze("\n");
}

#[test]
fn malformed_r_no_panic() {
    let _ = analyze("function(x { broken\n");
}

#[test]
fn many_arguments_function_handled() {
    let source = "complex_fn <- function(a, b, c, d, e, f, g, h) {\n    a + b + c + d + e + f + g + h\n}\n";
    let _ = analyze(source);
}

#[test]
fn switch_statement_handled() {
    let source = "categorize <- function(x) {\n    switch(x,\n        \"a\" = 1,\n        \"b\" = 2,\n        \"c\" = 3,\n        0)\n}\n";
    let _ = analyze(source);
}

#[test]
fn long_pipeline_handled() {
    let source = "pipeline <- function(data) {\n    data |> filter() |> mutate() |> select() |> arrange()\n}\n";
    let _ = analyze(source);
}

#[test]
fn nested_dollar_access_handled() {
    let source = "deep <- function(obj) {\n    obj$nested$deep$value\n}\n";
    let _ = analyze(source);
}
