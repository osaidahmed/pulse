use std::fmt::Write as _;

use pulse::audit::finding::{AuditFinding, AuditKind, NaturalnessEvidence};
use pulse::audit::suppression::AuditSuppression;
use pulse::audit::{self, AuditOpts, IgnoreFilter, PassChoice};
use pulse::config::IgnoreMatcher;

use crate::audit_common::*;

const WEIRD: &str = "def anomaly(qux):\n    zphwx = kkjlm(plover, frobnicate, bazqux, wibble, grault, snork)\n    wibblewobble = flibberty(zorch, splork, gnarl, frobozz, blorf, wamble)\n    crinkle = dweezil(narple, yegg, mungify, skronk, plugh, xyzzy)\n    thud = grunt(waldo, fredly, ploverish, garply, sploosh, kerfuffle)\n    snafu = foobar(corge, quux, bazzle, quxify, wibbler, frobnitz)\n    return zphwx + wibblewobble + crinkle + thud + snafu\n\n";

fn corpus_source(include_weird: bool, count: usize) -> String {
    let ops = ["+", "-", "*", "%"];
    let mut s = String::new();
    for i in 0..count {
        let op = ops[i % ops.len()];
        let _ = writeln!(s, "def calc_{i}(alpha, beta, gamma):\n    acc = alpha {op} beta");
        for _ in 0..=(i % 4) {
            let _ = writeln!(s, "    acc = acc {op} gamma");
        }
        let _ = writeln!(s, "    record = (alpha, beta, gamma, acc)\n    return record\n");
    }
    if include_weird {
        s.push_str(WEIRD);
    }
    s
}

fn homogeneous_source() -> String {
    let mut s = String::new();
    for i in 0..11 {
        let _ = writeln!(s, "def same_{i}(alpha, beta):\n    gamma = alpha + beta\n    delta = gamma * alpha\n    epsilon = delta - beta\n    return (gamma, delta, epsilon)\n");
    }
    s.push_str(WEIRD);
    s
}

fn run_naturalness_named(filename: &str, source: &str, pass: Option<PassChoice>) -> Vec<AuditFinding> {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(filename), source).unwrap();
    let matcher = IgnoreMatcher::from_patterns(&[]);
    let opts = AuditOpts {
        root: dir.path().to_path_buf(),
        pass,
        json: false,
        include_tests: true,
        show_noise: false,
        suppression: AuditSuppression::new(),
    };
    let filter = IgnoreFilter::new(&matcher, dir.path());
    audit::run_with_filter(&opts, &t().audit, &filter)
}

fn run_naturalness(source: &str, pass: Option<PassChoice>) -> Vec<AuditFinding> {
    run_naturalness_named("corpus.py", source, pass)
}

const GO_WEIRD: &str = "func anomaly(qux int) int {\n\tzphwx := kkjlm(plover, frobnicate, bazqux, wibble, grault, snork)\n\twibblewobble := flibberty(zorch, splork, gnarl, frobozz, blorf, wamble)\n\tcrinkle := dweezil(narple, yegg, mungify, skronk, plugh, xyzzy)\n\tthud := grunt(waldo, fredly, ploverish, garply, sploosh, kerfuffle)\n\treturn zphwx + wibblewobble + crinkle + thud\n}\n";

fn go_corpus(include_weird: bool, count: usize) -> String {
    let ops = ["+", "-", "*", "%"];
    let mut s = String::from("package main\n\n");
    for i in 0..count {
        let op = ops[i % ops.len()];
        let _ = writeln!(s, "func calc{i}(alpha int, beta int, gamma int) int {{");
        let _ = writeln!(s, "\tacc := alpha {op} beta");
        for _ in 0..=(i % 4) {
            let _ = writeln!(s, "\tacc = acc {op} gamma");
        }
        let _ = writeln!(s, "\trecord := alpha {op} beta {op} gamma {op} acc");
        let _ = writeln!(s, "\treturn record\n}}\n");
    }
    if include_weird {
        s.push_str(GO_WEIRD);
    }
    s
}

const TS_WEIRD: &str = "function anomaly(qux) {\n  let zphwx = kkjlm(plover, frobnicate, bazqux, wibble, grault, snork);\n  let wibblewobble = flibberty(zorch, splork, gnarl, frobozz, blorf, wamble);\n  let crinkle = dweezil(narple, yegg, mungify, skronk, plugh, xyzzy);\n  let thud = grunt(waldo, fredly, ploverish, garply, sploosh, kerfuffle);\n  return zphwx + wibblewobble + crinkle + thud;\n}\n";

