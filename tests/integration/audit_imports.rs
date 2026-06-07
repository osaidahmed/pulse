use std::path::{Path, PathBuf};

use pulse::audit::finding::ImportConfidence;
use pulse::audit::imports::{confidence_for, extract_imports, has_extractor, resolve_target, RawImport};
use pulse::parse::{self, Language};

fn extract(lang: Language, src: &str) -> Vec<RawImport> {
    let tree = parse::parse_only(src, lang).expect("parse");
    extract_imports(&tree, src, lang)
}

fn targets(raws: &[RawImport]) -> Vec<String> {
    raws.iter().map(|r| r.target.clone()).collect()
}

#[test]
fn rust_simple_use_extracts_path() {
    let raws = extract(Language::Rust, "use foo::bar;\n");
    assert_eq!(targets(&raws), vec!["foo::bar"]);
}

#[test]
fn rust_use_single_identifier() {
    let raws = extract(Language::Rust, "use foo;\n");
    assert_eq!(targets(&raws), vec!["foo"]);
}

#[test]
fn rust_use_with_grouped_imports_extracts_path_only() {
    let raws = extract(Language::Rust, "use foo::bar::{a, b, c};\n");
    let ts = targets(&raws);
    assert!(ts.iter().any(|t| t == "foo::bar"));
}

#[test]
fn rust_use_as_clause_extracts_underlying_path() {
    let raws = extract(Language::Rust, "use foo::bar as baz;\n");
    assert_eq!(targets(&raws), vec!["foo::bar"]);
}

#[test]
fn rust_use_wildcard_extracts_module_path() {
    let raws = extract(Language::Rust, "use foo::*;\n");
    assert_eq!(targets(&raws), vec!["foo"]);
}

#[test]
fn rust_extern_crate_extracts_name() {
    let raws = extract(Language::Rust, "extern crate serde;\n");
    assert_eq!(targets(&raws), vec!["serde"]);
}

#[test]
fn rust_crate_self_super_prefixes_preserved() {
    let raws = extract(Language::Rust, "use crate::foo;\nuse self::bar;\nuse super::baz;\n");
    let ts = targets(&raws);
    assert_eq!(ts.len(), 3);
    assert!(ts.contains(&"crate::foo".to_string()));
    assert!(ts.contains(&"self::bar".to_string()));
    assert!(ts.contains(&"super::baz".to_string()));
}

#[test]
fn rust_multiple_imports_all_extracted() {
    let src = "use foo;\nuse bar;\nuse baz::qux;\n";
    let raws = extract(Language::Rust, src);
    assert_eq!(raws.len(), 3);
}

#[test]
fn rust_no_imports_yields_empty_vec() {
    let raws = extract(Language::Rust, "fn main() { let x = 1; }\n");
    assert!(raws.is_empty());
}

#[test]
fn rust_imports_record_correct_line_numbers() {
    let src = "fn first() {}\n\nuse foo;\n\nuse bar;\n";
    let raws = extract(Language::Rust, src);
    assert_eq!(raws[0].line, 3);
    assert_eq!(raws[1].line, 5);
}

#[test]
fn rust_imports_stable_under_whitespace() {
    let a = extract(Language::Rust, "use foo::bar;\nuse baz;\n");
    let b = extract(Language::Rust, "use foo::bar;\n\n\n   use baz;\n\n");
    assert_eq!(targets(&a), targets(&b));
}

#[test]
fn rust_imports_stable_under_comments() {
    let a = extract(Language::Rust, "use foo::bar;\nuse baz;\n");
    let b = extract(Language::Rust, "// header comment\nuse foo::bar;\n// inline\nuse baz; // tail\n");
    assert_eq!(targets(&a), targets(&b));
}

#[test]
fn rust_extraction_is_deterministic_across_runs() {
    let src = "use foo::bar;\nuse baz::qux;\nuse alpha;\n";
    let a = extract(Language::Rust, src);
    let b = extract(Language::Rust, src);
    assert_eq!(targets(&a), targets(&b));
}

