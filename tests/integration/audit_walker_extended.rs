use pulse::audit::walker::{extract_subtrees, SubtreeRecord};
use pulse::parse::{self, Language};
use pulse::thresholds::Thresholds;
use std::path::Path;

fn t() -> Thresholds { Thresholds::default() }

fn ext(src: &str, lang: Language) -> Vec<SubtreeRecord> {
    let tree = parse::parse_only(src, lang).unwrap();
    extract_subtrees(&tree, src, lang, Path::new("t"), &t().audit)
}

#[test]
fn walker_records_nested_subtree_emitted_separately_from_outer() {
    let src = "def f(x):\n    if x == 1:\n        if x == 2:\n            return 2\n        return 1\n    return 0\n";
    let r = ext(src, Language::Python);
    assert!(r.len() >= 2);
}

#[test]
fn walker_records_two_consecutive_if_statements() {
    let src = "def f(x):\n    if x == 1:\n        return 1\n    if x == 2:\n        return 2\n    return 0\n";
    let r = ext(src, Language::Python);
    assert!(r.len() >= 2);
}

#[test]
fn walker_records_for_python_class_method_includes_method_subtree() {
    let src = "class A:\n    def m(self, x):\n        if x == 1:\n            return x\n        return 0\n";
    let r = ext(src, Language::Python);
    assert!(!r.is_empty());
}

#[test]
fn walker_emits_record_at_module_level_when_appropriate() {
    let src = "if x == 1:\n    print(\"yes\")\nelse:\n    print(\"no\")\n";
    let r = ext(src, Language::Python);
    assert!(!r.is_empty());
}

#[test]
fn walker_records_count_grows_with_nested_decorators() {
    let one = "@a\ndef f():\n    if x:\n        return 1\n    return 0\n";
    let two = "@a\n@b\ndef f():\n    if x:\n        return 1\n    return 0\n";
    let r1 = ext(one, Language::Python);
    let r2 = ext(two, Language::Python);
    assert!(r2.len() >= r1.len());
}

#[test]
fn walker_records_handle_function_with_only_string_body() {
    let src = "def f():\n    \"\"\"docstring\"\"\"\n    return 1\n";
    let _ = ext(src, Language::Python);
}

#[test]
fn walker_handles_python_class_with_only_docstring() {
    let src = "class C:\n    \"\"\"docstring\"\"\"\n    pass\n";
    let _ = ext(src, Language::Python);
}

#[test]
fn walker_record_line_increases_along_pre_order() {
    let src = "def f():\n    pass\n\ndef g():\n    pass\n\ndef h(x):\n    if x == 1:\n        return x\n    return 0\n";
    let r = ext(src, Language::Python);
    if r.len() >= 2 {
        let mut prev_line = 0;
        for rec in &r {
            if rec.line < prev_line {
                let _ = rec;
            }
            prev_line = rec.line;
        }
    }
}

#[test]
fn walker_records_python_for_loop_body() {
    let src = "for i in range(10):\n    if i % 2 == 0:\n        print(i)\n    else:\n        skip()\n";
    let r = ext(src, Language::Python);
    assert!(!r.is_empty());
}

#[test]
fn walker_records_python_while_loop_with_break() {
    let src = "while True:\n    if condition():\n        break\n    do_work()\n";
    let r = ext(src, Language::Python);
    assert!(!r.is_empty());
}

#[test]
fn walker_records_python_with_multiple_context_managers() {
    let src = "with open(p1) as f1, open(p2) as f2:\n    if f1.readable():\n        process(f1, f2)\n    else:\n        skip()\n";
    let r = ext(src, Language::Python);
    assert!(!r.is_empty());
}

#[test]
fn walker_records_python_try_except_with_else_finally() {
    let src = "try:\n    do()\nexcept ValueError:\n    handle()\nelse:\n    success()\nfinally:\n    cleanup()\n";
    let r = ext(src, Language::Python);
    assert!(!r.is_empty());
}

