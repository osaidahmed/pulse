use crate::common::*;

fn typed_params(decls: &[String]) -> String {
    decls.join(", ")
}

fn rust_fn(types: &[&str]) -> String {
    let params: Vec<String> = types
        .iter()
        .enumerate()
        .map(|(i, ty)| format!("a{i}: {ty}"))
        .collect();
    format!("fn f({}) {{}}\n", typed_params(&params))
}

fn py_fn(types: &[&str]) -> String {
    let params: Vec<String> = types
        .iter()
        .enumerate()
        .map(|(i, ty)| format!("a{i}: {ty}"))
        .collect();
    format!("def f({}):\n    pass\n", typed_params(&params))
}

fn primitive_count() -> usize {
    t().analysis.primitive_min_typed_params as usize + 1
}

#[test]
fn primitive_obsession_requires_data_clump_rust() {
    let pool = ["i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64"];
    let code = rust_fn(&pool[..primitive_count()]);
    assert!(
        !has_smell(&pulse_check_code(&code, "rs"), "Primitive Obsession"),
        "all-distinct primitive types are not a data clump, got: {code}"
    );
}

#[test]
fn primitive_obsession_fires_on_data_clump_rust() {
    let same = vec!["i32"; primitive_count()];
    let code = rust_fn(&same);
    assert!(
        has_smell(&pulse_check_code(&code, "rs"), "Primitive Obsession"),
        "repeated primitive type is a data clump, got: {code}"
    );
}

#[test]
fn primitive_obsession_requires_data_clump_python() {
    let pool = ["int", "float", "bool", "bytes", "complex", "str"];
    let code = py_fn(&pool[..primitive_count()]);
    assert!(
        !has_smell(&pulse_check_code(&code, "py"), "Primitive Obsession"),
        "all-distinct primitive types are not a data clump, got: {code}"
    );
}

#[test]
fn primitive_obsession_fires_on_data_clump_python() {
    let same = vec!["int"; primitive_count()];
    let code = py_fn(&same);
    assert!(
        has_smell(&pulse_check_code(&code, "py"), "Primitive Obsession"),
        "repeated primitive type is a data clump, got: {code}"
    );
}

fn switch_arms() -> usize {
    t().analysis.max_string_match_arms as usize + 1
}

fn rust_switch(default: bool) -> String {
    let mut arms = String::new();
    for i in 0..switch_arms() {
        arms.push_str(&format!("        \"k{i}\" => {{}}\n"));
    }
    let tail = if default { "        _ => {}\n" } else { "" };
    format!("fn f(s: &str) {{\n    match s {{\n{arms}{tail}    }}\n}}\n")
}

fn go_switch(default: bool) -> String {
    let mut arms = String::new();
    for i in 0..switch_arms() {
        arms.push_str(&format!("    case \"k{i}\":\n        x()\n"));
    }
    let tail = if default { "    default:\n        x()\n" } else { "" };
    format!("package main\nfunc f(s string) {{\n    switch s {{\n{arms}{tail}    }}\n}}\n")
}

fn ts_switch(default: bool) -> String {
    let mut arms = String::new();
    for i in 0..switch_arms() {
        arms.push_str(&format!("    case \"k{i}\": break;\n"));
    }
    let tail = if default { "    default: break;\n" } else { "" };
    format!("function f(s: string) {{\n  switch (s) {{\n{arms}{tail}  }}\n}}\n")
}

fn py_switch(default: bool) -> String {
    let mut arms = String::new();
    for i in 0..switch_arms() {
        arms.push_str(&format!("        case \"k{i}\":\n            pass\n"));
    }
    let tail = if default { "        case _:\n            pass\n" } else { "" };
    format!("def f(s):\n    match s:\n{arms}{tail}")
}

#[test]
fn stringly_typed_switch_fires_without_default() {
    for (code, ext) in [
        (rust_switch(false), "rs"),
        (go_switch(false), "go"),
        (ts_switch(false), "ts"),
        (py_switch(false), "py"),
    ] {
        assert!(
            has_smell(&pulse_check_code(&code, ext), "Stringly-Typed"),
            "{ext} string switch without a default should fire, got: {code}"
        );
    }
}

