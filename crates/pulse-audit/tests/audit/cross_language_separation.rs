use pulse_audit::walker::{extract_subtrees, SubtreeRecord};
use pulse_syntax::parse::{self, Language};
use pulse_thresholds::Thresholds;
use std::collections::HashSet;
use std::path::Path;

fn t() -> Thresholds {
    Thresholds::default()
}

fn fps_for(lang: Language, src: &str) -> HashSet<u64> {
    let tree = parse::parse_only(src, lang).unwrap();
    let recs: Vec<SubtreeRecord> = extract_subtrees(&tree, src, lang, Path::new("t"), &t().audit);
    recs.iter().map(|r| r.fingerprint).collect()
}

const PAIRS: &[(Language, Language, &str, &str)] = &[
    (
        Language::Python,
        Language::Rust,
        "def f(x):\n    if x == 1:\n        return x\n    return 0\n",
        "fn f(x: i32) -> i32 { if x == 1 { return x; } 0 }\n",
    ),
    (
        Language::Python,
        Language::JavaScript,
        "def f(x):\n    if x == 1:\n        return x\n    return 0\n",
        "function f(x) { if (x === 1) return x; return 0; }\n",
    ),
    (
        Language::Python,
        Language::TypeScript,
        "def f(x):\n    if x == 1:\n        return x\n    return 0\n",
        "function f(x: number): number { if (x === 1) return x; return 0; }\n",
    ),
    (
        Language::Python,
        Language::Go,
        "def f(x):\n    if x == 1:\n        return x\n    return 0\n",
        "package p\nfunc f(x int) int { if x == 1 { return x }; return 0 }\n",
    ),
    (
        Language::Python,
        Language::Java,
        "def f(x):\n    if x == 1:\n        return x\n    return 0\n",
        "class A { int f(int x) { if (x == 1) return x; return 0; } }\n",
    ),
    (
        Language::Python,
        Language::CSharp,
        "def f(x):\n    if x == 1:\n        return x\n    return 0\n",
        "class A { int F(int x) { if (x == 1) return x; return 0; } }\n",
    ),
    (
        Language::Python,
        Language::Swift,
        "def f(x):\n    if x == 1:\n        return x\n    return 0\n",
        "func f(x: Int) -> Int { if x == 1 { return x }; return 0 }\n",
    ),
    (
        Language::Python,
        Language::Kotlin,
        "def f(x):\n    if x == 1:\n        return x\n    return 0\n",
        "fun f(x: Int): Int { if (x == 1) return x; return 0 }\n",
    ),
    (
        Language::Python,
        Language::Ruby,
        "def f(x):\n    if x == 1:\n        return x\n    return 0\n",
        "def f(x)\n  if x == 1\n    return x\n  end\n  0\nend\n",
    ),
    (
        Language::Python,
        Language::Lua,
        "def f(x):\n    if x == 1:\n        return x\n    return 0\n",
        "function f(x) if x == 1 then return x end return 0 end\n",
    ),
    (
        Language::TypeScript,
        Language::JavaScript,
        "function f(x: number): number { if (x === 1) return x; return 0; }\n",
        "function f(x) { if (x === 1) return x; return 0; }\n",
    ),
    (
        Language::C,
        Language::Cpp,
        "int f(int x) { if (x == 1) { return x; } return 0; }\n",
        "int f(int x) { if (x == 1) { return x; } return 0; }\n",
    ),
    (
        Language::Java,
        Language::CSharp,
        "class A { int f(int x) { if (x == 1) return x; return 0; } }\n",
        "class A { int F(int x) { if (x == 1) return x; return 0; } }\n",
    ),
    (
        Language::Java,
        Language::Kotlin,
        "class A { int f(int x) { if (x == 1) return x; return 0; } }\n",
        "fun f(x: Int): Int { if (x == 1) return x; return 0 }\n",
    ),
    (
        Language::Swift,
        Language::Kotlin,
        "func f(x: Int) -> Int { if x == 1 { return x }; return 0 }\n",
        "fun f(x: Int): Int { if (x == 1) return x; return 0 }\n",
    ),
    (
        Language::Ruby,
        Language::Python,
        "def f(x)\n  if x == 1\n    return x\n  end\n  0\nend\n",
        "def f(x):\n    if x == 1:\n        return x\n    return 0\n",
    ),
    (
        Language::Php,
        Language::JavaScript,
        "<?php function f($x) { if ($x === 1) return $x; return 0; }\n",
        "function f(x) { if (x === 1) return x; return 0; }\n",
    ),
    (
        Language::Go,
        Language::Rust,
        "package p\nfunc f(x int) int { if x == 1 { return x }; return 0 }\n",
        "fn f(x: i32) -> i32 { if x == 1 { return x; } 0 }\n",
    ),
    (
        Language::Haskell,
        Language::Python,
        "f :: Int -> Int\nf x = if x == 1 then x else 0\n",
        "def f(x):\n    if x == 1:\n        return x\n    return 0\n",
    ),
    (
        Language::Lua,
        Language::Ruby,
        "function f(x) if x == 1 then return x end return 0 end\n",
        "def f(x)\n  if x == 1\n    return x\n  end\n  0\nend\n",
    ),
];