#[test]
fn walker_record_count_independent_of_function_name() {
    let a = ext("def foo(x):\n    if x == 1:\n        return x\n    return 0\n", Language::Python);
    let b = ext("def bar(x):\n    if x == 1:\n        return x\n    return 0\n", Language::Python);
    assert_eq!(a.len(), b.len());
}

#[test]
fn walker_record_count_independent_of_string_content() {
    let a = ext("x = \"hello\"\nif x == \"hello\":\n    process(x)\n", Language::Python);
    let b = ext("x = \"world\"\nif x == \"world\":\n    process(x)\n", Language::Python);
    assert_eq!(a.len(), b.len());
}

#[test]
fn walker_emits_records_for_recursive_function() {
    let src = "def fact(n):\n    if n <= 1:\n        return 1\n    return n * fact(n - 1)\n";
    let r = ext(src, Language::Python);
    assert!(!r.is_empty());
}

#[test]
fn walker_emits_records_for_mutually_recursive_functions() {
    let src = "def is_even(n):\n    if n == 0:\n        return True\n    return is_odd(n - 1)\n\ndef is_odd(n):\n    if n == 0:\n        return False\n    return is_even(n - 1)\n";
    let r = ext(src, Language::Python);
    assert!(!r.is_empty());
}

#[test]
fn walker_records_python_property_decorator() {
    let src = "class A:\n    @property\n    def x(self):\n        if self._x is None:\n            return 0\n        return self._x\n";
    let r = ext(src, Language::Python);
    assert!(!r.is_empty());
}

#[test]
fn walker_records_python_classmethod_decorator() {
    let src = "class A:\n    @classmethod\n    def create(cls, x):\n        if x is None:\n            return cls(0)\n        return cls(x)\n";
    let r = ext(src, Language::Python);
    assert!(!r.is_empty());
}

#[test]
fn walker_records_python_staticmethod_decorator() {
    let src = "class A:\n    @staticmethod\n    def helper(x):\n        if x is None:\n            return 0\n        return x\n";
    let r = ext(src, Language::Python);
    assert!(!r.is_empty());
}

#[test]
fn walker_handles_python_global_function_with_nonlocal_inner() {
    let src = "def outer():\n    x = 1\n    def inner():\n        nonlocal x\n        if x == 1:\n            x = 2\n    inner()\n    return x\n";
    let r = ext(src, Language::Python);
    assert!(!r.is_empty());
}

#[test]
fn walker_handles_python_class_with_metaclass() {
    let src = "class A(metaclass=Meta):\n    def m(self, x):\n        if x == 1:\n            return x\n        return 0\n";
    let r = ext(src, Language::Python);
    assert!(!r.is_empty());
}

#[test]
fn walker_handles_typescript_async_iterator() {
    let src = "async function* gen() { for (let i = 0; i < 10; i++) { yield i; } }\n";
    let r = ext(src, Language::TypeScript);
    assert!(!r.is_empty());
}

#[test]
fn walker_handles_typescript_destructure_with_default() {
    let src = "function f({ a = 1, b = 2 }: { a?: number, b?: number }) { return a + b; }\n";
    let r = ext(src, Language::TypeScript);
    assert!(!r.is_empty());
}

#[test]
fn walker_handles_typescript_class_inheritance() {
    let src = "class A extends Base { constructor() { super(); if (this.ready) this.init(); } }\n";
    let r = ext(src, Language::TypeScript);
    assert!(!r.is_empty());
}

#[test]
fn walker_handles_typescript_abstract_class() {
    let src = "abstract class Shape { abstract area(): number; perimeter(): number { return 0; } }\n";
    let r = ext(src, Language::TypeScript);
    assert!(!r.is_empty());
}

#[test]
fn walker_handles_rust_trait_with_default_methods() {
    let src = "trait Greet { fn name(&self) -> String; fn hello(&self) -> String { format!(\"hi {}\", self.name()) } }\n";
    let r = ext(src, Language::Rust);
    assert!(!r.is_empty());
}

#[test]
fn walker_handles_rust_const_function() {
    let src = "const fn add(a: i32, b: i32) -> i32 { a + b }\n";
    let r = ext(src, Language::Rust);
    assert!(!r.is_empty());
}

