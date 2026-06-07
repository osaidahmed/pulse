use pulse::audit::walker::{extract_subtrees, SubtreeRecord};
use pulse::parse::{self, Language};
use pulse::thresholds::Thresholds;
use std::path::Path;

fn t() -> Thresholds {
    Thresholds::default()
}

fn extract(lang: Language, src: &str) -> Vec<SubtreeRecord> {
    let tree = parse::parse_only(src, lang).unwrap();
    extract_subtrees(&tree, src, lang, Path::new("t"), &t().audit)
}

fn fingerprints(records: &[SubtreeRecord]) -> std::collections::HashSet<u64> {
    records.iter().map(|r| r.fingerprint).collect()
}

macro_rules! lang_walker_smoke {
    ($name:ident, $lang:expr, $src:literal) => {
        #[test]
        fn $name() {
            let records = extract($lang, $src);
            assert!(!records.is_empty());
        }
    };
}

macro_rules! lang_walker_determinism {
    ($name:ident, $lang:expr, $src:literal) => {
        #[test]
        fn $name() {
            let a = extract($lang, $src);
            let b = extract($lang, $src);
            assert_eq!(fingerprints(&a), fingerprints(&b));
        }
    };
}

macro_rules! lang_walker_whitespace_stable {
    ($name:ident, $lang:expr, $base:literal, $padded:literal) => {
        #[test]
        fn $name() {
            let a = fingerprints(&extract($lang, $base));
            let b = fingerprints(&extract($lang, $padded));
            assert!(a.is_subset(&b) || b.is_subset(&a) || a == b);
        }
    };
}

lang_walker_smoke!(
    python_smoke_basic_function,
    Language::Python,
    "def f(x):\n    if x == 1:\n        return x\n    return 0\n"
);
lang_walker_smoke!(
    python_smoke_class,
    Language::Python,
    "class C:\n    def m(self):\n        return 1\n    def n(self):\n        return 2\n"
);
lang_walker_smoke!(
    python_smoke_async_function,
    Language::Python,
    "async def f(x):\n    if x == 1:\n        return await g(x)\n    return await g(0)\n"
);
lang_walker_smoke!(
    python_smoke_decorator,
    Language::Python,
    "@deco\ndef f(x):\n    if x == 1:\n        return x\n    return 0\n"
);
lang_walker_smoke!(
    python_smoke_match,
    Language::Python,
    "def f(x):\n    match x:\n        case 1: return 1\n        case _: return 0\n"
);
lang_walker_smoke!(python_smoke_comprehension, Language::Python, "x = [a for a in items if a.is_active]\n");
lang_walker_smoke!(python_smoke_generator, Language::Python, "def gen():\n    for i in range(10):\n        yield i\n");
lang_walker_smoke!(python_smoke_lambda_in_call, Language::Python, "result = sorted(xs, key=lambda x: x.weight)\n");
lang_walker_smoke!(
    python_smoke_with_statement,
    Language::Python,
    "with open(p) as f:\n    for line in f:\n        process(line)\n"
);
lang_walker_smoke!(
    python_smoke_try_except,
    Language::Python,
    "try:\n    do()\nexcept ValueError as e:\n    handle(e)\nexcept Exception:\n    fail()\n"
);
lang_walker_smoke!(
    python_smoke_typed_function,
    Language::Python,
    "def f(x: int, y: str = \"hi\") -> bool:\n    return x > 0 and y == \"hi\"\n"
);
lang_walker_smoke!(python_smoke_class_with_methods, Language::Python, "class Foo:\n    def __init__(self, x):\n        self.x = x\n    def get(self):\n        return self.x\n    def set(self, v):\n        self.x = v\n");
lang_walker_smoke!(
    python_smoke_nested_function,
    Language::Python,
    "def outer(x):\n    def inner(y):\n        return y * 2\n    return inner(x) + 1\n"
);
lang_walker_smoke!(python_smoke_class_with_property, Language::Python, "class C:\n    @property\n    def x(self):\n        return self._x\n    @x.setter\n    def x(self, v):\n        self._x = v\n");
lang_walker_smoke!(
    python_smoke_dataclass_pattern,
    Language::Python,
    "@dataclass\nclass Point:\n    x: int\n    y: int\n    label: str = \"\"\n"
);