#[test]
fn cross_lang_python_rust_disjoint() {
    let (a, b, sa, sb) = &PAIRS[0];
    assert!(fps_for(*a, sa).is_disjoint(&fps_for(*b, sb)));
}

#[test]
fn cross_lang_python_javascript_disjoint() {
    let (a, b, sa, sb) = &PAIRS[1];
    assert!(fps_for(*a, sa).is_disjoint(&fps_for(*b, sb)));
}

#[test]
fn cross_lang_python_typescript_disjoint() {
    let (a, b, sa, sb) = &PAIRS[2];
    assert!(fps_for(*a, sa).is_disjoint(&fps_for(*b, sb)));
}

#[test]
fn cross_lang_python_go_disjoint() {
    let (a, b, sa, sb) = &PAIRS[3];
    assert!(fps_for(*a, sa).is_disjoint(&fps_for(*b, sb)));
}

#[test]
fn cross_lang_python_java_disjoint() {
    let (a, b, sa, sb) = &PAIRS[4];
    assert!(fps_for(*a, sa).is_disjoint(&fps_for(*b, sb)));
}

#[test]
fn cross_lang_python_csharp_disjoint() {
    let (a, b, sa, sb) = &PAIRS[5];
    assert!(fps_for(*a, sa).is_disjoint(&fps_for(*b, sb)));
}

#[test]
fn cross_lang_python_swift_disjoint() {
    let (a, b, sa, sb) = &PAIRS[6];
    assert!(fps_for(*a, sa).is_disjoint(&fps_for(*b, sb)));
}

#[test]
fn cross_lang_python_kotlin_disjoint() {
    let (a, b, sa, sb) = &PAIRS[7];
    assert!(fps_for(*a, sa).is_disjoint(&fps_for(*b, sb)));
}

#[test]
fn cross_lang_python_ruby_disjoint() {
    let (a, b, sa, sb) = &PAIRS[8];
    assert!(fps_for(*a, sa).is_disjoint(&fps_for(*b, sb)));
}

#[test]
fn cross_lang_python_lua_disjoint() {
    let (a, b, sa, sb) = &PAIRS[9];
    assert!(fps_for(*a, sa).is_disjoint(&fps_for(*b, sb)));
}

#[test]
fn cross_lang_typescript_javascript_disjoint() {
    let (a, b, sa, sb) = &PAIRS[10];
    assert!(fps_for(*a, sa).is_disjoint(&fps_for(*b, sb)));
}

#[test]
fn cross_lang_c_cpp_disjoint() {
    let (a, b, sa, sb) = &PAIRS[11];
    assert!(fps_for(*a, sa).is_disjoint(&fps_for(*b, sb)));
}

#[test]
fn cross_lang_java_csharp_disjoint() {
    let (a, b, sa, sb) = &PAIRS[12];
    assert!(fps_for(*a, sa).is_disjoint(&fps_for(*b, sb)));
}

#[test]
fn cross_lang_java_kotlin_disjoint() {
    let (a, b, sa, sb) = &PAIRS[13];
    assert!(fps_for(*a, sa).is_disjoint(&fps_for(*b, sb)));
}