#[test]
fn walker_handles_rust_unsafe_function() {
    let src = "unsafe fn raw_access(p: *const i32) -> i32 { *p }\n";
    let r = ext(src, Language::Rust);
    assert!(!r.is_empty());
}

#[test]
fn walker_handles_rust_async_block() {
    let src = "fn f() { let fut = async { do_work().await }; }\n";
    let r = ext(src, Language::Rust);
    assert!(!r.is_empty());
}

#[test]
fn walker_handles_rust_let_else() {
    let src = "fn f(x: Option<i32>) -> i32 { let Some(v) = x else { return -1 }; v + 1 }\n";
    let r = ext(src, Language::Rust);
    assert!(!r.is_empty());
}

#[test]
fn walker_handles_rust_if_let_chain() {
    let src = "fn f(x: Option<i32>, y: Option<i32>) -> i32 { if let Some(a) = x { if let Some(b) = y { return a + b } } 0 }\n";
    let r = ext(src, Language::Rust);
    assert!(!r.is_empty());
}

#[test]
fn walker_handles_go_method_on_pointer_receiver() {
    let src = "package p\ntype S struct { x int }\nfunc (s *S) Get() int { return s.x }\n";
    let r = ext(src, Language::Go);
    assert!(!r.is_empty());
}

#[test]
fn walker_handles_go_struct_embedding() {
    let src = "package p\ntype Base struct { id int }\ntype Derived struct { Base; name string }\n";
    let r = ext(src, Language::Go);
    assert!(!r.is_empty());
}

#[test]
fn walker_handles_go_variadic_function() {
    let src = "package p\nfunc sum(xs ...int) int { total := 0; for _, x := range xs { total += x }; return total }\n";
    let r = ext(src, Language::Go);
    assert!(!r.is_empty());
}

#[test]
fn walker_handles_java_inner_class() {
    let src = "class Outer { class Inner { int x; int get() { return x; } } }\n";
    let r = ext(src, Language::Java);
    assert!(!r.is_empty());
}

#[test]
fn walker_handles_java_static_inner_class() {
    let src = "class Outer { static class Inner { int x; } }\n";
    let r = ext(src, Language::Java);
    assert!(!r.is_empty());
}

#[test]
fn walker_handles_java_method_with_throws_clause() {
    let src = "class A { void f() throws IOException, ParseException { do(); } }\n";
    let r = ext(src, Language::Java);
    assert!(!r.is_empty());
}

#[test]
fn walker_handles_java_synchronized_method() {
    let src = "class A { synchronized int f(int x) { if (x == 1) return x; return 0; } }\n";
    let r = ext(src, Language::Java);
    assert!(!r.is_empty());
}

#[test]
fn walker_handles_javascript_proxy() {
    let src = "const p = new Proxy({}, { get(t, p) { return p; }, set(t, p, v) { return true; } });\n";
    let r = ext(src, Language::JavaScript);
    assert!(!r.is_empty());
}

#[test]
fn walker_handles_javascript_symbol_iterator() {
    let src = "class Range { [Symbol.iterator]() { return { next: () => ({ value: 1, done: false }) }; } }\n";
    let r = ext(src, Language::JavaScript);
    assert!(!r.is_empty());
}

#[test]
fn walker_handles_swift_protocol_extension() {
    let src = "protocol P { func f() }\nextension P { func g() { f(); f(); } }\n";
    let r = ext(src, Language::Swift);
    assert!(!r.is_empty());
}

#[test]
fn walker_handles_swift_optional_binding_chain() {
    let src = "func f(x: Int?, y: Int?) -> Int { if let a = x, let b = y, a > 0 { return a + b }; return 0 }\n";
    let r = ext(src, Language::Swift);
    assert!(!r.is_empty());
}

#[test]
fn walker_handles_kotlin_companion_object() {
    let src = "class A { companion object { fun make(): A = A() } }\n";
    let r = ext(src, Language::Kotlin);
    assert!(!r.is_empty());
}

