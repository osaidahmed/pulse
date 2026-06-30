use std::path::{Path, PathBuf};

use pulse_history::edges::{build_graph, directly_linked};
use pulse_syntax::parse::Language;
use tempfile::TempDir;

fn write_file(root: &Path, rel: &str, content: &str) -> PathBuf {
    let full = root.join(rel);
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&full, content).unwrap();
    full
}

struct Fixture {
    lang: Language,
    files: Vec<(PathBuf, Language)>,
    a: PathBuf,
    b: PathBuf,
    extra: TempDir,
}

fn linked_python() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "a.py", "from b import bar\n");
    let b = write_file(dir.path(), "b.py", "def bar(): pass\n");
    Fixture {
        lang: Language::Python,
        files: vec![(a.clone(), Language::Python), (b.clone(), Language::Python)],
        a,
        b,
        extra: dir,
    }
}

fn unlinked_python() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "a.py", "x = 1\n");
    let b = write_file(dir.path(), "b.py", "y = 2\n");
    Fixture {
        lang: Language::Python,
        files: vec![(a.clone(), Language::Python), (b.clone(), Language::Python)],
        a,
        b,
        extra: dir,
    }
}

fn linked_typescript() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "a.ts", "import { bar } from './b';\nbar();\n");
    let b = write_file(dir.path(), "b.ts", "export function bar() {}\n");
    Fixture {
        lang: Language::TypeScript,
        files: vec![(a.clone(), Language::TypeScript), (b.clone(), Language::TypeScript)],
        a,
        b,
        extra: dir,
    }
}

fn unlinked_typescript() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "a.ts", "const x = 1;\n");
    let b = write_file(dir.path(), "b.ts", "const y = 2;\n");
    Fixture {
        lang: Language::TypeScript,
        files: vec![(a.clone(), Language::TypeScript), (b.clone(), Language::TypeScript)],
        a,
        b,
        extra: dir,
    }
}

fn linked_javascript() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "a.js", "import { bar } from './b.js';\nbar();\n");
    let b = write_file(dir.path(), "b.js", "export function bar() {}\n");
    Fixture {
        lang: Language::JavaScript,
        files: vec![(a.clone(), Language::JavaScript), (b.clone(), Language::JavaScript)],
        a,
        b,
        extra: dir,
    }
}

fn unlinked_javascript() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "a.js", "const x = 1;\n");
    let b = write_file(dir.path(), "b.js", "const y = 2;\n");
    Fixture {
        lang: Language::JavaScript,
        files: vec![(a.clone(), Language::JavaScript), (b.clone(), Language::JavaScript)],
        a,
        b,
        extra: dir,
    }
}

fn linked_rust() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "src/foo.rs", "use crate::bar;\npub fn f() {}\n");
    let b = write_file(dir.path(), "src/bar.rs", "pub fn baz() {}\n");
    Fixture {
        lang: Language::Rust,
        files: vec![(a.clone(), Language::Rust), (b.clone(), Language::Rust)],
        a,
        b,
        extra: dir,
    }
}

fn unlinked_rust() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "src/foo.rs", "pub fn f() {}\n");
    let b = write_file(dir.path(), "src/bar.rs", "pub fn baz() {}\n");
    Fixture {
        lang: Language::Rust,
        files: vec![(a.clone(), Language::Rust), (b.clone(), Language::Rust)],
        a,
        b,
        extra: dir,
    }
}

fn linked_c() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "a.c", "#include \"b.h\"\nint main() { return 0; }\n");
    let b = write_file(dir.path(), "b.h", "void greet(void);\n");
    Fixture { lang: Language::C, files: vec![(a.clone(), Language::C), (b.clone(), Language::C)], a, b, extra: dir }
}

fn unlinked_c() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "a.c", "int x = 1;\n");
    let b = write_file(dir.path(), "b.c", "int y = 2;\n");
    Fixture { lang: Language::C, files: vec![(a.clone(), Language::C), (b.clone(), Language::C)], a, b, extra: dir }
}

