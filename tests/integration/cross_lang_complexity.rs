use crate::common::*;

struct Variant {
    ext: &'static str,
    code: &'static str,
}

fn assert_uniform(concept: &str, variants: &[Variant]) {
    let mut cc_ref: Option<u32> = None;
    let mut cogc_ref: Option<u32> = None;
    for v in variants {
        let dbg = pulse_debug_code(v.code, v.ext);
        let cc = function_metric(&dbg, "f", "cc")
            .unwrap_or_else(|| panic!("{concept}/{}: no cc parsed from:\n{dbg}", v.ext));
        let cogc = function_metric(&dbg, "f", "cogc")
            .unwrap_or_else(|| panic!("{concept}/{}: no cogc parsed from:\n{dbg}", v.ext));
        match cc_ref {
            None => cc_ref = Some(cc),
            Some(r) => assert_eq!(cc, r, "{concept}: cc for {} is {cc}, expected {r}", v.ext),
        }
        match cogc_ref {
            None => cogc_ref = Some(cogc),
            Some(r) => assert_eq!(cogc, r, "{concept}: cogc for {} is {cogc}, expected {r}", v.ext),
        }
    }
}

#[test]
fn guard_ladder_then_loop_is_uniform() {
    assert_uniform(
        "guard_ladder_then_loop",
        &[
            Variant {
                ext: "py",
                code: "def f(a, b, items):\n    if a:\n        return 0\n    if b:\n        return 1\n    for x in items:\n        y = x\n    return 2\n",
            },
            Variant {
                ext: "rs",
                code: "fn f(a: bool, b: bool, items: Vec<i32>) -> i32 {\n    if a { return 0; }\n    if b { return 1; }\n    for x in items { let _y = x; }\n    2\n}\n",
            },
            Variant {
                ext: "go",
                code: "package p\nfunc f(a bool, b bool, items []int) int {\n    if a {\n        return 0\n    }\n    if b {\n        return 1\n    }\n    for _, x := range items {\n        _ = x\n    }\n    return 2\n}\n",
            },
            Variant {
                ext: "js",
                code: "function f(a, b, items) {\n    if (a) { return 0; }\n    if (b) { return 1; }\n    for (const x of items) { let y = x; }\n    return 2;\n}\n",
            },
        ],
    );
}

#[test]
fn boolean_chain_is_uniform() {
    assert_uniform(
        "boolean_chain",
        &[
            Variant {
                ext: "py",
                code: "def f(a, b, c):\n    if a and b and c:\n        return 1\n    return 0\n",
            },
            Variant {
                ext: "rs",
                code: "fn f(a: bool, b: bool, c: bool) -> i32 {\n    if a && b && c { return 1; }\n    0\n}\n",
            },
            Variant {
                ext: "go",
                code: "package p\nfunc f(a bool, b bool, c bool) int {\n    if a && b && c {\n        return 1\n    }\n    return 0\n}\n",
            },
            Variant {
                ext: "js",
                code: "function f(a, b, c) {\n    if (a && b && c) { return 1; }\n    return 0;\n}\n",
            },
        ],
    );
}

#[test]
fn nested_loop_with_guard_is_uniform() {
    assert_uniform(
        "nested_loop_with_guard",
        &[
            Variant {
                ext: "py",
                code: "def f(items):\n    for x in items:\n        if x:\n            return x\n    return 0\n",
            },
            Variant {
                ext: "rs",
                code: "fn f(items: Vec<i32>) -> i32 {\n    for x in items {\n        if x > 0 { return x; }\n    }\n    0\n}\n",
            },
            Variant {
                ext: "go",
                code: "package p\nfunc f(items []int) int {\n    for _, x := range items {\n        if x > 0 {\n            return x\n        }\n    }\n    return 0\n}\n",
            },
            Variant {
                ext: "js",
                code: "function f(items) {\n    for (const x of items) {\n        if (x) { return x; }\n    }\n    return 0;\n}\n",
            },
        ],
    );
}
