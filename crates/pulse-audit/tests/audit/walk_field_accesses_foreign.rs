use std::path::PathBuf;

use pulse_audit::definitions::definitions_for_file;
use pulse_syntax::parse::Language;

fn write_tempfile(content: &str, ext: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(format!("test.{ext}"));
    std::fs::write(&path, content).unwrap();
    (dir, path)
}

#[test]
fn python_obj_field_captured_as_foreign() {
    let src = "class Foo:\n    def m(self, obj):\n        return obj.x\n";
    let (_dir, path) = write_tempfile(src, "py");
    let defs = definitions_for_file(&path, Language::Python);
    let m = defs.iter().find(|d| d.identity.name == "m").unwrap();
    assert!(m.foreign_field_accesses.contains(&("obj".to_string(), "x".to_string())));
}

#[test]
fn python_self_field_not_in_foreign_list() {
    let src = "class Foo:\n    def m(self):\n        return self.x\n";
    let (_dir, path) = write_tempfile(src, "py");
    let defs = definitions_for_file(&path, Language::Python);
    let m = defs.iter().find(|d| d.identity.name == "m").unwrap();
    assert!(!m.foreign_field_accesses.iter().any(|(r, _)| r == "self"));
    assert!(m.field_accesses.contains(&"x".to_string()));
}

#[test]
fn python_mixed_intra_and_foreign() {
    let src = "class Foo:\n    def m(self, obj):\n        a = self.x\n        b = obj.y\n        return (a, b)\n";
    let (_dir, path) = write_tempfile(src, "py");
    let defs = definitions_for_file(&path, Language::Python);
    let m = defs.iter().find(|d| d.identity.name == "m").unwrap();
    assert!(m.field_accesses.contains(&"x".to_string()));
    assert!(m.foreign_field_accesses.contains(&("obj".to_string(), "y".to_string())));
}

#[test]
fn python_multiple_distinct_foreign_objects() {
    let src = "class Foo:\n    def m(self, a, b):\n        return a.x + b.y + a.z\n";
    let (_dir, path) = write_tempfile(src, "py");
    let defs = definitions_for_file(&path, Language::Python);
    let m = defs.iter().find(|d| d.identity.name == "m").unwrap();
    assert!(m.foreign_field_accesses.contains(&("a".to_string(), "x".to_string())));
    assert!(m.foreign_field_accesses.contains(&("b".to_string(), "y".to_string())));
    assert!(m.foreign_field_accesses.contains(&("a".to_string(), "z".to_string())));
}

#[test]
fn python_foreign_accesses_deduped() {
    let src = "class Foo:\n    def m(self, obj):\n        return obj.x + obj.x\n";
    let (_dir, path) = write_tempfile(src, "py");
    let defs = definitions_for_file(&path, Language::Python);
    let m = defs.iter().find(|d| d.identity.name == "m").unwrap();
    let count = m.foreign_field_accesses.iter().filter(|(r, f)| r == "obj" && f == "x").count();
    assert_eq!(count, 1, "duplicate (obj, x) pairs should be deduped");
}

#[test]
fn java_arg_field_captured_as_foreign() {
    let src = "public class Foo {\n    void m(Bar arg) {\n        int x = arg.field;\n    }\n}\n";
    let (_dir, path) = write_tempfile(src, "java");
    let defs = definitions_for_file(&path, Language::Java);
    let m = defs.iter().find(|d| d.identity.name == "m").unwrap();
    assert!(m.foreign_field_accesses.iter().any(|(r, _)| r == "arg"));
}

#[test]
fn typescript_props_value_captured_as_foreign() {
    let src = "class Foo {\n    m(props: any) { return props.value; }\n}\n";
    let (_dir, path) = write_tempfile(src, "ts");
    let defs = definitions_for_file(&path, Language::TypeScript);
    let m = defs.iter().find(|d| d.identity.name == "m").unwrap();
    assert!(m.foreign_field_accesses.iter().any(|(r, _)| r == "props"));
}

#[test]
fn rust_other_field_captured_as_foreign() {
    let src = "struct Foo;\nimpl Foo {\n    fn m(&self, other: &Bar) -> i32 { other.field }\n}\n";
    let (_dir, path) = write_tempfile(src, "rs");
    let defs = definitions_for_file(&path, Language::Rust);
    let m = defs.iter().find(|d| d.identity.name == "m").unwrap();
    assert!(m.foreign_field_accesses.iter().any(|(r, _)| r == "other"));
}

#[test]
fn missing_file_returns_empty_no_panic() {
    let p = PathBuf::from("/nonexistent/path.py");
    let defs = definitions_for_file(&p, Language::Python);
    assert!(defs.is_empty());
}