fn linked_cpp() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "a.cpp", "#include \"b.hpp\"\nint main() { return 0; }\n");
    let b = write_file(dir.path(), "b.hpp", "void greet();\n");
    Fixture {
        lang: Language::Cpp,
        files: vec![(a.clone(), Language::Cpp), (b.clone(), Language::Cpp)],
        a,
        b,
        extra: dir,
    }
}

fn unlinked_cpp() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "a.cpp", "int x = 1;\n");
    let b = write_file(dir.path(), "b.cpp", "int y = 2;\n");
    Fixture {
        lang: Language::Cpp,
        files: vec![(a.clone(), Language::Cpp), (b.clone(), Language::Cpp)],
        a,
        b,
        extra: dir,
    }
}

fn linked_java() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "src/main/java/A.java", "import com.foo.B;\nclass A {}\n");
    let b = write_file(dir.path(), "src/main/java/com/foo/B.java", "package com.foo;\npublic class B {}\n");
    Fixture {
        lang: Language::Java,
        files: vec![(a.clone(), Language::Java), (b.clone(), Language::Java)],
        a,
        b,
        extra: dir,
    }
}

fn unlinked_java() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "src/main/java/A.java", "class A {}\n");
    let b = write_file(dir.path(), "src/main/java/B.java", "class B {}\n");
    Fixture {
        lang: Language::Java,
        files: vec![(a.clone(), Language::Java), (b.clone(), Language::Java)],
        a,
        b,
        extra: dir,
    }
}

fn linked_csharp() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "A.cs", "using B;\nclass A {}\n");
    let b = write_file(dir.path(), "B.cs", "namespace B { public class X {} }\n");
    Fixture {
        lang: Language::CSharp,
        files: vec![(a.clone(), Language::CSharp), (b.clone(), Language::CSharp)],
        a,
        b,
        extra: dir,
    }
}

fn unlinked_csharp() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "A.cs", "class A {}\n");
    let b = write_file(dir.path(), "B.cs", "class B {}\n");
    Fixture {
        lang: Language::CSharp,
        files: vec![(a.clone(), Language::CSharp), (b.clone(), Language::CSharp)],
        a,
        b,
        extra: dir,
    }
}

fn linked_go() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "a.go", "package main\nimport \"./pkgb\"\nfunc main() {}\n");
    let b = write_file(dir.path(), "pkgb/b.go", "package pkgb\nfunc Bar() {}\n");
    Fixture { lang: Language::Go, files: vec![(a.clone(), Language::Go), (b.clone(), Language::Go)], a, b, extra: dir }
}

fn unlinked_go() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "a.go", "package main\nfunc main() {}\n");
    let b = write_file(dir.path(), "b.go", "package main\nfunc Bar() {}\n");
    Fixture { lang: Language::Go, files: vec![(a.clone(), Language::Go), (b.clone(), Language::Go)], a, b, extra: dir }
}

fn linked_swift() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "A.swift", "import B\nclass A {}\n");
    let b = write_file(dir.path(), "B.swift", "public class B {}\n");
    Fixture {
        lang: Language::Swift,
        files: vec![(a.clone(), Language::Swift), (b.clone(), Language::Swift)],
        a,
        b,
        extra: dir,
    }
}

fn unlinked_swift() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "A.swift", "class A {}\n");
    let b = write_file(dir.path(), "B.swift", "class B {}\n");
    Fixture {
        lang: Language::Swift,
        files: vec![(a.clone(), Language::Swift), (b.clone(), Language::Swift)],
        a,
        b,
        extra: dir,
    }
}

fn unlinked_zig() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "a.zig", "pub fn main() void {}\n");
    let b = write_file(dir.path(), "b.zig", "pub fn bar() void {}\n");
    Fixture {
        lang: Language::Zig,
        files: vec![(a.clone(), Language::Zig), (b.clone(), Language::Zig)],
        a,
        b,
        extra: dir,
    }
}

fn linked_ruby() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "a.rb", "require_relative 'b'\nputs 'hi'\n");
    let b = write_file(dir.path(), "b.rb", "def bar; end\n");
    Fixture {
        lang: Language::Ruby,
        files: vec![(a.clone(), Language::Ruby), (b.clone(), Language::Ruby)],
        a,
        b,
        extra: dir,
    }
}

