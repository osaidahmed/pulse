mod common;

use common::*;
use std::process::Command;

const LANG: &str = "haskell";

#[test]
fn clean_file_produces_no_output() {
    let output = run_check(LANG, "Clean.hs");
    assert!(output.is_empty(), "got: {output}");
}

#[test]
fn complex_method_detected() {
    let output = run_check(LANG, "ComplexMethod.hs");
    assert!(has_smell(&output, "Complex Method"), "got: {output}");
    assert!(has_function(&output, "processOrder"));
}

#[test]
fn complex_method_cc_at_least_9() {
    let debug = run_debug(LANG, "ComplexMethod.hs");
    let cc = function_metric(&debug, "processOrder", "cc").unwrap_or(0);
    assert!(cc >= t().function.cc_warning, "cc should be >= {}, got: {}", t().function.cc_warning, cc);
}

#[test]
fn excess_args_detected() {
    let output = run_check(LANG, "ExcessArgs.hs");
    assert!(has_smell(&output, "Excess Arguments"), "got: {output}");
    assert!(has_function(&output, "createUser"));
}

#[test]
fn excess_args_count_correct() {
    let debug = run_debug(LANG, "ExcessArgs.hs");
    let args = function_metric(&debug, "createUser", "args").unwrap_or(0);
    assert!(args > t().function.arg_max, "got: {args}");
}

#[test]
fn simple_func_not_flagged() {
    let output = run_check(LANG, "ExcessArgs.hs");
    assert!(!has_function(&output, "simpleFunc"));
}

#[test]
fn deep_nesting_detected() {
    let output = run_check(LANG, "DeepNesting.hs");
    assert!(has_smell(&output, "Deep Nested"), "got: {output}");
    assert!(has_function(&output, "deeplyNested"));
}

#[test]
fn deep_nesting_depth_exceeds_4() {
    let debug = run_debug(LANG, "DeepNesting.hs");
    let depth = function_metric(&debug, "deeplyNested", "nesting").unwrap_or(0);
    assert!(depth > t().function.nesting_depth, "got: {depth}");
}

#[test]
fn moderate_nesting_not_flagged() {
    let output = run_check(LANG, "DeepNesting.hs");
    assert!(!has_function(&output, "moderatelyNested"));
}

#[test]
fn cc_base_case_is_1() {
    let debug = run_debug(LANG, "Clean.hs");
    let cc = function_metric(&debug, "add", "cc").unwrap_or(0);
    assert_eq!(cc, 1, "got: {cc}");
}

#[test]
fn output_starts_with_pulse() {
    let output = run_check(LANG, "ComplexMethod.hs");
    assert!(output.starts_with("pulse:"), "got: {output}");
}

#[test]
fn output_has_function_line_numbers() {
    let output = run_check(LANG, "ComplexMethod.hs");
    assert!(output.contains("(L"), "got: {output}");
}

#[test]
fn issue_count_matches_findings() {
    let output = run_check(LANG, "ComplexMethod.hs");
    let first = output.lines().next().unwrap_or("");
    let findings = output.lines().filter(|l| l.starts_with("  ")).count();
    assert!(first.contains(&format!("{findings} issue")));
}

#[test]
fn hook_clean_file_silent() {
    let path = fixtures_dir(LANG).join("Clean.hs");
    let output = run_hook(path.to_str().unwrap());
    assert!(output.is_empty(), "got: {output}");
}

#[test]
fn hook_smelly_file_produces_output() {
    let path = fixtures_dir(LANG).join("ComplexMethod.hs");
    let output = run_hook(path.to_str().unwrap());
    assert!(!output.is_empty(), "expected output for smelly file");
}

#[test]
fn hook_nonexistent_file_silent() {
    let output = run_hook("/tmp/nonexistent_haskell_file.hs");
    assert!(output.is_empty(), "got: {output}");
}

#[test]
fn empty_file() {
    let output = pulse_check_code("", "hs");
    assert!(output.is_empty(), "got: {output}");
}