#[test]
fn cross_lang_swift_kotlin_disjoint() {
    let (a, b, sa, sb) = &PAIRS[14];
    assert!(fps_for(*a, sa).is_disjoint(&fps_for(*b, sb)));
}

#[test]
fn cross_lang_ruby_python_disjoint() {
    let (a, b, sa, sb) = &PAIRS[15];
    assert!(fps_for(*a, sa).is_disjoint(&fps_for(*b, sb)));
}

#[test]
fn cross_lang_php_javascript_disjoint() {
    let (a, b, sa, sb) = &PAIRS[16];
    assert!(fps_for(*a, sa).is_disjoint(&fps_for(*b, sb)));
}

#[test]
fn cross_lang_go_rust_disjoint() {
    let (a, b, sa, sb) = &PAIRS[17];
    assert!(fps_for(*a, sa).is_disjoint(&fps_for(*b, sb)));
}

#[test]
fn cross_lang_haskell_python_disjoint() {
    let (a, b, sa, sb) = &PAIRS[18];
    assert!(fps_for(*a, sa).is_disjoint(&fps_for(*b, sb)));
}

#[test]
fn cross_lang_lua_ruby_disjoint() {
    let (a, b, sa, sb) = &PAIRS[19];
    assert!(fps_for(*a, sa).is_disjoint(&fps_for(*b, sb)));
}

#[test]
fn cross_lang_all_pairs_disjoint() {
    for (a, b, sa, sb) in PAIRS {
        assert!(fps_for(*a, sa).is_disjoint(&fps_for(*b, sb)), "pair ({a:?}, {b:?}) collided");
    }
}

#[test]
fn cross_lang_zero_collision_rate_across_pair_set() {
    let mut total = 0;
    let mut clean = 0;
    for (a, b, sa, sb) in PAIRS {
        total += 1;
        if fps_for(*a, sa).is_disjoint(&fps_for(*b, sb)) {
            clean += 1;
        }
    }
    assert_eq!(clean, total);
}

#[test]
fn cross_lang_python_self_returns_same_fingerprints_twice() {
    let src = "def f(x):\n    if x == 1:\n        return x\n    return 0\n";
    assert_eq!(fps_for(Language::Python, src), fps_for(Language::Python, src));
}

#[test]
fn cross_lang_rust_self_returns_same_fingerprints_twice() {
    let src = "fn f(x: i32) -> i32 { if x == 1 { return x; } 0 }\n";
    assert_eq!(fps_for(Language::Rust, src), fps_for(Language::Rust, src));
}

#[test]
fn cross_lang_python_two_distinct_sources_distinct_fingerprints() {
    let a = fps_for(Language::Python, "def f(x):\n    if x == 1:\n        return x\n    return 0\n");
    let b = fps_for(Language::Python, "class A:\n    def m(self):\n        return 1\n");
    assert!(a != b);
}

#[test]
fn cross_lang_typescript_two_distinct_sources_distinct_fingerprints() {
    let a = fps_for(Language::TypeScript, "function f(x: number): number { if (x === 1) return x; return 0; }\n");
    let b = fps_for(Language::TypeScript, "interface I { name: string; getId(): number; }\n");
    assert!(a != b);
}

#[test]
fn cross_lang_python_javascript_typescript_three_way_disjoint() {
    let py = fps_for(Language::Python, "def f(x):\n    if x == 1:\n        return x\n    return 0\n");
    let js = fps_for(Language::JavaScript, "function f(x) { if (x === 1) return x; return 0; }\n");
    let ts = fps_for(Language::TypeScript, "function f(x: number): number { if (x === 1) return x; return 0; }\n");
    assert!(py.is_disjoint(&js));
    assert!(py.is_disjoint(&ts));
    assert!(js.is_disjoint(&ts));
}

#[test]
fn cross_lang_c_family_three_way_disjoint() {
    let c = fps_for(Language::C, "int f(int x) { if (x == 1) { return x; } return 0; }\n");
    let cpp = fps_for(Language::Cpp, "int f(int x) { if (x == 1) { return x; } return 0; }\n");
    let objc = fps_for(Language::ObjectiveC, "int f(int x) { if (x == 1) { return x; } return 0; }\n");
    assert!(c.is_disjoint(&cpp));
    assert!(c.is_disjoint(&objc));
    assert!(cpp.is_disjoint(&objc));
}

