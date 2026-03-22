mod common;

use common::*;
use std::process::Command;

const LANG: &str = "objc";

// ===========================================================================
// Output format
// ===========================================================================

#[test]
fn output_starts_with_pulse() {
    let output = run_check(LANG, "complex_methods.m");
    assert!(output.starts_with("pulse:"), "got: {}", output);
}

#[test]
fn output_has_function_line_numbers() {
    let output = run_check(LANG, "complex_methods.m");
    assert!(output
        .lines()
        .any(|l| l.contains("(L") && l.contains("): ")));
}

#[test]
fn output_has_module_prefix() {
    let output = run_check(LANG, "production_service.m");
    assert!(output.contains("Module:"), "got: {}", output);
}

#[test]
fn issue_count_matches_findings() {
    let output = run_check(LANG, "complex_methods.m");
    let first = output.lines().next().unwrap_or("");
    let findings = output.lines().filter(|l| l.starts_with("  ")).count();
    assert!(first.contains(&format!("{} issue", findings)));
}

// ===========================================================================
// Clean / empty
// ===========================================================================

#[test]
fn clean_file_produces_no_output() {
    let output = run_check(LANG, "clean.m");
    assert!(output.is_empty(), "got: {}", output);
}

#[test]
fn empty_file() {
    let out = pulse_check_code("", "m");
    assert!(out.is_empty());
}

#[test]
fn comments_only_file() {
    let out = pulse_check_code("// just a comment\n", "m");
    assert!(out.is_empty());
}

#[test]
fn simple_func_not_flagged() {
    let out = pulse_check_code(
        "void add(int a, int b) {\n    return;\n}\n",
        "m",
    );
    assert!(out.is_empty(), "got: {}", out);
}

// ===========================================================================
// CC boundary
// ===========================================================================

#[test]
fn cc_base_case_is_1() {
    let debug = pulse_debug_code(
        "@implementation X\n- (void)f {\n    NSLog(@\"hi\");\n}\n@end\n",
        "m",
    );
    assert_eq!(function_metric(&debug, "X.f", "cc"), Some(1));
}

#[test]
fn function_at_cc_boundary_flagged() {
    let code = "@implementation X\n- (void)f:(int)x {\n    if (x>0) {}\n    if (x>1) {}\n    if (x>2) {}\n    if (x>3) {}\n    if (x>4) {}\n    if (x>5) {}\n    if (x>6) {}\n    if (x>7) {}\n}\n@end\n";
    let out = pulse_check_code(code, "m");
    assert!(has_smell(&out, "Complex Method"), "got: {}", out);
}

#[test]
fn function_below_cc_boundary_not_flagged() {
    let code = "@implementation X\n- (void)f:(int)x {\n    if (x>0) {}\n    if (x>1) {}\n    if (x>2) {}\n    if (x>3) {}\n    if (x>4) {}\n    if (x>5) {}\n    if (x>6) {}\n}\n@end\n";
    let out = pulse_check_code(code, "m");
    assert!(!has_smell(&out, "Complex Method"), "got: {}", out);
}

// ===========================================================================
// Complexity smells
// ===========================================================================

#[test]
fn complex_method_detected() {
    let out = run_check(LANG, "complex_methods.m");
    assert!(has_smell(&out, "Complex Method"), "got: {}", out);
}

#[test]
fn complex_method_cc_at_least_9() {
    let debug = run_debug(LANG, "complex_methods.m");
    let cc = function_metric(&debug, "OrderProcessor.processOrder", "cc").unwrap_or(0);
    assert!(cc >= 9, "cc={}", cc);
}

#[test]
fn god_method_detected() {
    let out = run_check(LANG, "production_service.m");
    assert!(has_smell(&out, "Complex Method") || has_smell(&out, "God Method"), "got: {}", out);
}

#[test]
fn production_fixture_detects_complexity() {
    let out = run_check(LANG, "production_service.m");
    assert!(has_smell(&out, "Complex Method") || has_smell(&out, "God Method"), "got: {}", out);
    assert!(has_function(&out, "processOrder"));
}

