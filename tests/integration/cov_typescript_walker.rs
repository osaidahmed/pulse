use pulse::parse::{parse_and_walk, parse_and_walk_scoped, Language};

fn ts(source: &str) -> pulse::walk::FileMetrics {
    parse_and_walk(source, Language::TypeScript).expect("parse TypeScript")
}

fn js(source: &str) -> pulse::walk::FileMetrics {
    parse_and_walk(source, Language::JavaScript).expect("parse JavaScript")
}

// ── Line 191: extras else-branch (zeros) for a function outside the edit scope ──
// A scoped walk skips fingerprint/skeleton/short-var extras for any function whose
// byte range falls outside the edited region, returning (0,0,0,0,0,0,0).
#[test]
fn scoped_walk_skips_extras_for_untouched_ts_function() {
    let src = "function untouched(): void {\n    const x = 1;\n}\n\nfunction edited(): void {\n    const y = 2;\n}\n";
    let pos = src.find("const y").expect("locate edited body");
    let m = parse_and_walk_scoped(src, Language::TypeScript, Some((pos, pos + 7)), false).expect("scoped walk");

    let edited = m.functions.iter().find(|f| f.name == "edited").expect("edited fn");
    let untouched = m.functions.iter().find(|f| f.name == "untouched").expect("untouched fn");

    assert_ne!(edited.skeleton_hash, 0, "edited fn must compute extras");
    assert_eq!(untouched.skeleton_hash, 0, "untouched fn must take the zeros else-branch");
    assert_eq!(untouched.structural_hash, 0, "untouched structural_hash zeroed");
    assert_eq!(untouched.short_var_count, 0, "untouched short_var_count zeroed");
    assert!(untouched.cc >= 1, "walk_body still runs for cc even when extras are skipped");
}

// ── Line 93: arrow collector skips a declarator with no identifier name (destructuring) ──
// A `const [a, b] = ...` / `const {x} = ...` declarator has no `identifier` child, so the
// arrow-function collector `continue`s past it. A real arrow afterward must still be collected.
#[test]
fn destructuring_declarator_skipped_then_arrow_collected() {
    let src = concat!(
        "const [first, second] = makePair();\n",
        "const { alpha, beta } = makeObj();\n",
        "const handler = (a: string, b: string): string => {\n",
        "    return a + b;\n",
        "};\n",
    );
    let m = ts(src);
    assert!(
        m.functions.iter().any(|f| f.name == "handler"),
        "named arrow after destructuring declarators must still be collected, got: {:?}",
        m.functions.iter().map(|f| f.name.clone()).collect::<Vec<_>>()
    );
    assert!(
        !m.functions.iter().any(|f| f.name == "first" || f.name == "alpha"),
        "destructured bindings must not be collected as functions"
    );
}

// ── Line 125: class method with no statement_block body is skipped ──
// An overload signature / bodyless method has no `statement_block`, so analyze_function
// returns None and `collect_class_methods` `continue`s. The implemented method is collected.
#[test]
fn bodyless_method_signature_skipped_real_method_collected() {
    let src = concat!(
        "class Service {\n",
        "    run(): void;\n",
        "    run(): void {\n",
        "        const x = 1;\n",
        "    }\n",
        "}\n",
    );
    let m = ts(src);
    assert!(
        m.functions.iter().any(|f| f.name == "Service.run"),
        "implemented method must be collected, got: {:?}",
        m.functions.iter().map(|f| f.name.clone()).collect::<Vec<_>>()
    );
}

// ── Line 336: TS count_parameters early-return when there is no formal_parameters ──
// A bare single-parameter arrow (`x => x`) carries its parameter as a direct identifier,
// not a `formal_parameters` node, so count_parameters returns (0, 0, 0, 0).
#[test]
fn ts_bare_param_arrow_has_zero_arg_count() {
    let src = "const identity = x => {\n    return x;\n};\n";
    let m = ts(src);
    let f = m.functions.iter().find(|f| f.name == "identity").expect("identity arrow");
    assert_eq!(f.arg_count, 0, "bare-param arrow yields the no-formal_parameters early return (TS)");
    assert_eq!(f.primitive_type_count, 0);
    assert_eq!(f.typed_param_count, 0);
}

// ── Line 361: JS count_parameters_untyped early-return when there is no formal_parameters ──
#[test]
fn js_bare_param_arrow_has_zero_arg_count() {
    let src = "const identity = x => {\n    return x;\n};\n";
    let m = js(src);
    let f = m.functions.iter().find(|f| f.name == "identity").expect("identity arrow");
    assert_eq!(f.arg_count, 0, "bare-param arrow yields the no-formal_parameters early return (JS)");
}

// ── Lines 297-304 (best-effort) / shared catch path: empty vs non-empty catch ──
// The normal try/catch flow routes catch handling through the shared branch walker; this
// drives the empty-catch detection and confirms the walker does not crash on a bare
// catch construct that could dispatch the local catch handler.
#[test]
fn try_catch_empty_and_nonempty_do_not_crash() {
    let empty = ts("function e(): void {\n    try {\n        risky();\n    } catch (err) {}\n}\n");
    let ef = empty.functions.iter().find(|f| f.name == "e").expect("fn e");
    assert!(ef.cc >= 2, "catch contributes to cc, got: {}", ef.cc);

    let nonempty =
        ts("function n(): void {\n    try {\n        risky();\n    } catch (err) {\n        log(err);\n    }\n}\n");
    assert!(nonempty.functions.iter().any(|f| f.name == "n"), "non-empty catch fn collected");
}

// ── Line 110 (best-effort): malformed class without a class_body must not crash ──
#[test]
fn malformed_class_without_body_does_not_crash() {
    let _ = ts("class Broken\n");
    let _ = ts("export class AlsoBroken extends Base\n");
}