#[test]
fn function_at_cc_boundary_flagged() {
    // Build case with cc_warning non-wildcard alternatives
    let mut code = String::from("f :: Int -> String\nf x = case x of\n");
    for i in 0..t().function.cc_warning {
        code.push_str(&format!("  {i} -> \"{i}\"\n"));
    }
    code.push_str("  _ -> \"z\"\n");
    let output = pulse_check_code(&code, "hs");
    assert!(has_smell(&output, "Complex Method"), "got: {output}");
}

#[test]
fn function_below_cc_boundary_not_flagged() {
    let mut code = String::from("f :: Int -> String\nf x = case x of\n");
    for i in 0..t().function.cc_warning - 2 {
        code.push_str(&format!("  {i} -> \"{i}\"\n"));
    }
    code.push_str("  _ -> \"z\"\n");
    let output = pulse_check_code(&code, "hs");
    assert!(!has_smell(&output, "Complex Method"), "got: {output}");
}

#[test]
fn large_method_detected() {
    let mut code = String::from("f :: Int -> Int\nf x =\n  let\n");
    for i in 0..fn_padding() {
        code.push_str(&format!("    x{i} = {i}\n"));
    }
    code.push_str("  in x\n");
    let output = pulse_check_code(&code, "hs");
    assert!(has_smell(&output, "Large Method"), "got: {output}");
}

#[test]
fn large_method_loc_at_least_threshold() {
    let mut code = String::from("f :: Int -> Int\nf x =\n  let\n");
    for i in 0..fn_padding() {
        code.push_str(&format!("    x{i} = {i}\n"));
    }
    code.push_str("  in x\n");
    let debug = pulse_debug_code(&code, "hs");
    let loc = function_metric(&debug, "f", "loc").unwrap_or(0);
    assert!(loc >= t().function.fn_loc_warning, "got: {loc}");
}

#[test]
fn god_method_detected() {
    let mut code = String::from("f :: Int -> Int\nf x =\n");
    for _ in 0..50 {
        code.push_str("  if x > 0 then x else\n");
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("  x{i} = {i}\n"));
    }
    code.push_str("  x\n");
    let output = pulse_check_code(&code, "hs");
    assert!(has_smell(&output, "God Method"), "got: {output}");
}

#[test]
fn god_method_not_reported_as_separate() {
    let mut code = String::from("f :: Int -> Int\nf x =\n");
    for _ in 0..50 {
        code.push_str("  if x > 0 then x else\n");
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("  x{i} = {i}\n"));
    }
    code.push_str("  x\n");
    let output = pulse_check_code(&code, "hs");
    assert!(!has_smell(&output, "Complex Method"), "got: {output}");
}

#[test]
fn complex_conditional_detected() {
    let output = pulse_check_code(
        "f :: Bool -> Bool -> Bool -> Bool\nf a b c =\n  if a && b || c then True\n  else if a || b && c then True\n  else if a && b && c then True\n  else False\n",
        "hs",
    );
    assert!(has_smell(&output, "Complex Conditional"), "got: {output}");
}

#[test]
fn file_too_large_detected() {
    let mut code = String::new();
    for i in 0..file_padding() {
        code.push_str(&format!("x{i} = {i}\n"));
    }
    let output = pulse_check_code(&code, "hs");
    assert!(has_smell(&output, "File Too Large"), "got: {output}");
}

#[test]
fn hook_invalid_json_silent() {
    let binary = env!("CARGO_BIN_EXE_pulse");
    let out = Command::new(binary)
        .arg("--hook")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            if let Some(ref mut stdin) = child.stdin {
                let _ = stdin.write_all(b"not json");
            }
            child.wait_with_output()
        })
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.is_empty(), "got: {stdout}");
}

#[test]
fn boolean_operators_increment_cc() {
    let debug = pulse_debug_code(
        "f :: Bool -> Bool -> Bool -> Bool\nf a b c = if a && b && c then True else False\n",
        "hs",
    );
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert!(cc >= 4, "cc should be >= 4 (1+if+2&&), got: {cc}");
}