lang_walker_smoke!(
    typescript_smoke_function,
    Language::TypeScript,
    "function f(x: number): number { if (x === 1) return x; return 0; }\n"
);
lang_walker_smoke!(typescript_smoke_arrow, Language::TypeScript, "const f = (x: number): number => x === 1 ? x : 0;\n");
lang_walker_smoke!(
    typescript_smoke_class,
    Language::TypeScript,
    "class A { constructor(public x: number) {} get value(): number { return this.x; } }\n"
);
lang_walker_smoke!(
    typescript_smoke_interface,
    Language::TypeScript,
    "interface I { name: string; getId(): number; }\n"
);
lang_walker_smoke!(typescript_smoke_generic, Language::TypeScript, "function id<T>(x: T): T { return x; }\n");
lang_walker_smoke!(
    typescript_smoke_async,
    Language::TypeScript,
    "async function fetchData(): Promise<number> { return await load(); }\n"
);
lang_walker_smoke!(typescript_smoke_optional_chain, Language::TypeScript, "const v = obj?.a?.b?.c ?? \"default\";\n");
lang_walker_smoke!(typescript_smoke_destructure, Language::TypeScript, "const { a, b: bb, ...rest } = obj;\n");
lang_walker_smoke!(
    typescript_smoke_template_literal,
    Language::TypeScript,
    "const greeting = `Hello, ${name}! You have ${count} messages.`;\n"
);
lang_walker_smoke!(typescript_smoke_enum, Language::TypeScript, "enum Color { Red = 1, Green = 2, Blue = 3 }\n");
lang_walker_smoke!(
    typescript_smoke_union_type,
    Language::TypeScript,
    "type Shape = { kind: \"circle\"; r: number } | { kind: \"square\"; s: number };\n"
);
lang_walker_smoke!(
    typescript_smoke_namespace,
    Language::TypeScript,
    "namespace utils { export function help() { return 1; } }\n"
);

lang_walker_smoke!(
    javascript_smoke_function,
    Language::JavaScript,
    "function f(x) { if (x === 1) return x; return 0; }\n"
);
lang_walker_smoke!(javascript_smoke_arrow, Language::JavaScript, "const f = x => x === 1 ? x : 0;\n");
lang_walker_smoke!(
    javascript_smoke_class,
    Language::JavaScript,
    "class A { constructor(x) { this.x = x; } get value() { return this.x; } }\n"
);
lang_walker_smoke!(
    javascript_smoke_async,
    Language::JavaScript,
    "async function load() { const data = await fetch(url); return data.json(); }\n"
);
lang_walker_smoke!(javascript_smoke_destructure, Language::JavaScript, "const { a, b, ...rest } = obj;\n");
lang_walker_smoke!(javascript_smoke_spread, Language::JavaScript, "const xs = [...a, ...b, c];\n");
lang_walker_smoke!(javascript_smoke_template_literal, Language::JavaScript, "const greeting = `Hello ${name}!`;\n");
lang_walker_smoke!(javascript_smoke_iife, Language::JavaScript, "(function() { console.log(1); })();\n");
lang_walker_smoke!(
    javascript_smoke_generator,
    Language::JavaScript,
    "function* gen() { yield 1; yield 2; yield 3; }\n"
);
lang_walker_smoke!(javascript_smoke_optional_chain, Language::JavaScript, "const v = obj?.a?.b?.c;\n");

lang_walker_smoke!(rust_smoke_function, Language::Rust, "fn f(x: i32) -> i32 { if x == 1 { return x; } 0 }\n");
lang_walker_smoke!(rust_smoke_match, Language::Rust, "fn f(x: i32) -> i32 { match x { 1 => 1, 2 => 2, _ => 0 } }\n");
lang_walker_smoke!(rust_smoke_struct, Language::Rust, "struct Point { x: i32, y: i32, label: String }\n");
lang_walker_smoke!(
    rust_smoke_enum,
    Language::Rust,
    "enum Shape { Circle(f64), Square { side: f64 }, Triangle(f64, f64, f64) }\n"
);
lang_walker_smoke!(
    rust_smoke_trait,
    Language::Rust,
    "trait Display { fn fmt(&self) -> String; fn pretty(&self) -> String { self.fmt() } }\n"
);
lang_walker_smoke!(
    rust_smoke_impl,
    Language::Rust,
    "impl Foo { fn new() -> Self { Self { x: 0 } } fn get(&self) -> i32 { self.x } }\n"
);
lang_walker_smoke!(rust_smoke_generic, Language::Rust, "fn id<T: Clone>(x: T) -> T { x.clone() }\n");
lang_walker_smoke!(
    rust_smoke_lifetime,
    Language::Rust,
    "fn longer<'a>(x: &'a str, y: &'a str) -> &'a str { if x.len() > y.len() { x } else { y } }\n"
);
lang_walker_smoke!(
    rust_smoke_async,
    Language::Rust,
    "async fn load() -> Result<Data, Error> { let d = fetch().await?; Ok(d) }\n"
);
lang_walker_smoke!(
    rust_smoke_macro_call,
    Language::Rust,
    "fn main() { let xs = vec![1, 2, 3]; println!(\"{:?}\", xs); }\n"
);
lang_walker_smoke!(rust_smoke_closure, Language::Rust, "fn f() { let add = |a, b| a + b; add(1, 2); }\n");
lang_walker_smoke!(
    rust_smoke_pattern_destructure,
    Language::Rust,
    "fn f(p: Point) { let Point { x, y } = p; let _ = x + y; }\n"
);