#[test]
fn cross_lang_jvm_family_three_way_disjoint() {
    let java = fps_for(Language::Java, "class A { int f(int x) { if (x == 1) return x; return 0; } }\n");
    let kotlin = fps_for(Language::Kotlin, "fun f(x: Int): Int { if (x == 1) return x; return 0 }\n");
    let groovy = fps_for(Language::Groovy, "def f(x) { if (x == 1) { return x }; return 0 }\n");
    assert!(java.is_disjoint(&kotlin));
    assert!(java.is_disjoint(&groovy));
    assert!(kotlin.is_disjoint(&groovy));
}

#[test]
fn cross_lang_dynamic_languages_three_way_disjoint() {
    let py = fps_for(Language::Python, "def f(x):\n    if x == 1:\n        return x\n    return 0\n");
    let rb = fps_for(Language::Ruby, "def f(x)\n  if x == 1\n    return x\n  end\n  0\nend\n");
    let lua = fps_for(Language::Lua, "function f(x) if x == 1 then return x end return 0 end\n");
    assert!(py.is_disjoint(&rb));
    assert!(py.is_disjoint(&lua));
    assert!(rb.is_disjoint(&lua));
}

#[test]
fn cross_lang_systems_languages_disjoint() {
    let rust = fps_for(Language::Rust, "fn f(x: i32) -> i32 { if x == 1 { return x; } 0 }\n");
    let go = fps_for(Language::Go, "package p\nfunc f(x int) int { if x == 1 { return x }; return 0 }\n");
    let zig = fps_for(Language::Zig, "fn f(x: i32) i32 { if (x == 1) { return x; } return 0; }\n");
    let d_lang = fps_for(Language::D, "int f(int x) { if (x == 1) { return x; } return 0; }\n");
    assert!(rust.is_disjoint(&go));
    assert!(rust.is_disjoint(&zig));
    assert!(rust.is_disjoint(&d_lang));
    assert!(go.is_disjoint(&zig));
    assert!(go.is_disjoint(&d_lang));
    assert!(zig.is_disjoint(&d_lang));
}

#[test]
fn cross_lang_swift_kotlin_disjoint_with_swift_using_protocol() {
    let s = fps_for(Language::Swift, "protocol P { func f(x: Int) -> Int }\n");
    let k = fps_for(Language::Kotlin, "interface I { fun f(x: Int): Int }\n");
    assert!(s.is_disjoint(&k));
}

#[test]
fn cross_lang_haskell_disjoint_from_all_imperative_languages() {
    let h = fps_for(Language::Haskell, "f :: Int -> Int\nf x = if x == 1 then x else 0\n");
    for (lang, src) in [
        (Language::Python, "def f(x):\n    if x == 1:\n        return x\n    return 0\n"),
        (Language::Rust, "fn f(x: i32) -> i32 { if x == 1 { x } else { 0 } }\n"),
        (Language::JavaScript, "function f(x) { if (x === 1) return x; return 0; }\n"),
    ] {
        assert!(h.is_disjoint(&fps_for(lang, src)), "haskell vs {lang:?} not disjoint");
    }
}

#[test]
fn cross_lang_php_disjoint_from_javascript_and_python() {
    let php = fps_for(Language::Php, "<?php function f($x) { if ($x === 1) { return $x; } return 0; }\n");
    let js = fps_for(Language::JavaScript, "function f(x) { if (x === 1) { return x; } return 0; }\n");
    let py = fps_for(Language::Python, "def f(x):\n    if x == 1:\n        return x\n    return 0\n");
    assert!(php.is_disjoint(&js));
    assert!(php.is_disjoint(&py));
}

#[test]
fn cross_lang_r_distinct_from_python_assignment_styles() {
    let r = fps_for(Language::R, "f <- function(x) { if (x == 1) { return(x) }; return(0) }\n");
    let py = fps_for(Language::Python, "def f(x):\n    if x == 1:\n        return x\n    return 0\n");
    assert!(r.is_disjoint(&py));
}