#[test]
fn rust_pub_use_treated_same_as_use() {
    let raws = extract(Language::Rust, "pub use foo::bar;\n");
    assert_eq!(targets(&raws), vec!["foo::bar"]);
}

#[test]
fn rust_nested_module_imports_picked_up() {
    let src = "mod inner {\n    use crate::outer;\n}\n";
    let raws = extract(Language::Rust, src);
    assert!(targets(&raws).contains(&"crate::outer".to_string()));
}

#[test]
fn go_simple_import_extracts_path_without_quotes() {
    let raws = extract(Language::Go, "package main\nimport \"fmt\"\n");
    assert_eq!(targets(&raws), vec!["fmt"]);
}

#[test]
fn go_grouped_imports_extracts_each() {
    let src = "package main\nimport (\n  \"fmt\"\n  \"os\"\n  \"io\"\n)\n";
    let raws = extract(Language::Go, src);
    let ts = targets(&raws);
    assert_eq!(ts.len(), 3);
    assert!(ts.contains(&"fmt".to_string()));
    assert!(ts.contains(&"os".to_string()));
    assert!(ts.contains(&"io".to_string()));
}

#[test]
fn go_aliased_import_extracts_path() {
    let src = "package main\nimport f \"fmt\"\n";
    let raws = extract(Language::Go, src);
    assert_eq!(targets(&raws), vec!["fmt"]);
}

#[test]
fn go_dot_import_extracts_path() {
    let src = "package main\nimport . \"fmt\"\n";
    let raws = extract(Language::Go, src);
    assert_eq!(targets(&raws), vec!["fmt"]);
}

#[test]
fn go_blank_import_extracts_path() {
    let src = "package main\nimport _ \"fmt\"\n";
    let raws = extract(Language::Go, src);
    assert_eq!(targets(&raws), vec!["fmt"]);
}

#[test]
fn go_no_imports_yields_empty_vec() {
    let raws = extract(Language::Go, "package main\nfunc main() {}\n");
    assert!(raws.is_empty());
}

#[test]
fn go_long_import_path_extracts_full_string() {
    let src = "package main\nimport \"github.com/spf13/cobra\"\n";
    let raws = extract(Language::Go, src);
    assert_eq!(targets(&raws), vec!["github.com/spf13/cobra"]);
}

#[test]
fn go_imports_record_correct_line_numbers() {
    let src = "package main\n\nimport \"fmt\"\n\nimport \"os\"\n";
    let raws = extract(Language::Go, src);
    assert!(raws[0].line >= 3);
    assert!(raws[1].line >= 5);
}

#[test]
fn go_imports_stable_under_whitespace() {
    let a = extract(Language::Go, "package main\nimport \"fmt\"\n");
    let b = extract(Language::Go, "package main\n\n\nimport    \"fmt\"\n\n");
    assert_eq!(targets(&a), targets(&b));
}

#[test]
fn go_extraction_is_deterministic_across_runs() {
    let src = "package main\nimport (\n  \"fmt\"\n  \"os\"\n)\n";
    let a = extract(Language::Go, src);
    let b = extract(Language::Go, src);
    assert_eq!(targets(&a), targets(&b));
}

#[test]
fn unknown_grammar_kinds_yield_empty_extraction() {
    let raws = extract(Language::Rust, "fn main() { let x = 1; }\n");
    assert!(raws.is_empty());
}

#[test]
fn java_simple_import_extracts_dotted_path() {
    let src = "package p;\nimport com.foo.Bar;\n";
    let raws = extract(Language::Java, src);
    assert_eq!(targets(&raws), vec!["com.foo.Bar"]);
}

#[test]
fn java_static_import_extracts_path() {
    let src = "package p;\nimport static com.foo.Bar.method;\n";
    let raws = extract(Language::Java, src);
    assert!(!raws.is_empty());
}