fn unlinked_ruby() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "a.rb", "x = 1\n");
    let b = write_file(dir.path(), "b.rb", "y = 2\n");
    Fixture {
        lang: Language::Ruby,
        files: vec![(a.clone(), Language::Ruby), (b.clone(), Language::Ruby)],
        a,
        b,
        extra: dir,
    }
}

fn linked_objc() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "a.m", "#import \"b.h\"\nint main() { return 0; }\n");
    let b = write_file(dir.path(), "b.h", "void greet();\n");
    Fixture {
        lang: Language::ObjectiveC,
        files: vec![(a.clone(), Language::ObjectiveC), (b.clone(), Language::C)],
        a,
        b,
        extra: dir,
    }
}

fn unlinked_objc() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "a.m", "int x = 1;\n");
    let b = write_file(dir.path(), "b.m", "int y = 2;\n");
    Fixture {
        lang: Language::ObjectiveC,
        files: vec![(a.clone(), Language::ObjectiveC), (b.clone(), Language::ObjectiveC)],
        a,
        b,
        extra: dir,
    }
}

fn linked_tcl() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "a.tcl", "source b.tcl\nputs hi\n");
    let b = write_file(dir.path(), "b.tcl", "proc bar {} {}\n");
    Fixture {
        lang: Language::Tcl,
        files: vec![(a.clone(), Language::Tcl), (b.clone(), Language::Tcl)],
        a,
        b,
        extra: dir,
    }
}

fn unlinked_tcl() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "a.tcl", "set x 1\n");
    let b = write_file(dir.path(), "b.tcl", "set y 2\n");
    Fixture {
        lang: Language::Tcl,
        files: vec![(a.clone(), Language::Tcl), (b.clone(), Language::Tcl)],
        a,
        b,
        extra: dir,
    }
}

fn linked_kotlin() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "src/main/kotlin/A.kt", "import com.foo.B\nclass A {}\n");
    let b = write_file(dir.path(), "src/main/kotlin/com/foo/B.kt", "package com.foo\nclass B\n");
    Fixture {
        lang: Language::Kotlin,
        files: vec![(a.clone(), Language::Kotlin), (b.clone(), Language::Kotlin)],
        a,
        b,
        extra: dir,
    }
}

fn unlinked_kotlin() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "src/main/kotlin/A.kt", "class A\n");
    let b = write_file(dir.path(), "src/main/kotlin/B.kt", "class B\n");
    Fixture {
        lang: Language::Kotlin,
        files: vec![(a.clone(), Language::Kotlin), (b.clone(), Language::Kotlin)],
        a,
        b,
        extra: dir,
    }
}

fn linked_haskell() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "src/A.hs", "module A where\nimport B\n");
    let b = write_file(dir.path(), "src/B.hs", "module B where\nbar :: Int\nbar = 1\n");
    Fixture {
        lang: Language::Haskell,
        files: vec![(a.clone(), Language::Haskell), (b.clone(), Language::Haskell)],
        a,
        b,
        extra: dir,
    }
}

fn unlinked_haskell() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "src/A.hs", "module A where\nfoo :: Int\nfoo = 1\n");
    let b = write_file(dir.path(), "src/B.hs", "module B where\nbar :: Int\nbar = 1\n");
    Fixture {
        lang: Language::Haskell,
        files: vec![(a.clone(), Language::Haskell), (b.clone(), Language::Haskell)],
        a,
        b,
        extra: dir,
    }
}

fn linked_lua() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "a.lua", "local b = require('b')\nprint('hi')\n");
    let b = write_file(dir.path(), "b.lua", "local M = {}\nreturn M\n");
    Fixture {
        lang: Language::Lua,
        files: vec![(a.clone(), Language::Lua), (b.clone(), Language::Lua)],
        a,
        b,
        extra: dir,
    }
}

fn unlinked_lua() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "a.lua", "local x = 1\n");
    let b = write_file(dir.path(), "b.lua", "local y = 2\n");
    Fixture {
        lang: Language::Lua,
        files: vec![(a.clone(), Language::Lua), (b.clone(), Language::Lua)],
        a,
        b,
        extra: dir,
    }
}

