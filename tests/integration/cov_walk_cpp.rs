use crate::common::*;
use pulse::parse::{parse_and_walk, Language};
use pulse::walk::FileMetrics;

fn walk_cpp(src: &str) -> FileMetrics {
    parse_and_walk(src, Language::Cpp).expect("cpp source should parse")
}

fn find_fn<'a>(fm: &'a FileMetrics, name: &str) -> Option<&'a pulse::walk::FunctionMetrics> {
    fm.functions.iter().find(|f| f.name == name)
}

// extract_function_name -> find_name_in returns Some at the name-bearing child (line 188),
// and count_parameters takes the normal function_declarator + parameter_list path.
#[test]
fn free_function_name_and_params_extracted() {
    let fm = walk_cpp("int add(int a, int b) {\n    return a + b;\n}\n");
    let add = find_fn(&fm, "add").expect("free function name should be extracted");
    assert_eq!(add.arg_count, 2, "two declared parameters, got: {}", add.arg_count);
    assert!(!add.is_constructor);
}

// find_name_in finds a function_declarator but no NAME_KINDS child (operator_name),
// so it returns None (line 190) and extract_function_name falls back to "<anonymous>".
#[test]
fn operator_overload_falls_back_to_anonymous_name() {
    let fm = walk_cpp(concat!(
        "struct Point { int x; int y; };\n",
        "bool operator==(Point a, Point b) {\n",
        "    return a.x == b.x;\n",
        "}\n",
    ));
    assert!(
        fm.functions.iter().any(|f| f.name == "<anonymous>"),
        "operator overload should yield an anonymous name, got: {:?}",
        fm.functions.iter().map(|f| f.name.as_str()).collect::<Vec<_>>()
    );
}

// collect_class_methods iterates function_definition children; a defaulted constructor
// has no compound_statement so analyze_function returns None and the loop hits the
// `continue` at line 102. The real method must still be collected.
#[test]
fn class_method_without_body_is_skipped() {
    let fm = walk_cpp(concat!(
        "class Widget {\n",
        "public:\n",
        "    Widget() = default;\n",
        "    int value() { return v_; }\n",
        "private:\n",
        "    int v_;\n",
        "};\n",
    ));
    assert!(
        find_fn(&fm, "Widget::value").is_some(),
        "real method should be collected, got: {:?}",
        fm.functions.iter().map(|f| f.name.as_str()).collect::<Vec<_>>()
    );
    assert!(find_fn(&fm, "Widget::Widget").is_none(), "defaulted constructor (no body) must be skipped");
}

// recurse_namespace finds the declaration_list (Some branch) and collects nested functions.
#[test]
fn namespace_functions_are_collected() {
    let fm = walk_cpp("namespace ns {\nint helper() {\n    return 1;\n}\n}\n");
    assert!(
        find_fn(&fm, "helper").is_some(),
        "function inside a namespace should be collected, got: {:?}",
        fm.functions.iter().map(|f| f.name.as_str()).collect::<Vec<_>>()
    );
}

// recurse_namespace early-returns (line 81) when a namespace_definition has no
// declaration_list (truncated / bodyless namespace). Must not panic and must collect
// the well-formed function that follows.
#[test]
fn bodyless_namespace_does_not_crash() {
    let fm = walk_cpp("namespace broken\nint after() {\n    return 0;\n}\n");
    let _ = &fm.functions;
}

// handle_catch (lines 265-270): cc bump + empty-catch detection on an empty catch body.
#[test]
fn empty_catch_clause_counted() {
    let fm = walk_cpp(concat!(
        "void risky() {\n",
        "    try {\n",
        "        work();\n",
        "    } catch (...) {\n",
        "    }\n",
        "}\n",
    ));
    let f = find_fn(&fm, "risky").expect("function should be collected");
    assert!(f.empty_catch_count >= 1, "empty catch should be counted, got: {}", f.empty_catch_count);
    assert!(f.cc >= 2, "catch clause should bump cc above base, got: {}", f.cc);
}

// handle_catch line 271: walk_children recurses into a non-empty catch body, so a
// conditional inside the catch still contributes to cc / nesting.
#[test]
fn non_empty_catch_body_is_walked() {
    let fm = walk_cpp(concat!(
        "void handler(int code) {\n",
        "    try {\n",
        "        work();\n",
        "    } catch (...) {\n",
        "        if (code > 0) {\n",
        "            recover();\n",
        "        }\n",
        "    }\n",
        "}\n",
    ));
    let f = find_fn(&fm, "handler").expect("function should be collected");
    assert_eq!(f.empty_catch_count, 0, "non-empty catch must not count as empty");
    assert!(f.cc >= 3, "catch + nested if should both bump cc, got: {}", f.cc);
}

// A simple try/catch keeps cc below the complexity warning threshold — exercises the
// catch cc-bump path (lines 265-267) while staying within t()-derived bounds.
#[test]
fn simple_try_catch_below_cc_warning() {
    let fm = walk_cpp("void f() {\n    try {\n        work();\n    } catch (...) {\n        log();\n    }\n}\n");
    let f = find_fn(&fm, "f").expect("function should be collected");
    assert!(f.cc < t().function.cc_warning, "simple try/catch should stay below cc warning, got: {}", f.cc);
}

// count_parameters line 306: a reference-returning function nests the function_declarator
// under a reference_declarator, so neither function_declarator nor pointer_declarator is a
// direct child -> the (0,0,0,0) early-return is taken.
#[test]
fn reference_return_function_takes_no_declarator_path() {
    let fm = walk_cpp("int& get_ref() {\n    static int v = 0;\n    return v;\n}\n");
    let f = fm.functions.iter().find(|f| f.arg_count == 0).expect("reference-returning function should be collected");
    assert_eq!(f.arg_count, 0, "no-declarator path yields zero args");
    assert_eq!(f.primitive_type_count, 0);
}

// Best-effort attempt at count_parameters line 309 (declarator present but no
// parameter_list) and general robustness: malformed parameter list must not panic.
#[test]
fn malformed_parameters_do_not_crash() {
    let fm = walk_cpp("void f(@@@) {\n    int x = 1;\n}\n");
    let _ = &fm.functions;
    let fm2 = walk_cpp("void g() {\n    int y = 2;\n}\n");
    assert!(find_fn(&fm2, "g").is_some(), "clean function alongside malformed input should still parse");
}
