use std::collections::HashSet;
use std::process::Command;

use pulse::analyze::{analyze_source, ScanOptions};
use pulse::audit::finding::{AuditFinding, AuditKind, NaturalnessEvidence};
use pulse::config::PulseConfig;
use pulse::parse::Language;

use crate::common::t;

fn cpg_config() -> PulseConfig {
    toml::from_str("[thresholds.cpg]\nenabled = true\nuse_before_def = true\n").unwrap()
}

fn captured_of(src: &str, lang: Language, ext: &str, fname: &str) -> HashSet<String> {
    let cfg = cpg_config();
    let path = format!("t.{ext}");
    let result = analyze_source(&path, src, lang, Some(&cfg), ScanOptions::check()).expect("analyze");
    let func = result.metrics.functions.iter().find(|f| f.name == fname).expect("function present");
    func.cpg.as_ref().expect("cpg populated when enabled").captured.clone()
}

#[test]
fn rust_closure_reads_outer_local_as_captured_free_variable() {
    let src = "fn outer() -> i32 {\n    let base = 10;\n    let g = |x: i32| base + x;\n    g(1)\n}\n";
    let captured = captured_of(src, Language::Rust, "rs", "outer");
    assert!(
        captured.contains("base"),
        "an outer local read inside the closure is a captured free variable: {captured:?}"
    );
}

#[test]
fn rust_closure_parameter_is_bound_not_captured() {
    let src = "fn outer() -> i32 {\n    let base = 10;\n    let g = |x: i32| base + x;\n    g(1)\n}\n";
    let captured = captured_of(src, Language::Rust, "rs", "outer");
    assert!(!captured.contains("x"), "a closure parameter is bound inside the scope, not free: {captured:?}");
}

#[test]
fn rust_closure_local_binding_is_not_captured() {
    let src =
        "fn outer() -> i32 {\n    let g = || {\n        let local = make();\n        local + 1\n    };\n    g()\n}\n";
    let captured = captured_of(src, Language::Rust, "rs", "outer");
    assert!(
        !captured.contains("local"),
        "a binding declared inside the closure body is not a free variable: {captured:?}"
    );
}

#[test]
fn rust_nested_item_is_a_scope_boundary_not_walked_for_captures() {
    let src = "fn outer() -> i32 {\n    let kept = 1;\n    struct Local { field: i32 }\n    let g = || kept + 1;\n    g()\n}\n";
    let captured = captured_of(src, Language::Rust, "rs", "outer");
    assert!(
        captured.contains("kept"),
        "the closure still captures kept even past a nested item declaration: {captured:?}"
    );
    assert!(!captured.contains("field"), "names inside a nested item must not surface as captures: {captured:?}");
}

#[test]
fn rust_nested_function_item_params_are_not_outer_captures() {
    let src = "fn outer() -> i32 {\n    let kept = 5;\n    fn helper(value: i32) -> i32 { value + 1 }\n    let g = || kept + helper(2);\n    g()\n}\n";
    let captured = captured_of(src, Language::Rust, "rs", "outer");
    assert!(
        !captured.contains("value"),
        "a nested fn-item parameter is bound, not an outer free variable: {captured:?}"
    );
}

#[test]
fn rust_closure_nested_inside_closure_captures_through_both_scopes() {
    let src = "fn outer() -> i32 {\n    let top = 3;\n    let g = |a: i32| {\n        let inner = |b: i32| top + a + b;\n        inner(1)\n    };\n    g(2)\n}\n";
    let captured = captured_of(src, Language::Rust, "rs", "outer");
    assert!(captured.contains("top"), "the deepest closure reads top, free across both closure scopes: {captured:?}");
    assert!(!captured.contains("a"), "the outer closure parameter a is bound for the inner closure: {captured:?}");
    assert!(!captured.contains("b"), "the inner closure parameter b is bound, not captured: {captured:?}");
}

#[test]
fn typescript_arrow_captures_outer_const() {
    let src = "function outer() {\n  const total = expensive();\n  const g = () => total + 1;\n  return g();\n}\n";
    let captured = captured_of(src, Language::TypeScript, "ts", "outer");
    assert!(captured.contains("total"), "the arrow body reads total, a captured free variable: {captured:?}");
}