#[test]
fn stringly_typed_switch_suppressed_with_default() {
    for (code, ext) in [
        (rust_switch(true), "rs"),
        (go_switch(true), "go"),
        (ts_switch(true), "ts"),
        (py_switch(true), "py"),
    ] {
        assert!(
            !has_smell(&pulse_check_code(&code, ext), "Stringly-Typed"),
            "{ext} string switch with a default is a closed domain, got: {code}"
        );
    }
}

fn rust_ctor(types: &[&str]) -> String {
    let params: Vec<String> =
        types.iter().enumerate().map(|(i, ty)| format!("a{i}: {ty}")).collect();
    format!("struct S {{}}\nimpl S {{\n    fn new({}) -> Self {{ S {{}} }}\n}}\n", params.join(", "))
}

#[test]
fn constructor_over_injection_fires_on_non_primitive_deps() {
    let n = t().analysis.constructor_dep_injection_min as usize;
    let deps: Vec<&str> = (0..n).map(|_| "Service").collect();
    let code = rust_ctor(&deps);
    assert!(
        has_smell(&pulse_check_code(&code, "rs"), "Constructor Over-Injection"),
        "{n} non-primitive dependencies should fire below the arg threshold, got: {code}"
    );
}

#[test]
fn constructor_primitive_value_object_not_over_injection() {
    let n = t().function.constructor_arg_max as usize;
    let values: Vec<&str> = (0..n).map(|_| "i32").collect();
    let code = rust_ctor(&values);
    assert!(
        !has_smell(&pulse_check_code(&code, "rs"), "Constructor Over-Injection"),
        "{n} primitive value params (at the arg threshold) is a value object, not DI, got: {code}"
    );
}

#[test]
fn constructor_over_threshold_args_still_fires() {
    let n = t().function.constructor_arg_max as usize + 1;
    let values: Vec<&str> = (0..n).map(|_| "i32").collect();
    let code = rust_ctor(&values);
    assert!(
        has_smell(&pulse_check_code(&code, "rs"), "Constructor Over-Injection"),
        "{n} params exceeds the arg threshold, got: {code}"
    );
}

fn parse_functions(code: &str, ext: &str) -> Vec<pulse::walk::FunctionMetrics> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(format!("dup.{ext}"));
    std::fs::write(&path, code).unwrap();
    let lang = pulse::parse::detect_language(&path).expect("language");
    let source = std::fs::read_to_string(&path).unwrap();
    pulse::parse::parse_and_walk(&source, lang).expect("parse").functions
}

fn has_exact_duplication(code: &str, ext: &str) -> bool {
    let fns = parse_functions(code, ext);
    let mut findings = Vec::new();
    pulse::duplication::detect_code_duplication(&fns, &t(), &mut findings);
    findings.iter().any(|f| f.smell == pulse::smells::Smell::CodeDuplication)
}

const TRIVIAL_CLONE_RS: &str = "fn alpha() {\n    {}\n    {}\n    {}\n    {}\n    {}\n}\nfn beta() {\n    {}\n    {}\n    {}\n    {}\n    {}\n}\n";

const RICH_CLONE_RS: &str = "fn gamma(n: i32) -> i32 {\n    let mut total = n;\n    for i in total..n {\n        if i > total {\n            total += i;\n        }\n    }\n    total\n}\nfn delta(n: i32) -> i32 {\n    let mut total = n;\n    for i in total..n {\n        if i > total {\n            total += i;\n        }\n    }\n    total\n}\n";

#[test]
fn exact_clone_of_trivial_functions_suppressed_by_distinct_kind_floor() {
    assert!(
        !has_exact_duplication(TRIVIAL_CLONE_RS, "rs"),
        "structurally trivial clones (few distinct node kinds) should not be flagged"
    );
}

#[test]
fn exact_clone_of_rich_functions_still_detected() {
    assert!(
        has_exact_duplication(RICH_CLONE_RS, "rs"),
        "structurally rich clones must remain detected after the floor"
    );
}
