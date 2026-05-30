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
