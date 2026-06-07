use std::path::PathBuf;

use pulse::parse::{detect_language, parse_and_walk};
use pulse::walk::FunctionMetrics;

const MIN_NONTRIVIAL_FUNCTIONS: usize = 2;

fn fixture_path(lang_dir: &str, fixture: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures").join(lang_dir).join(fixture)
}

fn is_nontrivial(f: &FunctionMetrics) -> bool {
    f.loc > 0 && (f.cc > 1 || f.cognitive_complexity > 0 || f.max_nesting > 0) && f.structural_hash != 0
}

fn assert_walker_completeness(lang_dir: &str, fixture: &str) {
    let path = fixture_path(lang_dir, fixture);
    let source = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let lang = detect_language(&path).unwrap_or_else(|| panic!("detect_language failed for {}", path.display()));
    let metrics =
        parse_and_walk(&source, lang).unwrap_or_else(|| panic!("parse_and_walk failed for {}", path.display()));

    let nontrivial: Vec<&FunctionMetrics> = metrics.functions.iter().filter(|f| is_nontrivial(f)).collect();

    assert!(
        nontrivial.len() >= MIN_NONTRIVIAL_FUNCTIONS,
        "{lang_dir}/{fixture}: expected >= {MIN_NONTRIVIAL_FUNCTIONS} non-trivial functions, found {} (of {} total). Sample: {:?}",
        nontrivial.len(),
        metrics.functions.len(),
        metrics
            .functions
            .iter()
            .take(5)
            .map(|f| (
                f.name.as_str(),
                f.loc,
                f.cc,
                f.cognitive_complexity,
                f.max_nesting,
                f.structural_hash != 0
            ))
            .collect::<Vec<_>>()
    );
}

#[test]
fn python_walker_completeness() {
    assert_walker_completeness("python", "production_api_service.py");
}

#[test]
fn typescript_walker_completeness() {
    assert_walker_completeness("typescript", "production_api_service.ts");
}

#[test]
fn javascript_walker_completeness() {
    assert_walker_completeness("javascript", "production_api_service.js");
}

#[test]
fn rust_walker_completeness() {
    assert_walker_completeness("rust", "production_api_service.rs");
}

#[test]
fn java_walker_completeness() {
    assert_walker_completeness("java", "ProductionApiService.java");
}

#[test]
fn kotlin_walker_completeness() {
    assert_walker_completeness("kotlin", "ProductionApiService.kt");
}

#[test]
fn swift_walker_completeness() {
    assert_walker_completeness("swift", "production_api_service.swift");
}

#[test]
fn go_walker_completeness() {
    assert_walker_completeness("go", "production_api_service.go");
}

#[test]
fn ruby_walker_completeness() {
    assert_walker_completeness("ruby", "production_api_service.rb");
}

#[test]
fn php_walker_completeness() {
    assert_walker_completeness("php", "production_service.php");
}

#[test]
fn c_walker_completeness() {
    assert_walker_completeness("c", "production_api_service.c");
}

#[test]
fn cpp_walker_completeness() {
    assert_walker_completeness("cpp", "production_api_service.cpp");
}

#[test]
fn csharp_walker_completeness() {
    assert_walker_completeness("csharp", "production_service.cs");
}

#[test]
fn objc_walker_completeness() {
    assert_walker_completeness("objc", "production_api_service.m");
}

#[test]
fn lua_walker_completeness() {
    assert_walker_completeness("lua", "production_service.lua");
}

#[test]
fn tcl_walker_completeness() {
    assert_walker_completeness("tcl", "production_service.tcl");
}

#[test]
fn cobol_walker_completeness() {
    assert_walker_completeness("cobol", "production_api_service.cob");
}

#[test]
fn r_walker_completeness() {
    assert_walker_completeness("r", "production_api_service.r");
}

#[test]
fn d_walker_completeness() {
    assert_walker_completeness("d", "production_api_service.d");
}

#[test]
fn zig_walker_completeness() {
    assert_walker_completeness("zig", "production_service.zig");
}

#[test]
fn groovy_walker_completeness() {
    assert_walker_completeness("groovy", "production_api_service.groovy");
}

#[test]
fn haskell_walker_completeness() {
    assert_walker_completeness("haskell", "ProductionApiService.hs");
}