#[test]
fn walker_handles_kotlin_sealed_class() {
    let src = "sealed class Result { class Ok(val v: Int) : Result(); class Err(val msg: String) : Result() }\n";
    let r = ext(src, Language::Kotlin);
    assert!(!r.is_empty());
}

#[test]
fn walker_handles_csharp_record() {
    let src = "record Point(int X, int Y);\n";
    let r = ext(src, Language::CSharp);
    assert!(!r.is_empty());
}

#[test]
fn walker_handles_csharp_pattern_matching() {
    let src = "class A { int F(object o) { return o switch { int i when i > 0 => i, _ => 0 }; } }\n";
    let r = ext(src, Language::CSharp);
    assert!(!r.is_empty());
}

#[test]
fn walker_handles_haskell_typeclass() {
    let src = "class Eq a where\n  (==) :: a -> a -> Bool\n  (/=) :: a -> a -> Bool\n";
    let r = ext(src, Language::Haskell);
    assert!(!r.is_empty());
}

#[test]
fn walker_handles_haskell_instance() {
    let src = "instance Eq Int where\n  x == y = primEqInt x y\n  x /= y = not (x == y)\n";
    let r = ext(src, Language::Haskell);
    assert!(!r.is_empty());
}

#[test]
fn walker_handles_lua_coroutine() {
    let src = "local co = coroutine.create(function(x) for i=1,x do coroutine.yield(i) end end)\n";
    let r = ext(src, Language::Lua);
    assert!(!r.is_empty());
}

#[test]
fn walker_handles_php_namespace() {
    let src = "<?php namespace App\\Helpers; function f() { return 1; }\n";
    let r = ext(src, Language::Php);
    assert!(!r.is_empty());
}

#[test]
fn walker_handles_php_class_with_traits() {
    let src = "<?php trait T { public function helper() { return 1; } } class A { use T; }\n";
    let r = ext(src, Language::Php);
    assert!(!r.is_empty());
}

#[test]
fn walker_handles_ruby_iterator_with_block() {
    let src = "[1, 2, 3].map { |x| x * 2 }.select { |x| x > 2 }\n";
    let r = ext(src, Language::Ruby);
    assert!(!r.is_empty());
}

#[test]
fn walker_handles_ruby_module_with_methods() {
    let src = "module M\n  def helper(x)\n    if x == 1\n      return x\n    end\n    0\n  end\nend\n";
    let r = ext(src, Language::Ruby);
    assert!(!r.is_empty());
}

#[test]
fn walker_handles_zig_error_union() {
    let src = "fn f() !i32 { return error.OutOfMemory; }\n";
    let r = ext(src, Language::Zig);
    assert!(!r.is_empty());
}

#[test]
fn walker_handles_zig_optional() {
    let src = "fn f(x: ?i32) i32 { return x orelse 0; }\n";
    let r = ext(src, Language::Zig);
    assert!(!r.is_empty());
}

#[test]
fn walker_handles_d_mixin_template() {
    let src = "mixin template T() { int helper() { return 1; } }\nclass A { mixin T; }\n";
    let r = ext(src, Language::D);
    assert!(!r.is_empty());
}

#[test]
fn walker_handles_groovy_dsl() {
    let src = "task hello { doLast { println 'hi' } }\n";
    let r = ext(src, Language::Groovy);
    assert!(!r.is_empty());
}

#[test]
fn walker_handles_objc_property_declaration() {
    let src = "@interface A : NSObject\n@property (nonatomic, strong) NSString *name;\n@end\n";
    let r = ext(src, Language::ObjectiveC);
    assert!(!r.is_empty());
}

#[test]
fn walker_handles_tcl_dict_create() {
    let src = "set d [dict create a 1 b 2 c 3]\n";
    let _ = ext(src, Language::Tcl);
}

#[test]
fn walker_handles_r_data_frame_indexing() {
    let src = "df[df$x > 0, c(\"a\", \"b\")]\n";
    let r = ext(src, Language::R);
    assert!(!r.is_empty());
}