#[test]
fn cross_lang_tcl_distinct_from_python_lua_ruby() {
    let tcl = fps_for(Language::Tcl, "proc f {x} {\n    if {$x == 1} {\n        return $x\n    }\n    return 0\n}\n");
    for (lang, src) in [
        (Language::Python, "def f(x):\n    if x == 1:\n        return x\n    return 0\n"),
        (Language::Lua, "function f(x) if x == 1 then return x end return 0 end\n"),
        (Language::Ruby, "def f(x)\n  if x == 1\n    return x\n  end\n  0\nend\n"),
    ] {
        assert!(tcl.is_disjoint(&fps_for(lang, src)));
    }
}

#[test]
fn cross_lang_cobol_distinct_from_all_modern_languages() {
    let cb = fps_for(Language::Cobol, "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. F.\n       PROCEDURE DIVISION.\n           IF X = 1 DISPLAY X END-IF.\n           STOP RUN.\n");
    for (lang, src) in [
        (Language::Python, "def f(x):\n    if x == 1:\n        return x\n    return 0\n"),
        (Language::JavaScript, "function f(x) { if (x === 1) { return x; } return 0; }\n"),
        (Language::C, "int f(int x) { if (x == 1) { return x; } return 0; }\n"),
    ] {
        assert!(cb.is_disjoint(&fps_for(lang, src)));
    }
}

#[test]
fn cross_lang_zig_distinct_from_c_and_rust() {
    let z = fps_for(Language::Zig, "fn f(x: i32) i32 { if (x == 1) { return x; } return 0; }\n");
    let c = fps_for(Language::C, "int f(int x) { if (x == 1) { return x; } return 0; }\n");
    let r = fps_for(Language::Rust, "fn f(x: i32) -> i32 { if x == 1 { x } else { 0 } }\n");
    assert!(z.is_disjoint(&c));
    assert!(z.is_disjoint(&r));
}

#[test]
fn cross_lang_groovy_distinct_from_java_and_kotlin() {
    let g = fps_for(Language::Groovy, "def f(x) { if (x == 1) { return x }; return 0 }\n");
    let j = fps_for(Language::Java, "class A { int f(int x) { if (x == 1) return x; return 0; } }\n");
    let k = fps_for(Language::Kotlin, "fun f(x: Int): Int { if (x == 1) return x; return 0 }\n");
    assert!(g.is_disjoint(&j));
    assert!(g.is_disjoint(&k));
}

#[test]
fn cross_lang_objc_distinct_from_c_and_cpp() {
    let o = fps_for(Language::ObjectiveC, "int f(int x) { if (x == 1) { return x; } return 0; }\n");
    let c = fps_for(Language::C, "int f(int x) { if (x == 1) { return x; } return 0; }\n");
    let cpp = fps_for(Language::Cpp, "int f(int x) { if (x == 1) { return x; } return 0; }\n");
    assert!(o.is_disjoint(&c));
    assert!(o.is_disjoint(&cpp));
}

#[test]
fn cross_lang_d_distinct_from_c_cpp_rust() {
    let d_lang = fps_for(Language::D, "int f(int x) { if (x == 1) { return x; } return 0; }\n");
    let c = fps_for(Language::C, "int f(int x) { if (x == 1) { return x; } return 0; }\n");
    let cpp = fps_for(Language::Cpp, "int f(int x) { if (x == 1) { return x; } return 0; }\n");
    let rust = fps_for(Language::Rust, "fn f(x: i32) -> i32 { if x == 1 { x } else { 0 } }\n");
    assert!(d_lang.is_disjoint(&c));
    assert!(d_lang.is_disjoint(&cpp));
    assert!(d_lang.is_disjoint(&rust));
}

#[test]
fn cross_lang_csharp_distinct_from_java_and_typescript() {
    let cs = fps_for(Language::CSharp, "class A { int F(int x) { if (x == 1) return x; return 0; } }\n");
    let j = fps_for(Language::Java, "class A { int f(int x) { if (x == 1) return x; return 0; } }\n");
    let ts = fps_for(Language::TypeScript, "function f(x: number): number { if (x === 1) return x; return 0; }\n");
    assert!(cs.is_disjoint(&j));
    assert!(cs.is_disjoint(&ts));
}

#[test]
fn cross_lang_python_match_distinct_from_javascript_switch() {
    let py = fps_for(Language::Python, "match x:\n    case 1:\n        pass\n    case 2:\n        pass\n");
    let js = fps_for(Language::JavaScript, "switch (x) { case 1: break; case 2: break; }\n");
    assert!(py.is_disjoint(&js));
}