#[test]
fn typescript_nested_function_declaration_inside_arrow_captures_outer() {
    let src = "function outer() {\n  const seed = 9;\n  const g = () => {\n    function inner(p) { return seed + p; }\n    return inner(1);\n  };\n  return g();\n}\n";
    let captured = captured_of(src, Language::TypeScript, "ts", "outer");
    assert!(captured.contains("seed"), "a nested function declaration inside the arrow captures seed: {captured:?}");
    assert!(!captured.contains("p"), "the nested function parameter p is bound, not captured: {captured:?}");
}

#[test]
fn typescript_type_alias_inside_arrow_does_not_leak_type_names() {
    let src = "function outer() {\n  const kept = 1;\n  const g = () => {\n    type Local = number;\n    return kept;\n  };\n  return g();\n}\n";
    let captured = captured_of(src, Language::TypeScript, "ts", "outer");
    assert!(captured.contains("kept"), "the arrow still captures kept past the nested type alias: {captured:?}");
    assert!(!captured.contains("Local"), "a type alias name is a nested item, not a free variable: {captured:?}");
}

#[test]
fn python_lambda_captures_outer_local() {
    let src = "def outer():\n    base = 4\n    g = lambda x: base + x\n    return g(1)\n";
    let captured = captured_of(src, Language::Python, "py", "outer");
    assert!(captured.contains("base"), "the lambda body reads base, a captured free variable: {captured:?}");
    assert!(!captured.contains("x"), "the lambda parameter x is bound: {captured:?}");
}

#[test]
fn python_nested_def_captures_outer_local() {
    let src = "def outer():\n    seed = 7\n    def inner(p):\n        return seed + p\n    return inner(1)\n";
    let captured = captured_of(src, Language::Python, "py", "outer");
    assert!(captured.contains("seed"), "the nested def reads seed from the enclosing scope: {captured:?}");
    assert!(!captured.contains("p"), "the nested def parameter p is bound: {captured:?}");
}

#[test]
fn kotlin_lambda_literal_seeds_implicit_it_as_bound() {
    let src = "fun outer(): Int {\n    val base = 3\n    val g = { base + it }\n    return g()\n}\n";
    let captured = captured_of(src, Language::Kotlin, "kt", "outer");
    assert!(captured.contains("base"), "the lambda body reads base, a captured free variable: {captured:?}");
    assert!(
        !captured.contains("it"),
        "the implicit lambda receiver `it` is seeded as bound, never a capture: {captured:?}"
    );
}

#[test]
fn php_anonymous_function_body_reads_outer_free_names() {
    let src = "<?php\nfunction outer() {\n  $g = function($x) { return helper($x) + tail($x); };\n  return $g(1);\n}\n";
    let captured = captured_of(src, Language::Php, "php", "outer");
    assert!(!captured.contains("$x"), "the anonymous-function parameter is bound, not captured: {captured:?}");
}

#[test]
fn clean_function_without_nested_scope_captures_nothing() {
    let src = "fn f(a: i32) -> i32 {\n    let x = a + 1;\n    x\n}\n";
    let captured = captured_of(src, Language::Rust, "rs", "f");
    assert!(captured.is_empty(), "a function with no nested closure or item captures nothing: {captured:?}");
}

#[test]
fn underscore_closure_parameter_is_never_recorded_as_capture() {
    let src = "fn outer() -> i32 {\n    let kept = 1;\n    let g = |_: i32| kept;\n    g(0)\n}\n";
    let captured = captured_of(src, Language::Rust, "rs", "outer");
    assert!(!captured.contains("_"), "the underscore placeholder is never a captured name: {captured:?}");
    assert!(captured.contains("kept"), "kept is still captured alongside the underscore parameter: {captured:?}");
}

fn write_named(dir: &std::path::Path, name: &str, body: &str) -> std::path::PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, body).unwrap();
    p
}

fn surprisal_corpus(count: usize) -> String {
    use std::fmt::Write as _;
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
    s
}

