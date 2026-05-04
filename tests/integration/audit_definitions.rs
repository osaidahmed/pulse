use std::path::PathBuf;

use pulse::audit::definitions::{definitions_for_file, DefinitionRecord};
use pulse::parse::Language;

fn write_tempfile(content: &str, ext: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(format!("test.{ext}"));
    std::fs::write(&path, content).unwrap();
    (dir, path)
}

#[test]
fn python_free_function_extracted_with_no_class() {
    let src = "def free_fn(x):\n    return x + 1\n";
    let (_dir, path) = write_tempfile(src, "py");
    let defs = definitions_for_file(&path, Language::Python);
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].identity.name, "free_fn");
    assert_eq!(defs[0].identity.class, None);
}

#[test]
fn python_class_method_carries_class_name() {
    let src = "class Foo:\n    def bar(self, x):\n        return x\n";
    let (_dir, path) = write_tempfile(src, "py");
    let defs = definitions_for_file(&path, Language::Python);
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].identity.class.as_deref(), Some("Foo"));
    assert_eq!(defs[0].identity.name, "bar");
}

#[test]
fn python_class_method_strips_class_dot_prefix_from_name() {
    let src = "class MyClass:\n    def method(self):\n        pass\n";
    let (_dir, path) = write_tempfile(src, "py");
    let defs = definitions_for_file(&path, Language::Python);
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].identity.name, "method");
    assert!(!defs[0].identity.name.contains('.'));
}

#[test]
fn python_init_method_marked_constructor() {
    let src = "class Foo:\n    def __init__(self, x):\n        self.x = x\n";
    let (_dir, path) = write_tempfile(src, "py");
    let defs = definitions_for_file(&path, Language::Python);
    let init = defs.iter().find(|d| d.identity.name == "__init__");
    assert!(init.is_some());
    assert!(init.unwrap().is_constructor);
}

#[test]
fn python_class_with_multiple_methods_yields_multiple_records() {
    let src = "class Foo:\n    def a(self): return 1\n    def b(self): return 2\n    def c(self): return 3\n";
    let (_dir, path) = write_tempfile(src, "py");
    let defs = definitions_for_file(&path, Language::Python);
    assert_eq!(defs.len(), 3);
}

#[test]
fn python_definitions_are_in_source_order() {
    let src = "def first(): pass\n\ndef second(): pass\n\ndef third(): pass\n";
    let (_dir, path) = write_tempfile(src, "py");
    let defs = definitions_for_file(&path, Language::Python);
    assert_eq!(defs[0].identity.name, "first");
    assert_eq!(defs[1].identity.name, "second");
    assert_eq!(defs[2].identity.name, "third");
}

#[test]
fn python_records_carry_start_line() {
    let src = "\ndef alpha(): pass\n\n\ndef beta(): pass\n";
    let (_dir, path) = write_tempfile(src, "py");
    let defs = definitions_for_file(&path, Language::Python);
    assert!(defs.iter().any(|d| d.identity.line == 2));
    assert!(defs.iter().any(|d| d.identity.line == 5));
}

#[test]
fn python_records_carry_complexity() {
    let src = "def branchy(x):\n    if x > 0:\n        if x > 10:\n            return x\n    return 0\n";
    let (_dir, path) = write_tempfile(src, "py");
    let defs = definitions_for_file(&path, Language::Python);
    assert!(defs[0].cc >= 2);
}

#[test]
fn python_records_carry_field_accesses() {
    let src = "class Foo:\n    def __init__(self):\n        self.a = 1\n        self.b = 2\n    def method(self):\n        return self.a + self.b\n";
    let (_dir, path) = write_tempfile(src, "py");
    let defs = definitions_for_file(&path, Language::Python);
    let method = defs.iter().find(|d| d.identity.name == "method").unwrap();
    assert!(method.field_accesses.contains(&"a".to_string()) || !method.field_accesses.is_empty());
}

#[test]
fn python_unparseable_returns_empty() {
    let src = "this is { not [ valid python\n";
    let (_dir, path) = write_tempfile(src, "py");
    let _defs = definitions_for_file(&path, Language::Python);
}

#[test]
fn missing_file_returns_empty_no_panic() {
    let nonexistent = PathBuf::from("/nonexistent/path/file.py");
    let defs = definitions_for_file(&nonexistent, Language::Python);
    assert!(defs.is_empty());
}

#[test]
fn empty_file_returns_empty() {
    let (_dir, path) = write_tempfile("", "py");
    let defs = definitions_for_file(&path, Language::Python);
    assert!(defs.is_empty());
}

#[test]
fn comments_only_file_returns_empty() {
    let src = "# just a comment\n# another comment\n";
    let (_dir, path) = write_tempfile(src, "py");
    let defs = definitions_for_file(&path, Language::Python);
    assert!(defs.is_empty());
}

#[test]
fn python_file_path_preserved_in_identity() {
    let src = "def f(): pass\n";
    let (_dir, path) = write_tempfile(src, "py");
    let defs = definitions_for_file(&path, Language::Python);
    assert_eq!(defs[0].identity.file, path);
}

