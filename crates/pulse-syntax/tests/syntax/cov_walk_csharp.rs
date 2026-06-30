use pulse_syntax::parse::{parse_and_walk, Language};

use crate::common::t;

fn analyze(source: &str) -> pulse_syntax::walk::FileMetrics {
    parse_and_walk(source, Language::CSharp).expect("parse C#")
}

fn names(metrics: &pulse_syntax::walk::FileMetrics) -> Vec<String> {
    metrics.functions.iter().map(|f| f.name.clone()).collect()
}

// recurse_namespace early-returns (line 80) when a namespace has no declaration_list.
#[test]
fn namespace_without_body_is_skipped_without_panic() {
    let m = analyze("namespace Empty;\nclass C { void M() { int x = 1; } }\n");
    assert!(
        m.functions.iter().any(|f| f.name == "C.M"),
        "class after file-scoped namespace still collected: {:?}",
        names(&m)
    );
}

// if / else-if / else (walk_children else_clause + handle_elif + walk_nested_block saw_else +
// walk_else_clause) and try/catch empty + non-empty (handle_catch).
#[test]
fn control_flow_and_catch_paths_are_walked() {
    let src = concat!(
        "class C {\n",
        "    int classify(int n, string label) {\n",
        "        if (n > 0 && label != null) {\n",
        "            return 1;\n",
        "        } else if (n < 0) {\n",
        "            return -1;\n",
        "        } else {\n",
        "            return 0;\n",
        "        }\n",
        "    }\n",
        "    void risky() {\n",
        "        try { Work(); } catch { }\n",
        "        try { Work(); } catch (System.Exception e) { Log(e); }\n",
        "    }\n",
        "}\n",
    );
    let m = analyze(src);
    let classify = m.functions.iter().find(|f| f.name == "C.classify").expect("classify collected");
    assert!(classify.cc >= 3, "if/else-if branches bump cc, got: {}", classify.cc);
    assert_eq!(classify.arg_count, 2, "two typed params: {}", classify.arg_count);
    let risky = m.functions.iter().find(|f| f.name == "C.risky").expect("risky collected");
    assert!(risky.empty_catch_count >= 1, "the empty catch is counted, got: {}", risky.empty_catch_count);
    assert!(classify.cc < t().function.cc_alert, "stays below the cc alert ceiling");
}

// Line 115: an expression-bodied constructor has no `block` (its body is an
// arrow_expression_clause), so analyze_callable(CTOR_CFG) returns None and the
// constructor arm `continue`s. The constructor must NOT appear as a function,
// while a sibling block-bodied method is still collected.
#[test]
fn expression_bodied_constructor_skipped() {
    let source = concat!(
        "public class Cls {\n",
        "    private int _x;\n",
        "    public Cls(int x) => _x = x;\n",
        "    public int Run() { return _x; }\n",
        "}\n",
    );
    let metrics = analyze(source);
    let fn_names = names(&metrics);
    assert!(
        fn_names.iter().any(|n| n == "Cls.Run"),
        "block-bodied sibling method must be collected, got: {fn_names:?}"
    );
    assert!(
        !fn_names.iter().any(|n| n == "Cls.Cls"),
        "expression-bodied constructor must be skipped (analyze_callable -> None), got: {fn_names:?}"
    );
}

// Companion to the line-115 skip: a normal block-bodied constructor IS collected,
// exercising the surrounding constructor arm (lines 113-121: name format,
// is_constructor flag, class/parent wiring).
#[test]
fn block_bodied_constructor_collected() {
    let source =
        concat!("public class Blk {\n", "    private int _x;\n", "    public Blk(int x) { _x = x; }\n", "}\n",);
    let metrics = analyze(source);
    let ctor = metrics.functions.iter().find(|f| f.name == "Blk.Blk");
    let ctor = ctor.expect("block-bodied constructor should be collected as Blk.Blk");
    assert!(ctor.is_constructor, "constructor metrics must set is_constructor");
    assert!(
        ctor.arg_count < t().function.arg_max,
        "single-arg ctor must sit below the excess-args threshold, got arg_count={}",
        ctor.arg_count
    );
}

// A class with ONLY an expression-bodied constructor (no other callable members)
// collapses to zero functions: the sole constructor is dropped at the line-115
// `continue`, leaving nothing to push.
#[test]
fn class_with_only_expression_constructor_has_no_functions() {
    let source = concat!(
        "public class Only {\n",
        "    private readonly int _v;\n",
        "    public Only(int v) => _v = v;\n",
        "}\n",
    );
    let metrics = analyze(source);
    assert!(
        metrics.functions.is_empty(),
        "expression-bodied-only constructor class must yield no functions, got: {:?}",
        names(&metrics)
    );
}

// Drives the constructor arm again under an inherited class so parent_class
// resolution executes alongside the skip/keep decision.
#[test]
fn inherited_class_expression_constructor_skipped() {
    let source = concat!(
        "public class Base {\n",
        "    public Base() {}\n",
        "}\n",
        "public class Derived : Base {\n",
        "    private int _n;\n",
        "    public Derived(int n) => _n = n;\n",
        "    public int Get() { return _n; }\n",
        "}\n",
    );
    let metrics = analyze(source);
    let fn_names = names(&metrics);
    assert!(fn_names.iter().any(|n| n == "Derived.Get"), "method survives, got: {fn_names:?}");
    assert!(
        !fn_names.iter().any(|n| n == "Derived.Derived"),
        "expression-bodied derived constructor must be skipped, got: {fn_names:?}"
    );
    assert!(fn_names.iter().any(|n| n == "Base.Base"), "block-bodied base ctor kept, got: {fn_names:?}");
}

// Namespaced methods: drives the recurse_namespace `Some(body)` path (line 82)
// and collect_functions descent through a declaration_list.
#[test]
fn namespace_block_body_collects_methods() {
    let source = concat!(
        "namespace App {\n",
        "    public class Svc {\n",
        "        public int Handle(int x) { return x; }\n",
        "    }\n",
        "}\n",
    );
    let metrics = analyze(source);
    assert!(
        names(&metrics).iter().any(|n| n == "Svc.Handle"),
        "method inside namespace must be found, got: {:?}",
        names(&metrics)
    );
}

// Mixed primitive + user-typed parameters drive primitive_type_of: `int` parses
// as predefined_type (the returning arm) while `MyType` parses as a plain
// identifier (falling through to None), so the primitive count counts only int.
#[test]
fn mixed_parameter_types_count_primitives() {
    let source = concat!("public class P {\n", "    public int F(int a, MyType b) { return a; }\n", "}\n",);
    let metrics = analyze(source);
    let f = metrics.functions.iter().find(|f| f.name == "P.F").expect("P.F collected");
    assert_eq!(f.arg_count, 2, "two parameters expected");
    assert_eq!(f.primitive_type_count, 1, "only the int param is a recognized primitive");
}

// A zero-parameter method exercises count_parameters with an empty parameter_list
// (count stays 0) without tripping the no-parameter_list early return.
#[test]
fn zero_parameter_method_counts_zero() {
    let source = "public class Z {\n    public int F() { return 1; }\n}\n";
    let metrics = analyze(source);
    let f = metrics.functions.iter().find(|f| f.name == "Z.F").expect("Z.F collected");
    assert_eq!(f.arg_count, 0);
    assert!(f.arg_count < t().function.arg_max);
}