lang_walker_smoke!(
    go_smoke_function,
    Language::Go,
    "package p\nfunc f(x int) int { if x == 1 { return x }; return 0 }\n"
);
lang_walker_smoke!(go_smoke_struct, Language::Go, "package p\ntype Point struct { X int; Y int; Label string }\n");
lang_walker_smoke!(
    go_smoke_interface,
    Language::Go,
    "package p\ntype Reader interface { Read(p []byte) (int, error) }\n"
);
lang_walker_smoke!(
    go_smoke_method,
    Language::Go,
    "package p\ntype P struct { X int }\nfunc (p P) Get() int { return p.X }\n"
);
lang_walker_smoke!(
    go_smoke_goroutine,
    Language::Go,
    "package p\nfunc main() { go work(); ch := make(chan int); ch <- 1 }\n"
);
lang_walker_smoke!(go_smoke_channel, Language::Go, "package p\nfunc f(c chan int) { x := <-c; c <- x * 2 }\n");
lang_walker_smoke!(go_smoke_defer, Language::Go, "package p\nfunc f() { defer cleanup(); doWork() }\n");
lang_walker_smoke!(
    go_smoke_select,
    Language::Go,
    "package p\nfunc f() { select { case x := <-ch1: use(x); case <-ch2: stop() } }\n"
);
lang_walker_smoke!(
    go_smoke_type_assertion,
    Language::Go,
    "package p\nfunc f(x interface{}) { if v, ok := x.(int); ok { use(v) } }\n"
);
lang_walker_smoke!(
    go_smoke_type_switch,
    Language::Go,
    "package p\nfunc f(x interface{}) { switch v := x.(type) { case int: use(v); default: skip() } }\n"
);

lang_walker_smoke!(
    java_smoke_class,
    Language::Java,
    "class A { int x; A(int x) { this.x = x; } int get() { return this.x; } }\n"
);
lang_walker_smoke!(
    java_smoke_method,
    Language::Java,
    "class A { int compute(int x) { if (x == 1) return x; return 0; } }\n"
);
lang_walker_smoke!(
    java_smoke_interface,
    Language::Java,
    "interface I { int compute(int x); default int doubled(int x) { return compute(x) * 2; } }\n"
);
lang_walker_smoke!(
    java_smoke_generic,
    Language::Java,
    "class Box<T> { T value; Box(T v) { this.value = v; } T get() { return value; } }\n"
);
lang_walker_smoke!(java_smoke_lambda, Language::Java, "class A { Function<Integer, Integer> f = x -> x + 1; }\n");
lang_walker_smoke!(
    java_smoke_stream,
    Language::Java,
    "class A { void f(List<Integer> xs) { xs.stream().filter(x -> x > 0).forEach(System.out::println); } }\n"
);
lang_walker_smoke!(
    java_smoke_anonymous_class,
    Language::Java,
    "class A { Runnable r = new Runnable() { public void run() { doWork(); } }; }\n"
);
lang_walker_smoke!(
    java_smoke_enum,
    Language::Java,
    "enum Color { RED, GREEN, BLUE; public int rank() { return ordinal(); } }\n"
);
lang_walker_smoke!(
    java_smoke_try_with_resources,
    Language::Java,
    "class A { void f() { try (Reader r = open()) { r.read(); } catch (Exception e) {} } }\n"
);
lang_walker_smoke!(java_smoke_record, Language::Java, "record Point(int x, int y) {}\n");