#[test]
fn cross_lang_python_dict_distinct_from_ruby_hash() {
    let py = fps_for(Language::Python, "x = {\"a\": 1, \"b\": 2}\n");
    let rb = fps_for(Language::Ruby, "x = {a: 1, b: 2}\n");
    assert!(py.is_disjoint(&rb));
}

#[test]
fn cross_lang_python_list_distinct_from_javascript_array() {
    let py = fps_for(Language::Python, "x = [1, 2, 3]\n");
    let js = fps_for(Language::JavaScript, "let x = [1, 2, 3];\n");
    assert!(py.is_disjoint(&js));
}

#[test]
fn cross_lang_python_class_distinct_from_typescript_class() {
    let py = fps_for(Language::Python, "class A:\n    def m(self):\n        return 1\n");
    let ts = fps_for(Language::TypeScript, "class A { m(): number { return 1; } }\n");
    assert!(py.is_disjoint(&ts));
}

#[test]
fn cross_lang_rust_struct_distinct_from_go_struct() {
    let r = fps_for(Language::Rust, "struct A { x: i32, y: i32 }\n");
    let g = fps_for(Language::Go, "package p\ntype A struct { X int; Y int }\n");
    assert!(r.is_disjoint(&g));
}

#[test]
fn cross_lang_rust_enum_distinct_from_python_enum_class() {
    let r = fps_for(Language::Rust, "enum E { A, B, C }\n");
    let py = fps_for(Language::Python, "class E:\n    A = 1\n    B = 2\n    C = 3\n");
    assert!(r.is_disjoint(&py));
}

#[test]
fn cross_lang_python_generator_distinct_from_javascript_generator() {
    let py = fps_for(Language::Python, "def gen():\n    for i in range(10):\n        yield i\n");
    let js = fps_for(Language::JavaScript, "function* gen() { for (let i = 0; i < 10; i++) yield i; }\n");
    assert!(py.is_disjoint(&js));
}

#[test]
fn cross_lang_python_async_distinct_from_javascript_async() {
    let py = fps_for(Language::Python, "async def f():\n    return await load()\n");
    let js = fps_for(Language::JavaScript, "async function f() { return await load(); }\n");
    assert!(py.is_disjoint(&js));
}

#[test]
fn cross_lang_lambda_python_vs_javascript_arrow() {
    let py = fps_for(Language::Python, "f = lambda x: x + 1\n");
    let js = fps_for(Language::JavaScript, "const f = x => x + 1;\n");
    assert!(py.is_disjoint(&js));
}

#[test]
fn cross_lang_python_decorator_distinct_from_typescript_decorator() {
    let py = fps_for(Language::Python, "@dec\ndef f():\n    pass\n");
    let ts = fps_for(Language::TypeScript, "@dec class A { }\n");
    assert!(py.is_disjoint(&ts));
}

#[test]
fn cross_lang_swift_optional_distinct_from_typescript_optional() {
    let s = fps_for(Language::Swift, "func f(x: Int?) -> Int { return x ?? 0 }\n");
    let ts = fps_for(Language::TypeScript, "function f(x?: number): number { return x ?? 0; }\n");
    assert!(s.is_disjoint(&ts));
}

#[test]
fn cross_lang_kotlin_when_distinct_from_python_match() {
    let k = fps_for(Language::Kotlin, "fun f(x: Int): Int = when (x) { 1 -> 1; else -> 0 }\n");
    let py = fps_for(
        Language::Python,
        "def f(x):\n    match x:\n        case 1:\n            return 1\n        case _:\n            return 0\n",
    );
    assert!(k.is_disjoint(&py));
}

#[test]
fn cross_lang_rust_match_distinct_from_python_match() {
    let r = fps_for(Language::Rust, "fn f(x: i32) -> i32 { match x { 1 => 1, _ => 0 } }\n");
    let py = fps_for(Language::Python, "match x:\n    case 1:\n        pass\n    case _:\n        pass\n");
    assert!(r.is_disjoint(&py));
}

