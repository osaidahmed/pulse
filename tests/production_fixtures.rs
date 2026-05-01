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
    assert!(cc >= 9, "cc should be >= 9, got: {cc}");
    assert!(loc >= 50, "loc should be >= 50, got: {loc}");
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
        "get_user should have no individual smells, got: {function_lines:?}"
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

// ===========================================================================
// Lua production_service.lua — comprehensive smell validation
// ===========================================================================

#[test]
fn lua_production_constructor_over_injection() {
    let output = run_check("lua", "production_service.lua");
    assert!(has_smell(&output, "Constructor Over-Injection"));
    assert!(has_function(&output, "OrderService.new"));
}

#[test]
fn lua_production_complex_method() {
    let output = run_check("lua", "production_service.lua");
    assert!(has_smell(&output, "Complex Method"));
    assert!(has_function(&output, "processOrder"));
}

#[test]
fn lua_production_complex_method_has_high_cc_and_loc() {
    let debug = run_debug("lua", "production_service.lua");
    let cc = function_metric(&debug, "processOrder", "cc").unwrap_or(0);
    let loc = function_metric(&debug, "processOrder", "loc").unwrap_or(0);
    assert!(cc >= 9, "cc should be >= 9, got: {cc}");
    assert!(loc >= 40, "loc should be >= 40, got: {loc}");
}

#[test]
fn lua_production_deep_nesting() {
    let output = run_check("lua", "production_service.lua");
    assert!(has_smell(&output, "Deep Nested"));
    assert!(has_function(&output, "processOrder"));
}

#[test]
fn lua_production_nested_chunks() {
    let output = run_check("lua", "production_service.lua");
    assert!(has_smell(&output, "Nested Conditional Chunks"));
}

#[test]
fn lua_production_excess_args() {
    let output = run_check("lua", "production_service.lua");
    assert!(has_smell(&output, "Excess Arguments"));
    assert!(has_function(&output, "processOrder"));
}

#[test]
fn lua_production_exact_duplication() {
    let output = run_check("lua", "production_service.lua");
    assert!(has_function(&output, "generateInvoice"));
    assert!(has_function(&output, "generateReceipt"));
}

#[test]
fn lua_production_report_duplication() {
    let output = run_check("lua", "production_service.lua");
    assert!(has_function(&output, "formatUserReport"));
    assert!(has_function(&output, "formatAdminReport"));
}

#[test]
fn lua_production_low_cohesion() {
    let output = run_check("lua", "production_service.lua");
    assert!(has_smell(&output, "Low Cohesion"));
    assert!(output.contains("OrderService"));
}

#[test]
fn lua_production_clean_functions_not_flagged_individually() {
    let output = run_check("lua", "production_service.lua");
    let function_lines: Vec<&str> = output
        .lines()
        .filter(|l| l.starts_with("  ") && !l.contains("Module:") && l.contains("getOrder"))
        .collect();
    assert!(
        function_lines.is_empty(),
        "getOrder should have no individual smells, got: {function_lines:?}"
    );
}

// ===========================================================================
// Lua production_api_service.lua — comprehensive smell validation
// ===========================================================================

#[test]
fn lua_api_production_constructor_over_injection() {
    let output = run_check("lua", "production_api_service.lua");
    assert!(has_smell(&output, "Constructor Over-Injection"));
    assert!(has_function(&output, "ApiService.new"));
}

#[test]
fn lua_api_production_complex_method() {
    let output = run_check("lua", "production_api_service.lua");
    assert!(has_smell(&output, "Complex Method"));
    assert!(has_function(&output, "handleRequest"));
}

#[test]
fn lua_api_production_excess_args() {
    let output = run_check("lua", "production_api_service.lua");
    assert!(has_smell(&output, "Excess Arguments"));
    assert!(has_function(&output, "handleRequest"));
}

#[test]
fn lua_api_production_nested_chunks() {
    let output = run_check("lua", "production_api_service.lua");
    assert!(has_smell(&output, "Nested Conditional Chunks"));
}

#[test]
fn lua_api_production_deep_nesting() {
    let output = run_check("lua", "production_api_service.lua");
    assert!(has_smell(&output, "Deep Nested"));
    assert!(has_function(&output, "handleRequest"));
}

#[test]
fn lua_api_production_clean_helpers_not_flagged() {
    let output = run_check("lua", "production_api_service.lua");
    assert!(!has_function(&output, "getUser"));
    assert!(!has_function(&output, "searchUsers"));
    assert!(!has_function(&output, "deleteUser"));
}

// ===========================================================================
// COBOL production_service.cob — comprehensive smell validation
// ===========================================================================

#[test]
fn cobol_production_complex_method() {
    let output = run_check("cobol", "production_service.cob");
    assert!(has_smell(&output, "Complex Method"));
    assert!(has_function(&output, "PROCESS-ORDER"));
}

#[test]
fn cobol_production_complex_method_has_high_cc_and_loc() {
    let debug = run_debug("cobol", "production_service.cob");
    let cc = function_metric(&debug, "PROCESS-ORDER", "cc").unwrap_or(0);
    let loc = function_metric(&debug, "PROCESS-ORDER", "loc").unwrap_or(0);
    assert!(cc >= 9, "cc should be >= 9, got: {cc}");
    assert!(loc >= 40, "loc should be >= 40, got: {loc}");
}

#[test]
fn cobol_production_deep_nesting() {
    let output = run_check("cobol", "production_service.cob");
    assert!(has_smell(&output, "Deep Nested"));
    assert!(has_function(&output, "PROCESS-ORDER"));
}