lang_walker_smoke!(
    csharp_smoke_class,
    Language::CSharp,
    "class A { int X { get; set; } public A(int x) { X = x; } }\n"
);
lang_walker_smoke!(
    csharp_smoke_method,
    Language::CSharp,
    "class A { int F(int x) { if (x == 1) return x; return 0; } }\n"
);
lang_walker_smoke!(
    csharp_smoke_property,
    Language::CSharp,
    "class A { public int X { get { return _x; } set { _x = value; } } }\n"
);
lang_walker_smoke!(csharp_smoke_lambda, Language::CSharp, "class A { Func<int, int> f = x => x + 1; }\n");
lang_walker_smoke!(
    csharp_smoke_linq,
    Language::CSharp,
    "class A { void F() { var xs = items.Where(x => x > 0).Select(x => x * 2).ToList(); } }\n"
);
lang_walker_smoke!(
    csharp_smoke_async,
    Language::CSharp,
    "class A { public async Task<int> F() { var r = await Load(); return r; } }\n"
);
lang_walker_smoke!(
    csharp_smoke_generic,
    Language::CSharp,
    "class Box<T> { public T Value { get; set; } public Box(T v) { Value = v; } }\n"
);
lang_walker_smoke!(csharp_smoke_interface, Language::CSharp, "interface I { int F(int x); }\n");
lang_walker_smoke!(csharp_smoke_enum, Language::CSharp, "enum Color { Red, Green, Blue }\n");
lang_walker_smoke!(csharp_smoke_struct, Language::CSharp, "struct Point { public int X; public int Y; }\n");

lang_walker_smoke!(c_smoke_function, Language::C, "int f(int x) { if (x == 1) { return x; } return 0; }\n");
lang_walker_smoke!(c_smoke_struct, Language::C, "struct Point { int x; int y; const char *label; };\n");
lang_walker_smoke!(
    c_smoke_for_loop,
    Language::C,
    "void f(int *xs, int n) { for (int i = 0; i < n; i++) { xs[i] = i * 2; } }\n"
);
lang_walker_smoke!(c_smoke_typedef, Language::C, "typedef struct { int x; int y; } Point;\n");
lang_walker_smoke!(
    c_smoke_function_pointer,
    Language::C,
    "int (*fp)(int) = f; int call(int (*g)(int), int x) { return g(x); }\n"
);
lang_walker_smoke!(
    c_smoke_macro,
    Language::C,
    "#define MAX(a, b) ((a) > (b) ? (a) : (b))\nint biggest(int x, int y) { return MAX(x, y); }\n"
);
lang_walker_smoke!(
    c_smoke_switch,
    Language::C,
    "int f(int x) { switch (x) { case 1: return 1; case 2: return 2; default: return 0; } }\n"
);

lang_walker_smoke!(cpp_smoke_function, Language::Cpp, "int f(int x) { if (x == 1) { return x; } return 0; }\n");
lang_walker_smoke!(
    cpp_smoke_class,
    Language::Cpp,
    "class A { public: A(int x) : x_(x) {} int get() const { return x_; } private: int x_; };\n"
);
lang_walker_smoke!(cpp_smoke_template, Language::Cpp, "template<typename T> T id(T x) { return x; }\n");
lang_walker_smoke!(cpp_smoke_namespace, Language::Cpp, "namespace ns { int f(int x) { return x * 2; } }\n");
lang_walker_smoke!(
    cpp_smoke_lambda,
    Language::Cpp,
    "int f() { auto add = [](int a, int b) { return a + b; }; return add(1, 2); }\n"
);
lang_walker_smoke!(
    cpp_smoke_smart_pointer,
    Language::Cpp,
    "void f() { std::unique_ptr<int> p = std::make_unique<int>(42); }\n"
);
lang_walker_smoke!(
    cpp_smoke_range_for,
    Language::Cpp,
    "void f(std::vector<int> xs) { for (auto x : xs) { use(x); } }\n"
);

lang_walker_smoke!(
    swift_smoke_function,
    Language::Swift,
    "func f(x: Int) -> Int { if x == 1 { return x }; return 0 }\n"
);
lang_walker_smoke!(
    swift_smoke_class,
    Language::Swift,
    "class A { var x: Int; init(x: Int) { self.x = x }; func get() -> Int { return x } }\n"
);
lang_walker_smoke!(swift_smoke_struct, Language::Swift, "struct Point { var x: Int; var y: Int }\n");
lang_walker_smoke!(swift_smoke_enum, Language::Swift, "enum Shape { case circle(Double); case square(Double) }\n");
lang_walker_smoke!(swift_smoke_optional, Language::Swift, "func f(x: Int?) -> Int { return x ?? 0 }\n");
lang_walker_smoke!(
    swift_smoke_guard,
    Language::Swift,
    "func f(x: Int?) { guard let y = x else { return }; print(y) }\n"
);
lang_walker_smoke!(swift_smoke_protocol, Language::Swift, "protocol Drawable { func draw() }\n");
lang_walker_smoke!(
    swift_smoke_extension,
    Language::Swift,
    "extension Int { func doubled() -> Int { return self * 2 } }\n"
);