fn linked_r() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "a.R", "source('b.R')\nprint('hi')\n");
    let b = write_file(dir.path(), "b.R", "bar <- function() 1\n");
    Fixture { lang: Language::R, files: vec![(a.clone(), Language::R), (b.clone(), Language::R)], a, b, extra: dir }
}

fn unlinked_r() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "a.R", "x <- 1\n");
    let b = write_file(dir.path(), "b.R", "y <- 2\n");
    Fixture { lang: Language::R, files: vec![(a.clone(), Language::R), (b.clone(), Language::R)], a, b, extra: dir }
}

fn linked_php() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "a.php", "<?php require_once 'b.php';\n");
    let b = write_file(dir.path(), "b.php", "<?php function bar() {}\n");
    Fixture {
        lang: Language::Php,
        files: vec![(a.clone(), Language::Php), (b.clone(), Language::Php)],
        a,
        b,
        extra: dir,
    }
}

fn unlinked_php() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "a.php", "<?php $x = 1;\n");
    let b = write_file(dir.path(), "b.php", "<?php $y = 2;\n");
    Fixture {
        lang: Language::Php,
        files: vec![(a.clone(), Language::Php), (b.clone(), Language::Php)],
        a,
        b,
        extra: dir,
    }
}

fn linked_cobol() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(
        dir.path(),
        "a.cbl",
        "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. A.\n       PROCEDURE DIVISION.\n           COPY B.\n       STOP RUN.\n",
    );
    let b = write_file(dir.path(), "B.cbl", "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. B.\n");
    Fixture {
        lang: Language::Cobol,
        files: vec![(a.clone(), Language::Cobol), (b.clone(), Language::Cobol)],
        a,
        b,
        extra: dir,
    }
}

fn unlinked_cobol() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "a.cbl", "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. A.\n");
    let b = write_file(dir.path(), "b.cbl", "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. B.\n");
    Fixture {
        lang: Language::Cobol,
        files: vec![(a.clone(), Language::Cobol), (b.clone(), Language::Cobol)],
        a,
        b,
        extra: dir,
    }
}

fn linked_d() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "a.d", "import b;\nvoid main() {}\n");
    let b = write_file(dir.path(), "b.d", "module b;\nvoid bar() {}\n");
    Fixture { lang: Language::D, files: vec![(a.clone(), Language::D), (b.clone(), Language::D)], a, b, extra: dir }
}

fn unlinked_d() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "a.d", "void main() {}\n");
    let b = write_file(dir.path(), "b.d", "void bar() {}\n");
    Fixture { lang: Language::D, files: vec![(a.clone(), Language::D), (b.clone(), Language::D)], a, b, extra: dir }
}

fn linked_groovy() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "src/main/groovy/A.groovy", "import com.foo.B\nclass A {}\n");
    let b = write_file(dir.path(), "src/main/groovy/com/foo/B.groovy", "package com.foo\nclass B {}\n");
    Fixture {
        lang: Language::Groovy,
        files: vec![(a.clone(), Language::Groovy), (b.clone(), Language::Groovy)],
        a,
        b,
        extra: dir,
    }
}

fn unlinked_groovy() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "src/main/groovy/A.groovy", "class A {}\n");
    let b = write_file(dir.path(), "src/main/groovy/B.groovy", "class B {}\n");
    Fixture {
        lang: Language::Groovy,
        files: vec![(a.clone(), Language::Groovy), (b.clone(), Language::Groovy)],
        a,
        b,
        extra: dir,
    }
}

fn assert_linked(f: Fixture) {
    let graph = build_graph(&f.files, f.extra.path());
    assert!(directly_linked(&graph, &f.a, &f.b), "{:?}: with-import fixture should detect static link", f.lang);
}

fn assert_not_linked(f: Fixture) {
    let graph = build_graph(&f.files, f.extra.path());
    assert!(!directly_linked(&graph, &f.a, &f.b), "{:?}: no-import fixture should NOT detect static link", f.lang);
}