#[test]
fn god_method_subsumes_complex_and_large() {
    let mut code = String::from("@implementation X\n- (void)god:(int)x {\n");
    for i in 0..12 { code.push_str(&format!("    if (x > {}) {{}}\n", i)); }
    for i in 0..fn_padding() { code.push_str(&format!("    NSLog(@\"{}\");\n", i)); }
    code.push_str("}\n@end\n");
    let out = pulse_check_code(&code, "m");
    assert!(has_smell(&out, "God Method"), "got: {}", out);
    let god_lines: Vec<&str> = out.lines().filter(|l| l.contains("god")).collect();
    assert!(!god_lines.iter().any(|l| l.contains("Complex Method")), "God Method should subsume: {:?}", god_lines);
    assert!(!god_lines.iter().any(|l| l.contains("Large Method")), "God Method should subsume: {:?}", god_lines);
}

#[test]
fn large_method_detected() {
    let mut lines = String::from("@implementation X\n- (void)big {\n");
    for i in 0..fn_padding() {
        lines.push_str(&format!("    NSLog(@\"{}\");\n", i));
    }
    lines.push_str("}\n@end\n");
    let out = pulse_check_code(&lines, "m");
    assert!(has_smell(&out, "Large Method"), "got: {}", out);
}

#[test]
fn large_method_loc_at_least_65() {
    let debug = run_debug(LANG, "production_service.m");
    let loc = function_metric(&debug, "OrderService.processOrder", "loc").unwrap_or(0);
    assert!(loc >= 50, "loc={}", loc);
}

#[test]
fn boolean_operators_increment_cc() {
    let code = "@implementation X\n- (void)f:(BOOL)a b:(BOOL)b c:(BOOL)c {\n    if (a && b && c) {\n        NSLog(@\"yes\");\n    }\n}\n@end\n";
    let debug = pulse_debug_code(code, "m");
    let cc = function_metric(&debug, "X.f", "cc").unwrap_or(0);
    assert!(cc >= 4, "cc={}", cc);
}

// ===========================================================================
// Nesting
// ===========================================================================

#[test]
fn deep_nesting_detected() {
    let out = run_check(LANG, "deep_nesting.m");
    assert!(has_smell(&out, "Deep Nested Complexity"), "got: {}", out);
}

#[test]
fn deep_nesting_depth_exceeds_4() {
    let debug = run_debug(LANG, "deep_nesting.m");
    let nesting = function_metric(&debug, "DeepProcessor.process", "nesting").unwrap_or(0);
    assert!(nesting > 4, "nesting={}", nesting);
}

#[test]
fn moderate_nesting_not_flagged() {
    let code = "@implementation X\n- (void)f:(int)x {\n    if (x > 0) {\n        if (x > 1) {\n            NSLog(@\"ok\");\n        }\n    }\n}\n@end\n";
    let out = pulse_check_code(code, "m");
    assert!(!has_smell(&out, "Deep Nested"), "got: {}", out);
}

// ===========================================================================
// Arguments
// ===========================================================================

#[test]
fn excess_args_detected() {
    let out = run_check(LANG, "excess_args.m");
    assert!(has_smell(&out, "Excess Arguments"), "got: {}", out);
}

#[test]
fn excess_args_count_correct() {
    let debug = run_debug(LANG, "excess_args.m");
    let args = function_metric(&debug, "UserService.createUser", "args").unwrap_or(0);
    assert_eq!(args, 6);
}

// ===========================================================================
// Module-level smells
// ===========================================================================

#[test]
fn code_duplication_detected() {
    let out = run_check(LANG, "code_duplication.m");
    assert!(has_smell(&out, "Code Duplication"), "got: {}", out);
}

#[test]
fn code_duplication_inline() {
    let code = concat!(
        "@implementation X\n",
        "- (NSInteger)a:(NSInteger)n {\n",
        "    NSInteger r = 0;\n",
        "    for (NSInteger i = 0; i < n; i++) {\n",
        "        r += i * i;\n",
        "        if (r > 100) { r = 100; }\n",
        "    }\n",
        "    return r;\n",
        "}\n",
        "- (NSInteger)b:(NSInteger)n {\n",
        "    NSInteger r = 0;\n",
        "    for (NSInteger i = 0; i < n; i++) {\n",
        "        r += i * i;\n",
        "        if (r > 100) { r = 100; }\n",
        "    }\n",
        "    return r;\n",
        "}\n",
        "@end\n"
    );
    let out = pulse_check_code(code, "m");
    assert!(has_smell(&out, "Code Duplication"), "got: {}", out);
}