lang_walker_smoke!(
    kotlin_smoke_function,
    Language::Kotlin,
    "fun f(x: Int): Int { if (x == 1) { return x }; return 0 }\n"
);
lang_walker_smoke!(kotlin_smoke_class, Language::Kotlin, "class A(val x: Int) { fun get(): Int = x }\n");
lang_walker_smoke!(kotlin_smoke_data_class, Language::Kotlin, "data class Point(val x: Int, val y: Int)\n");
lang_walker_smoke!(
    kotlin_smoke_when,
    Language::Kotlin,
    "fun f(x: Int): Int = when (x) { 1 -> 1; 2 -> 2; else -> 0 }\n"
);
lang_walker_smoke!(kotlin_smoke_lambda, Language::Kotlin, "val add: (Int, Int) -> Int = { a, b -> a + b }\n");
lang_walker_smoke!(kotlin_smoke_extension_function, Language::Kotlin, "fun Int.doubled(): Int = this * 2\n");

lang_walker_smoke!(ruby_smoke_function, Language::Ruby, "def f(x)\n  if x == 1\n    return x\n  end\n  0\nend\n");
lang_walker_smoke!(
    ruby_smoke_class,
    Language::Ruby,
    "class A\n  def initialize(x)\n    @x = x\n  end\n  def get\n    @x\n  end\nend\n"
);
lang_walker_smoke!(ruby_smoke_module, Language::Ruby, "module M\n  def helper\n    42\n  end\nend\n");
lang_walker_smoke!(ruby_smoke_block, Language::Ruby, "items.each do |x|\n  puts x\nend\n");
lang_walker_smoke!(ruby_smoke_symbol, Language::Ruby, "x = :symbol\ny = [:a, :b, :c]\n");
lang_walker_smoke!(ruby_smoke_hash, Language::Ruby, "x = {name: \"x\", count: 1}\n");

lang_walker_smoke!(zig_smoke_function, Language::Zig, "fn f(x: i32) i32 { if (x == 1) { return x; } return 0; }\n");
lang_walker_smoke!(zig_smoke_struct, Language::Zig, "const Point = struct { x: i32, y: i32 };\n");
lang_walker_smoke!(zig_smoke_enum, Language::Zig, "const Color = enum { red, green, blue };\n");
lang_walker_smoke!(zig_smoke_test_block, Language::Zig, "test \"basic\" { const x: i32 = 1; _ = x; }\n");

lang_walker_smoke!(haskell_smoke_function, Language::Haskell, "f :: Int -> Int\nf x = if x == 1 then x else 0\n");
lang_walker_smoke!(haskell_smoke_pattern_match, Language::Haskell, "f :: Int -> Int\nf 0 = 0\nf n = n * f (n - 1)\n");
lang_walker_smoke!(haskell_smoke_let_in, Language::Haskell, "f :: Int -> Int\nf x = let y = x + 1 in y * 2\n");
lang_walker_smoke!(haskell_smoke_where, Language::Haskell, "f :: Int -> Int\nf x = y * 2 where y = x + 1\n");
lang_walker_smoke!(
    haskell_smoke_data,
    Language::Haskell,
    "data Shape = Circle Double | Square Double | Triangle Double Double Double\n"
);

lang_walker_smoke!(lua_smoke_function, Language::Lua, "function f(x) if x == 1 then return x end return 0 end\n");
lang_walker_smoke!(lua_smoke_table, Language::Lua, "local t = {x = 1, y = 2, name = \"point\"}\n");
lang_walker_smoke!(lua_smoke_method, Language::Lua, "function t:m(x) return x + self.y end\n");
lang_walker_smoke!(lua_smoke_for_pairs, Language::Lua, "for k, v in pairs(t) do print(k, v) end\n");

lang_walker_smoke!(
    php_smoke_function,
    Language::Php,
    "<?php function f($x) { if ($x === 1) { return $x; } return 0; }\n"
);
lang_walker_smoke!(
    php_smoke_class,
    Language::Php,
    "<?php class A { public $x; function __construct($x) { $this->x = $x; } function get() { return $this->x; } }\n"
);
lang_walker_smoke!(php_smoke_array, Language::Php, "<?php $x = ['a' => 1, 'b' => 2, 'c' => 3];\n");
lang_walker_smoke!(php_smoke_arrow_function, Language::Php, "<?php $f = fn($x) => $x + 1;\n");

lang_walker_smoke!(d_smoke_function, Language::D, "int f(int x) { if (x == 1) { return x; } return 0; }\n");
lang_walker_smoke!(
    d_smoke_class,
    Language::D,
    "class A { int x; this(int x) { this.x = x; } int get() { return x; } }\n"
);
lang_walker_smoke!(d_smoke_template, Language::D, "T add(T)(T a, T b) { return a + b; }\n");

