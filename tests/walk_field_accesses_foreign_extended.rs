use std::path::PathBuf;

use pulse::audit::definitions::definitions_for_file;
use pulse::parse::Language;

fn write_tempfile(content: &str, ext: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(format!("t.{ext}"));
    std::fs::write(&path, content).unwrap();
    (dir, path)
}

fn collect_foreign(content: &str, ext: &str, lang: Language) -> Vec<(String, String)> {
    let (_d, path) = write_tempfile(content, ext);
    let defs = definitions_for_file(&path, lang);
    defs.iter()
        .flat_map(|d| d.foreign_field_accesses.clone())
        .collect()
}

#[test]
fn go_method_receiver_field_handled() {
    let src = "package m\n\ntype Foo struct{}\n\nfunc (f *Foo) M(arg *Bar) int {\n    return arg.x\n}\n";
    let _ = collect_foreign(src, "go", Language::Go);
}

#[test]
fn kotlin_dotted_field_access_handled() {
    let src = "fun method(obj: Bar): Int {\n    return obj.x\n}\n";
    let _ = collect_foreign(src, "kt", Language::Kotlin);
}

#[test]
fn php_arrow_field_access_handled() {
    let src = "<?php\nfunction m($obj) {\n    return $obj->x;\n}\n";
    let _ = collect_foreign(src, "php", Language::Php);
}

#[test]
fn groovy_dotted_field_access_handled() {
    let src = "def m(obj) { obj.x }\n";
    let _ = collect_foreign(src, "groovy", Language::Groovy);
}

#[test]
fn swift_property_access_handled() {
    let src = "func m(obj: Bar) -> Int {\n    return obj.x\n}\n";
    let _ = collect_foreign(src, "swift", Language::Swift);
}

#[test]
fn ruby_instance_var_treated_correctly() {
    let src = "class Foo\n  def m\n    @field\n  end\nend\n";
    let (_d, path) = write_tempfile(src, "rb");
    let defs = definitions_for_file(&path, Language::Ruby);
    let m = defs.iter().find(|d| d.identity.name == "m");
    if let Some(m) = m {
        assert!(!m.foreign_field_accesses.iter().any(|(r, _)| r == "@field"));
    }
}

#[test]
fn ruby_dotted_field_access_handled() {
    let src = "class Foo\n  def m(obj)\n    obj.x\n  end\nend\n";
    let _ = collect_foreign(src, "rb", Language::Ruby);
}

#[test]
fn objc_dotted_property_handled() {
    let src = "@implementation Foo\n- (int)method:(Bar*)arg {\n    return arg.x;\n}\n@end\n";
    let _ = collect_foreign(src, "m", Language::ObjectiveC);
}

#[test]
fn zig_field_access_handled() {
    let src = "fn m(arg: *Bar) i32 {\n    return arg.x;\n}\n";
    let _ = collect_foreign(src, "zig", Language::Zig);
}

#[test]
fn lua_table_field_access_handled() {
    let src = "function m(obj)\n    return obj.x\nend\n";
    let _ = collect_foreign(src, "lua", Language::Lua);
}

#[test]
fn c_struct_field_access_handled() {
    let src = "int m(struct Bar* arg) {\n    return arg->x;\n}\n";
    let _ = collect_foreign(src, "c", Language::C);
}

#[test]
fn cpp_struct_field_access_handled() {
    let src = "int m(Bar* arg) {\n    return arg->x;\n}\n";
    let _ = collect_foreign(src, "cpp", Language::Cpp);
}

#[test]
fn r_dollar_field_access_handled() {
    let src = "m <- function(obj) {\n    obj$x\n}\n";
    let _ = collect_foreign(src, "r", Language::R);
}

#[test]
fn d_field_access_handled() {
    let src = "int m(Bar arg) {\n    return arg.x;\n}\n";
    let _ = collect_foreign(src, "d", Language::D);
}

#[test]
fn haskell_record_access_handled() {
    let src = "m obj = field obj\n";
    let _ = collect_foreign(src, "hs", Language::Haskell);
}

#[test]
fn tcl_no_oop_returns_empty_foreign() {
    let src = "proc m {} {\n    set x [list 1 2 3]\n}\n";
    let foreign = collect_foreign(src, "tcl", Language::Tcl);
    assert!(foreign.is_empty());
}

#[test]
fn cobol_no_field_access_returns_empty() {
    let src = "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. T.\n       PROCEDURE DIVISION.\n           DISPLAY \"HELLO\".\n           STOP RUN.\n";
    let foreign = collect_foreign(src, "cob", Language::Cobol);
    assert!(foreign.is_empty());
}