#[test]
fn java_wildcard_import_extracts_package() {
    let src = "package p;\nimport com.foo.*;\n";
    let raws = extract(Language::Java, src);
    assert!(!raws.is_empty());
}

#[test]
fn java_multiple_imports_all_extracted() {
    let src = "package p;\nimport com.a.A;\nimport com.b.B;\nimport com.c.C;\n";
    let raws = extract(Language::Java, src);
    assert_eq!(raws.len(), 3);
}

#[test]
fn java_no_imports_yields_empty_vec() {
    let raws = extract(Language::Java, "package p;\nclass Foo {}\n");
    assert!(raws.is_empty());
}

#[test]
fn java_extraction_is_deterministic() {
    let src = "package p;\nimport com.foo.Bar;\nimport com.baz.Qux;\n";
    let a = extract(Language::Java, src);
    let b = extract(Language::Java, src);
    assert_eq!(targets(&a), targets(&b));
}

#[test]
fn java_imports_stable_under_whitespace() {
    let a = extract(Language::Java, "package p;\nimport com.foo.Bar;\n");
    let b = extract(Language::Java, "package p;\n\n\n   import com.foo.Bar;\n\n");
    assert_eq!(targets(&a), targets(&b));
}

#[test]
fn csharp_simple_using_extracts_namespace() {
    let src = "using System.Collections.Generic;\n";
    let raws = extract(Language::CSharp, src);
    assert_eq!(targets(&raws), vec!["System.Collections.Generic"]);
}

#[test]
fn csharp_aliased_using_extracts_target() {
    let src = "using SC = System.Collections;\n";
    let raws = extract(Language::CSharp, src);
    assert!(!raws.is_empty());
}

#[test]
fn csharp_static_using_extracts_target() {
    let src = "using static System.Math;\n";
    let raws = extract(Language::CSharp, src);
    assert!(!raws.is_empty());
}

#[test]
fn csharp_multiple_usings_all_extracted() {
    let src = "using A.B;\nusing C.D;\nusing E.F;\n";
    let raws = extract(Language::CSharp, src);
    assert_eq!(raws.len(), 3);
}

#[test]
fn csharp_no_usings_yields_empty_vec() {
    let raws = extract(Language::CSharp, "namespace N { class C {} }\n");
    assert!(raws.is_empty());
}

#[test]
fn csharp_extraction_is_deterministic() {
    let src = "using A.B;\nusing C.D;\n";
    let a = extract(Language::CSharp, src);
    let b = extract(Language::CSharp, src);
    assert_eq!(targets(&a), targets(&b));
}

#[test]
fn kotlin_simple_import_extracts_dotted_path() {
    let src = "package p\nimport com.foo.Bar\n";
    let raws = extract(Language::Kotlin, src);
    assert_eq!(targets(&raws), vec!["com.foo.Bar"]);
}

#[test]
fn kotlin_aliased_import_extracts_target() {
    let src = "package p\nimport com.foo.Bar as B\n";
    let raws = extract(Language::Kotlin, src);
    assert!(!raws.is_empty());
}

#[test]
fn kotlin_wildcard_import_extracts_package() {
    let src = "package p\nimport com.foo.*\n";
    let raws = extract(Language::Kotlin, src);
    assert!(!raws.is_empty());
}

#[test]
fn kotlin_multiple_imports_all_extracted() {
    let src = "package p\nimport com.a.A\nimport com.b.B\n";
    let raws = extract(Language::Kotlin, src);
    assert_eq!(raws.len(), 2);
}

#[test]
fn kotlin_no_imports_yields_empty_vec() {
    let raws = extract(Language::Kotlin, "package p\nclass Foo\n");
    assert!(raws.is_empty());
}