#[test]
fn output_has_module_prefix() {
    let count = t().module.file_function_count + 5;
    let mut code = String::new();
    for i in 0..count {
        code.push_str(&format!("f{i} :: Int -> Int\nf{i} x = x\n"));
    }
    let output = pulse_check_code(&code, "hs");
    assert!(has_smell(&output, "Too Many Functions"), "got: {output}");
}

#[test]
fn comments_only_file() {
    let output = pulse_check_code("-- just a comment\n{- block comment -}\n", "hs");
    assert!(output.is_empty(), "got: {output}");
}

#[test]
fn hook_unsupported_extension_silent() {
    let output = run_hook("/tmp/file.xyz");
    assert!(output.is_empty(), "got: {output}");
}

#[test]
fn analysis_completes_under_500ms() {
    let count = t().module.file_function_count + 5;
    let mut code = String::new();
    for i in 0..count {
        code.push_str(&format!("f{i} :: Int -> Int\nf{i} x =\n"));
        for j in 0..20 {
            code.push_str(&format!("  x{j} = {j}\n"));
        }
        code.push_str("  x\n\n");
    }
    let start = std::time::Instant::now();
    let _ = pulse_check_code(&code, "hs");
    let elapsed = start.elapsed();
    assert!(elapsed.as_millis() < 2000, "took {}ms", elapsed.as_millis());
}

#[test]
fn embedded_block_detected() {
    let lines = t().function.embedded_block_loc + 5;
    let mut code = String::from("f :: String\nf = \"");
    for i in 0..lines {
        code.push_str(&format!("line {i}\\n\\\n\\"));
    }
    code.push_str("\"\n");
    let output = pulse_check_code(&code, "hs");
    assert!(has_smell(&output, "Large Embedded Block"), "got: {output}");
}

#[test]
fn simple_string_not_flagged() {
    let output = pulse_check_code("f :: String\nf = \"hello\"\n", "hs");
    assert!(!has_smell(&output, "Large Embedded Block"), "got: {output}");
}

#[test]
fn case_expression_increments_cc() {
    let output = run_check(LANG, "CaseExpression.hs");
    assert!(has_smell(&output, "Complex Method"), "got: {output}");
}

#[test]
fn case_expression_cc_value() {
    let debug = run_debug(LANG, "CaseExpression.hs");
    let cc = function_metric(&debug, "dispatch", "cc").unwrap_or(0);
    assert!(cc > t().function.cc_warning, "cc should be > cc_warning, got: {cc}");
}

#[test]
fn code_duplication_detected() {
    let loc = t().analysis.duplication_min_loc + 2;
    let mut a_body = String::new();
    let mut b_body = String::new();
    for i in 0..loc {
        a_body.push_str(&format!("      v{i} = {i}\n"));
        b_body.push_str(&format!("      v{i} = {i}\n"));
    }
    let code = format!(
        "a :: Int -> Int\na x =\n  let\n{a_body}  in x\n\nb :: Int -> Int\nb x =\n  let\n{b_body}  in x\n"
    );
    let output = pulse_check_code(&code, "hs");
    assert!(has_smell(&output, "Code Duplication"), "got: {output}");
}

#[test]
fn primitive_obsession_recognizes_haskell_types() {
    let output = pulse_check_code(
        "f :: Int -> Float -> Double -> Bool -> Char -> Int\nf a b c d e = 0\n",
        "hs",
    );
    assert!(has_smell(&output, "Primitive Obsession"), "got: {output}");
}

#[test]
fn primitive_obsession_mixed_not_flagged() {
    let output = pulse_check_code(
        "f :: Int -> Float -> [String] -> Maybe Int -> Int\nf a b c d = 0\n",
        "hs",
    );
    assert!(!has_smell(&output, "Primitive Obsession"), "got: {output}");
}

#[test]
fn nested_conditional_chunks_detected() {
    let output = pulse_check_code(
        "f :: Int -> Int -> Int -> Int\nf x y z =\n  case x of\n    0 -> if y > 0 then if z > 0 then 1 else 0 else 0\n    1 -> if y > 1 then if z > 1 then 2 else 0 else 0\n    _ -> if y > 2 then if z > 2 then 3 else 0 else 0\n",
        "hs",
    );
    assert!(has_smell(&output, "Nested Conditional"), "got: {output}");
}

