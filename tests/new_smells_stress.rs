mod common;

use common::*;

fn check(code: &str, ext: &str) -> String { pulse_check_code(code, ext) }
fn dbg(code: &str, ext: &str) -> String { pulse_debug_code(code, ext) }

// ===========================================================================
// Large Struct — Rust
// ===========================================================================

#[test]
fn rust_large_struct_detected() {
    let mut code = String::from("struct Big {\n");
    for i in 0..struct_fields_above() { code.push_str(&format!("    f{i}: i32,\n")); }
    code.push_str("}\n");
    let out = check(&code, "rs");
    assert!(has_smell(&out, "Large Struct"), "{} fields should trigger, got: {}", struct_fields_above(), out);
}

#[test]
fn rust_small_struct_clean() {
    let out = check("struct Pt { x: f64, y: f64 }\n", "rs");
    assert!(!has_smell(&out, "Large Struct"));
}

#[test]
fn rust_struct_at_threshold_not_flagged() {
    let mut code = String::from("struct Exact {\n");
    for i in 0..struct_fields_at() { code.push_str(&format!("    f{i}: i32,\n")); }
    code.push_str("}\n");
    let out = check(&code, "rs");
    assert!(!has_smell(&out, "Large Struct"), "{} fields = threshold, should NOT flag", struct_fields_at());
}

#[test]
fn rust_struct_field_count_in_debug() {
    let mut code = String::from("struct S {\n");
    for i in 0..8 { code.push_str(&format!("    f{i}: u32,\n")); }
    code.push_str("}\nfn f() {}\n");
    let out = dbg(&code, "rs");
    assert!(out.contains("Module:"), "debug should work");
}

// ===========================================================================
// Short Variable Names — Python
// ===========================================================================

#[test]
fn python_short_vars_detected() {
    let mut code = String::from("def func():\n");
    for c in "abcdefgh".chars() { code.push_str(&format!("    {c} = 1\n")); }
    for i in 0..t().analysis.short_var_min_fn_loc as usize { code.push_str(&format!("    var{i} = {i}\n")); }
    code.push_str("    return 0\n");
    assert!(has_smell(&check(&code, "py"), "Short Variable Names"));
}

#[test]
fn python_short_vars_exempt_ijk() {
    let mut code = String::from("def func():\n    i = 0\n    j = 0\n    k = 0\n");
    for n in 0..t().analysis.short_var_min_fn_loc as usize { code.push_str(&format!("    x{n} = {n}\n")); }
    code.push_str("    return 0\n");
    assert!(!has_smell(&check(&code, "py"), "Short Variable Names"), "exempt vars should not trigger");
}

#[test]
fn python_short_vars_below_loc_threshold_clean() {
    let out = check("def f():\n    a = 1\n    b = 2\n    c = 3\n    d = 4\n    return a\n", "py");
    assert!(!has_smell(&out, "Short Variable Names"), "short func should not flag");
}

#[test]
fn python_short_vars_count_in_debug() {
    let mut code = String::from("def func():\n");
    for c in "abcdef".chars() { code.push_str(&format!("    {c} = 1\n")); }
    for i in 0..t().analysis.short_var_min_fn_loc as usize { code.push_str(&format!("    x{i} = {i}\n")); }
    code.push_str("    return 0\n");
    let out = dbg(&code, "py");
    let sv = function_metric(&out, "func", "short_vars").unwrap_or(0);
    assert!(sv >= 6, "should count 6 short vars, got: {sv}");
}

// ===========================================================================
// Short Variable Names — Rust
// ===========================================================================

#[test]
fn rust_short_vars_detected() {
    let mut code = String::from("fn func() -> i32 {\n");
    for c in "abcdefgh".chars() { code.push_str(&format!("    let {c} = 1;\n")); }
    for i in 0..t().analysis.short_var_min_fn_loc as usize { code.push_str(&format!("    let v{i} = {i};\n")); }
    code.push_str("    0\n}\n");
    assert!(has_smell(&check(&code, "rs"), "Short Variable Names"));
}

#[test]
fn rust_short_vars_exempt_loop_counter() {
    let mut code = String::from("fn func() -> i32 {\n    let i = 0;\n    let j = 0;\n    let k = 0;\n");
    for n in 0..t().analysis.short_var_min_fn_loc as usize { code.push_str(&format!("    let v{n} = {n};\n")); }
    code.push_str("    0\n}\n");
    assert!(!has_smell(&check(&code, "rs"), "Short Variable Names"));
}

// ===========================================================================
// Short Variable Names — TypeScript
// ===========================================================================

