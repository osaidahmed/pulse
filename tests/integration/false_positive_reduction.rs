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
    let arms: String = (0..switch_arms()).map(|i| format!("        \"k{i}\" => {{}}\n")).collect();
    let tail = if default { "        _ => {}\n" } else { "" };
    format!("fn f(s: &str) {{\n    match s {{\n{arms}{tail}    }}\n}}\n")
}

fn go_switch(default: bool) -> String {
    let arms: String = (0..switch_arms()).map(|i| format!("    case \"k{i}\":\n        x()\n")).collect();
    let tail = if default { "    default:\n        x()\n" } else { "" };
    format!("package main\nfunc f(s string) {{\n    switch s {{\n{arms}{tail}    }}\n}}\n")
}

fn ts_switch(default: bool) -> String {
    let arms: String = (0..switch_arms()).map(|i| format!("    case \"k{i}\": break;\n")).collect();
    let tail = if default { "    default: break;\n" } else { "" };
    format!("function f(s: string) {{\n  switch (s) {{\n{arms}{tail}  }}\n}}\n")
}

fn py_switch(default: bool) -> String {
    let arms: String =
        (0..switch_arms()).map(|i| format!("        case \"k{i}\":\n            pass\n")).collect();
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