const WEIRD_OUTLIER: &str = "def anomaly(qux):\n    zphwx = kkjlm(plover, frobnicate, bazqux, wibble, grault, snork)\n    wibblewobble = flibberty(zorch, splork, gnarl, frobozz, blorf, wamble)\n    crinkle = dweezil(narple, yegg, mungify, skronk, plugh, xyzzy)\n    thud = grunt(waldo, fredly, ploverish, garply, sploosh, kerfuffle)\n    snafu = foobar(corge, quux, bazzle, quxify, wibbler, frobnitz)\n    return zphwx + wibblewobble + crinkle + thud + snafu\n\n";

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
fn naturalness_run_entry_flags_lexical_outlier() {
    let dir = tempfile::tempdir().unwrap();
    let mut body = surprisal_corpus(12);
    body.push_str(WEIRD_OUTLIER);
    let path = write_named(dir.path(), "corpus.py", &body);
    let found = pulse::naturalness::run(&[(path, Language::Python)], &t().audit);
    assert!(
        unnatural(&found).iter().any(|e| e.function == "anomaly"),
        "the public run() entry over real files should surface the all-unique-token outlier: {found:?}"
    );
}

#[test]
fn naturalness_run_entry_below_min_corpus_is_silent() {
    let dir = tempfile::tempdir().unwrap();
    let mut body = surprisal_corpus(4);
    body.push_str(WEIRD_OUTLIER);
    let path = write_named(dir.path(), "small.py", &body);
    let found = pulse::naturalness::run(&[(path, Language::Python)], &t().audit);
    assert!(unnatural(&found).is_empty(), "too few functions to establish a naturalness baseline: {found:?}");
}

#[test]
fn naturalness_run_entry_on_empty_file_list_yields_nothing() {
    let found = pulse::naturalness::run(&[], &t().audit);
    assert!(found.is_empty(), "no files means no naturalness findings: {found:?}");
}

#[test]
fn naturalness_run_skips_unparsable_member_without_dropping_the_outlier() {
    let dir = tempfile::tempdir().unwrap();
    let mut clean = surprisal_corpus(12);
    clean.push_str(WEIRD_OUTLIER);
    let supported = write_named(dir.path(), "corpus.py", &clean);
    let broken = write_named(dir.path(), "broken.py", "def def def (((\n");
    let found = pulse::naturalness::run(&[(supported, Language::Python), (broken, Language::Python)], &t().audit);
    assert!(
        unnatural(&found).iter().any(|e| e.function == "anomaly"),
        "a syntactically broken corpus member is skipped while the real outlier is still flagged: {found:?}"
    );
}

#[test]
fn naturalness_run_homogeneous_corpus_is_not_all_flagged() {
    use std::fmt::Write as _;
    let dir = tempfile::tempdir().unwrap();
    let mut body = String::new();
    for i in 0..11 {
        let _ = writeln!(body, "def same_{i}(alpha, beta):\n    gamma = alpha + beta\n    delta = gamma * alpha\n    epsilon = delta - beta\n    return (gamma, delta, epsilon)\n");
    }
    body.push_str(WEIRD_OUTLIER);
    let path = write_named(dir.path(), "homog.py", &body);
    let found = pulse::naturalness::run(&[(path, Language::Python)], &t().audit);
    let flagged = unnatural(&found);
    assert!(
        flagged.iter().any(|e| e.function == "anomaly"),
        "the mean-absolute-deviation fallback flags the outlier even when MAD is zero: {found:?}"
    );
    assert!(
        flagged.iter().filter(|e| e.function.starts_with("same_")).count() < 11,
        "naturalness must not flag the whole homogeneous corpus: {found:?}"
    );
}

struct GitProject {
    dir: tempfile::TempDir,
    baseline_dir: tempfile::TempDir,
}

impl GitProject {
    fn new() -> Self {
        let p = Self { dir: tempfile::tempdir().unwrap(), baseline_dir: tempfile::tempdir().unwrap() };
        p.git(&["init", "-q"]);
        p
    }

    fn git(&self, args: &[&str]) {
        let output = Command::new("git")
            .arg("-C")
            .arg(self.dir.path())
            .args(["-c", "user.email=t@example.com", "-c", "user.name=t", "-c", "commit.gpgsign=false"])
            .args(args)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .output()
            .expect("git available in test environment");
        assert!(output.status.success(), "git {args:?} failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    fn write(&self, name: &str, body: &str) {
        let path = self.dir.path().join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, body).unwrap();
    }