#[test]
fn ts_short_vars_detected() {
    let mut code = String::from("function func(): number {\n");
    for c in "abcdefgh".chars() { code.push_str(&format!("    let {c} = 1;\n")); }
    for i in 0..t().analysis.short_var_min_fn_loc as usize { code.push_str(&format!("    let v{i} = {i};\n")); }
    code.push_str("    return 0;\n}\n");
    assert!(has_smell(&check(&code, "ts"), "Short Variable Names"));
}

// ===========================================================================
// Short Variable Names — Go
// ===========================================================================

#[test]
fn go_short_vars_detected() {
    let mut code = String::from("package main\n\nfunc process() int {\n");
    for c in "abcdefgh".chars() { code.push_str(&format!("    {c} := 1\n")); }
    for i in 0..t().analysis.short_var_min_fn_loc as usize { code.push_str(&format!("    v{i} := {i}\n")); }
    code.push_str("    return 0\n}\n");
    assert!(has_smell(&check(&code, "go"), "Short Variable Names"));
}

// ===========================================================================
// Short Variable Names — Java
// ===========================================================================

#[test]
fn java_short_vars_detected() {
    let mut code = String::from("class T {\n    int func() {\n");
    for c in "abcdefgh".chars() { code.push_str(&format!("        int {c} = 1;\n")); }
    for i in 0..t().analysis.short_var_min_fn_loc as usize { code.push_str(&format!("        int v{i} = {i};\n")); }
    code.push_str("        return 0;\n    }\n}\n");
    assert!(has_smell(&check(&code, "java"), "Short Variable Names"));
}

// ===========================================================================
// Short Variable Names — C
// ===========================================================================

#[test]
fn c_short_vars_detected() {
    let mut code = String::from("int func() {\n");
    for c in "abcdefgh".chars() { code.push_str(&format!("    int {c} = 1;\n")); }
    for i in 0..t().analysis.short_var_min_fn_loc as usize { code.push_str(&format!("    int v{i} = {i};\n")); }
    code.push_str("    return 0;\n}\n");
    assert!(has_smell(&check(&code, "c"), "Short Variable Names"));
}

// ===========================================================================
// Short Variable Names — Swift
// ===========================================================================

#[test]
fn swift_short_vars_detected() {
    let mut code = String::from("func process() -> Int {\n");
    for c in "abcdefgh".chars() { code.push_str(&format!("    let {c} = 1\n")); }
    for i in 0..t().analysis.short_var_min_fn_loc as usize { code.push_str(&format!("    let v{i} = {i}\n")); }
    code.push_str("    return 0\n}\n");
    assert!(has_smell(&check(&code, "swift"), "Short Variable Names"));
}

// ===========================================================================
// Stringly-Typed Switch — Rust
// ===========================================================================

#[test]
fn rust_string_match_detected() {
    let code = r#"fn dispatch(cmd: &str) -> i32 {
    match cmd {
        "alpha" => 1, "beta" => 2, "gamma" => 3,
        "delta" => 4, "epsilon" => 5, "zeta" => 6,
        _ => 0,
    }
}"#;
    assert!(has_smell(&check(code, "rs"), "Stringly-Typed Switch"));
}

#[test]
fn rust_string_match_below_threshold_clean() {
    let code = r#"fn small(s: &str) -> i32 {
    match s { "a" => 1, "b" => 2, _ => 0 }
}"#;
    assert!(!has_smell(&check(code, "rs"), "Stringly-Typed"));
}

#[test]
fn rust_string_match_count_in_debug() {
    let code = r#"fn dispatch(cmd: &str) -> i32 {
    match cmd {
        "a" => 1, "b" => 2, "c" => 3,
        "d" => 4, "e" => 5, "f" => 6, "g" => 7,
        _ => 0,
    }
}"#;
    let out = dbg(code, "rs");
    let arms = function_metric(&out, "dispatch", "str_match").unwrap_or(0);
    assert!(arms >= 7, "should count 7 string arms, got: {arms}");
}

// ===========================================================================
// Stringly-Typed Switch — Go
// ===========================================================================

#[test]
fn go_string_switch_detected() {
    let code = concat!(
        "package main\n\nfunc dispatch(cmd string) int {\n",
        "    switch cmd {\n",
        "    case \"a\":\n        return 1\n",
        "    case \"b\":\n        return 2\n",
        "    case \"c\":\n        return 3\n",
        "    case \"d\":\n        return 4\n",
        "    case \"e\":\n        return 5\n",
        "    case \"f\":\n        return 6\n",
        "    default:\n        return 0\n",
        "    }\n",
        "}\n",
    );
    assert!(has_smell(&check(code, "go"), "Stringly-Typed Switch"));
}

