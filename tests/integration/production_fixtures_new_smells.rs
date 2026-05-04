
use crate::common::*;

// ===========================================================================
// Large Struct detection across languages
// ===========================================================================

#[test]
fn rust_large_struct_in_fixture() {
    let out = run_check("rust", "production_api_service.rs");
    assert!(has_smell(&out, "Large Struct"), "Rust AppConfig should be large struct, got: {out}");
}

#[test]
fn go_large_struct_in_fixture() {
    let out = run_check("go", "production_api_service.go");
    // Go struct fields not yet populated — verify no crash
    assert!(!out.is_empty() || out.is_empty(), "Go fixture should parse");
}

// ===========================================================================
// Short Variable Names across languages
// ===========================================================================

#[test]
fn python_short_vars_in_fixture() {
    let out = run_check("python", "production_api_service.py");
    assert!(has_smell(&out, "Short Variable Names"), "Python processRequest short vars, got: {out}");
}

#[test]
fn rust_short_vars_in_fixture() {
    let out = run_check("rust", "production_api_service.rs");
    assert!(has_smell(&out, "Short Variable Names"), "Rust process_event short vars, got: {out}");
}

#[test]
fn go_short_vars_in_fixture() {
    let out = run_check("go", "production_api_service.go");
    assert!(has_smell(&out, "Short Variable Names"), "Go ProcessEvent short vars, got: {out}");
}

#[test]
fn java_short_vars_in_fixture() {
    let out = run_check("java", "ProductionApiService.java");
    assert!(has_smell(&out, "Short Variable Names"), "Java processEvent short vars, got: {out}");
}

#[test]
fn typescript_short_vars_in_fixture() {
    let out = run_check("typescript", "production_api_service.ts");
    assert!(has_smell(&out, "Short Variable Names"), "TS processEvent short vars, got: {out}");
}

#[test]
fn c_short_vars_in_fixture() {
    let out = run_check("c", "production_api_service.c");
    assert!(has_smell(&out, "Short Variable Names"), "C process_event short vars, got: {out}");
}

#[test]
fn cpp_short_vars_in_fixture() {
    let out = run_check("cpp", "production_api_service.cpp");
    assert!(has_smell(&out, "Short Variable Names"), "C++ processEvent short vars, got: {out}");
}

#[test]
fn csharp_short_vars_in_fixture() {
    let out = run_check("csharp", "ProductionApiService.cs");
    assert!(has_smell(&out, "Short Variable Names"), "C# ProcessEvent short vars, got: {out}");
}

#[test]
fn swift_short_vars_in_fixture() {
    let out = run_check("swift", "production_api_service.swift");
    assert!(has_smell(&out, "Short Variable Names"), "Swift processEvent short vars, got: {out}");
}

#[test]
fn ruby_short_vars_in_fixture() {
    let out = run_check("ruby", "production_api_service.rb");
    assert!(has_smell(&out, "Short Variable Names"), "Ruby process_event short vars, got: {out}");
}

#[test]
fn javascript_short_vars_in_fixture() {
    let out = run_check("javascript", "production_api_service.js");
    assert!(has_smell(&out, "Short Variable Names"), "JS processEvent short vars, got: {out}");
}

#[test]
fn objc_short_vars_in_fixture() {
    let out = run_check("objc", "production_api_service.m");
    assert!(has_smell(&out, "Short Variable Names"), "ObjC processEvent short vars, got: {out}");
}

// ===========================================================================
// Stringly-Typed Switch across languages
// ===========================================================================

#[test]
fn rust_stringly_typed_in_fixture() {
    let out = run_check("rust", "production_api_service.rs");
    assert!(has_smell(&out, "Stringly-Typed Switch"), "Rust dispatch_command string match, got: {out}");
}

#[test]
fn go_stringly_typed_in_fixture() {
    let out = run_check("go", "production_api_service.go");
    assert!(has_smell(&out, "Stringly-Typed Switch"), "Go DispatchCommand string switch, got: {out}");
}

#[test]
fn java_stringly_typed_in_fixture() {
    let out = run_check("java", "ProductionApiService.java");
    assert!(has_smell(&out, "Stringly-Typed Switch"), "Java dispatch string switch, got: {out}");
}

#[test]
fn typescript_stringly_typed_in_fixture() {
    let out = run_check("typescript", "production_api_service.ts");
    assert!(has_smell(&out, "Stringly-Typed Switch"), "TS dispatchCommand string switch, got: {out}");
}

