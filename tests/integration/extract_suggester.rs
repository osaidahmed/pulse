use pulse::extract::{suggest_extract, LineSpan};
use pulse::parse::Language;

fn span(start: u32, end: u32) -> LineSpan {
    LineSpan { start, end }
}

#[test]
fn rust_deepest_nested_region_is_the_inner_loop() {
    let src = "fn f() {\n    if a() {\n        for x in xs {\n            work(x);\n        }\n    }\n}\n";
    let r = suggest_extract(src, Language::Rust, span(1, 7)).expect("a nested region exists");
    assert_eq!(r.start_line, 3, "the for-loop at line 3 is the deepest control-flow node");
    assert_eq!(r.nesting, 2);
}

#[test]
fn python_deepest_nested_region_is_reported() {
    let src = "def f():\n    if a:\n        for x in xs:\n            work(x)\n";
    let r = suggest_extract(src, Language::Python, span(1, 4)).expect("a nested region exists");
    assert_eq!(r.start_line, 3);
    assert_eq!(r.nesting, 2);
}

#[test]
fn typescript_deepest_nested_region_is_reported() {
    let src = "function f() {\n    if (a) {\n        while (b) {\n            work();\n        }\n    }\n}\n";
    let r = suggest_extract(src, Language::TypeScript, span(1, 7)).expect("a nested region exists");
    assert_eq!(r.start_line, 3);
    assert_eq!(r.nesting, 2);
}

#[test]
fn a_flat_function_yields_no_suggestion() {
    let src = "fn f() {\n    if a() {\n        work();\n    }\n}\n";
    assert!(
        suggest_extract(src, Language::Rust, span(1, 5)).is_none(),
        "a single un-nested branch is not worth an extract suggestion"
    );
}

#[test]
fn go_deepest_nested_region_is_reported() {
    let src = "package main\nfunc f() {\n    if a {\n        for {\n            work()\n        }\n    }\n}\n";
    let r = suggest_extract(src, Language::Go, span(2, 8)).expect("a nested region exists");
    assert_eq!(r.start_line, 4, "the for-loop nested in the if is the deepest region");
    assert_eq!(r.nesting, 2);
}

#[test]
fn java_deepest_nested_region_is_reported() {
    let src = "class C {\n    void f() {\n        if (a) {\n            while (b) {\n                work();\n            }\n        }\n    }\n}\n";
    let r = suggest_extract(src, Language::Java, span(1, 9)).expect("a nested region exists");
    assert_eq!(r.start_line, 4);
    assert_eq!(r.nesting, 2);
}

#[test]
fn csharp_deepest_nested_region_is_reported() {
    let src = "class C {\n    void F() {\n        if (a) {\n            foreach (var x in xs) {\n                Work(x);\n            }\n        }\n    }\n}\n";
    let r = suggest_extract(src, Language::CSharp, span(1, 9)).expect("a nested region exists");
    assert_eq!(r.start_line, 4);
    assert_eq!(r.nesting, 2);
}

#[test]
fn a_flat_newly_covered_function_yields_no_suggestion() {
    let src = "package main\nfunc f() {\n    work()\n}\n";
    assert!(
        suggest_extract(src, Language::Go, span(2, 4)).is_none(),
        "a function with no nested control flow is not worth an extract suggestion"
    );
}

#[test]
fn deeper_nesting_outranks_shallower() {
    let src = "fn f() {\n    if a() {\n        work();\n    }\n    if b() {\n        for x in xs {\n            for y in ys {\n                go(y);\n            }\n        }\n    }\n}\n";
    let r = suggest_extract(src, Language::Rust, span(1, 13)).expect("a nested region exists");
    assert_eq!(r.start_line, 7, "the innermost (depth-3) loop wins over shallower branches");
    assert_eq!(r.nesting, 3);
}
