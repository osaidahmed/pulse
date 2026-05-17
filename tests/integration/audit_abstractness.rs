use std::path::PathBuf;

use pulse::audit::abstractness::abstractness_for_file;
use pulse::audit::finding::ImportConfidence;
use pulse::parse::Language;

fn write_tempfile(content: &str, ext: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(format!("test.{ext}"));
    std::fs::write(&path, content).unwrap();
    (dir, path)
}

#[test]
fn java_one_interface_one_class_yields_half() {
    let src = "interface Foo {}\nclass Bar implements Foo {}\n";
    let (_dir, path) = write_tempfile(src, "java");
    let r = abstractness_for_file(&path, Language::Java);
    assert!((r.abstractness - 0.5).abs() < 0.001);
    assert_eq!(r.confidence, ImportConfidence::High);
}

#[test]
fn java_only_interfaces_yields_one() {
    let src = "interface A {}\ninterface B {}\n";
    let (_dir, path) = write_tempfile(src, "java");
    let r = abstractness_for_file(&path, Language::Java);
    assert!((r.abstractness - 1.0).abs() < 0.001);
}

#[test]
fn java_only_classes_yields_zero() {
    let src = "class A {}\nclass B {}\n";
    let (_dir, path) = write_tempfile(src, "java");
    let r = abstractness_for_file(&path, Language::Java);
    assert!(r.abstractness.abs() < 0.001);
    assert_eq!(r.confidence, ImportConfidence::High);
}

#[test]
fn java_no_types_yields_naabstraction() {
    let src = "// just a comment\n";
    let (_dir, path) = write_tempfile(src, "java");
    let r = abstractness_for_file(&path, Language::Java);
    assert_eq!(r.confidence, ImportConfidence::NaAbstraction);
}

#[test]
fn rust_trait_counts_as_abstract() {
    let src = "trait Foo {}\nstruct Bar;\n";
    let (_dir, path) = write_tempfile(src, "rs");
    let r = abstractness_for_file(&path, Language::Rust);
    assert!((r.abstractness - 0.5).abs() < 0.001);
    assert_eq!(r.confidence, ImportConfidence::High);
}

#[test]
fn rust_only_structs_yields_zero() {
    let src = "struct A;\nstruct B;\nenum C {}\n";
    let (_dir, path) = write_tempfile(src, "rs");
    let r = abstractness_for_file(&path, Language::Rust);
    assert!(r.abstractness.abs() < 0.001);
}

#[test]
fn rust_two_traits_no_structs_yields_one() {
    let src = "trait A {}\ntrait B {}\n";
    let (_dir, path) = write_tempfile(src, "rs");
    let r = abstractness_for_file(&path, Language::Rust);
    assert!((r.abstractness - 1.0).abs() < 0.001);
}

#[test]
fn typescript_interface_counts_as_abstract() {
    let src = "interface Foo {}\nclass Bar implements Foo {}\n";
    let (_dir, path) = write_tempfile(src, "ts");
    let r = abstractness_for_file(&path, Language::TypeScript);
    assert!((r.abstractness - 0.5).abs() < 0.001);
    assert_eq!(r.confidence, ImportConfidence::High);
}

#[test]
fn typescript_class_only_yields_zero() {
    let src = "class A { foo() {} }\nclass B { bar() {} }\n";
    let (_dir, path) = write_tempfile(src, "ts");
    let r = abstractness_for_file(&path, Language::TypeScript);
    assert!(r.abstractness.abs() < 0.001);
}

#[test]
fn javascript_class_only_yields_zero_besteffort() {
    let src = "class Foo { method() {} }\n";
    let (_dir, path) = write_tempfile(src, "js");
    let r = abstractness_for_file(&path, Language::JavaScript);
    assert!(r.abstractness.abs() < 0.001);
    assert_eq!(r.confidence, ImportConfidence::BestEffort);
}

#[test]
fn javascript_no_types_yields_naabstraction() {
    let src = "function f() { return 1; }\n";
    let (_dir, path) = write_tempfile(src, "js");
    let r = abstractness_for_file(&path, Language::JavaScript);
    assert_eq!(r.confidence, ImportConfidence::NaAbstraction);
}

#[test]
fn python_class_only_yields_zero_medium() {
    let src = "class Foo:\n    pass\n";
    let (_dir, path) = write_tempfile(src, "py");
    let r = abstractness_for_file(&path, Language::Python);
    assert!(r.abstractness.abs() < 0.001);
    assert_eq!(r.confidence, ImportConfidence::Medium);
}

#[test]
fn python_no_classes_yields_naabstraction() {
    let src = "def f(): pass\nx = 1\n";
    let (_dir, path) = write_tempfile(src, "py");
    let r = abstractness_for_file(&path, Language::Python);
    assert_eq!(r.confidence, ImportConfidence::NaAbstraction);
}

#[test]
fn go_interface_counts_as_abstract() {
    let src = "package main\ntype Foo interface { Bar() }\ntype Concrete struct {}\n";
    let (_dir, path) = write_tempfile(src, "go");
    let r = abstractness_for_file(&path, Language::Go);
    assert!(r.abstractness > 0.0);
    assert_eq!(r.confidence, ImportConfidence::Medium);
}

#[test]
fn csharp_interface_counts_as_abstract() {
    let src = "interface IFoo {}\nclass Bar : IFoo {}\n";
    let (_dir, path) = write_tempfile(src, "cs");
    let r = abstractness_for_file(&path, Language::CSharp);
    assert!((r.abstractness - 0.5).abs() < 0.001);
    assert_eq!(r.confidence, ImportConfidence::High);
}