#[test]
fn python_with_import_linked() {
    assert_linked(linked_python());
}
#[test]
fn python_without_import_unlinked() {
    assert_not_linked(unlinked_python());
}
#[test]
fn typescript_with_import_linked() {
    assert_linked(linked_typescript());
}
#[test]
fn typescript_without_import_unlinked() {
    assert_not_linked(unlinked_typescript());
}
#[test]
fn javascript_with_import_linked() {
    assert_linked(linked_javascript());
}
#[test]
fn javascript_without_import_unlinked() {
    assert_not_linked(unlinked_javascript());
}
#[test]
fn rust_with_use_linked() {
    assert_linked(linked_rust());
}
#[test]
fn rust_without_use_unlinked() {
    assert_not_linked(unlinked_rust());
}
#[test]
fn c_with_include_linked() {
    assert_linked(linked_c());
}
#[test]
fn c_without_include_unlinked() {
    assert_not_linked(unlinked_c());
}
#[test]
fn cpp_with_include_linked() {
    assert_linked(linked_cpp());
}
#[test]
fn cpp_without_include_unlinked() {
    assert_not_linked(unlinked_cpp());
}
#[test]
fn java_with_import_linked() {
    assert_linked(linked_java());
}
#[test]
fn java_without_import_unlinked() {
    assert_not_linked(unlinked_java());
}
#[test]
fn csharp_with_using_linked() {
    assert_linked(linked_csharp());
}
#[test]
fn csharp_without_using_unlinked() {
    assert_not_linked(unlinked_csharp());
}
#[test]
fn go_with_import_linked() {
    assert_linked(linked_go());
}
#[test]
fn go_without_import_unlinked() {
    assert_not_linked(unlinked_go());
}
#[test]
fn swift_with_import_linked() {
    assert_linked(linked_swift());
}
#[test]
fn swift_without_import_unlinked() {
    assert_not_linked(unlinked_swift());
}
#[test]
fn zig_without_atimport_unlinked() {
    assert_not_linked(unlinked_zig());
}
#[test]
fn ruby_with_require_relative_linked() {
    assert_linked(linked_ruby());
}
#[test]
fn ruby_without_require_unlinked() {
    assert_not_linked(unlinked_ruby());
}
#[test]
fn objc_with_import_linked() {
    assert_linked(linked_objc());
}
#[test]
fn objc_without_import_unlinked() {
    assert_not_linked(unlinked_objc());
}
#[test]
fn tcl_with_source_linked() {
    assert_linked(linked_tcl());
}
#[test]
fn tcl_without_source_unlinked() {
    assert_not_linked(unlinked_tcl());
}
#[test]
fn kotlin_with_import_linked() {
    assert_linked(linked_kotlin());
}
#[test]
fn kotlin_without_import_unlinked() {
    assert_not_linked(unlinked_kotlin());
}
#[test]
fn haskell_with_import_linked() {
    assert_linked(linked_haskell());
}
#[test]
fn haskell_without_import_unlinked() {
    assert_not_linked(unlinked_haskell());
}
#[test]
fn lua_with_require_linked() {
    assert_linked(linked_lua());
}
#[test]
fn lua_without_require_unlinked() {
    assert_not_linked(unlinked_lua());
}
#[test]
fn r_with_source_linked() {
    assert_linked(linked_r());
}
#[test]
fn r_without_source_unlinked() {
    assert_not_linked(unlinked_r());
}
#[test]
fn php_with_require_once_linked() {
    assert_linked(linked_php());
}
#[test]
fn php_without_require_unlinked() {
    assert_not_linked(unlinked_php());
}
#[test]
fn cobol_with_copy_linked() {
    assert_linked(linked_cobol());
}
#[test]
fn cobol_without_copy_unlinked() {
    assert_not_linked(unlinked_cobol());
}
#[test]
fn d_with_import_linked() {
    assert_linked(linked_d());
}
#[test]
fn d_without_import_unlinked() {
    assert_not_linked(unlinked_d());
}
#[test]
fn groovy_with_import_linked() {
    assert_linked(linked_groovy());
}
#[test]
fn groovy_without_import_unlinked() {
    assert_not_linked(unlinked_groovy());
}