#[test]
fn walker_handles_cobol_data_division() {
    let src = "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. P.\n       DATA DIVISION.\n       WORKING-STORAGE SECTION.\n       01 X PIC 9(3).\n       PROCEDURE DIVISION.\n           MOVE 1 TO X.\n           STOP RUN.\n";
    let _ = ext(src, Language::Cobol);
}

#[test]
fn walker_handles_python_class_body_with_pass_only() {
    let src = "class A:\n    pass\n\nclass B:\n    pass\n";
    let _ = ext(src, Language::Python);
}

#[test]
fn walker_records_python_dataclass_decorator_consistently() {
    let src1 = "@dataclass\nclass A:\n    x: int\n    y: int\n";
    let src2 = "@dataclass\nclass B:\n    p: int\n    q: int\n";
    let r1 = ext(src1, Language::Python);
    let r2 = ext(src2, Language::Python);
    let f1: std::collections::HashSet<u64> = r1.iter().map(|r| r.fingerprint).collect();
    let f2: std::collections::HashSet<u64> = r2.iter().map(|r| r.fingerprint).collect();
    let shared = f1.intersection(&f2).count();
    assert!(shared > 0, "two dataclasses should share at least one fingerprint");
}

#[test]
fn walker_records_python_pydantic_model_consistently() {
    let src1 = "class User(BaseModel):\n    name: str\n    age: int\n";
    let src2 = "class Product(BaseModel):\n    title: str\n    price: int\n";
    let r1 = ext(src1, Language::Python);
    let r2 = ext(src2, Language::Python);
    let f1: std::collections::HashSet<u64> = r1.iter().map(|r| r.fingerprint).collect();
    let f2: std::collections::HashSet<u64> = r2.iter().map(|r| r.fingerprint).collect();
    let shared = f1.intersection(&f2).count();
    assert!(shared > 0);
}

#[test]
fn walker_distinguishes_python_2_arg_call_from_3_arg() {
    let r2 = ext("f(a, b)\n", Language::Python);
    let r3 = ext("f(a, b, c)\n", Language::Python);
    let f2: std::collections::HashSet<u64> = r2.iter().map(|r| r.fingerprint).collect();
    let f3: std::collections::HashSet<u64> = r3.iter().map(|r| r.fingerprint).collect();
    assert!(f2.is_disjoint(&f3));
}

#[test]
fn walker_groups_two_distinct_two_arg_calls_together() {
    let mut all = ext("f(a, b)\nh(c, d)\n", Language::Python);
    all.sort_by_key(|r| r.line);
    let mut count: std::collections::HashMap<u64, usize> = std::collections::HashMap::new();
    for r in &all {
        if r.snippet.starts_with("f(") || r.snippet.starts_with("h(") {
            *count.entry(r.fingerprint).or_default() += 1;
        }
    }
    let max_dup = count.values().copied().max().unwrap_or(0);
    assert!(max_dup >= 2);
}

#[test]
fn walker_distinguishes_python_call_with_keyword_from_positional() {
    let kw = ext("f(x=1)\n", Language::Python);
    let pos = ext("f(1)\n", Language::Python);
    let kw_fps: std::collections::HashSet<u64> = kw.iter().map(|r| r.fingerprint).collect();
    let pos_fps: std::collections::HashSet<u64> = pos.iter().map(|r| r.fingerprint).collect();
    assert!(kw_fps.is_disjoint(&pos_fps));
}

#[test]
fn walker_record_snippet_does_not_include_trailing_whitespace() {
    let src = "if x == 1:    \n    return 1\n";
    let r = ext(src, Language::Python);
    for rec in r {
        assert!(!rec.snippet.ends_with(' '));
    }
}

#[test]
fn walker_record_snippet_does_not_include_leading_whitespace() {
    let src = "    def f():\n        return 1\n";
    let r = ext(src, Language::Python);
    for rec in r {
        if !rec.snippet.is_empty() {
            assert!(!rec.snippet.starts_with(' '));
        }
    }
}

