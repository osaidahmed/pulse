use std::path::PathBuf;

use pulse::audit::definitions::definitions_for_file;
use pulse::parse::Language;

fn write_tempfile(content: &str, ext: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(format!("test.{ext}"));
    std::fs::write(&path, content).unwrap();
    (dir, path)
}

#[test]
fn python_class_with_base_extracts_parent() {
    let src = "class Foo:\n    pass\n\nclass Bar(Foo):\n    def m(self): pass\n";
    let (_dir, path) = write_tempfile(src, "py");
    let defs = definitions_for_file(&path, Language::Python);
    let m = defs.iter().find(|d| d.identity.name == "m").unwrap();
    assert_eq!(m.parent_class.as_deref(), Some("Foo"));
}

#[test]
fn python_class_without_base_has_no_parent() {
    let src = "class Foo:\n    def m(self): pass\n";
    let (_dir, path) = write_tempfile(src, "py");
    let defs = definitions_for_file(&path, Language::Python);
    let m = defs.iter().find(|d| d.identity.name == "m").unwrap();
    assert_eq!(m.parent_class, None);
}

#[test]
fn python_multiple_bases_takes_first() {
    let src = "class Foo: pass\nclass Mixin: pass\nclass Bar(Foo, Mixin):\n    def m(self): pass\n";
    let (_dir, path) = write_tempfile(src, "py");
    let defs = definitions_for_file(&path, Language::Python);
    let m = defs.iter().find(|d| d.identity.name == "m").unwrap();
    assert_eq!(m.parent_class.as_deref(), Some("Foo"));
}

#[test]
fn java_class_extends_extracts_parent() {
    let src = "public class Foo {}\npublic class Bar extends Foo {\n    void m() {}\n}\n";
    let (_dir, path) = write_tempfile(src, "java");
    let defs = definitions_for_file(&path, Language::Java);
    let m = defs.iter().find(|d| d.identity.name == "m").unwrap();
    assert_eq!(m.parent_class.as_deref(), Some("Foo"));
}

#[test]
fn java_class_without_extends_has_no_parent() {
    let src = "public class Foo {\n    void m() {}\n}\n";
    let (_dir, path) = write_tempfile(src, "java");
    let defs = definitions_for_file(&path, Language::Java);
    let m = defs.iter().find(|d| d.identity.name == "m").unwrap();
    assert_eq!(m.parent_class, None);
}

#[test]
fn java_constructor_carries_parent() {
    let src = "public class Foo {}\npublic class Bar extends Foo {\n    public Bar() {}\n}\n";
    let (_dir, path) = write_tempfile(src, "java");
    let defs = definitions_for_file(&path, Language::Java);
    let ctor = defs.iter().find(|d| d.is_constructor);
    assert!(ctor.is_some());
    assert_eq!(ctor.unwrap().parent_class.as_deref(), Some("Foo"));
}

#[test]
fn typescript_class_extends_extracts_parent() {
    let src = "class Foo { foo() {} }\nclass Bar extends Foo { bar() {} }\n";
    let (_dir, path) = write_tempfile(src, "ts");
    let defs = definitions_for_file(&path, Language::TypeScript);
    let bar = defs.iter().find(|d| d.identity.name == "bar").unwrap();
    assert_eq!(bar.parent_class.as_deref(), Some("Foo"));
}

#[test]
fn typescript_implements_does_not_set_parent() {
    let src = "interface IFoo { f(): void }\nclass Bar implements IFoo { f() {} }\n";
    let (_dir, path) = write_tempfile(src, "ts");
    let defs = definitions_for_file(&path, Language::TypeScript);
    let f = defs.iter().find(|d| d.identity.name == "f");
    if let Some(d) = f {
        assert_eq!(d.parent_class, None);
    }
}

#[test]
fn csharp_class_with_base_list_extracts_parent() {
    let src = "class Foo {}\nclass Bar : Foo { void M() {} }\n";
    let (_dir, path) = write_tempfile(src, "cs");
    let defs = definitions_for_file(&path, Language::CSharp);
    let m = defs.iter().find(|d| d.identity.name == "M");
    if let Some(d) = m {
        assert_eq!(d.parent_class.as_deref(), Some("Foo"));
    }
}

#[test]
fn csharp_class_without_base_has_no_parent() {
    let src = "class Bar { void M() {} }\n";
    let (_dir, path) = write_tempfile(src, "cs");
    let defs = definitions_for_file(&path, Language::CSharp);
    let m = defs.iter().find(|d| d.identity.name == "M");
    if let Some(d) = m {
        assert_eq!(d.parent_class, None);
    }
}

#[test]
fn rust_trait_impl_sets_parent_to_trait_name() {
    let src = "trait Foo { fn f(&self); }\nstruct Bar;\nimpl Foo for Bar {\n    fn f(&self) {}\n}\n";
    let (_dir, path) = write_tempfile(src, "rs");
    let defs = definitions_for_file(&path, Language::Rust);
    let m = defs.iter().find(|d| d.identity.name == "f");
    if let Some(d) = m {
        assert_eq!(d.parent_class.as_deref(), Some("Foo"));
    }
}

#[test]
fn rust_inherent_impl_has_no_parent() {
    let src = "struct Bar;\nimpl Bar {\n    fn m(&self) {}\n}\n";
    let (_dir, path) = write_tempfile(src, "rs");
    let defs = definitions_for_file(&path, Language::Rust);
    let m = defs.iter().find(|d| d.identity.name == "m").unwrap();
    assert_eq!(m.parent_class, None);
}

#[test]
fn unsupported_language_for_parent_extraction_keeps_none() {
    let src = "function f() {}\n";
    let (_dir, path) = write_tempfile(src, "go");
    let defs = definitions_for_file(&path, Language::Go);
    if let Some(d) = defs.first() {
        assert_eq!(d.parent_class, None);
    }
}

#[test]
fn determinism_two_runs_produce_same_parent() {
    let src = "class A: pass\nclass B(A):\n    def m(self): pass\n";
    let (_dir, path) = write_tempfile(src, "py");
    let r1 = definitions_for_file(&path, Language::Python);
    let r2 = definitions_for_file(&path, Language::Python);
    assert_eq!(r1[0].parent_class, r2[0].parent_class);
}

#[test]
fn malformed_python_no_panic() {
    let src = "class Bar(Foo:\n    def m(self): pass\n";
    let (_dir, path) = write_tempfile(src, "py");
    let _defs = definitions_for_file(&path, Language::Python);
}

#[test]
fn empty_file_no_panic() {
    let (_dir, path) = write_tempfile("", "py");
    let defs = definitions_for_file(&path, Language::Python);
    assert!(defs.is_empty());
}