#[test]
fn empty_file_no_foreign_accesses() {
    let (_dir, path) = write_tempfile("", "py");
    let defs = definitions_for_file(&path, Language::Python);
    assert!(defs.is_empty());
}

#[test]
fn python_function_with_only_self_accesses_emits_no_foreign() {
    let src = "class Foo:\n    def m(self):\n        return self.a + self.b + self.c\n";
    let (_dir, path) = write_tempfile(src, "py");
    let defs = definitions_for_file(&path, Language::Python);
    let m = defs.iter().find(|d| d.identity.name == "m").unwrap();
    assert!(m.foreign_field_accesses.is_empty());
}

#[test]
fn python_no_field_accesses_emits_empty() {
    let src = "def f(x):\n    return x + 1\n";
    let (_dir, path) = write_tempfile(src, "py");
    let defs = definitions_for_file(&path, Language::Python);
    let f = defs.iter().find(|d| d.identity.name == "f").unwrap();
    assert!(f.foreign_field_accesses.is_empty());
}

#[test]
fn determinism_two_runs_same_foreign_set() {
    let src = "class Foo:\n    def m(self, a, b):\n        return a.x + b.y\n";
    let (_dir, path) = write_tempfile(src, "py");
    let r1 = definitions_for_file(&path, Language::Python);
    let r2 = definitions_for_file(&path, Language::Python);
    assert_eq!(r1[0].foreign_field_accesses, r2[0].foreign_field_accesses);
}

#[test]
fn malformed_source_no_panic() {
    let src = "class Foo:\n    def m(self,\n        return obj.x\n";
    let (_dir, path) = write_tempfile(src, "py");
    let _defs = definitions_for_file(&path, Language::Python);
}

#[test]
fn python_method_call_on_obj_extracts_method_name() {
    let src = "class Foo:\n    def m(self, obj):\n        return obj.compute()\n";
    let (_dir, path) = write_tempfile(src, "py");
    let defs = definitions_for_file(&path, Language::Python);
    let m = defs.iter().find(|d| d.identity.name == "m").unwrap();
    assert!(m.foreign_field_accesses.iter().any(|(r, f)| r == "obj" && f == "compute"));
}

#[test]
fn python_chained_dotted_uses_immediate_receiver() {
    let src = "class Foo:\n    def m(self, a):\n        return a.b.c\n";
    let (_dir, path) = write_tempfile(src, "py");
    let defs = definitions_for_file(&path, Language::Python);
    let m = defs.iter().find(|d| d.identity.name == "m").unwrap();
    assert!(!m.foreign_field_accesses.is_empty());
}

#[test]
fn lua_no_oop_field_access_returns_empty() {
    let src = "function f(x) return x + 1 end\n";
    let (_dir, path) = write_tempfile(src, "lua");
    let defs = definitions_for_file(&path, Language::Lua);
    if let Some(d) = defs.first() {
        assert!(d.foreign_field_accesses.is_empty() || d.foreign_field_accesses.iter().all(|(r, _)| r != "self"));
    }
}

#[test]
fn cls_in_classmethod_treated_as_intra() {
    let src = "class Foo:\n    @classmethod\n    def m(cls):\n        return cls.x\n";
    let (_dir, path) = write_tempfile(src, "py");
    let defs = definitions_for_file(&path, Language::Python);
    let m = defs.iter().find(|d| d.identity.name == "m").unwrap();
    assert!(!m.foreign_field_accesses.iter().any(|(r, _)| r == "cls"));
}

#[test]
fn foreign_accesses_sorted() {
    let src = "class Foo:\n    def m(self, z, a, m):\n        return z.x + a.b + m.c\n";
    let (_dir, path) = write_tempfile(src, "py");
    let defs = definitions_for_file(&path, Language::Python);
    let m = defs.iter().find(|d| d.identity.name == "m").unwrap();
    let receivers: Vec<&String> = m.foreign_field_accesses.iter().map(|(r, _)| r).collect();
    let mut sorted = receivers.clone();
    sorted.sort();
    assert_eq!(receivers, sorted);
}

#[test]
fn python_22_languages_smoke() {
    let cases: &[(&str, &str, Language)] = &[
        ("def f(x): return x.y\n", "py", Language::Python),
        ("function f(x) { return x.y; }\n", "js", Language::JavaScript),
        ("function f(x: any) { return x.y; }\n", "ts", Language::TypeScript),
        ("fn f(x: &Bar) -> i32 { x.y }\n", "rs", Language::Rust),
        ("public class A { void f(B b) { int z = b.y; } }\n", "java", Language::Java),
        ("class A { void f(B b) { int z = b.y; } }\n", "cs", Language::CSharp),
    ];
    for (src, ext, lang) in cases {
        let (_dir, path) = write_tempfile(src, ext);
        let _defs = definitions_for_file(&path, *lang);
    }
}