    fn commit_all(&self) {
        self.git(&["add", "-A"]);
        self.git(&["commit", "-q", "-m", "x", "--allow-empty"]);
    }

    fn pulse(&self, flag: &str) -> String {
        let output = Command::new(env!("CARGO_BIN_EXE_pulse"))
            .arg(flag)
            .current_dir(self.dir.path())
            .env("PULSE_BASELINE_DIR", self.baseline_dir.path())
            .stdin(std::process::Stdio::null())
            .output()
            .expect("failed to run pulse");
        String::from_utf8(output.stdout).unwrap()
    }
}

fn spicy_fn(name: &str) -> String {
    let ifs = t().function.cc_warning;
    let mut s = format!("def {name}(flag):\n");
    for _ in 0..ifs {
        s.push_str("    if flag:\n        x = 0\n");
    }
    s
}

#[test]
fn turn_scan_skips_a_changed_test_file() {
    let p = GitProject::new();
    p.commit_all();
    p.pulse("--cleanup");
    p.write("test_worker.py", &spicy_fn("worker"));
    let out = p.pulse("--stop");
    assert!(
        !out.to_lowercase().contains("complex method"),
        "a changed test file is excluded from the turn regression scan: {out}"
    );
}

#[test]
fn turn_scan_skips_a_changed_file_inside_a_tests_directory() {
    let p = GitProject::new();
    p.commit_all();
    p.pulse("--cleanup");
    p.write("tests/svc.py", &spicy_fn("worker"));
    let out = p.pulse("--stop");
    assert!(
        !out.to_lowercase().contains("complex method"),
        "a file under a tests/ directory is treated as a test file and skipped: {out}"
    );
}

#[test]
fn turn_scan_skips_a_changed_fixture_file() {
    let p = GitProject::new();
    p.commit_all();
    p.pulse("--cleanup");
    p.write("fixtures/svc.py", &spicy_fn("worker"));
    let out = p.pulse("--stop");
    assert!(
        !out.to_lowercase().contains("complex method"),
        "a file under a fixtures/ directory is excluded from the turn scan: {out}"
    );
}

#[test]
fn turn_scan_respects_a_pulse_toml_ignore_pattern() {
    let p = GitProject::new();
    p.write(".pulse.toml", "[ignore]\npaths = [\"generated/**\"]\n");
    p.commit_all();
    p.pulse("--cleanup");
    p.write("generated/svc.py", &spicy_fn("worker"));
    let out = p.pulse("--stop");
    assert!(
        !out.to_lowercase().contains("complex method"),
        "an ignored path must not produce a turn regression: {out}"
    );
}

#[test]
fn turn_scan_flags_a_non_ignored_changed_source_file() {
    let p = GitProject::new();
    p.write(".pulse.toml", "[ignore]\npaths = [\"generated/**\"]\n");
    p.commit_all();
    p.pulse("--cleanup");
    p.write("svc.py", &spicy_fn("worker"));
    let out = p.pulse("--stop");
    assert!(
        out.to_lowercase().contains("complex method"),
        "a production source file outside the ignore set is still scanned: {out}"
    );
}

#[test]
fn turn_scan_skips_a_changed_file_with_no_detectable_language() {
    let p = GitProject::new();
    p.commit_all();
    p.pulse("--cleanup");
    p.write("notes.unknownext", &spicy_fn("worker"));
    let out = p.pulse("--stop");
    assert!(out.is_empty(), "a file with no recognized language is silently skipped: {out}");
}

#[test]
fn canonical_path_falls_back_to_input_for_missing_path() {
    let missing = "/no/such/path/that/exists/here.py";
    let resolved = pulse::turn_scan::canonical_path(missing);
    assert_eq!(resolved, missing, "an uncanonicalizable path returns the original string unchanged");
}

#[test]
fn canonical_path_resolves_an_existing_file() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("real.py");
    std::fs::write(&file, "x = 1\n").unwrap();
    let resolved = pulse::turn_scan::canonical_path(file.to_str().unwrap());
    assert!(
        resolved.ends_with("real.py"),
        "an existing path canonicalizes to an absolute form ending in the name: {resolved}"
    );
    assert!(!resolved.is_empty(), "canonicalization of a real file yields a non-empty path");
}