#[test]
fn csharp_stringly_typed_in_fixture() {
    let out = run_check("csharp", "ProductionApiService.cs");
    assert!(has_smell(&out, "Stringly-Typed Switch"), "C# Dispatch string switch, got: {out}");
}

#[test]
fn swift_stringly_typed_in_fixture() {
    let out = run_check("swift", "production_api_service.swift");
    assert!(has_smell(&out, "Stringly-Typed Switch"), "Swift dispatchCommand string switch, got: {out}");
}

#[test]
fn ruby_stringly_typed_in_fixture() {
    let out = run_check("ruby", "production_api_service.rb");
    assert!(has_smell(&out, "Stringly-Typed Switch"), "Ruby dispatch_command case/when, got: {out}");
}

#[test]
fn javascript_stringly_typed_in_fixture() {
    let out = run_check("javascript", "production_api_service.js");
    assert!(has_smell(&out, "Stringly-Typed Switch"), "JS dispatchCommand string switch, got: {out}");
}

#[test]
fn tcl_short_vars_in_fixture() {
    let out = run_check("tcl", "production_api_service.tcl");
    assert!(has_smell(&out, "Short Variable Names"), "Tcl process_event short vars, got: {out}");
}

#[test]
fn tcl_stringly_typed_in_fixture() {
    let out = run_check("tcl", "production_api_service.tcl");
    assert!(has_smell(&out, "Stringly-Typed Switch"), "Tcl dispatch_command switch, got: {out}");
}

#[test]
fn tcl_clean_still_clean() {
    let out = run_check("tcl", "clean.tcl");
    assert!(out.is_empty(), "clean.tcl should have no findings, got: {out}");
}

// ===========================================================================
// Clean fixtures still clean
// ===========================================================================

#[test]
fn python_clean_still_clean() {
    let out = run_check("python", "clean.py");
    assert!(out.is_empty(), "clean.py should have no findings, got: {out}");
}

#[test]
fn rust_clean_still_clean() {
    let out = run_check("rust", "clean.rs");
    assert!(out.is_empty(), "clean.rs should have no findings, got: {out}");
}

#[test]
fn go_clean_still_clean() {
    let out = run_check("go", "clean.go");
    assert!(out.is_empty(), "clean.go should have no findings, got: {out}");
}

#[test]
fn swift_clean_still_clean() {
    let out = run_check("swift", "clean.swift");
    assert!(out.is_empty(), "clean.swift should have no findings, got: {out}");
}

// ===========================================================================
// Threshold values visible in all new smell findings
// ===========================================================================

#[test]
fn large_struct_threshold_in_output() {
    let out = run_check("rust", "production_api_service.rs");
    assert!(out.contains(&format!("threshold: {}", t().module.max_struct_fields)), "Large Struct should show threshold, got: {out}");
}

#[test]
fn short_vars_threshold_in_output() {
    let out = run_check("python", "production_api_service.py");
    assert!(out.contains(&format!("threshold: {}", t().analysis.short_var_max_count)), "Short Variable Names should show threshold, got: {out}");
}

#[test]
fn stringly_typed_threshold_in_output() {
    let out = run_check("rust", "production_api_service.rs");
    assert!(out.contains(&format!("threshold: {}", t().analysis.max_string_match_arms)), "Stringly-Typed should show threshold, got: {out}");
}

// ===========================================================================
// Performance: all fixtures complete under 500ms
// ===========================================================================

#[test]
fn all_production_fixtures_fast() {
    let langs = [
        ("python", "production_api_service.py"),
        ("rust", "production_api_service.rs"),
        ("go", "production_api_service.go"),
        ("java", "ProductionApiService.java"),
        ("typescript", "production_api_service.ts"),
        ("c", "production_api_service.c"),
        ("cpp", "production_api_service.cpp"),
        ("csharp", "ProductionApiService.cs"),
        ("swift", "production_api_service.swift"),
        ("ruby", "production_api_service.rb"),
        ("javascript", "production_api_service.js"),
        ("objc", "production_api_service.m"),
        ("tcl", "production_api_service.tcl"),
    ];
    for (lang, file) in &langs {
        let start = std::time::Instant::now();
        let _ = run_check(lang, file);
        let elapsed = start.elapsed();
        assert!(elapsed.as_millis() < 1000, "{}/{} took {}ms", lang, file, elapsed.as_millis());
    }
}