fn ts_corpus(include_weird: bool, count: usize) -> String {
    let ops = ["+", "-", "*", "%"];
    let mut s = String::new();
    for i in 0..count {
        let op = ops[i % ops.len()];
        let _ = writeln!(s, "function calc{i}(alpha, beta, gamma) {{");
        let _ = writeln!(s, "  let acc = alpha {op} beta;");
        for _ in 0..=(i % 4) {
            let _ = writeln!(s, "  acc = acc {op} gamma;");
        }
        let _ = writeln!(s, "  let record = alpha {op} beta {op} gamma {op} acc;");
        let _ = writeln!(s, "  return record;\n}}\n");
    }
    if include_weird {
        s.push_str(TS_WEIRD);
    }
    s
}

fn unnatural(findings: &[AuditFinding]) -> Vec<&NaturalnessEvidence> {
    findings
        .iter()
        .filter_map(|f| match &f.kind {
            AuditKind::UnnaturalCode(e) => Some(e),
            _ => None,
        })
        .collect()
}

#[test]
fn flags_the_lexical_outlier() {
    let found = run_naturalness(&corpus_source(true, 12), Some(PassChoice::Naturalness));
    let flagged = unnatural(&found);
    assert!(flagged.iter().any(|e| e.function == "anomaly"), "the all-unique-token function should read as unnatural");
}

#[test]
fn uniform_normals_are_not_all_flagged() {
    let found = run_naturalness(&corpus_source(true, 12), Some(PassChoice::Naturalness));
    let normal_flagged = unnatural(&found).iter().filter(|e| e.function.starts_with("calc_")).count();
    let total_normals = 12usize;
    assert!(normal_flagged < total_normals, "naturalness must not flag the whole corpus as unnatural");
}

#[test]
fn below_min_corpus_yields_no_findings() {
    let found = run_naturalness(&corpus_source(true, 4), Some(PassChoice::Naturalness));
    assert!(unnatural(&found).is_empty(), "too few functions to establish a naturalness baseline");
}

#[test]
fn naturalness_opt_in_not_in_default_passes() {
    let found = run_naturalness(&corpus_source(true, 12), None);
    let any = found.iter().any(|f| matches!(f.kind, AuditKind::UnnaturalCode(_)));
    assert!(!any, "naturalness must not run in the default (All) pass");
}

#[test]
fn homogeneous_corpus_still_flags_outlier() {
    let found = run_naturalness(&homogeneous_source(), Some(PassChoice::Naturalness));
    assert!(
        unnatural(&found).iter().any(|e| e.function == "anomaly"),
        "the mean-absolute-deviation fallback should flag the outlier even when MAD is zero"
    );
}

#[test]
fn naturalness_is_deterministic() {
    let source = corpus_source(true, 12);
    let sig = |fs: &[AuditFinding]| -> Vec<(String, u32, u64)> {
        let mut v: Vec<(String, u32, u64)> =
            unnatural(fs).iter().map(|e| (e.function.clone(), e.line, e.surprisal.to_bits())).collect();
        v.sort();
        v
    };
    let first = sig(&run_naturalness(&source, Some(PassChoice::Naturalness)));
    let second = sig(&run_naturalness(&source, Some(PassChoice::Naturalness)));
    assert_eq!(first, second, "naturalness scoring must be reproducible bit-for-bit");
}

#[test]
fn flags_a_go_lexical_outlier() {
    let found = run_naturalness_named("corpus.go", &go_corpus(true, 14), Some(PassChoice::Naturalness));
    assert!(
        unnatural(&found).iter().any(|e| e.function == "anomaly"),
        "naturalness now covers Go: the all-unique-token function reads as unnatural"
    );
}

#[test]
fn go_normals_are_not_all_flagged() {
    let found = run_naturalness_named("corpus.go", &go_corpus(true, 14), Some(PassChoice::Naturalness));
    let normals = unnatural(&found).iter().filter(|e| e.function.starts_with("calc")).count();
    assert!(normals < 14, "naturalness must not flag the whole go corpus as unnatural");
}

#[test]
fn flags_a_typescript_lexical_outlier() {
    let found = run_naturalness_named("corpus.ts", &ts_corpus(true, 14), Some(PassChoice::Naturalness));
    assert!(
        unnatural(&found).iter().any(|e| e.function == "anomaly"),
        "naturalness now covers TypeScript: the all-unique-token function reads as unnatural"
    );
}
