use pulse::parse::{self, Language};
use pulse::smells::{self, Finding};

use crate::common::t;

fn detect_all(code: &str, lang: Language) -> Vec<Finding> {
    let metrics = parse::parse_and_walk_guarded(code, lang).expect("fixture must parse");
    smells::detect(&metrics, code, &t())
}

fn assert_silent(code: &str, lang: Language, label: &str) {
    let findings = detect_all(code, lang);
    assert!(findings.is_empty(), "{label} must produce zero findings, got: {findings:#?}");
}

#[test]
fn maximal_legal_function_is_silent() {
    let th = t();
    let mut code =
        format!("def near({}):\n", (0..th.function.arg_max).map(|i| format!("a{i}")).collect::<Vec<_>>().join(", "));
    for d in 0..(th.function.nesting_depth - 1) {
        code.push_str(&"    ".repeat(d as usize + 1));
        code.push_str("if a0:\n");
    }
    code.push_str(&"    ".repeat(th.function.nesting_depth as usize));
    code.push_str("val = 0\n");
    code.push_str("    if a0 and a1 and a2:\n        val = 1\n");
    for i in 0..th.analysis.consecutive_asserts_max {
        code.push_str(&format!("    assert a0 == {i}\n"));
    }
    for name in ["b", "c", "d"].iter().take(th.analysis.short_var_max_count as usize) {
        code.push_str(&format!("    {name} = 0\n"));
    }
    let body_line_count = code.lines().count() as u32;
    for i in body_line_count..(th.function.fn_loc_warning - 1) {
        code.push_str(&format!("    pad{i} = 0\n"));
    }
    let metrics = parse::parse_and_walk_guarded(&code, Language::Python).unwrap();
    let f = &metrics.functions[0];
    assert!(f.cc < th.function.cc_warning, "fixture cc must stay legal, got {}", f.cc);
    assert_eq!(f.loc, th.function.fn_loc_warning - 1, "fixture loc calibration");
    assert_silent(&code, Language::Python, "maximal legal function");
}

#[test]
fn maximal_legal_module_is_silent() {
    let th = t();
    let mut code = String::new();
    for i in 0..th.module.max_declarations {
        code.push_str(&format!("class K{i}:\n    n = {i}\n"));
    }
    for i in 0..th.module.file_function_count {
        code.push_str(&format!("def g{i}(v):\n    return v + {i}\n"));
    }
    let used = code.lines().count() as u32;
    for i in used..(th.module.file_loc_warning - 1) {
        code.push_str(&format!("x{i} = {i}\n"));
    }
    let metrics = parse::parse_and_walk_guarded(&code, Language::Python).unwrap();
    assert_eq!(metrics.module.total_loc, th.module.file_loc_warning - 1, "module loc calibration");
    assert_eq!(metrics.module.total_functions, th.module.file_function_count, "fn count calibration");
    assert!(metrics.module.sum_cc <= th.module.file_total_cc, "total cc must stay legal");
    assert_silent(&code, Language::Python, "maximal legal module");
}

#[test]
fn maximal_legal_class_and_clones_are_silent() {
    let th = t();
    let dup_loc = th.analysis.duplication.min_loc - 1;
    let mut body = String::new();
    for i in 0..(dup_loc - 2) {
        body.push_str(&format!("    v{i} = x + {i}\n"));
    }
    body.push_str("    return x\n");
    let mut code = format!("def first(x):\n{body}def second(x):\n{body}");
    code.push_str("class C:\n");
    code.push_str("    def shared_a(self):\n        self.g0 = 1\n");
    code.push_str("    def shared_b(self):\n        return self.g0\n");
    for g in 1..(th.analysis.lcom4_warning - 1) {
        code.push_str(&format!("    def lone{g}(self):\n        return self.g{g}\n"));
    }
    assert_silent(&code, Language::Python, "legal clones and cohesion");
}

#[test]
fn maximal_legal_struct_is_silent() {
    let th = t();
    let fields: Vec<String> = (0..th.module.max_struct_fields).map(|i| format!("    f{i}: u32,")).collect();
    let code = format!("struct S {{\n{}\n}}\n", fields.join("\n"));
    assert_silent(&code, Language::Rust, "maximal legal struct");
}