#[test]
fn kotlin_interface_does_not_panic() {
    let src = "interface Foo {}\nclass Bar : Foo {}\n";
    let (_dir, path) = write_tempfile(src, "kt");
    let _r = abstractness_for_file(&path, Language::Kotlin);
}

#[test]
fn swift_protocol_counts_as_abstract() {
    let src = "protocol Foo {}\nclass Bar: Foo {}\n";
    let (_dir, path) = write_tempfile(src, "swift");
    let r = abstractness_for_file(&path, Language::Swift);
    assert!((r.abstractness - 0.5).abs() < 0.001);
}

#[test]
fn haskell_does_not_panic() {
    let src = "class Foo a where\n  foo :: a -> a\n\ndata Bar = Bar\n";
    let (_dir, path) = write_tempfile(src, "hs");
    let _r = abstractness_for_file(&path, Language::Haskell);
}

#[test]
fn php_interface_counts_as_abstract() {
    let src = "<?php\ninterface Foo {}\nclass Bar implements Foo {}\n";
    let (_dir, path) = write_tempfile(src, "php");
    let r = abstractness_for_file(&path, Language::Php);
    assert!((r.abstractness - 0.5).abs() < 0.001);
    assert_eq!(r.confidence, ImportConfidence::High);
}

#[test]
fn d_interface_counts_as_abstract() {
    let src = "interface Foo {}\nclass Bar : Foo {}\n";
    let (_dir, path) = write_tempfile(src, "d");
    let r = abstractness_for_file(&path, Language::D);
    assert!(r.abstractness > 0.0);
    assert_eq!(r.confidence, ImportConfidence::High);
}

#[test]
fn missing_file_returns_naabstraction() {
    let p = PathBuf::from("/nonexistent/path.java");
    let r = abstractness_for_file(&p, Language::Java);
    assert_eq!(r.confidence, ImportConfidence::NaAbstraction);
    assert!(r.abstractness.abs() < 0.001);
}

#[test]
fn empty_file_returns_naabstraction() {
    let (_dir, path) = write_tempfile("", "java");
    let r = abstractness_for_file(&path, Language::Java);
    assert_eq!(r.confidence, ImportConfidence::NaAbstraction);
}

#[test]
fn java_three_classes_one_interface_yields_quarter() {
    let src = "interface A {}\nclass B {}\nclass C {}\nclass D {}\n";
    let (_dir, path) = write_tempfile(src, "java");
    let r = abstractness_for_file(&path, Language::Java);
    assert!((r.abstractness - 0.25).abs() < 0.001);
}

#[test]
fn rust_two_traits_two_structs_yields_half() {
    let src = "trait A {}\ntrait B {}\nstruct C;\nstruct D;\n";
    let (_dir, path) = write_tempfile(src, "rs");
    let r = abstractness_for_file(&path, Language::Rust);
    assert!((r.abstractness - 0.5).abs() < 0.001);
}

#[test]
fn unsupported_language_falls_through_to_naabstraction() {
    let (_dir, path) = write_tempfile("if {1} else {2}\n", "lua");
    let r = abstractness_for_file(&path, Language::Lua);
    assert_eq!(r.confidence, ImportConfidence::NaAbstraction);
}

#[test]
fn determinism_two_runs_same_file() {
    let src = "trait A {}\nstruct B;\n";
    let (_dir, path) = write_tempfile(src, "rs");
    let r1 = abstractness_for_file(&path, Language::Rust);
    let r2 = abstractness_for_file(&path, Language::Rust);
    assert!((r1.abstractness - r2.abstractness).abs() < 0.001);
}

#[test]
fn nested_class_in_class_still_counts() {
    let src = "class Outer {\n    class Inner {}\n}\n";
    let (_dir, path) = write_tempfile(src, "java");
    let r = abstractness_for_file(&path, Language::Java);
    assert!(r.abstractness >= 0.0);
}

#[test]
fn rust_union_counts_as_concrete() {
    let src = "union U { a: i32, b: f32 }\n";
    let (_dir, path) = write_tempfile(src, "rs");
    let r = abstractness_for_file(&path, Language::Rust);
    assert!(r.abstractness.abs() < 0.001);
}

#[test]
fn high_confidence_languages_with_known_kinds_label_correctly() {
    let cases: &[(Language, &str, &str)] = &[
        (Language::Java, "interface F {}\n", "java"),
        (Language::CSharp, "interface F {}\n", "cs"),
        (Language::Swift, "protocol F {}\n", "swift"),
        (Language::Rust, "trait F {}\n", "rs"),
        (Language::TypeScript, "interface F {}\n", "ts"),
        (Language::Php, "<?php interface F {}\n", "php"),
        (Language::D, "interface F {}\n", "d"),
    ];
    for (lang, src, ext) in cases {
        let (_dir, path) = write_tempfile(src, ext);
        let r = abstractness_for_file(&path, *lang);
        assert_eq!(r.confidence, ImportConfidence::High, "lang {lang:?}");
    }
}

#[test]
fn besteffort_languages_correctly_labeled() {
    let cases: &[(Language, &str, &str)] = &[
        (Language::JavaScript, "class F {}\n", "js"),
        (Language::Cpp, "class F { int x; };\n", "cpp"),
        (Language::C, "struct F { int x; };\n", "c"),
        (Language::Zig, "const F = struct {};\n", "zig"),
    ];
    for (lang, src, ext) in cases {
        let (_dir, path) = write_tempfile(src, ext);
        let r = abstractness_for_file(&path, *lang);
        assert_eq!(r.confidence, ImportConfidence::BestEffort, "lang {lang:?}");
    }
}