#[test]
fn kotlin_extraction_is_deterministic() {
    let src = "package p\nimport com.foo.Bar\n";
    let a = extract(Language::Kotlin, src);
    let b = extract(Language::Kotlin, src);
    assert_eq!(targets(&a), targets(&b));
}

#[test]
fn java_resolve_to_existing_file() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_file(&root.join("Caller.java"), "");
    write_file(&root.join("com/foo/Bar.java"), "");
    let source = root.join("Caller.java");
    let resolved = resolve_target("com.foo.Bar", &source, root, Language::Java);
    assert!(resolved.is_some());
}

#[test]
fn java_resolve_external_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_file(&root.join("Caller.java"), "");
    let source = root.join("Caller.java");
    let resolved = resolve_target("java.util.List", &source, root, Language::Java);
    assert!(resolved.is_none());
}

#[test]
fn java_resolve_via_maven_layout() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_file(&root.join("src/main/java/Caller.java"), "");
    write_file(&root.join("src/main/java/com/foo/Bar.java"), "");
    let source = root.join("src/main/java/Caller.java");
    let resolved = resolve_target("com.foo.Bar", &source, root, Language::Java);
    assert!(resolved.is_some());
    assert!(resolved.unwrap().to_string_lossy().contains("com/foo/Bar.java"));
}

#[test]
fn kotlin_resolve_to_existing_kt_file() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_file(&root.join("Caller.kt"), "");
    write_file(&root.join("com/foo/Bar.kt"), "");
    let source = root.join("Caller.kt");
    let resolved = resolve_target("com.foo.Bar", &source, root, Language::Kotlin);
    assert!(resolved.is_some());
}

#[test]
fn csharp_resolve_to_existing_cs_file() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_file(&root.join("Caller.cs"), "");
    write_file(&root.join("Foo/Bar.cs"), "");
    let source = root.join("Caller.cs");
    let resolved = resolve_target("Foo.Bar", &source, root, Language::CSharp);
    assert!(resolved.is_some());
}

#[test]
fn confidence_for_java_csharp_kotlin_is_high() {
    assert_eq!(confidence_for(Language::Java), ImportConfidence::High);
    assert_eq!(confidence_for(Language::CSharp), ImportConfidence::High);
    assert_eq!(confidence_for(Language::Kotlin), ImportConfidence::High);
}

#[test]
fn confidence_for_swift_haskell_zig_d_is_high() {
    assert_eq!(confidence_for(Language::Swift), ImportConfidence::High);
    assert_eq!(confidence_for(Language::Haskell), ImportConfidence::High);
    assert_eq!(confidence_for(Language::Zig), ImportConfidence::High);
    assert_eq!(confidence_for(Language::D), ImportConfidence::High);
}

#[test]
fn swift_simple_import_extracts_module_name() {
    let raws = extract(Language::Swift, "import Foundation\n");
    assert_eq!(targets(&raws), vec!["Foundation"]);
}

#[test]
fn swift_dotted_import_extracts_dotted_path() {
    let raws = extract(Language::Swift, "import class UIKit.UIView\n");
    assert!(!raws.is_empty());
    assert!(raws[0].target.contains("UIKit"));
}

#[test]
fn swift_multiple_imports_all_extracted() {
    let src = "import A\nimport B\nimport C\n";
    let raws = extract(Language::Swift, src);
    assert_eq!(raws.len(), 3);
}

#[test]
fn swift_no_imports_yields_empty_vec() {
    let raws = extract(Language::Swift, "class Foo {}\n");
    assert!(raws.is_empty());
}

#[test]
fn swift_extraction_is_deterministic() {
    let src = "import Foundation\nimport UIKit\n";
    let a = extract(Language::Swift, src);
    let b = extract(Language::Swift, src);
    assert_eq!(targets(&a), targets(&b));
}

#[test]
fn haskell_simple_import_extracts_dotted_module() {
    let src = "module M where\nimport Foo.Bar\n";
    let raws = extract(Language::Haskell, src);
    assert!(targets(&raws).contains(&"Foo.Bar".to_string()));
}