#[test]
fn cobol_production_nested_chunks() {
    let output = run_check("cobol", "production_service.cob");
    assert!(has_smell(&output, "Nested Conditional Chunks"));
}

#[test]
fn cobol_production_excessive_declarations() {
    let output = run_check("cobol", "production_service.cob");
    assert!(has_smell(&output, "Excessive Declarations"));
}

#[test]
fn cobol_production_exact_duplication() {
    let output = run_check("cobol", "production_service.cob");
    assert!(has_function(&output, "FORMAT-USER-REPORT"));
    assert!(has_function(&output, "FORMAT-ADMIN-REPORT"));
}

#[test]
fn cobol_production_report_duplication() {
    let output = run_check("cobol", "production_service.cob");
    assert!(has_smell(&output, "Code Duplication"));
}

#[test]
fn cobol_production_no_false_excess_args() {
    // COBOL paragraphs have no parameters — Excess Arguments should never trigger
    let output = run_check("cobol", "production_service.cob");
    assert!(!has_smell(&output, "Excess Arguments"));
}

#[test]
fn cobol_production_no_false_constructor_injection() {
    // COBOL has no constructors
    let output = run_check("cobol", "production_service.cob");
    assert!(!has_smell(&output, "Constructor Over-Injection"));
}

#[test]
fn cobol_production_clean_functions_not_flagged_individually() {
    let output = run_check("cobol", "production_service.cob");
    // GET-ORDER has no individual smell
    let function_lines: Vec<&str> = output
        .lines()
        .filter(|l| l.starts_with("  ") && !l.contains("Module:") && l.contains("GET-ORDER"))
        .collect();
    assert!(
        function_lines.is_empty(),
        "GET-ORDER should have no individual smells, got: {function_lines:?}"
    );
}

// ===========================================================================
// COBOL production_api_service.cob — comprehensive smell validation
// ===========================================================================

#[test]
fn cobol_api_production_code_duplication() {
    let output = run_check("cobol", "production_api_service.cob");
    assert!(has_smell(&output, "Code Duplication"));
}

#[test]
fn cobol_api_production_log_duplication() {
    let output = run_check("cobol", "production_api_service.cob");
    assert!(has_function(&output, "LOG-ERROR"));
    assert!(has_function(&output, "LOG-INFO"));
}

#[test]
fn cobol_api_production_handler_duplication() {
    let output = run_check("cobol", "production_api_service.cob");
    assert!(has_function(&output, "HANDLE-POST"));
    assert!(has_function(&output, "HANDLE-PUT"));
}

#[test]
fn cobol_api_production_evaluate_metrics() {
    let debug = run_debug("cobol", "production_api_service.cob");
    let cc = function_metric(&debug, "HANDLE-REQUEST", "cc").unwrap_or(0);
    assert!(cc >= 5, "HANDLE-REQUEST should have high cc from EVALUATE, got: {cc}");
    let arms = function_metric(&debug, "HANDLE-REQUEST", "str_match").unwrap_or(0);
    assert!(arms >= 4, "HANDLE-REQUEST should have WHEN arms, got: {arms}");
}

#[test]
fn cobol_api_production_clean_helpers_not_flagged() {
    let output = run_check("cobol", "production_api_service.cob");
    assert!(!has_function(&output, "SEND-CONFIRM") || !output.contains("SEND-CONFIRM"));
}

// ===========================================================================
// D production_service.d — comprehensive smell validation
// ===========================================================================

#[test]
fn d_production_constructor_over_injection() {
    let output = run_check("d", "production_service.d");
    assert!(has_smell(&output, "Constructor Over-Injection"));
    assert!(has_function(&output, "PaymentService.this"));
}

#[test]
fn d_production_complex_method() {
    let output = run_check("d", "production_service.d");
    assert!(has_smell(&output, "Complex Method") || has_smell(&output, "God Method"));
    assert!(has_function(&output, "processPayment"));
}

#[test]
fn d_production_complex_method_has_high_cc() {
    let debug = run_debug("d", "production_service.d");
    let cc = function_metric(&debug, "PaymentService.processPayment", "cc").unwrap_or(0);
    assert!(cc >= 9, "cc should be >= 9, got: {cc}");
}

#[test]
fn d_production_exact_duplication() {
    let output = run_check("d", "production_service.d");
    assert!(has_smell(&output, "Code Duplication"));
}

#[test]
fn d_production_low_cohesion() {
    let output = run_check("d", "production_service.d");
    assert!(has_smell(&output, "Low Cohesion"), "got: {output}");
}

#[test]
fn d_production_clean_functions_not_flagged() {
    let output = run_check("d", "production_service.d");
    assert!(!has_function(&output, "getConfig"), "simple getter should not be flagged");
}

// ===========================================================================
// D production_api_service.d — API handler smell validation
// ===========================================================================

#[test]
fn d_api_production_complex_handler() {
    let output = run_check("d", "production_api_service.d");
    assert!(has_smell(&output, "Complex Method") || has_smell(&output, "Excess Arguments"));
}

#[test]
fn d_api_production_string_switch() {
    let debug = run_debug("d", "production_api_service.d");
    let arms = function_metric(&debug, "routeRequest", "str_match").unwrap_or(0);
    assert!(arms >= 6, "routeRequest should have string switch arms, got: {arms}");
}

#[test]
fn d_api_production_clean_helper_not_flagged() {
    let output = run_check("d", "production_api_service.d");
    assert!(!has_function(&output, "formatResponse"), "simple helper should not be flagged");
}