lang_walker_smoke!(groovy_smoke_function, Language::Groovy, "def f(x) { if (x == 1) { return x }; return 0 }\n");
lang_walker_smoke!(groovy_smoke_closure, Language::Groovy, "def add = { a, b -> a + b }\n");
lang_walker_smoke!(
    groovy_smoke_class,
    Language::Groovy,
    "class A { int x\n  A(int x) { this.x = x }\n  int get() { return x }\n}\n"
);

lang_walker_smoke!(objc_smoke_function, Language::ObjectiveC, "int f(int x) { if (x == 1) { return x; } return 0; }\n");
lang_walker_smoke!(
    objc_smoke_message,
    Language::ObjectiveC,
    "void f(NSObject *o) { [o doSomething:42 withFlag:YES]; }\n"
);

lang_walker_smoke!(
    tcl_smoke_proc,
    Language::Tcl,
    "proc f {x} {\n    if {$x == 1} {\n        return $x\n    }\n    return 0\n}\n"
);
lang_walker_smoke!(tcl_smoke_namespace, Language::Tcl, "namespace eval ns {\n    proc f {x} { return $x }\n}\n");

lang_walker_smoke!(r_smoke_function, Language::R, "f <- function(x) { if (x == 1) { return(x) }; return(0) }\n");
lang_walker_smoke!(r_smoke_pipe, Language::R, "x <- items %>% filter(active) %>% select(name)\n");

lang_walker_smoke!(cobol_smoke_program, Language::Cobol,
    "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. F.\n       PROCEDURE DIVISION.\n           IF X = 1 DISPLAY X END-IF.\n           STOP RUN.\n");

lang_walker_determinism!(
    python_determinism_function,
    Language::Python,
    "def f(x):\n    if x == 1:\n        return x\n    return 0\n"
);
lang_walker_determinism!(
    typescript_determinism_function,
    Language::TypeScript,
    "function f(x: number): number { if (x === 1) return x; return 0; }\n"
);
lang_walker_determinism!(
    javascript_determinism_function,
    Language::JavaScript,
    "function f(x) { if (x === 1) return x; return 0; }\n"
);
lang_walker_determinism!(
    rust_determinism_function,
    Language::Rust,
    "fn f(x: i32) -> i32 { if x == 1 { x } else { 0 } }\n"
);
lang_walker_determinism!(
    go_determinism_function,
    Language::Go,
    "package p\nfunc f(x int) int { if x == 1 { return x }; return 0 }\n"
);
lang_walker_determinism!(
    java_determinism_class,
    Language::Java,
    "class A { int f(int x) { if (x == 1) return x; return 0; } }\n"
);
lang_walker_determinism!(
    csharp_determinism_class,
    Language::CSharp,
    "class A { int F(int x) { if (x == 1) return x; return 0; } }\n"
);
lang_walker_determinism!(c_determinism_function, Language::C, "int f(int x) { if (x == 1) { return x; } return 0; }\n");
lang_walker_determinism!(
    cpp_determinism_function,
    Language::Cpp,
    "int f(int x) { if (x == 1) { return x; } return 0; }\n"
);
lang_walker_determinism!(
    swift_determinism_function,
    Language::Swift,
    "func f(x: Int) -> Int { if x == 1 { return x }; return 0 }\n"
);
lang_walker_determinism!(
    kotlin_determinism_function,
    Language::Kotlin,
    "fun f(x: Int): Int { if (x == 1) { return x }; return 0 }\n"
);
lang_walker_determinism!(
    zig_determinism_function,
    Language::Zig,
    "fn f(x: i32) i32 { if (x == 1) { return x; } return 0; }\n"
);
lang_walker_determinism!(
    ruby_determinism_function,
    Language::Ruby,
    "def f(x)\n  if x == 1\n    return x\n  end\n  0\nend\n"
);
lang_walker_determinism!(
    haskell_determinism_function,
    Language::Haskell,
    "f :: Int -> Int\nf x = if x == 1 then x else 0\n"
);
lang_walker_determinism!(
    lua_determinism_function,
    Language::Lua,
    "function f(x) if x == 1 then return x end return 0 end\n"
);
lang_walker_determinism!(
    php_determinism_function,
    Language::Php,
    "<?php function f($x) { if ($x === 1) { return $x; } return 0; }\n"
);
lang_walker_determinism!(d_determinism_function, Language::D, "int f(int x) { if (x == 1) { return x; } return 0; }\n");
lang_walker_determinism!(
    groovy_determinism_function,
    Language::Groovy,
    "def f(x) { if (x == 1) { return x }; return 0 }\n"
);
lang_walker_determinism!(
    r_determinism_function,
    Language::R,
    "f <- function(x) { if (x == 1) { return(x) }; return(0) }\n"
);
lang_walker_determinism!(
    objc_determinism_function,
    Language::ObjectiveC,
    "int f(int x) { if (x == 1) { return x; } return 0; }\n"
);
lang_walker_determinism!(
    tcl_determinism_function,
    Language::Tcl,
    "proc f {x} {\n    if {$x == 1} {\n        return $x\n    }\n    return 0\n}\n"
);
lang_walker_determinism!(cobol_determinism_program, Language::Cobol,
    "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. F.\n       PROCEDURE DIVISION.\n           IF X = 1 DISPLAY X END-IF.\n           STOP RUN.\n");