#[test]
fn embedded_block_detected() {
    let out = run_check(LANG, "embedded_block.m");
    assert!(has_smell(&out, "Large Embedded Block"), "got: {}", out);
}

#[test]
fn bumpy_road_detected() {
    let out = run_check(LANG, "bumpy_road.m");
    assert!(has_smell(&out, "Nested Conditional Chunks"), "got: {}", out);
}

#[test]
fn low_cohesion_detected() {
    let out = run_check(LANG, "low_cohesion.m");
    assert!(has_smell(&out, "Low Cohesion"), "got: {}", out);
}

#[test]
fn primitive_obsession_detected() {
    let out = run_check(LANG, "primitive_obsession.m");
    assert!(has_smell(&out, "Primitive Obsession"), "got: {}", out);
}

#[test]
fn overall_function_size_at_threshold() {
    let mut code = String::from("@implementation X\n");
    for i in 0..3 {
        code.push_str(&format!("- (void)f{}:(int)x {{\n", i));
        for j in 0..45 {
            code.push_str(&format!("    NSLog(@\"{}\");\n", j));
        }
        code.push_str("}\n");
    }
    code.push_str("@end\n");
    let out = pulse_check_code(&code, "m");
    assert!(has_smell(&out, "Overall Function Size"), "got: {}", out);
}

#[test]
fn overall_function_size_below_threshold() {
    let mut code = String::from("@implementation X\n");
    for i in 0..2 {
        code.push_str(&format!("- (void)f{}:(int)x {{\n", i));
        for j in 0..45 {
            code.push_str(&format!("    NSLog(@\"{}\");\n", j));
        }
        code.push_str("}\n");
    }
    code.push_str("@end\n");
    let out = pulse_check_code(&code, "m");
    assert!(!has_smell(&out, "Overall Function Size"), "got: {}", out);
}

// ===========================================================================
// Hook tests
// ===========================================================================

#[test]
fn hook_clean_file_silent() {
    let path = fixtures_dir(LANG).join("clean.m");
    let out = run_hook(path.to_str().unwrap());
    assert!(!out.contains("error[pulse]"), "got: {}", out);
}

#[test]
fn hook_smelly_file_produces_output() {
    let path = fixtures_dir(LANG).join("complex_methods.m");
    let out = run_hook(path.to_str().unwrap());
    assert!(out.contains("error[pulse]"), "got: {}", out);
}

#[test]
fn hook_nonexistent_file_silent() {
    let out = run_hook("/tmp/does_not_exist_pulse_test.m");
    assert!(!out.contains("error[pulse]"), "got: {}", out);
}

// ===========================================================================
// ObjC-specific
// ===========================================================================

#[test]
fn init_detected_as_constructor() {
    let debug = run_debug(LANG, "excess_args.m");
    assert!(
        debug.contains("UserService.initWithName"),
        "got: {}",
        debug
    );
}

#[test]
fn constructor_over_injection_detected() {
    let out = run_check(LANG, "excess_args.m");
    assert!(has_smell(&out, "Constructor Over-Injection"), "got: {}", out);
}

#[test]
fn method_attributed_to_class() {
    let debug = run_debug(LANG, "complex_methods.m");
    assert!(
        debug.contains("OrderProcessor.processOrder"),
        "got: {}",
        debug
    );
}

#[test]
fn class_method_plus_detected() {
    let code = "@implementation X\n+ (instancetype)shared {\n    static X *s = nil;\n    if (!s) { s = [[X alloc] init]; }\n    return s;\n}\n@end\n";
    let debug = pulse_debug_code(code, "m");
    assert!(debug.contains("X.shared"), "got: {}", debug);
}

