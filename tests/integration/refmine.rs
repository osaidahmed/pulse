use pulse::parse::Language;
use pulse::refmine::body_match_ratio;

fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn identical_sources_match_fully() {
    let src = "fn f() { let a = 1; bar(a); }";
    let r = body_match_ratio(src, src, Language::Rust).expect("statements exist");
    assert!(approx(r, 1.0), "identical bodies fully match: {r}");
}

#[test]
fn a_renamed_function_preserves_its_body() {
    let pre = "fn foo() { let a = 1; bar(a); }";
    let cur = "fn renamed() { let a = 1; bar(a); }";
    let r = body_match_ratio(pre, cur, Language::Rust).expect("statements exist");
    assert!(approx(r, 1.0), "a pure rename leaves the body statements intact: {r}");
}

#[test]
fn a_rewritten_body_does_not_match() {
    let pre = "fn f() { let a = 1; g(a); }";
    let cur = "fn f() { compute(); finish(); }";
    let r = body_match_ratio(pre, cur, Language::Rust).expect("statements exist");
    assert!(approx(r, 0.0), "a fully rewritten body has no preserved statements: {r}");
}

#[test]
fn a_partial_change_gives_a_partial_ratio() {
    let pre = "fn f() { let a = 1; g(a); h(); }";
    let cur = "fn f() { let a = 1; different(); h(); }";
    let r = body_match_ratio(pre, cur, Language::Rust).expect("statements exist");
    assert!(approx(r, 2.0 / 3.0), "two of three statements survive: {r}");
}

#[test]
fn python_rename_preserves_body() {
    let pre = "def foo():\n    a = 1\n    bar(a)\n";
    let cur = "def renamed():\n    a = 1\n    bar(a)\n";
    let r = body_match_ratio(pre, cur, Language::Python).expect("statements exist");
    assert!(approx(r, 1.0), "python rename preserves body: {r}");
}

#[test]
fn typescript_rename_preserves_body() {
    let pre = "function f() { const a = 1; bar(a); }";
    let cur = "function g() { const a = 1; bar(a); }";
    let r = body_match_ratio(pre, cur, Language::TypeScript).expect("statements exist");
    assert!(approx(r, 1.0), "typescript rename preserves body: {r}");
}

#[test]
fn unsupported_language_is_none() {
    let src = "package main\nfunc f() { a := 1\n _ = a }";
    assert!(body_match_ratio(src, src, Language::Go).is_none(), "refmine is front-loaded to a few languages");
}

#[test]
fn an_empty_body_is_none() {
    assert!(body_match_ratio("fn f() {}", "fn f() {}", Language::Rust).is_none(), "no statements to compare");
}

#[test]
fn typescript_switch_rename_preserves_body() {
    let pre = "function f(x) { switch (x) { case 1: a(); break; default: b(); } }";
    let cur = "function renamed(x) { switch (x) { case 1: a(); break; default: b(); } }";
    let r = body_match_ratio(pre, cur, Language::TypeScript).expect("statements exist");
    assert!(approx(r, 1.0), "a rename preserves the whole switch body: {r}");
}

#[test]
fn typescript_switch_rewrite_is_detected() {
    let pre = "function f(x) { switch (x) { case 1: a(); break; default: b(); } }";
    let cur = "function f(x) { switch (x) { case 1: totally(); different(); break; default: c(); } }";
    let r = body_match_ratio(pre, cur, Language::TypeScript).expect("statements exist");
    assert!(r < 1.0, "a rewritten switch case is detected, not masked: {r}");
}

#[test]
fn rust_match_rename_preserves_body() {
    let pre = "fn f(x: i32) { match x { 1 => a(), _ => b(), } }";
    let cur = "fn renamed(x: i32) { match x { 1 => a(), _ => b(), } }";
    let r = body_match_ratio(pre, cur, Language::Rust).expect("statements exist");
    assert!(approx(r, 1.0), "a rename preserves the match: {r}");
}

#[test]
fn whole_file_comparison_inflates_the_ratio_so_slice2_must_scope() {
    let pre = "fn f() { let a = 1; let b = 2; keep(a, b); } fn g() { old1(); old2(); }";
    let cur = "fn f() { let a = 1; let b = 2; keep(a, b); } fn g() { new1(); new2(); }";
    let r = body_match_ratio(pre, cur, Language::Rust).expect("statements exist");
    assert!(
        r > 0.5,
        "g is fully rewritten yet unchanged f inflates the whole-file ratio to {r}; slice 2 must scope to the target function's body"
    );
}