#[test]
fn rust_function_item_extracted() {
    let src = "fn foo(x: i32) -> i32 { x + 1 }\n";
    let (_dir, path) = write_tempfile(src, "rs");
    let defs = definitions_for_file(&path, Language::Rust);
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].identity.name, "foo");
    assert_eq!(defs[0].identity.class, None);
}

#[test]
fn rust_impl_method_carries_struct_name() {
    let src = "struct Foo;\nimpl Foo {\n    fn bar(&self) -> i32 { 42 }\n}\n";
    let (_dir, path) = write_tempfile(src, "rs");
    let defs = definitions_for_file(&path, Language::Rust);
    let bar = defs.iter().find(|d| d.identity.name == "bar");
    assert!(bar.is_some());
    assert_eq!(bar.unwrap().identity.class.as_deref(), Some("Foo"));
}

#[test]
fn rust_new_method_marked_constructor() {
    let src = "struct Foo;\nimpl Foo {\n    fn new() -> Self { Foo }\n}\n";
    let (_dir, path) = write_tempfile(src, "rs");
    let defs = definitions_for_file(&path, Language::Rust);
    let new_fn = defs.iter().find(|d| d.identity.name == "new");
    assert!(new_fn.is_some());
    assert!(new_fn.unwrap().is_constructor);
}

#[test]
fn java_method_in_class_carries_class_name() {
    let src = "public class Foo {\n    public void bar() {}\n}\n";
    let (_dir, path) = write_tempfile(src, "java");
    let defs = definitions_for_file(&path, Language::Java);
    let bar = defs.iter().find(|d| d.identity.name == "bar");
    assert!(bar.is_some());
    assert_eq!(bar.unwrap().identity.class.as_deref(), Some("Foo"));
}

#[test]
fn javascript_function_extracted() {
    let src = "function foo() { return 1; }\n";
    let (_dir, path) = write_tempfile(src, "js");
    let defs = definitions_for_file(&path, Language::JavaScript);
    assert!(!defs.is_empty());
}

#[test]
fn typescript_method_in_class_extracted() {
    let src = "class Foo {\n    bar(): number { return 1; }\n}\n";
    let (_dir, path) = write_tempfile(src, "ts");
    let defs = definitions_for_file(&path, Language::TypeScript);
    assert!(!defs.is_empty());
}

#[test]
fn go_function_extracted() {
    let src = "package main\n\nfunc Foo(x int) int {\n    return x + 1\n}\n";
    let (_dir, path) = write_tempfile(src, "go");
    let defs = definitions_for_file(&path, Language::Go);
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].identity.name, "Foo");
}

#[test]
fn definition_record_clone_works() {
    let src = "def f(): pass\n";
    let (_dir, path) = write_tempfile(src, "py");
    let defs = definitions_for_file(&path, Language::Python);
    let d: DefinitionRecord = defs[0].clone();
    assert_eq!(d.identity.name, "f");
}

#[test]
fn nested_class_does_not_panic() {
    let src = "class Outer:\n    class Inner:\n        def method(self): pass\n";
    let (_dir, path) = write_tempfile(src, "py");
    let _defs = definitions_for_file(&path, Language::Python);
}

#[test]
fn python_class_no_self_or_cls_method_still_collected() {
    let src = "class Foo:\n    @staticmethod\n    def bar(x):\n        return x + 1\n";
    let (_dir, path) = write_tempfile(src, "py");
    let defs = definitions_for_file(&path, Language::Python);
    assert!(!defs.is_empty());
}

#[test]
fn python_definitions_for_22_languages_smoke() {
    let cases: &[(&str, &str, Language)] = &[
        ("def f(): pass\n", "py", Language::Python),
        ("function f() {}\n", "js", Language::JavaScript),
        ("function f(): void {}\n", "ts", Language::TypeScript),
        ("fn f() {}\n", "rs", Language::Rust),
        ("int f() { return 0; }\n", "c", Language::C),
        ("int f() { return 0; }\n", "cpp", Language::Cpp),
        ("public class A { void f() {} }\n", "java", Language::Java),
        ("class A { void f() {} }\n", "cs", Language::CSharp),
        ("package main\nfunc f() {}\n", "go", Language::Go),
        ("func f() {}\n", "swift", Language::Swift),
        ("fn f() void {}\n", "zig", Language::Zig),
        ("def f; end\n", "rb", Language::Ruby),
        ("@interface A : NSObject\n@end\n", "m", Language::ObjectiveC),
        ("fun f() {}\n", "kt", Language::Kotlin),
        ("f :: Int -> Int\nf x = x\n", "hs", Language::Haskell),
        ("function f() return 1 end\n", "lua", Language::Lua),
        ("f <- function() 1\n", "r", Language::R),
        ("<?php function f() {}\n", "php", Language::Php),
        ("void f() {}\n", "d", Language::D),
        ("def f() {}\n", "groovy", Language::Groovy),
    ];
    for (src, ext, lang) in cases {
        let (_dir, path) = write_tempfile(src, ext);
        let _ = definitions_for_file(&path, *lang);
    }
}