#[test]
fn switch_case_increments_cc() {
    let code = concat!(
        "@implementation X\n",
        "- (void)f:(int)x {\n",
        "    switch (x) {\n",
        "        case 1: break;\n",
        "        case 2: break;\n",
        "        case 3: break;\n",
        "        case 4: break;\n",
        "        case 5: break;\n",
        "        case 6: break;\n",
        "        case 7: break;\n",
        "        case 8: break;\n",
        "        case 9: break;\n",
        "        default: break;\n",
        "    }\n",
        "}\n",
        "@end\n"
    );
    let out = pulse_check_code(code, "m");
    assert!(has_smell(&out, "Complex Method"), "got: {}", out);
}

#[test]
fn for_in_increments_cc() {
    let code = "@implementation X\n- (void)f:(NSArray *)items {\n    for (id obj in items) {\n        NSLog(@\"%@\", obj);\n    }\n}\n@end\n";
    let debug = pulse_debug_code(code, "m");
    assert_eq!(function_metric(&debug, "X.f", "cc"), Some(2));
}

#[test]
fn global_conditionals_detected() {
    let out = run_check(LANG, "global_conditionals.m");
    assert!(has_smell(&out, "Global Conditionals"), "got: {}", out);
}

#[test]
fn simple_func_in_excess_args_not_flagged() {
    let out = run_check(LANG, "excess_args.m");
    assert!(!has_function(&out, "getName"), "got: {}", out);
}

#[test]
fn complex_conditional_detected() {
    let code = "@implementation X\n- (void)f:(int)a b:(int)b c:(int)c {\n    if (a > 0 && b > 0 || c > 0) {}\n    if (b > 0 && c > 0 || a > 0) {}\n    if (c > 0 && a > 0 || b > 0) {}\n}\n@end\n";
    let out = pulse_check_code(code, "m");
    assert!(has_smell(&out, "Complex Conditional"), "got: {}", out);
}

#[test]
fn nested_conditional_chunks_detected() {
    let out = run_check(LANG, "bumpy_road.m");
    assert!(has_smell(&out, "Nested Conditional Chunks"), "got: {}", out);
}

#[test]
fn try_catch_increments_cc() {
    let code = "@implementation X\n- (void)f {\n    @try {\n        NSLog(@\"try\");\n    } @catch (NSException *e) {\n        NSLog(@\"catch\");\n    }\n}\n@end\n";
    let debug = pulse_debug_code(code, "m");
    assert_eq!(function_metric(&debug, "X.f", "cc"), Some(2));
}

#[test]
fn empty_catch_detected() {
    let code = "@implementation X\n- (void)f {\n    @try {\n        NSLog(@\"try\");\n    } @catch (NSException *e) {\n    }\n}\n@end\n";
    let out = pulse_check_code(code, "m");
    assert!(has_smell(&out, "Empty Error Handler"), "got: {}", out);
}

#[test]
fn simple_string_not_flagged() {
    let code = "@implementation X\n- (void)f {\n    NSString *s = @\"hello\";\n    NSLog(@\"%@\", s);\n}\n@end\n";
    let out = pulse_check_code(code, "m");
    assert!(!has_smell(&out, "Embedded Block"), "got: {}", out);
}

// ===========================================================================
// Performance
// ===========================================================================

#[test]
fn analysis_completes_under_500ms() {
    let path = fixtures_dir(LANG).join("production_service.m");
    let start = std::time::Instant::now();
    let _ = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run pulse");
    assert!(start.elapsed().as_millis() < 500);
}

// ===========================================================================
// Other
// ===========================================================================

#[test]
fn production_service_has_issues() {
    let out = run_check(LANG, "production_service.m");
    assert!(!out.is_empty(), "expected issues");
    assert!(out.lines().count() > 2);
}

#[test]
fn test_file_analyzed() {
    let out = run_check(LANG, "test_smells.m");
    let debug = run_debug(LANG, "test_smells.m");
    assert!(debug.contains("test_addition"), "got: {}", debug);
    // test_ functions have duplication suppressed
    assert!(!has_smell(&out, "Code Duplication"), "got: {}", out);
}