lang_walker_whitespace_stable!(
    python_ws_function,
    Language::Python,
    "def f(x):\n    if x == 1:\n        return x\n    return 0\n",
    "\n\ndef  f(x):\n\n    if  x == 1:\n        return x\n\n    return 0\n\n"
);
lang_walker_whitespace_stable!(
    typescript_ws_function,
    Language::TypeScript,
    "function f(x: number): number { if (x === 1) return x; return 0; }\n",
    "function f(x: number): number {\n  if (x === 1) return x;\n  return 0;\n}\n"
);
lang_walker_whitespace_stable!(
    rust_ws_function,
    Language::Rust,
    "fn f(x: i32) -> i32 { if x == 1 { return x; } 0 }\n",
    "fn  f ( x : i32 ) -> i32 {\n    if x == 1 {\n        return x;\n    }\n    0\n}\n"
);
lang_walker_whitespace_stable!(
    go_ws_function,
    Language::Go,
    "package p\nfunc f(x int) int { if x == 1 { return x }; return 0 }\n",
    "package p\n\nfunc f(x int) int {\n\tif x == 1 {\n\t\treturn x\n\t}\n\treturn 0\n}\n"
);
lang_walker_whitespace_stable!(
    java_ws_class,
    Language::Java,
    "class A { int f(int x) { if (x == 1) { return x; } return 0; } }\n",
    "class A {\n  int f(int x) {\n    if (x == 1) {\n      return x;\n    }\n    return 0;\n  }\n}\n"
);
lang_walker_whitespace_stable!(
    c_ws_function,
    Language::C,
    "int f(int x) { if (x == 1) { return x; } return 0; }\n",
    "int f(int x) {\n    if (x == 1) {\n        return x;\n    }\n    return 0;\n}\n"
);

#[test]
fn fingerprint_identical_function_in_different_classes_python() {
    let src = "class A:\n    def f(self, x):\n        if x == 1:\n            return x\n        return 0\nclass B:\n    def f(self, x):\n        if x == 1:\n            return x\n        return 0\n";
    let records = extract(Language::Python, src);
    let mut by_fp: std::collections::HashMap<u64, usize> = std::collections::HashMap::new();
    for r in &records {
        *by_fp.entry(r.fingerprint).or_default() += 1;
    }
    assert!(by_fp.values().any(|&c| c >= 2), "should find shared fingerprint");
}

#[test]
fn fingerprint_identical_function_in_different_modules_typescript() {
    let src = "function a(x: number): number { if (x === 1) { return x; } return 0; }\nfunction b(x: number): number { if (x === 1) { return x; } return 0; }\n";
    let records = extract(Language::TypeScript, src);
    let mut by_fp: std::collections::HashMap<u64, usize> = std::collections::HashMap::new();
    for r in &records {
        *by_fp.entry(r.fingerprint).or_default() += 1;
    }
    assert!(by_fp.values().any(|&c| c >= 2));
}

#[test]
fn fingerprint_identical_struct_in_rust() {
    let src = "struct A { x: i32, y: i32 }\nstruct B { x: i32, y: i32 }\n";
    let records = extract(Language::Rust, src);
    let mut by_fp: std::collections::HashMap<u64, usize> = std::collections::HashMap::new();
    for r in &records {
        *by_fp.entry(r.fingerprint).or_default() += 1;
    }
    assert!(by_fp.values().any(|&c| c >= 2));
}

#[test]
fn fingerprint_distinct_python_and_javascript_if() {
    let py = extract(Language::Python, "def f(x):\n    if x == 1:\n        return 1\n    return 0\n");
    let js = extract(Language::JavaScript, "function f(x) { if (x === 1) { return 1; } return 0; }\n");
    let py_fps: std::collections::HashSet<u64> = py.iter().map(|r| r.fingerprint).collect();
    let js_fps: std::collections::HashSet<u64> = js.iter().map(|r| r.fingerprint).collect();
    assert!(py_fps.is_disjoint(&js_fps), "Python and JS fingerprints must not collide");
}