#[test]
fn declarations_above_threshold() {
    let count = t().module.max_declarations + 5;
    let mut code = String::new();
    for i in 0..count {
        code.push_str(&format!("data T{i} = T{i}\n"));
    }
    let output = pulse_check_code(&code, "hs");
    assert!(has_smell(&output, "Excessive Declarations"), "got: {output}");
}

#[test]
fn overall_function_size_below_threshold() {
    let fn_count = t().module.large_fn_count - 1;
    let mut code = String::new();
    for i in 0..fn_count {
        code.push_str(&format!("f{i} :: Int -> Int\nf{i} x =\n"));
        for j in 0..(t().module.large_fn_loc + 5) {
            code.push_str(&format!("  x{j} = {j}\n"));
        }
        code.push_str("  x\n\n");
    }
    let output = pulse_check_code(&code, "hs");
    assert!(!has_smell(&output, "Overall Function Size"), "got: {output}");
}

#[test]
fn overall_function_size_at_threshold() {
    let fn_count = t().module.large_fn_count;
    let mut code = String::new();
    for i in 0..fn_count {
        code.push_str(&format!("f{i} :: Int -> Int\nf{i} x =\n"));
        for j in 0..(t().module.large_fn_loc + 5) {
            code.push_str(&format!("  x{j} = {j}\n"));
        }
        code.push_str("  x\n\n");
    }
    let output = pulse_check_code(&code, "hs");
    assert!(has_smell(&output, "Overall Function Size"), "got: {output}");
}

#[test]
fn god_class_requires_god_method() {
    let count = t().module.file_function_count + 5;
    let mut code = String::new();
    for i in 0..count {
        code.push_str(&format!("f{i} :: Int -> Int\nf{i} x =\n"));
        for j in 0..20 {
            code.push_str(&format!("  x{j} = {j}\n"));
        }
        code.push_str("  x\n\n");
    }
    let output = pulse_check_code(&code, "hs");
    assert!(!has_smell(&output, "God Class"), "got: {output}");
}

#[test]
fn god_class_triggers_with_god_method() {
    let mut code = String::from("god :: Int -> Int\ngod x =\n");
    for _ in 0..50 {
        code.push_str("  if x > 0 then x else\n");
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("  x{i} = {i}\n"));
    }
    code.push_str("  x\n\n");
    let remaining = t().module.file_function_count;
    for i in 1..=remaining {
        code.push_str(&format!("f{i} :: Int -> Int\nf{i} x =\n"));
        for j in 0..20 {
            code.push_str(&format!("  x{j} = {j}\n"));
        }
        code.push_str("  x\n\n");
    }
    let output = pulse_check_code(&code, "hs");
    assert!(has_smell(&output, "God Class"), "got: {output}");
}

#[test]
fn guards_produce_cc() {
    let debug = pulse_debug_code(
        "f :: Int -> String\nf x\n  | x < 0 = \"neg\"\n  | x == 0 = \"zero\"\n  | x > 100 = \"big\"\n  | otherwise = \"ok\"\n",
        "hs",
    );
    let cc = function_metric(&debug, "f", "cc").unwrap_or(0);
    assert!(cc >= 4, "expected cc >= 4, got: {cc}");
}

#[test]
fn haskell_features_type_class() {
    let debug = run_debug(LANG, "HaskellFeatures.hs");
    assert!(debug.contains("Describable.describe"), "expected type class method, got: {debug}");
}

#[test]
fn haskell_features_instance() {
    let debug = run_debug(LANG, "HaskellFeatures.hs");
    assert!(debug.contains("Describable.describe"), "expected instance method, got: {debug}");
}

#[test]
fn haskell_features_guards() {
    let debug = run_debug(LANG, "HaskellFeatures.hs");
    let cc = function_metric(&debug, "classify", "cc").unwrap_or(0);
    assert!(cc >= 3, "expected cc >= 3 for guards, got: {cc}");
}