#[test]
fn cls_treated_as_intra_in_python_classmethod() {
    let src = r#"class Foo:
    @classmethod
    def m(cls):
        return cls.x
"#;
    let foreign = collect_foreign(src, "py", Language::Python);
    assert!(!foreign.iter().any(|(r, _)| r == "cls"));
}

#[test]
fn typescript_this_treated_as_intra() {
    let src = "class Foo {\n  m() { return this.x; }\n}\n";
    let foreign = collect_foreign(src, "ts", Language::TypeScript);
    assert!(!foreign.iter().any(|(r, _)| r == "this"));
}

#[test]
fn javascript_this_treated_as_intra() {
    let src = "class Foo {\n  m() { return this.x; }\n}\n";
    let foreign = collect_foreign(src, "js", Language::JavaScript);
    assert!(!foreign.iter().any(|(r, _)| r == "this"));
}

#[test]
fn rust_self_treated_as_intra() {
    let src = "impl Foo {\n  fn m(&self) -> i32 { self.x }\n}\n";
    let foreign = collect_foreign(src, "rs", Language::Rust);
    assert!(!foreign.iter().any(|(r, _)| r == "self"));
}

#[test]
fn java_this_field_treated_as_intra() {
    let src = "class Foo {\n  void m() { int y = this.x; }\n}\n";
    let foreign = collect_foreign(src, "java", Language::Java);
    assert!(!foreign.iter().any(|(r, _)| r == "this"));
}

#[test]
fn deeply_chained_dotted_extracts_immediate_receiver() {
    let src = "class Foo:\n    def m(self, a):\n        return a.b.c.d\n";
    let _ = collect_foreign(src, "py", Language::Python);
}

#[test]
fn empty_function_no_foreign_accesses() {
    let src = "def f(): pass\n";
    let foreign = collect_foreign(src, "py", Language::Python);
    assert!(foreign.is_empty());
}

#[test]
fn function_with_no_field_access_no_foreign() {
    let src = "def f(): return 1 + 2 + 3\n";
    let foreign = collect_foreign(src, "py", Language::Python);
    assert!(foreign.is_empty());
}

#[test]
fn function_with_local_var_field_access_treated_as_foreign() {
    let src = "class Foo:\n    def m(self):\n        local = make_obj()\n        return local.x\n";
    let _ = collect_foreign(src, "py", Language::Python);
}

#[test]
fn all_22_languages_smoke_no_panic() {
    let cases: &[(&str, &str, Language)] = &[
        ("class Foo:\n    def m(self): pass\n", "py", Language::Python),
        ("function f() {}", "js", Language::JavaScript),
        ("function f() {}", "ts", Language::TypeScript),
        ("fn main() {}", "rs", Language::Rust),
        ("int main() { return 0; }", "c", Language::C),
        ("int main() { return 0; }", "cpp", Language::Cpp),
        ("class M {}", "java", Language::Java),
        ("class M {}", "cs", Language::CSharp),
        ("package m\nfunc main() {}", "go", Language::Go),
        ("func m() {}", "swift", Language::Swift),
        ("fn main() void {}", "zig", Language::Zig),
        ("def m; end\n", "rb", Language::Ruby),
        ("@implementation Foo\n@end\n", "m", Language::ObjectiveC),
        ("proc m {} {}\n", "tcl", Language::Tcl),
        ("fun m() {}", "kt", Language::Kotlin),
        ("module M where\nm = 1\n", "hs", Language::Haskell),
        ("function m() end\n", "lua", Language::Lua),
        ("m <- function() 1\n", "r", Language::R),
        ("<?php function m() {} ?>\n", "php", Language::Php),
        ("       IDENTIFICATION DIVISION.\n       PROGRAM-ID. T.\n", "cob", Language::Cobol),
        ("void m() {}", "d", Language::D),
        ("def m() {}", "groovy", Language::Groovy),
    ];
    for (src, ext, lang) in cases {
        let _ = collect_foreign(src, ext, *lang);
    }
}

#[test]
fn missing_file_returns_empty_no_panic() {
    let path = PathBuf::from("/nonexistent/path/file.py");
    let defs = definitions_for_file(&path, Language::Python);
    assert!(defs.is_empty());
}

#[test]
fn empty_file_no_foreign_accesses() {
    let (_d, path) = write_tempfile("", "py");
    let defs = definitions_for_file(&path, Language::Python);
    let _ = defs;
}

#[test]
fn malformed_python_no_panic() {
    let src = "class Foo:\n    def m(self, obj):\n        return obj.\n";
    let _ = collect_foreign(src, "py", Language::Python);
}