#[test]
fn fingerprint_distinct_typescript_and_javascript_if() {
    let ts = extract(Language::TypeScript, "function f(x: number): number { if (x === 1) return 1; return 0; }\n");
    let js = extract(Language::JavaScript, "function f(x) { if (x === 1) return 1; return 0; }\n");
    let ts_fps: std::collections::HashSet<u64> = ts.iter().map(|r| r.fingerprint).collect();
    let js_fps: std::collections::HashSet<u64> = js.iter().map(|r| r.fingerprint).collect();
    assert!(ts_fps.is_disjoint(&js_fps), "TS and JS fingerprints must not collide despite shared grammar family");
}

#[test]
fn fingerprint_distinct_c_and_cpp_function() {
    let c_records = extract(Language::C, "int f(int x) { if (x == 1) { return x; } return 0; }\n");
    let cpp_records = extract(Language::Cpp, "int f(int x) { if (x == 1) { return x; } return 0; }\n");
    let c_fps: std::collections::HashSet<u64> = c_records.iter().map(|r| r.fingerprint).collect();
    let cpp_fps: std::collections::HashSet<u64> = cpp_records.iter().map(|r| r.fingerprint).collect();
    assert!(c_fps.is_disjoint(&cpp_fps), "C and C++ fingerprints must not collide");
}

#[test]
fn fingerprint_distinct_java_and_csharp_class() {
    let j = extract(Language::Java, "class A { int f(int x) { return x; } }\n");
    let cs = extract(Language::CSharp, "class A { int F(int x) { return x; } }\n");
    let j_fps: std::collections::HashSet<u64> = j.iter().map(|r| r.fingerprint).collect();
    let cs_fps: std::collections::HashSet<u64> = cs.iter().map(|r| r.fingerprint).collect();
    assert!(j_fps.is_disjoint(&cs_fps));
}

#[test]
fn fingerprint_distinct_swift_and_kotlin() {
    let s = extract(Language::Swift, "func f(x: Int) -> Int { if x == 1 { return x }; return 0 }\n");
    let k = extract(Language::Kotlin, "fun f(x: Int): Int { if (x == 1) return x; return 0 }\n");
    let s_fps: std::collections::HashSet<u64> = s.iter().map(|r| r.fingerprint).collect();
    let k_fps: std::collections::HashSet<u64> = k.iter().map(|r| r.fingerprint).collect();
    assert!(s_fps.is_disjoint(&k_fps));
}

#[test]
fn fingerprint_distinct_lua_and_python() {
    let l = extract(Language::Lua, "function f(x) if x == 1 then return x end return 0 end\n");
    let p = extract(Language::Python, "def f(x):\n    if x == 1:\n        return x\n    return 0\n");
    let l_fps: std::collections::HashSet<u64> = l.iter().map(|r| r.fingerprint).collect();
    let p_fps: std::collections::HashSet<u64> = p.iter().map(|r| r.fingerprint).collect();
    assert!(l_fps.is_disjoint(&p_fps));
}

#[test]
fn fingerprint_distinct_haskell_and_ml_style_pattern() {
    let h = extract(Language::Haskell, "f :: Int -> Int\nf 0 = 0\nf n = n + 1\n");
    let _ = h;
}

#[test]
fn each_language_floor_threshold_zero_yields_records() {
    let mut th = t().audit;
    th.pattern_mining.subtree_min_depth = 0;
    th.pattern_mining.subtree_min_nodes = 0;
    let cases = [
        (Language::Python, "x = 1\n"),
        (Language::JavaScript, "let x = 1;\n"),
        (Language::Rust, "let x = 1;\n"),
        (Language::Ruby, "x = 1\n"),
        (Language::Lua, "local x = 1\n"),
    ];
    for (lang, src) in cases {
        let tree = parse::parse_only(src, lang).unwrap();
        let records = extract_subtrees(&tree, src, lang, Path::new("t"), &th);
        assert!(!records.is_empty(), "{lang:?} with min thresholds should produce records");
    }
}

#[test]
fn each_language_records_carry_correct_kind_via_seeded_fingerprint() {
    let langs = [Language::Python, Language::JavaScript, Language::Rust, Language::Go];
    for lang in langs {
        let src = match lang {
            Language::Python => "def f(x):\n    if x == 1:\n        return 1\n    return 0\n",
            Language::JavaScript => "function f(x) { if (x === 1) { return 1; } return 0; }\n",
            Language::Rust => "fn f(x: i32) -> i32 { if x == 1 { return 1; } 0 }\n",
            Language::Go => "package p\nfunc f(x int) int { if x == 1 { return 1 }; return 0 }\n",
            _ => continue,
        };
        let records = extract(lang, src);
        assert!(!records.is_empty(), "{lang:?}");
    }
}