#[test]
fn haskell_qualified_import_extracts_module() {
    let src = "module M where\nimport qualified Data.Map as M\n";
    let raws = extract(Language::Haskell, src);
    assert!(!raws.is_empty());
    assert!(raws[0].target.contains("Data.Map"));
}

#[test]
fn haskell_multiple_imports_all_extracted() {
    let src = "module M where\nimport A\nimport B\nimport C.D\n";
    let raws = extract(Language::Haskell, src);
    assert_eq!(raws.len(), 3);
}

#[test]
fn haskell_no_imports_yields_empty_vec() {
    let raws = extract(Language::Haskell, "module M where\nfoo = 1\n");
    assert!(raws.is_empty());
}

#[test]
fn haskell_extraction_is_deterministic() {
    let src = "module M where\nimport A.B\nimport C.D\n";
    let a = extract(Language::Haskell, src);
    let b = extract(Language::Haskell, src);
    assert_eq!(targets(&a), targets(&b));
}

#[test]
fn zig_at_import_extracts_string_arg() {
    let src = "const std = @import(\"std\");\n";
    let raws = extract(Language::Zig, src);
    assert!(targets(&raws).contains(&"std".to_string()));
}

#[test]
fn zig_at_import_extracts_relative_zig_file() {
    let src = "const foo = @import(\"foo.zig\");\n";
    let raws = extract(Language::Zig, src);
    assert!(targets(&raws).contains(&"foo.zig".to_string()));
}

#[test]
fn zig_multiple_at_imports_all_extracted() {
    let src = "const a = @import(\"a\");\nconst b = @import(\"b\");\n";
    let raws = extract(Language::Zig, src);
    assert_eq!(raws.len(), 2);
}

#[test]
fn zig_other_builtins_not_extracted() {
    let src = "const x = @intCast(u32, 5);\n";
    let raws = extract(Language::Zig, src);
    assert!(raws.is_empty());
}

#[test]
fn zig_no_imports_yields_empty_vec() {
    let raws = extract(Language::Zig, "fn main() void {}\n");
    assert!(raws.is_empty());
}

#[test]
fn zig_extraction_is_deterministic() {
    let src = "const a = @import(\"a\");\nconst b = @import(\"b\");\n";
    let a = extract(Language::Zig, src);
    let b = extract(Language::Zig, src);
    assert_eq!(targets(&a), targets(&b));
}

#[test]
fn d_simple_import_extracts_dotted_module() {
    let src = "module m;\nimport std.stdio;\n";
    let raws = extract(Language::D, src);
    assert!(targets(&raws).contains(&"std.stdio".to_string()));
}

#[test]
fn d_selective_import_extracts_module() {
    let src = "module m;\nimport foo.bar : baz;\n";
    let raws = extract(Language::D, src);
    assert!(!raws.is_empty());
    assert!(raws[0].target.contains("foo.bar"));
}

#[test]
fn d_multiple_imports_all_extracted() {
    let src = "module m;\nimport a;\nimport b.c;\nimport d.e.f;\n";
    let raws = extract(Language::D, src);
    assert_eq!(raws.len(), 3);
}

#[test]
fn d_no_imports_yields_empty_vec() {
    let raws = extract(Language::D, "module m;\nvoid main() {}\n");
    assert!(raws.is_empty());
}

#[test]
fn d_extraction_is_deterministic() {
    let src = "module m;\nimport std.stdio;\nimport core.thread;\n";
    let a = extract(Language::D, src);
    let b = extract(Language::D, src);
    assert_eq!(targets(&a), targets(&b));
}

#[test]
fn haskell_resolve_to_existing_hs_file() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_file(&root.join("Main.hs"), "");
    write_file(&root.join("Foo/Bar.hs"), "");
    let source = root.join("Main.hs");
    let resolved = resolve_target("Foo.Bar", &source, root, Language::Haskell);
    assert!(resolved.is_some());
}

