mod common;

use common::*;

// ===========================================================================
// Python production_service.py — comprehensive smell validation
// ===========================================================================

#[test]
fn py_production_constructor_over_injection() {
    let output = run_check("python", "production_service.py");
    assert!(has_smell(&output, "Constructor Over-Injection"));
    assert!(has_function(&output, "UserService.__init__"));
}

#[test]
fn py_production_god_method() {
    let output = run_check("python", "production_service.py");
    assert!(has_smell(&output, "Complex Method") || has_smell(&output, "God Method"));
    assert!(has_function(&output, "process_order"));
}

#[test]
fn py_production_god_method_has_high_cc_and_loc() {
    let debug = run_debug("python", "production_service.py");
    let cc = function_metric(&debug, "process_order", "cc").unwrap_or(0);
    let loc = function_metric(&debug, "process_order", "loc").unwrap_or(0);
    assert!(cc >= 9, "cc should be >= 9, got: {}", cc);
    assert!(loc >= 50, "loc should be >= 50, got: {}", loc);
}

#[test]
fn py_production_deep_nesting() {
    let output = run_check("python", "production_service.py");
    assert!(has_smell(&output, "Deep Nested"));
    assert!(has_function(&output, "process_order"));
}

#[test]
fn py_production_nested_chunks() {
    let output = run_check("python", "production_service.py");
    assert!(has_smell(&output, "Nested Conditional Chunks"));
}

#[test]
fn py_production_excess_args() {
    let output = run_check("python", "production_service.py");
    assert!(has_smell(&output, "Excess Arguments"));
    assert!(has_function(&output, "process_order"));
}

#[test]
fn py_production_exact_duplication() {
    let output = run_check("python", "production_service.py");
    assert!(has_function(&output, "generate_invoice"));
    assert!(has_function(&output, "generate_receipt"));
}

#[test]
fn py_production_report_duplication() {
    let output = run_check("python", "production_service.py");
    assert!(has_function(&output, "format_user_report"));
    assert!(has_function(&output, "format_admin_report"));
}

#[test]
fn py_production_low_cohesion() {
    let output = run_check("python", "production_service.py");
    assert!(has_smell(&output, "Low Cohesion"));
    assert!(output.contains("UserService"));
}

#[test]
fn py_production_primitive_obsession() {
    let output = run_check("python", "production_service.py");
    assert!(has_smell(&output, "Primitive Obsession"));
}

#[test]
fn py_production_clean_functions_not_flagged_individually() {
    let output = run_check("python", "production_service.py");
    // get_user has no individual smell (it may appear in duplication groups, which is fine)
    let function_lines: Vec<&str> = output
        .lines()
        .filter(|l| l.starts_with("  ") && !l.contains("Module:") && l.contains("get_user"))
        .collect();
    assert!(
        function_lines.is_empty(),
        "get_user should have no individual smells, got: {:?}",
        function_lines
    );
}

// ===========================================================================
// TypeScript production_api.ts — comprehensive smell validation
// ===========================================================================

#[test]
fn ts_production_constructor_over_injection() {
    let output = run_check("typescript", "production_api.ts");
    assert!(has_smell(&output, "Constructor Over-Injection"));
    assert!(has_function(&output, "ApiService.constructor"));
}

#[test]
fn ts_production_complex_method() {
    let output = run_check("typescript", "production_api.ts");
    assert!(has_smell(&output, "Complex Method"));
    assert!(has_function(&output, "handleRequest"));
}

#[test]
fn ts_production_excess_args() {
    let output = run_check("typescript", "production_api.ts");
    assert!(has_smell(&output, "Excess Arguments"));
    assert!(has_function(&output, "handleRequest"));
}

#[test]
fn ts_production_nested_chunks() {
    let output = run_check("typescript", "production_api.ts");
    assert!(has_smell(&output, "Nested Conditional Chunks"));
}

#[test]
fn ts_production_exact_duplication() {
    let output = run_check("typescript", "production_api.ts");
    assert!(has_function(&output, "formatUserReport"));
    assert!(has_function(&output, "formatAdminReport"));
}

#[test]
fn ts_production_primitive_obsession() {
    let output = run_check("typescript", "production_api.ts");
    assert!(has_smell(&output, "Primitive Obsession"));
}

#[test]
fn ts_production_clean_helpers_not_flagged() {
    let output = run_check("typescript", "production_api.ts");
    assert!(!has_function(&output, "getUser"));
    assert!(!has_function(&output, "searchUsers"));
    assert!(!has_function(&output, "deleteUser"));
}