#[test]
fn haskell_features_where_clause() {
    let debug = run_debug(LANG, "HaskellFeatures.hs");
    assert!(debug.contains("withWhere.doubled"), "expected where func, got: {debug}");
    assert!(debug.contains("withWhere.tripled"), "expected where func, got: {debug}");
}

#[test]
fn haskell_features_let_in() {
    let debug = run_debug(LANG, "HaskellFeatures.hs");
    let cc = function_metric(&debug, "withLet", "cc").unwrap_or(0);
    assert_eq!(cc, 1, "let-in should not add CC, got: {cc}");
}

#[test]
fn haskell_features_do_notation() {
    let debug = run_debug(LANG, "HaskellFeatures.hs");
    let cc = function_metric(&debug, "doExample", "cc").unwrap_or(0);
    assert_eq!(cc, 1, "do-notation should not add CC, got: {cc}");
}

// ── Production fixture tests ──────────────────────────────────────────

#[test]
fn production_api_complex_method() {
    let output = run_check(LANG, "ProductionApiService.hs");
    assert!(has_smell(&output, "Complex Method"), "got: {output}");
    assert!(has_function(&output, "processPayment"));
}

#[test]
fn production_api_process_payment_cc() {
    let debug = run_debug(LANG, "ProductionApiService.hs");
    let cc = function_metric(&debug, "processPayment", "cc").unwrap_or(0);
    assert!(cc >= t().function.cc_warning, "expected cc >= {}, got: {}", t().function.cc_warning, cc);
}

#[test]
fn production_api_guard_functions_detected() {
    let debug = run_debug(LANG, "ProductionApiService.hs");
    let cc = function_metric(&debug, "getPaymentStatus", "cc").unwrap_or(0);
    assert!(cc >= 5, "expected guards to produce cc >= 5, got: {cc}");
}

#[test]
fn production_api_helper_functions_clean() {
    let output = run_check(LANG, "ProductionApiService.hs");
    assert!(!has_function(&output, "ccTxId"));
    assert!(!has_function(&output, "btTxId"));
}

#[test]
fn production_api_refund_guard_cc() {
    let debug = run_debug(LANG, "ProductionApiService.hs");
    let cc = function_metric(&debug, "refund", "cc").unwrap_or(0);
    assert!(cc >= 3, "expected cc >= 3 for guarded refund, got: {cc}");
}

#[test]
fn production_pipeline_process_event_cc() {
    let debug = run_debug(LANG, "ProductionDataPipeline.hs");
    let cc = function_metric(&debug, "processEvent", "cc").unwrap_or(0);
    assert!(cc >= 7, "expected cc >= 7 for processEvent, got: {cc}");
}

#[test]
fn production_pipeline_where_functions() {
    let debug = run_debug(LANG, "ProductionDataPipeline.hs");
    assert!(debug.contains("validateConfig.batchErr"), "expected where func, got: {debug}");
    assert!(debug.contains("validateConfig.retryErr"), "expected where func, got: {debug}");
    assert!(debug.contains("validateConfig.timeoutErr"), "expected where func, got: {debug}");
}

#[test]
fn production_pipeline_dispatch_command() {
    let debug = run_debug(LANG, "ProductionDataPipeline.hs");
    let cc = function_metric(&debug, "dispatchCommand", "cc").unwrap_or(0);
    assert!(cc >= 7, "expected cc >= 7 for case dispatch, got: {cc}");
}

#[test]
fn production_pipeline_format_output_case() {
    let debug = run_debug(LANG, "ProductionDataPipeline.hs");
    let cc = function_metric(&debug, "formatOutput", "cc").unwrap_or(0);
    assert!(cc >= 4, "expected cc >= 4 for format cases, got: {cc}");
}

#[test]
fn production_pipeline_classify_where_lambda() {
    let debug = run_debug(LANG, "ProductionDataPipeline.hs");
    assert!(debug.contains("classifyEvents.classify"), "expected where func classify, got: {debug}");
}

#[test]
fn production_pipeline_declarations_counted() {
    let debug = run_debug(LANG, "ProductionDataPipeline.hs");
    assert!(debug.contains("declarations=3"), "expected 3 declarations, got: {debug}");
}