#[test]
fn d_resolve_to_existing_d_file() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_file(&root.join("app.d"), "");
    write_file(&root.join("foo/bar.d"), "");
    let source = root.join("app.d");
    let resolved = resolve_target("foo.bar", &source, root, Language::D);
    assert!(resolved.is_some());
}

#[test]
fn swift_resolve_to_existing_swift_file_via_sources() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_file(&root.join("Sources/Main.swift"), "");
    write_file(&root.join("Sources/Foundation.swift"), "");
    let source = root.join("Sources/Main.swift");
    let resolved = resolve_target("Foundation", &source, root, Language::Swift);
    assert!(resolved.is_some());
}

#[test]
fn confidence_for_rust_is_high() {
    assert_eq!(confidence_for(Language::Rust), ImportConfidence::High);
}

#[test]
fn confidence_for_go_is_high() {
    assert_eq!(confidence_for(Language::Go), ImportConfidence::High);
}

#[test]
fn confidence_tiers_assigned_per_language_capability() {
    assert_eq!(confidence_for(Language::Python), ImportConfidence::Medium);
    assert_eq!(confidence_for(Language::JavaScript), ImportConfidence::Medium);
    assert_eq!(confidence_for(Language::TypeScript), ImportConfidence::Medium);
    assert_eq!(confidence_for(Language::Php), ImportConfidence::Medium);
    assert_eq!(confidence_for(Language::Ruby), ImportConfidence::Medium);
    assert_eq!(confidence_for(Language::ObjectiveC), ImportConfidence::Medium);
    assert_eq!(confidence_for(Language::Groovy), ImportConfidence::Medium);
    assert_eq!(confidence_for(Language::C), ImportConfidence::Low);
    assert_eq!(confidence_for(Language::Cpp), ImportConfidence::Low);
    assert_eq!(confidence_for(Language::Lua), ImportConfidence::BestEffort);
    assert_eq!(confidence_for(Language::R), ImportConfidence::BestEffort);
    assert_eq!(confidence_for(Language::Tcl), ImportConfidence::BestEffort);
    assert_eq!(confidence_for(Language::Cobol), ImportConfidence::BestEffort);
}

#[test]
fn has_extractor_for_all_supported_languages() {
    let supported = [
        Language::Rust,
        Language::Go,
        Language::Java,
        Language::CSharp,
        Language::Kotlin,
        Language::Swift,
        Language::Haskell,
        Language::Zig,
        Language::D,
        Language::Groovy,
        Language::Python,
        Language::JavaScript,
        Language::TypeScript,
        Language::Php,
        Language::Ruby,
        Language::ObjectiveC,
        Language::Lua,
        Language::R,
        Language::Tcl,
        Language::Cobol,
        Language::C,
        Language::Cpp,
    ];
    for lang in supported {
        assert!(has_extractor(lang), "extractor missing for {lang:?}");
    }
}

fn write_file(p: &Path, content: &str) {
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(p, content).unwrap();
}

#[test]
fn rust_resolve_crate_to_src_root() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_file(&root.join("src/lib.rs"), "");
    write_file(&root.join("src/foo.rs"), "");
    let source = root.join("src/lib.rs");
    let resolved = resolve_target("crate::foo", &source, root, Language::Rust);
    assert_eq!(resolved.unwrap(), root.join("src/foo.rs"));
}

#[test]
fn rust_resolve_crate_to_mod_rs() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_file(&root.join("src/lib.rs"), "");
    write_file(&root.join("src/foo/mod.rs"), "");
    let source = root.join("src/lib.rs");
    let resolved = resolve_target("crate::foo", &source, root, Language::Rust);
    assert_eq!(resolved.unwrap(), root.join("src/foo/mod.rs"));
}

