use pulse::intensity::{for_finding, normalize_exceedance, rank_findings};
use pulse::smells::{self, Location};
use pulse::walk::FileMetrics;

use crate::common::t;

fn analyze(code: &str, ext: &str) -> FileMetrics {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(format!("f.{ext}"));
    std::fs::write(&path, code).unwrap();
    let lang = pulse::parse::detect_language(&path).expect("language");
    let source = std::fs::read_to_string(&path).unwrap();
    pulse::parse::parse_and_walk(&source, lang).expect("parse")
}

fn approx(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-9, "expected {expected}, got {actual}");
}

#[test]
fn normalize_exceedance_at_min_is_zero() {
    approx(normalize_exceedance(10.0, 10.0, 20.0), 0.0);
}

#[test]
fn normalize_exceedance_at_max_is_ten() {
    approx(normalize_exceedance(20.0, 10.0, 20.0), 10.0);
}

#[test]
fn normalize_exceedance_midpoint_is_five() {
    approx(normalize_exceedance(15.0, 10.0, 20.0), 5.0);
}

#[test]
fn normalize_exceedance_clamps_above_and_below() {
    approx(normalize_exceedance(25.0, 10.0, 20.0), 10.0);
    approx(normalize_exceedance(5.0, 10.0, 20.0), 0.0);
}

#[test]
fn normalize_exceedance_degenerate_band_no_nan() {
    approx(normalize_exceedance(12.0, 10.0, 10.0), 10.0);
    approx(normalize_exceedance(8.0, 10.0, 10.0), 0.0);
    approx(normalize_exceedance(10.0, 10.0, 10.0), 0.0);
}

#[test]
fn normalize_exceedance_monotone_in_value() {
    assert!(normalize_exceedance(16.0, 10.0, 20.0) > normalize_exceedance(14.0, 10.0, 20.0));
}

fn rust_ifs(n: u32) -> String {
    (0..n).map(|i| format!("    if a == {i} {{ b += 1; }}\n")).collect()
}

fn rust_lines(n: u32) -> String {
    (0..n).map(|_| "    b += 1;\n".to_string()).collect()
}

#[test]
fn intensity_in_range_for_real_findings() {
    let body = rust_ifs(t().function.cc_alert + 4);
    let code = format!("fn busy(a: i32, b: &mut i32) {{\n{body}}}\n");
    let metrics = analyze(&code, "rs");
    let findings = smells::detect(&metrics, &code, &t());
    for f in &findings {
        if let Location::Function { name, start_line, .. } = &f.location {
            let fm = metrics
                .functions
                .iter()
                .find(|m| &m.name == name && m.start_line == *start_line)
                .unwrap();
            let score = for_finding(f.smell, fm, &t());
            assert!((0.0..=10.0).contains(&score), "{:?} intensity {score}", f.smell);
        }
    }
}

#[test]
fn god_method_outranks_large_method_in_block_order() {
    let large = format!(
        "fn the_large(b: &mut i32) {{\n{}}}\n",
        rust_lines(t().function.fn_loc_warning + 5)
    );
    let god = format!(
        "fn the_god(a: i32, b: &mut i32) {{\n{}}}\n",
        rust_ifs(t().function.fn_loc_alert + 5)
    );
    let code = format!("{large}\n{god}");
    let metrics = analyze(&code, "rs");
    let findings = smells::detect(&metrics, &code, &t());
    let ranked = rank_findings(&findings, &metrics, &t());
    assert!(!ranked.is_empty());
    let top = &ranked[0];
    let Location::Function { name, .. } = &top.location else {
        panic!("expected function finding first")
    };
    assert_eq!(name, "the_god", "highest-intensity finding ranks first, got {ranked:?}");
}

#[test]
fn ranking_is_deterministic_across_runs() {
    let code = format!(
        "fn one(b: &mut i32) {{\n{}}}\nfn two(a: i32, b: &mut i32) {{\n{}}}\n",
        rust_lines(t().function.fn_loc_warning + 3),
        rust_ifs(t().function.cc_warning + 3)
    );
    let metrics = analyze(&code, "rs");
    let findings = smells::detect(&metrics, &code, &t());
    let r1 = rank_findings(&findings, &metrics, &t());
    let r2 = rank_findings(&findings, &metrics, &t());
    let names1: Vec<_> = r1.iter().map(|f| (f.smell, f.detail.clone())).collect();
    let names2: Vec<_> = r2.iter().map(|f| (f.smell, f.detail.clone())).collect();
    assert_eq!(names1, names2);
}