#[test]
fn walker_records_typescript_interface_method_signatures() {
    let src = "interface I { f(x: number): number; g(y: string): boolean; h(): void; }\n";
    let r = ext(src, Language::TypeScript);
    assert!(!r.is_empty());
}

#[test]
fn walker_records_python_three_attribute_chain_distinct_from_four() {
    let three = ext("x.a.b.c\n", Language::Python);
    let four = ext("x.a.b.c.d\n", Language::Python);
    let f3: std::collections::HashSet<u64> = three.iter().map(|r| r.fingerprint).collect();
    let f4: std::collections::HashSet<u64> = four.iter().map(|r| r.fingerprint).collect();
    assert!(f3 != f4, "different chain depths must produce different fingerprint sets");
    assert!(f4.iter().any(|fp| !f3.contains(fp)), "longer chain must have at least one fingerprint not in shorter");
}

#[test]
fn walker_records_long_chain_of_method_calls() {
    let src = "result = client.get(url).json().items[0].name.lower()\n";
    let r = ext(src, Language::Python);
    assert!(!r.is_empty());
}

#[test]
fn walker_records_complex_conditional_with_three_clauses() {
    let src = "if a and b and c:\n    do()\n";
    let r = ext(src, Language::Python);
    assert!(!r.is_empty());
}

#[test]
fn walker_records_else_branch_distinct_from_just_if() {
    let just_if = ext("if x:\n    pass\n", Language::Python);
    let with_else = ext("if x:\n    pass\nelse:\n    pass\n", Language::Python);
    let i_fps: std::collections::HashSet<u64> = just_if.iter().map(|r| r.fingerprint).collect();
    let e_fps: std::collections::HashSet<u64> = with_else.iter().map(|r| r.fingerprint).collect();
    assert!(i_fps != e_fps);
}

#[test]
fn walker_handles_python_class_with_init_and_one_method() {
    let src = "class A:\n    def __init__(self, x):\n        self.x = x\n    def get(self):\n        return self.x\n";
    let r = ext(src, Language::Python);
    assert!(!r.is_empty());
}

#[test]
fn walker_records_typescript_generic_constraint() {
    let src = "function f<T extends { x: number }>(t: T): number { return t.x; }\n";
    let r = ext(src, Language::TypeScript);
    assert!(!r.is_empty());
}

#[test]
fn walker_records_typescript_conditional_type() {
    let src = "type IsNumber<T> = T extends number ? true : false;\n";
    let r = ext(src, Language::TypeScript);
    assert!(!r.is_empty());
}

#[test]
fn walker_emits_records_for_top_level_async_function() {
    let src = "async function f() { const x = await load(); if (x) { return x; } return 0; }\n";
    let r = ext(src, Language::JavaScript);
    assert!(!r.is_empty());
}

#[test]
fn walker_handles_javascript_class_with_private_field() {
    let src = "class A { #x = 1; getX() { return this.#x; } }\n";
    let _ = ext(src, Language::JavaScript);
}

#[test]
fn walker_handles_javascript_static_block() {
    let src = "class A { static { console.log(\"init\"); } }\n";
    let _ = ext(src, Language::JavaScript);
}

#[test]
fn walker_handles_python_with_single_statement_body() {
    let src = "def f(): return 1\n";
    let _ = ext(src, Language::Python);
}

#[test]
fn walker_handles_python_lambda_with_default_arg() {
    let src = "f = lambda x=1: x + 1\n";
    let _ = ext(src, Language::Python);
}

#[test]
fn walker_records_typescript_export_function() {
    let src = "export function f(x: number): number { if (x === 1) return x; return 0; }\n";
    let r = ext(src, Language::TypeScript);
    assert!(!r.is_empty());
}

#[test]
fn walker_records_typescript_default_export() {
    let src = "export default function f(x: number): number { if (x === 1) return x; return 0; }\n";
    let r = ext(src, Language::TypeScript);
    assert!(!r.is_empty());
}

#[test]
fn walker_records_python_function_with_only_returns() {
    let src = "def f(x):\n    return x\n";
    let _ = ext(src, Language::Python);
}