#[test]
fn rust_resolve_external_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_file(&root.join("src/lib.rs"), "");
    let source = root.join("src/lib.rs");
    let resolved = resolve_target("std::collections", &source, root, Language::Rust);
    assert!(resolved.is_none());
}

#[test]
fn rust_resolve_super_walks_up_one_directory() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_file(&root.join("src/foo/bar.rs"), "");
    write_file(&root.join("src/baz.rs"), "");
    let source = root.join("src/foo/bar.rs");
    let resolved = resolve_target("super::baz", &source, root, Language::Rust);
    assert_eq!(resolved.unwrap(), root.join("src/baz.rs"));
}

#[test]
fn rust_resolve_self_resolves_in_same_directory() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_file(&root.join("src/foo/mod.rs"), "");
    write_file(&root.join("src/foo/bar.rs"), "");
    let source = root.join("src/foo/mod.rs");
    let resolved = resolve_target("self::bar", &source, root, Language::Rust);
    assert_eq!(resolved.unwrap(), root.join("src/foo/bar.rs"));
}

#[test]
fn rust_resolve_nested_path() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_file(&root.join("src/lib.rs"), "");
    write_file(&root.join("src/audit/imports.rs"), "");
    let source = root.join("src/lib.rs");
    let resolved = resolve_target("crate::audit::imports", &source, root, Language::Rust);
    assert_eq!(resolved.unwrap(), root.join("src/audit/imports.rs"));
}

#[test]
fn rust_resolve_returns_none_when_target_does_not_exist() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_file(&root.join("src/lib.rs"), "");
    let source = root.join("src/lib.rs");
    let resolved = resolve_target("crate::missing", &source, root, Language::Rust);
    assert!(resolved.is_none());
}

#[test]
fn go_resolve_relative_dot_path() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_file(&root.join("main.go"), "");
    write_file(&root.join("pkg/util.go"), "");
    let source = root.join("main.go");
    let resolved = resolve_target("./pkg", &source, root, Language::Go);
    assert!(resolved.is_some());
    assert!(resolved.unwrap().to_string_lossy().contains("pkg"));
}

#[test]
fn go_resolve_external_module_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_file(&root.join("main.go"), "");
    let source = root.join("main.go");
    let resolved = resolve_target("github.com/spf13/cobra", &source, root, Language::Go);
    assert!(resolved.is_none());
}

#[test]
fn go_resolve_root_relative_path() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_file(&root.join("main.go"), "");
    write_file(&root.join("pkg/sub/foo.go"), "");
    let source = root.join("main.go");
    let resolved = resolve_target("pkg/sub", &source, root, Language::Go);
    assert!(resolved.is_some());
}

#[test]
fn rust_malformed_use_does_not_panic() {
    let raws = extract(Language::Rust, "use ;\n");
    assert!(raws.is_empty());
}

#[test]
fn go_malformed_import_does_not_panic() {
    let raws = extract(Language::Go, "package main\nimport \n");
    assert!(raws.is_empty());
}

#[test]
fn extraction_handles_unicode_in_paths() {
    let raws = extract(Language::Rust, "use 模块::子模块;\n");
    assert_eq!(targets(&raws).len(), 1);
}

#[test]
fn rust_use_inside_function_body_is_picked_up() {
    let src = "fn main() {\n    use foo::bar;\n}\n";
    let raws = extract(Language::Rust, src);
    assert!(targets(&raws).contains(&"foo::bar".to_string()));
}

#[test]
fn rust_resolve_with_non_canonical_root_works() {
    let dir = tempfile::tempdir().unwrap();
    let root_buf: PathBuf = dir.path().to_path_buf();
    write_file(&root_buf.join("src/lib.rs"), "");
    write_file(&root_buf.join("src/foo.rs"), "");
    let source = root_buf.join("src/lib.rs");
    let resolved = resolve_target("crate::foo", &source, &root_buf, Language::Rust);
    assert!(resolved.is_some());
}