#[test]
fn cross_lang_java_lambda_distinct_from_csharp_lambda() {
    let j = fps_for(Language::Java, "class A { Function<Integer, Integer> f = x -> x + 1; }\n");
    let cs = fps_for(Language::CSharp, "class A { Func<int, int> f = x => x + 1; }\n");
    assert!(j.is_disjoint(&cs));
}

#[test]
fn cross_lang_zero_pairs_collide_under_repeated_walk() {
    for (a, b, sa, sb) in PAIRS {
        for _ in 0..3 {
            assert!(fps_for(*a, sa).is_disjoint(&fps_for(*b, sb)));
        }
    }
}

#[test]
fn cross_lang_large_pair_set_consistent_disjoint() {
    let extra: &[(Language, Language, &str, &str)] = &[
        (Language::Cpp, Language::Java, "void f() {}\n", "class A { void f() {} }\n"),
        (
            Language::Php,
            Language::Ruby,
            "<?php function f($x) { if ($x === 1) return $x; return 0; }\n",
            "def f(x)\n  if x == 1\n    return x\n  end\n  0\nend\n",
        ),
    ];
    for (a, b, sa, sb) in extra {
        assert!(fps_for(*a, sa).is_disjoint(&fps_for(*b, sb)));
    }
}

#[test]
fn cross_lang_pair_set_extends_beyond_twenty() {
    assert!(PAIRS.len() >= 20);
}

#[test]
fn cross_lang_each_pair_yields_nonempty_sets() {
    for (a, b, sa, sb) in PAIRS {
        assert!(!fps_for(*a, sa).is_empty());
        assert!(!fps_for(*b, sb).is_empty());
    }
}

#[test]
fn cross_lang_consistent_disjointness_under_swap() {
    for (a, b, sa, sb) in PAIRS {
        let ab = fps_for(*a, sa).is_disjoint(&fps_for(*b, sb));
        let ba = fps_for(*b, sb).is_disjoint(&fps_for(*a, sa));
        assert_eq!(ab, ba);
    }
}

#[test]
fn cross_lang_python_distinct_from_each_imperative() {
    let py = fps_for(Language::Python, "def f(x):\n    if x == 1:\n        return x\n    return 0\n");
    for (lang, src) in [
        (Language::C, "int f(int x) { if (x == 1) { return x; } return 0; }\n"),
        (Language::Cpp, "int f(int x) { if (x == 1) { return x; } return 0; }\n"),
        (Language::Java, "class A { int f(int x) { if (x == 1) return x; return 0; } }\n"),
        (Language::Go, "package p\nfunc f(x int) int { if x == 1 { return x }; return 0 }\n"),
        (Language::Rust, "fn f(x: i32) -> i32 { if x == 1 { x } else { 0 } }\n"),
    ] {
        assert!(py.is_disjoint(&fps_for(lang, src)));
    }
}

#[test]
fn cross_lang_javascript_distinct_from_each_typed_relative() {
    let js = fps_for(Language::JavaScript, "function f(x) { if (x === 1) return x; return 0; }\n");
    for (lang, src) in [
        (Language::TypeScript, "function f(x: number): number { if (x === 1) return x; return 0; }\n"),
        (Language::Python, "def f(x):\n    if x == 1:\n        return x\n    return 0\n"),
        (Language::Php, "<?php function f($x) { if ($x === 1) return $x; return 0; }\n"),
    ] {
        assert!(js.is_disjoint(&fps_for(lang, src)));
    }
}

#[test]
fn cross_lang_aggregate_disjoint_count_matches_pair_count() {
    let mut disjoint = 0;
    for (a, b, sa, sb) in PAIRS {
        if fps_for(*a, sa).is_disjoint(&fps_for(*b, sb)) {
            disjoint += 1;
        }
    }
    assert_eq!(disjoint, PAIRS.len());
}

#[test]
fn cross_lang_test_pair_set_includes_all_top_tier_languages() {
    let mut langs_seen = HashSet::new();
    for (a, b, _, _) in PAIRS {
        langs_seen.insert(*a);
        langs_seen.insert(*b);
    }
    let critical = [Language::Python, Language::JavaScript, Language::TypeScript, Language::Rust];
    for c in critical {
        assert!(langs_seen.contains(&c), "{c:?} missing from cross-lang pair set");
    }
}
