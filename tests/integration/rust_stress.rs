
use crate::common::*;
use std::process::Command;

lang_helpers!("rs");

// ===========================================================================
// CC counting precision
// ===========================================================================

#[test]
fn cc_counts_if() {
    let out = debug("fn f() {\n    if true {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_else_if() {
    let out = debug("fn f(x: i32) {\n    if x == 1 {} else if x == 2 {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_counts_for() {
    let out = debug("fn f() {\n    for x in 0..10 {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_while() {
    let out = debug("fn f() {\n    while true {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_loop() {
    let out = debug("fn f() {\n    loop { break; }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

#[test]
fn cc_counts_match_arms() {
    let out = debug("fn f(x: i32) {\n    match x {\n        1 => {},\n        2 => {},\n        3 => {},\n        _ => {},\n    }\n}\n");
    // base(1) + 3 non-wildcard arms = 4
    assert_eq!(function_metric(&out, "f", "cc"), Some(4));
}

#[test]
fn cc_counts_and_operator() {
    let out = debug("fn f(a: bool, b: bool) {\n    if a && b {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_counts_or_operator() {
    let out = debug("fn f(a: bool, b: bool) {\n    if a || b {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_nested_if_in_for() {
    let out = debug("fn f() {\n    for x in 0..10 {\n        if x > 5 {}\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(3));
}

#[test]
fn cc_chained_boolean() {
    let out = debug("fn f(a: bool, b: bool, c: bool, d: bool) {\n    if a && b && c && d {}\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(cc >= 4, "chained boolean should increase cc, got: {cc}");
}

// ===========================================================================
// Nesting depth precision
// ===========================================================================

#[test]
fn nesting_0_flat() {
    let out = debug("fn f() -> i32 {\n    42\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(0));
}

#[test]
fn nesting_1_single_if() {
    let out = debug("fn f() {\n    if true {}\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(1));
}

#[test]
fn nesting_2_nested_if() {
    let out = debug("fn f() {\n    if true {\n        if true {}\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(2));
}

#[test]
fn nesting_3_for_if_for() {
    let out = debug("fn f() {\n    if true {\n        for x in 0..1 {\n            if true {}\n        }\n    }\n}\n");
    assert_eq!(function_metric(&out, "f", "nesting"), Some(3));
}

// ===========================================================================
// Argument counting
// ===========================================================================

#[test]
fn args_counts_positional() {
    let out = debug("fn f(a: i32, b: i32, c: i32) {}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(3));
}

#[test]
fn args_zero() {
    let out = debug("fn f() {}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(0));
}

#[test]
fn args_self_excluded_in_method() {
    let out = debug("struct S;\nimpl S {\n    fn m(&self, a: i32, b: i32) {}\n}\n");
    assert_eq!(function_metric(&out, "S.m", "args"), Some(2));
}

#[test]
fn args_mut_self_excluded() {
    let out = debug("struct S;\nimpl S {\n    fn m(&mut self, a: i32) {}\n}\n");
    assert_eq!(function_metric(&out, "S.m", "args"), Some(1));
}

// ===========================================================================
// Primitive obsession
// ===========================================================================

#[test]
fn primitive_obsession_all_primitives() {
    let out = check("fn f(a: i32, b: u64, c: f32, d: bool, e: i32) {}\n");
    assert!(has_smell(&out, "Primitive Obsession"));
}

#[test]
fn primitive_obsession_mixed_below_threshold() {
    let out = check("fn f(a: i32, b: MyStruct, c: OtherType, d: Vec<u8>) {}\n");
    assert!(!has_smell(&out, "Primitive Obsession"));
}

#[test]
fn primitive_obsession_reference_types() {
    let out = check("fn f(a: &str, b: &str, c: &str, d: &str) {}\n");
    assert!(has_smell(&out, "Primitive Obsession"));
}

// ===========================================================================
// LCOM4
// ===========================================================================

#[test]
fn lcom4_cohesive_struct_not_flagged() {
    let out = check("struct S { data: Vec<i32> }\nimpl S {\n    fn add(&mut self, x: i32) { self.data.push(x); }\n    fn get(&self) -> &[i32] { &self.data }\n    fn clear(&mut self) { self.data.clear(); }\n}\n");
    assert!(!has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_three_disconnected_groups() {
    let out = check(concat!(
        "struct M { x: i32, y: i32, z: i32 }\n",
        "impl M {\n",
        "    fn use_x(&self) -> i32 { self.x }\n",
        "    fn use_y(&self) -> i32 { self.y }\n",
        "    fn use_z(&self) -> i32 { self.z }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_transitive_connection() {
    let out = check(concat!(
        "struct C { a: i32, b: i32 }\n",
        "impl C {\n",
        "    fn m1(&self) -> i32 { self.a }\n",
        "    fn m2(&self) -> i32 { self.a + self.b }\n",
        "    fn m3(&self) -> i32 { self.b }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_methods_connected_by_call() {
    let out = check(concat!(
        "struct Coord { state: i32 }\n",
        "impl Coord {\n",
        "    fn process(&self, e: i32) -> bool { self.validate(e) && self.dispatch(e) }\n",
        "    fn validate(&self, e: i32) -> bool { e > 0 }\n",
        "    fn dispatch(&self, e: i32) -> bool { self.send(e) }\n",
        "    fn send(&self, _e: i32) -> bool { true }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"), "got: {out}");
}

#[test]
fn lcom4_mixed_field_and_call_connection() {
    let out = check(concat!(
        "struct Mixed { x: i32 }\n",
        "impl Mixed {\n",
        "    fn a(&self) -> i32 { self.x }\n",
        "    fn b(&mut self) -> i32 { self.x = 1; self.c() }\n",
        "    fn c(&self) -> i32 { 42 }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"));
}

#[test]
fn lcom4_god_struct_still_fires() {
    let out = check(concat!(
        "struct Svc { db: i32, cache: i32, mailer: i32, events: i32, audit: i32 }\n",
        "impl Svc {\n",
        "    fn get_user(&self) -> i32 { self.db }\n",
        "    fn cache_user(&self) -> i32 { self.cache }\n",
        "    fn send_welcome(&self) -> i32 { self.mailer }\n",
        "    fn publish(&self) -> i32 { self.events }\n",
        "    fn audit_log(&self) -> i32 { self.audit }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"), "got: {out}");
}

#[test]
fn lcom4_dependency_method_calls_dont_falsely_connect() {
    let out = check(concat!(
        "struct Db; impl Db { fn foo(&self) -> i32 { 0 } }\n",
        "struct Cache; impl Cache { fn foo(&self) -> i32 { 0 } }\n",
        "struct Log; impl Log { fn foo(&self) -> i32 { 0 } }\n",
        "struct Svc { db: Db, cache: Cache, log: Log }\n",
        "impl Svc {\n",
        "    fn a(&self) -> i32 { self.db.foo() }\n",
        "    fn b(&self) -> i32 { self.cache.foo() }\n",
        "    fn c(&self) -> i32 { self.log.foo() }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"));
}

// ===========================================================================
// Code duplication
// ===========================================================================

#[test]
fn duplication_detected() {
    let out = check(concat!(
        "fn report_a(data: &[Item]) -> Vec<Entry> {\n",
        "    let mut r = Vec::new();\n",
        "    for item in data {\n",
        "        let e = Entry { id: item.id, name: item.name.clone(), val: item.val };\n",
        "        r.push(e);\n",
        "    }\n",
        "    r\n",
        "}\n\n",
        "fn report_b(data: &[Item]) -> Vec<Entry> {\n",
        "    let mut r = Vec::new();\n",
        "    for item in data {\n",
        "        let e = Entry { id: item.id, name: item.name.clone(), val: item.val };\n",
        "        r.push(e);\n",
        "    }\n",
        "    r\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Code Duplication"));
}

#[test]
fn duplication_test_functions_suppressed() {
    let out = check(concat!(
        "fn test_a() {\n    let r = compute();\n    assert_eq!(r.id, 1);\n    assert_eq!(r.name, \"a\");\n    assert_eq!(r.val, 10);\n    assert!(r.ok);\n}\n\n",
        "fn test_b() {\n    let r = compute();\n    assert_eq!(r.id, 1);\n    assert_eq!(r.name, \"a\");\n    assert_eq!(r.val, 10);\n    assert!(r.ok);\n}\n",
    ));
    assert!(!has_smell(&out, "Code Duplication"));
}

// ===========================================================================
// Declarations
// ===========================================================================

#[test]
fn declarations_above_threshold() {
    let mut code = String::new();
    for i in 0..declarations_above() {
        code.push_str(&format!("struct T{i} {{}}\n"));
    }
    let out = check(&code);
    assert!(has_smell(&out, "Declarations"));
}

// ===========================================================================
// God Class / God Method
// ===========================================================================

#[test]
fn god_method_detected() {
    let mut code = String::from("fn monster() {\n");
    for i in 0..cc_branches() {
        code.push_str(&format!("    if {i} > 0 {{}}\n"));
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("    let x{i} = {i};\n"));
    }
    code.push_str("}\n");
    let out = check(&code);
    assert!(has_smell(&out, "God Method"));
}

// ===========================================================================
// Overall function size
// ===========================================================================

#[test]
fn overall_function_size_triggered() {
    let mut code = String::new();
    for i in 0..t().module.large_fn_count as usize {
        code.push_str(&format!("fn lg{i}() {{\n"));
        for j in 0..large_fn_lines() {
            code.push_str(&format!("    let x{j} = {j};\n"));
        }
        code.push_str("}\n\n");
    }
    let out = check(&code);
    assert!(has_smell(&out, "Overall Function Size"));
}

// ===========================================================================
// Deep nesting
// ===========================================================================

#[test]
fn deep_nesting_detected() {
    let out = check("fn deep() {\n    for x in 0..1 {\n        if true {\n            for y in 0..1 {\n                if true {\n                    for z in 0..1 {\n                        if true {}\n                    }\n                }\n            }\n        }\n    }\n}\n");
    assert!(has_smell(&out, "Deep Nested"));
}

// ===========================================================================
// Constructor vs excess args
// ===========================================================================

#[test]
fn constructor_reports_over_injection() {
    let out = check("struct S {}\nimpl S {\n    fn new(a: i32, b: i32, c: i32, d: i32, e: i32, f: i32) -> Self { S {} }\n}\n");
    assert!(has_smell(&out, "Constructor Over-Injection"));
}

#[test]
fn regular_function_reports_excess_args() {
    let out = check("fn f(a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32) {}\n");
    assert!(has_smell(&out, "Excess Arguments"));
    assert!(!has_smell(&out, "Constructor Over-Injection"));
}

// ===========================================================================
// Embedded block
// ===========================================================================

#[test]
fn embedded_block_detected() {
    let mut code = String::from("fn query() -> &'static str {\n    let q = r#\"\n");
    for i in 0..embedded_lines_above() {
        code.push_str(&format!("        SELECT field_{i} FROM table_{i}\n"));
    }
    code.push_str("    \"#;\n    q\n}\n");
    let out = check(&code);
    assert!(has_smell(&out, "Large Embedded Block"));
}

// ===========================================================================
// Global nesting
// ===========================================================================

#[test]
fn global_nesting_not_common_in_rust() {
    // Rust rarely has global conditionals — this is correct behavior
    let out = check("const X: i32 = 42;\nfn main() {}\n");
    assert!(!has_smell(&out, "Global Conditionals"));
    assert!(!has_smell(&out, "Deep Global Nesting"));
}

// ===========================================================================
// Performance
// ===========================================================================

#[test]
fn performance_1000_loc() {
    let mut code = String::new();
    for i in 0..50 {
        code.push_str(&format!(
            "fn func{i}(data: &Data) -> Result<(), Error> {{\n"
        ));
        for j in 0..18 {
            code.push_str(&format!("    let f{j} = data.field{j};\n"));
        }
        code.push_str("    Ok(())\n}\n\n");
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("big.rs");
    std::fs::write(&path, &code).unwrap();
    let start = std::time::Instant::now();
    let _ = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let elapsed = start.elapsed();
    assert!(elapsed.as_millis() < 500, "took: {}ms", elapsed.as_millis());
}

#[test]
fn performance_impl_blocks() {
    let mut code = String::new();
    for i in 0..10 {
        code.push_str(&format!(
            "struct S{i} {{ data: Vec<i32> }}\nimpl S{i} {{\n"
        ));
        for j in 0..5 {
            code.push_str(&format!(
                "    fn m{j}(&self) -> &[i32] {{ &self.data }}\n"
            ));
        }
        code.push_str("}\n\n");
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("impls.rs");
    std::fs::write(&path, &code).unwrap();
    let start = std::time::Instant::now();
    let _ = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let elapsed = start.elapsed();
    assert!(elapsed.as_millis() < 500, "took: {}ms", elapsed.as_millis());
}

// ===========================================================================
// CC: loop keyword
// ===========================================================================

#[test]
fn cc_counts_loop_keyword() {
    let out = debug("fn f() {\n    loop { break; }\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

// ===========================================================================
// Nesting: match depth
// ===========================================================================

#[test]
fn nesting_match_counts_depth() {
    let out = debug("fn f(x: i32) {\n    match x {\n        1 => {\n            if true {}\n        },\n        _ => {},\n    }\n}\n");
    let depth = function_metric(&out, "f", "nesting").unwrap_or(0);
    assert!(depth >= 2, "match+if should be >= 2, got: {depth}");
}

// ===========================================================================
// Args: typed params (all Rust params are typed)
// ===========================================================================

#[test]
fn args_typed_params() {
    let out = debug("fn f(a: i32, b: String, c: f64) {}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(3));
}

#[test]
fn args_typed_with_defaults_via_option() {
    let out = debug("fn f(a: Option<i32>, b: Option<String>) {}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(2));
}

// ===========================================================================
// Primitive obsession: below min typed
// ===========================================================================

#[test]
fn primitive_obsession_below_min_typed() {
    let out = check("fn f(a: i32, b: u64, c: f32) {}\n");
    assert!(!has_smell(&out, "Primitive Obsession"));
}

#[test]
fn primitive_obsession_recognizes_usize_isize() {
    let out = check("fn f(a: usize, b: isize, c: u8, d: i16, e: usize) {}\n");
    assert!(has_smell(&out, "Primitive Obsession"));
}

// ===========================================================================
// LCOM4: single method not flagged
// ===========================================================================

#[test]
fn lcom4_single_method_not_flagged() {
    let out = check("struct T { x: i32 }\nimpl T {\n    fn get(&self) -> i32 { self.x }\n}\n");
    assert!(!has_smell(&out, "Low Cohesion"));
}

// ===========================================================================
// LCOM4: constructor excluded
// ===========================================================================

#[test]
fn lcom4_new_excluded() {
    let out = check(concat!(
        "struct Init { a: i32, b: i32, c: i32 }\n",
        "impl Init {\n",
        "    fn new() -> Self { Init { a: 1, b: 2, c: 3 } }\n",
        "    fn use_a(&self) -> i32 { self.a }\n",
        "    fn use_b(&self) -> i32 { self.b }\n",
        "    fn use_c(&self) -> i32 { self.c }\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Low Cohesion"));
}

// ===========================================================================
// LCOM4: 2 disconnected groups (below threshold)
// ===========================================================================

#[test]
fn lcom4_two_disconnected_groups_not_flagged() {
    let out = check(concat!(
        "struct Split { field_a: i32, field_b: i32 }\n",
        "impl Split {\n",
        "    fn a_work(&self) -> i32 { self.field_a }\n",
        "    fn a_read(&self) -> i32 { self.field_a + 1 }\n",
        "    fn b_work(&self) -> i32 { self.field_b }\n",
        "    fn b_read(&self) -> i32 { self.field_b + 1 }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"));
}

// ===========================================================================
// Duplication: test functions suppressed
// ===========================================================================

#[test]
fn duplication_test_suppressed() {
    let out = check(concat!(
        "fn test_a() {\n    let r = compute();\n    assert_eq!(r.id, 1);\n    assert_eq!(r.name, \"a\");\n    assert_eq!(r.val, 10);\n    assert!(r.ok);\n}\n\n",
        "fn test_b() {\n    let r = compute();\n    assert_eq!(r.id, 1);\n    assert_eq!(r.name, \"a\");\n    assert_eq!(r.val, 10);\n    assert!(r.ok);\n}\n",
    ));
    assert!(!has_smell(&out, "Code Duplication"));
}

// ===========================================================================
// God class: requires god method
// ===========================================================================

#[test]
fn god_class_requires_god_method() {
    let mut code = String::new();
    for i in 0..declarations_above() {
        code.push_str(&format!("fn fn{i}() -> i32 {{ {i} }}\n"));
    }
    for i in 0..file_padding() {
        code.push_str(&format!("const VAR{i}: i32 = {i};\n"));
    }
    let out = check(&code);
    assert!(!has_smell(&out, "God Class"));
}

// ===========================================================================
// God class: triggers when large+many+god
// ===========================================================================

#[test]
fn god_class_triggers_with_god_method() {
    let mut code = String::from("fn monster() {\n");
    for i in 0..cc_branches() {
        code.push_str(&format!("    if {i} > 0 {{}}\n"));
    }
    for i in 0..fn_padding() {
        code.push_str(&format!("    let y{i} = {i};\n"));
    }
    code.push_str("}\n\n");
    for i in 0..functions_above() {
        code.push_str(&format!("fn fn{i}() -> i32 {{ {i} }}\n"));
    }
    for i in 0..file_padding() {
        code.push_str(&format!("const V{i}: i32 = {i};\n"));
    }
    let out = check(&code);
    assert!(has_smell(&out, "God Method"));
    assert!(has_smell(&out, "God Class"));
}

// ===========================================================================
// Assertion block edge cases
// ===========================================================================

#[test]
fn assertion_block_interrupted_resets() {
    let out = check(concat!(
        "fn test_interleaved() {\n",
        "    assert_eq!(x, 1);\n",
        "    assert_eq!(y, 2);\n",
        "    assert_eq!(z, 3);\n",
        "    do_something();\n",
        "    assert_eq!(a, 4);\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Large Assertion Block"));
}

#[test]
fn assertion_block_at_threshold() {
    let mut code = String::from("fn test_exact() {\n");
    for i in 0..asserts_at() {
        code.push_str(&format!("    assert_eq!(x{i}, {i});\n"));
    }
    code.push_str("}\n");
    let out = check(&code);
    assert!(!has_smell(&out, "Large Assertion Block"));
}

#[test]
fn assertion_block_above_threshold() {
    let mut code = String::from("fn test_big() {\n");
    for i in 0..asserts_above() {
        code.push_str(&format!("    assert_eq!(x{i}, {i});\n"));
    }
    code.push_str("}\n");
    let out = check(&code);
    assert!(has_smell(&out, "Large Assertion Block"));
}

// ===========================================================================
// Overall function size: below threshold
// ===========================================================================

#[test]
fn overall_function_size_below_threshold() {
    let mut code = String::new();
    for i in 0..(t().module.large_fn_count as usize - 1) {
        code.push_str(&format!("fn lg{i}() {{\n"));
        for j in 0..large_fn_lines() {
            code.push_str(&format!("    let x{j} = {j};\n"));
        }
        code.push_str("}\n\n");
    }
    let out = check(&code);
    assert!(!has_smell(&out, "Overall Function Size"));
}

// ===========================================================================
// Declarations: below threshold
// ===========================================================================

#[test]
fn declarations_below_threshold() {
    let mut code = String::new();
    for i in 0..10 {
        code.push_str(&format!("struct T{i} {{}}\n"));
    }
    let out = check(&code);
    assert!(!has_smell(&out, "Declarations"));
}

// ===========================================================================
// Embedded block: small not flagged
// ===========================================================================

#[test]
fn small_string_not_flagged_as_embedded() {
    let out = check("fn f() -> &'static str {\n    let x = \"hello world\";\n    x\n}\n");
    assert!(!has_smell(&out, "Large Embedded Block"));
}

#[test]
fn multiline_raw_string_flagged() {
    let mut code = String::from("fn f() -> &'static str {\n    let q = r#\"\n");
    for i in 0..embedded_lines_above() {
        code.push_str(&format!("        SELECT field_{i} FROM table_{i}\n"));
    }
    code.push_str("    \"#;\n    q\n}\n");
    let out = check(&code);
    assert!(has_smell(&out, "Large Embedded Block"));
}

// ===========================================================================
// Deep global nesting: not common in Rust
// ===========================================================================

#[test]
fn shallow_global_not_flagged() {
    let out = check("const X: i32 = 42;\nfn main() {}\n");
    assert!(!has_smell(&out, "Deep Global Nesting"));
}

// ===========================================================================
// Multiple smells on same function
// ===========================================================================

#[test]
fn function_can_have_multiple_smells() {
    let mut code = String::from(
        "fn terrible(a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32) {\n",
    );
    code.push_str("    let q = r#\"\n");
    for i in 0..embedded_lines_above() {
        code.push_str(&format!("        SELECT field_{i}\n"));
    }
    code.push_str("    \"#;\n");
    code.push_str("    for x in 0..1 {\n");
    code.push_str("        if true {\n");
    code.push_str("            for y in 0..1 {\n");
    code.push_str("                if true {\n");
    code.push_str("                    for z in 0..1 {\n");
    code.push_str("                        if true {}\n");
    code.push_str("                    }\n");
    code.push_str("                }\n");
    code.push_str("            }\n");
    code.push_str("        }\n");
    code.push_str("    }\n");
    code.push_str("}\n");
    let out = check(&code);
    assert!(has_smell(&out, "Excess Arguments"));
    assert!(has_smell(&out, "Large Embedded Block"));
    assert!(has_smell(&out, "Deep Nested"));
}

// ===========================================================================
// Hook JSON edge cases
// ===========================================================================

#[test]
fn hook_missing_tool_input() {
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["--hook"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child
                .stdin
                .take()
                .unwrap()
                .write_all(b"{\"other\": 1}")
                .unwrap();
            child.wait_with_output()
        })
        .expect("failed to run");
    assert!(out.stdout.is_empty());
}

#[test]
fn hook_missing_file_path_key() {
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["--hook"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child
                .stdin
                .take()
                .unwrap()
                .write_all(b"{\"tool_input\": {\"content\": \"hello\"}}")
                .unwrap();
            child.wait_with_output()
        })
        .expect("failed to run");
    assert!(out.stdout.is_empty());
}

#[test]
fn hook_empty_stdin() {
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["--hook"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            drop(child.stdin.take());
            child.wait_with_output()
        })
        .expect("failed to run");
    assert!(out.stdout.is_empty());
}

// ===========================================================================
// CC: not/! operator
// ===========================================================================

#[test]
fn cc_counts_not_operator() {
    let out = debug("fn f(a: bool) {\n    if !a {}\n}\n");
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(cc >= 2, "if !a should have cc >= 2, got: {cc}");
}

// ===========================================================================
// CC: while let
// ===========================================================================

#[test]
fn cc_counts_while_let() {
    let out = debug("fn f() {\n    while let Some(x) = iter.next() {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

// ===========================================================================
// CC: if let
// ===========================================================================

#[test]
fn cc_counts_if_let() {
    let out = debug("fn f(x: Option<i32>) {\n    if let Some(v) = x {}\n}\n");
    assert_eq!(function_metric(&out, "f", "cc"), Some(2));
}

// ===========================================================================
// Nesting: loop keyword
// ===========================================================================

#[test]
fn nesting_loop_counts_depth() {
    let out =
        debug("fn f() {\n    loop {\n        if true {\n            break;\n        }\n    }\n}\n");
    let depth = function_metric(&out, "f", "nesting").unwrap_or(0);
    assert!(depth >= 2, "loop+if should be >= 2, got: {depth}");
}

// ===========================================================================
// Args: self variants
// ===========================================================================

#[test]
fn args_self_owned_excluded() {
    let out = debug("struct S;\nimpl S {\n    fn consume(self, a: i32) {}\n}\n");
    assert_eq!(function_metric(&out, "S.consume", "args"), Some(1));
}

// ===========================================================================
// Duplication: async same body
// ===========================================================================

#[test]
fn duplication_async_same_body() {
    let out = check(concat!(
        "async fn fetch_a(url: &str) -> Result<(), ()> {\n",
        "    let r = get(url).await;\n",
        "    let status = r.status();\n",
        "    let body = r.text().await;\n",
        "    let parsed = parse(body);\n",
        "    Ok(())\n",
        "}\n\n",
        "async fn fetch_b(url: &str) -> Result<(), ()> {\n",
        "    let r = get(url).await;\n",
        "    let status = r.status();\n",
        "    let body = r.text().await;\n",
        "    let parsed = parse(body);\n",
        "    Ok(())\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Code Duplication"));
}

// ===========================================================================
// Duplication: mixed test+prod
// ===========================================================================

#[test]
fn duplication_mixed_test_and_prod_flagged() {
    let out = check(concat!(
        "fn test_something() {\n",
        "    let r = compute();\n",
        "    assert_eq!(r.id, 1);\n",
        "    assert_eq!(r.name, \"a\");\n",
        "    assert_eq!(r.val, 10);\n",
        "    assert!(r.ok);\n",
        "}\n\n",
        "fn process_data() {\n",
        "    let r = compute();\n",
        "    assert_eq!(r.id, 1);\n",
        "    assert_eq!(r.name, \"a\");\n",
        "    assert_eq!(r.val, 10);\n",
        "    assert!(r.ok);\n",
        "}\n",
    ));
    assert!(has_smell(&out, "Code Duplication"));
}

// ===========================================================================
// Decorated structs counted as declarations
// ===========================================================================

#[test]
fn decorated_structs_counted_as_declarations() {
    let mut code = String::new();
    for i in 0..declarations_above() {
        code.push_str(&format!("#[derive(Debug)]\nstruct T{i} {{}}\n"));
    }
    let out = check(&code);
    assert!(has_smell(&out, "Declarations"));
}

// ===========================================================================
// Real-world patterns: clean Rust module
// ===========================================================================

#[test]
fn clean_rust_module_not_flagged() {
    let out = check(concat!(
        "struct Config {\n",
        "    host: String,\n",
        "    port: u16,\n",
        "}\n\n",
        "impl Config {\n",
        "    fn new(host: String, port: u16) -> Self {\n",
        "        Config { host, port }\n",
        "    }\n",
        "    fn address(&self) -> String {\n",
        "        format!(\"{}:{}\", self.host, self.port)\n",
        "    }\n",
        "}\n",
    ));
    assert!(
        out.is_empty(),
        "clean Rust module should not be flagged, got: {out}"
    );
}

// ===========================================================================
// Comments only file
// ===========================================================================

#[test]
fn comments_only_file() {
    let out = check("// just comments\n// nothing else\n");
    assert!(out.is_empty());
}

// ===========================================================================
// Empty file inline
// ===========================================================================

#[test]
fn empty_file_inline() {
    let out = check("");
    assert!(out.is_empty());
}

// ===========================================================================
// CC: match with many arms
// ===========================================================================

#[test]
fn cc_match_many_arms() {
    let out = debug(concat!(
        "fn f(x: i32) -> &'static str {\n",
        "    match x {\n",
        "        1 => \"a\",\n",
        "        2 => \"b\",\n",
        "        3 => \"c\",\n",
        "        4 => \"d\",\n",
        "        5 => \"e\",\n",
        "        6 => \"f\",\n",
        "        7 => \"g\",\n",
        "        8 => \"h\",\n",
        "        _ => \"?\",\n",
        "    }\n",
        "}\n",
    ));
    let cc = function_metric(&out, "f", "cc").unwrap();
    assert!(
        cc >= 9,
        "8 match arms + base should give cc >= 9, got: {cc}"
    );
}

// ===========================================================================
// Constructor vs excess args: constructor reports injection
// ===========================================================================

#[test]
fn constructor_reports_injection_not_excess() {
    let out = check("struct S {}\nimpl S {\n    fn new(a: i32, b: i32, c: i32, d: i32, e: i32, f: i32) -> Self { S {} }\n}\n");
    assert!(has_smell(&out, "Constructor Over-Injection"));
    let lines: Vec<&str> = out.lines().filter(|l| l.contains("new")).collect();
    assert!(!lines.iter().any(|l| l.contains("Excess Arguments")));
}

// ===========================================================================
// Regular function reports excess, not constructor
// ===========================================================================

#[test]
fn regular_function_reports_excess_not_constructor() {
    let out = check("fn f(a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32) {}\n");
    assert!(has_smell(&out, "Excess Arguments"));
    assert!(!has_smell(&out, "Constructor Over-Injection"));
}

// ===========================================================================
// Overall function size: at threshold
// ===========================================================================

#[test]
fn overall_function_size_at_threshold() {
    let mut code = String::new();
    for i in 0..t().module.large_fn_count as usize {
        code.push_str(&format!("fn lg{i}() {{\n"));
        for j in 0..large_fn_lines() {
            code.push_str(&format!("    let x{j} = {j};\n"));
        }
        code.push_str("}\n\n");
    }
    let out = check(&code);
    assert!(has_smell(&out, "Overall Function Size"));
}

// ===========================================================================
// Global conditionals: no false positive
// ===========================================================================

#[test]
fn no_false_positive_global_conditionals() {
    let out = check("const X: i32 = 42;\nfn main() {}\n");
    assert!(!has_smell(&out, "Global Conditionals"));
}

// ===========================================================================
// Primitive obsession: untyped not counted (not applicable in Rust)
// ===========================================================================

#[test]
fn primitive_obsession_complex_types_not_flagged() {
    let out = check("fn f(a: Vec<i32>, b: HashMap<String, i32>, c: &[u8], d: MyStruct) {}\n");
    assert!(!has_smell(&out, "Primitive Obsession"));
}

// ===========================================================================
// LCOM4: all methods share field
// ===========================================================================

#[test]
fn lcom4_all_methods_share_field() {
    let out = check(concat!(
        "struct C { data: Vec<i32> }\n",
        "impl C {\n",
        "    fn add(&mut self, x: i32) { self.data.push(x); }\n",
        "    fn get(&self) -> &[i32] { &self.data }\n",
        "    fn clear(&mut self) { self.data.clear(); }\n",
        "}\n",
    ));
    assert!(!has_smell(&out, "Low Cohesion"));
}

// ===========================================================================
// Cognitive Complexity (CogC)
// ===========================================================================

#[test]
fn cogc_flat_branches() {
    let out = debug(concat!(
        "fn f(x: i32) {\n",
        "    if x == 1 {}\n",
        "    if x == 2 {}\n",
        "    if x == 3 {}\n",
        "    if x == 4 {}\n",
        "    if x == 5 {}\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cogc"), Some(5));
}

#[test]
fn cogc_nested_ifs() {
    let out = debug(concat!(
        "fn f(x: i32) {\n",
        "    if x > 0 {\n",
        "        if x > 1 {\n",
        "            if x > 2 {\n",
        "                if x > 3 {}\n",
        "            }\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cogc"), Some(10));
}

#[test]
fn cogc_else_if_no_nesting() {
    let out = debug(concat!(
        "fn f(x: i32) {\n",
        "    if x == 1 {\n",
        "    } else if x == 2 {\n",
        "    } else if x == 3 {\n",
        "    }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cogc"), Some(3));
}

#[test]
fn cogc_else_increases_nesting() {
    let out = debug(concat!(
        "fn f(x: i32) {\n",
        "    if x > 0 {\n",
        "    } else {\n",
        "        if x < -10 {}\n",
        "    }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cogc"), Some(4));
}

#[test]
fn cogc_match_counted() {
    let out = debug(concat!(
        "fn f(x: i32) {\n",
        "    match x {\n",
        "        1 => {},\n",
        "        _ => {},\n",
        "    }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cogc"), Some(1));
}

#[test]
fn cogc_for_loop_nested() {
    let out = debug(concat!(
        "fn f(x: i32) {\n",
        "    if x > 0 {\n",
        "        for i in 0..10 {}\n",
        "    }\n",
        "}\n",
    ));
    assert_eq!(function_metric(&out, "f", "cogc"), Some(3));
}

#[test]
fn cogc_triggers_complex_method() {
    let code = concat!(
        "fn f(x: i32) {\n",
        "    if x > 0 {\n",
        "        if x > 1 {\n",
        "            if x > 2 {\n",
        "                if x > 3 {\n",
        "                    if x > 4 {}\n",
        "                }\n",
        "            }\n",
        "        }\n",
        "    }\n",
        "}\n",
    );
    let out = check(code);
    let d = debug(code);
    let cogc = function_metric(&d, "f", "cogc").unwrap();
    let cc = function_metric(&d, "f", "cc").unwrap();
    assert!(cogc >= 15, "cogc should be >= 15, got: {cogc}");
    assert!(cc < 9, "cc should be < 9, got: {cc}");
    assert!(has_smell(&out, "Complex Method"));
}

// ===========================================================================
// Coverage: trait methods, edge cases
// ===========================================================================

#[test]
fn trait_method_without_body_skipped() {
    let out = debug("trait Foo {\n    fn bar(&self);\n}\nfn f() { if true {} }\n");
    assert!(!out.contains("bar"), "trait method without body should be skipped: {out}");
    assert!(out.contains('f'));
}