// ===========================================================================
// Stringly-Typed Switch — Swift
// ===========================================================================

#[test]
fn swift_string_switch_detected() {
    let code = concat!(
        "func dispatch(cmd: String) -> Int {\n",
        "    switch cmd {\n",
        "    case \"a\":\n        return 1\n",
        "    case \"b\":\n        return 2\n",
        "    case \"c\":\n        return 3\n",
        "    case \"d\":\n        return 4\n",
        "    case \"e\":\n        return 5\n",
        "    case \"f\":\n        return 6\n",
        "    default:\n        return 0\n",
        "    }\n",
        "}\n",
    );
    assert!(has_smell(&check(code, "swift"), "Stringly-Typed Switch"));
}

// ===========================================================================
// Threshold values visible in findings
// ===========================================================================

#[test]
fn large_struct_shows_threshold() {
    let mut code = String::from("struct Big {\n");
    for i in 0..struct_fields_above() { code.push_str(&format!("    f{i}: i32,\n")); }
    code.push_str("}\n");
    assert!(check(&code, "rs").contains(&format!("threshold: {}", t().module.max_struct_fields)));
}

#[test]
fn short_vars_shows_threshold() {
    let mut code = String::from("def func():\n");
    for c in "abcdefgh".chars() { code.push_str(&format!("    {c} = 1\n")); }
    for i in 0..t().analysis.short_var_min_fn_loc as usize { code.push_str(&format!("    x{i} = {i}\n")); }
    code.push_str("    return 0\n");
    assert!(check(&code, "py").contains(&format!("threshold: {}", t().analysis.short_var_max_count)));
}

#[test]
fn stringly_typed_shows_threshold() {
    let code = r#"fn d(s: &str) -> i32 {
    match s { "a" => 1, "b" => 2, "c" => 3, "d" => 4, "e" => 5, "f" => 6, _ => 0 }
}"#;
    assert!(check(code, "rs").contains(&format!("threshold: {}", t().analysis.max_string_match_arms)));
}

// ===========================================================================
// Negative stress: boundary values
// ===========================================================================

#[test]
fn struct_at_12_fields_not_flagged() {
    let mut code = String::from("struct S {\n");
    for i in 0..struct_fields_at() { code.push_str(&format!("    f{i}: i32,\n")); }
    code.push_str("}\n");
    assert!(!has_smell(&check(&code, "rs"), "Large Struct"));
}

#[test]
fn struct_at_13_fields_flagged() {
    let mut code = String::from("struct S {\n");
    for i in 0..=struct_fields_at() { code.push_str(&format!("    f{i}: i32,\n")); }
    code.push_str("}\n");
    assert!(has_smell(&check(&code, "rs"), "Large Struct"));
}

#[test]
fn short_vars_at_3_not_flagged() {
    let mut code = String::from("def func():\n    a = 1\n    b = 2\n    c = 3\n");
    for i in 0..t().analysis.short_var_min_fn_loc as usize { code.push_str(&format!("    v{i} = {i}\n")); }
    code.push_str("    return 0\n");
    assert!(!has_smell(&check(&code, "py"), "Short Variable Names"), "3 short vars = threshold, should NOT flag");
}

#[test]
fn short_vars_at_4_flagged() {
    let mut code = String::from("def func():\n    a = 1\n    b = 2\n    c = 3\n    d = 4\n");
    for i in 0..t().analysis.short_var_min_fn_loc as usize { code.push_str(&format!("    v{i} = {i}\n")); }
    code.push_str("    return 0\n");
    assert!(has_smell(&check(&code, "py"), "Short Variable Names"), "4 short vars > threshold, should flag");
}

#[test]
fn string_switch_at_5_not_flagged() {
    let code = r#"fn d(s: &str) -> i32 {
    match s { "a" => 1, "b" => 2, "c" => 3, "d" => 4, "e" => 5, _ => 0 }
}"#;
    assert!(!has_smell(&check(code, "rs"), "Stringly-Typed"), "5 arms = threshold, should NOT flag");
}

#[test]
fn string_switch_at_6_flagged() {
    let code = r#"fn d(s: &str) -> i32 {
    match s { "a" => 1, "b" => 2, "c" => 3, "d" => 4, "e" => 5, "f" => 6, _ => 0 }
}"#;
    assert!(has_smell(&check(code, "rs"), "Stringly-Typed"), "6 arms > threshold, should flag");
}
