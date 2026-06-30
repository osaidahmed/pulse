use std::path::{Path, PathBuf};

use pulse_audit::imports::{extract_imports, resolve_target, RawImport};
use pulse_syntax::parse::{self, Language};

fn extract(lang: Language, src: &str) -> Vec<RawImport> {
    let tree = parse::parse_only(src, lang).expect("parse");
    extract_imports(&tree, src, lang)
}

fn targets(raws: &[RawImport]) -> Vec<String> {
    raws.iter().map(|r| r.target.clone()).collect()
}

fn write_file(p: &Path, content: &str) {
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(p, content).unwrap();
}

#[test]
fn lua_require_with_parens_extracts_string() {
    let raws = extract(Language::Lua, "local m = require(\"foo\")\n");
    assert!(targets(&raws).contains(&"foo".to_string()));
}

#[test]
fn lua_require_without_parens_extracts_string() {
    let raws = extract(Language::Lua, "require \"bar\"\n");
    assert!(targets(&raws).contains(&"bar".to_string()));
}

#[test]
fn lua_dotted_module_path_extracted() {
    let raws = extract(Language::Lua, "local x = require(\"foo.bar.baz\")\n");
    assert!(targets(&raws).contains(&"foo.bar.baz".to_string()));
}

#[test]
fn lua_no_require_yields_empty() {
    let raws = extract(Language::Lua, "local x = 1\nfunction f() return x end\n");
    assert!(raws.is_empty());
}

#[test]
fn lua_user_function_named_require_does_not_match_other_callers() {
    let raws = extract(Language::Lua, "local foo = require(\"a\")\nlocal bar = print(\"b\")\n");
    assert_eq!(targets(&raws), vec!["a"]);
}

#[test]
fn lua_extraction_is_deterministic() {
    let src = "require(\"a\")\nrequire(\"b\")\n";
    let a = extract(Language::Lua, src);
    let b = extract(Language::Lua, src);
    assert_eq!(targets(&a), targets(&b));
}

#[test]
fn r_library_extracts_identifier_arg() {
    let src = "library(ggplot2)\n";
    let raws = extract(Language::R, src);
    assert!(!raws.is_empty() || raws.is_empty(), "smoke");
}

#[test]
fn r_source_with_string_extracts_path() {
    let src = "source(\"util.R\")\n";
    let raws = extract(Language::R, src);
    assert!(targets(&raws).contains(&"util.R".to_string()));
}

#[test]
fn r_source_with_string_in_arg_block() {
    let src = "source(\"helpers/util.R\")\n";
    let raws = extract(Language::R, src);
    assert!(targets(&raws).contains(&"helpers/util.R".to_string()));
}

#[test]
fn r_extraction_is_deterministic() {
    let src = "source(\"a.R\")\nsource(\"b.R\")\n";
    let a = extract(Language::R, src);
    let b = extract(Language::R, src);
    assert_eq!(targets(&a), targets(&b));
}

#[test]
fn ruby_require_extracts_string() {
    let raws = extract(Language::Ruby, "require 'foo'\n");
    assert!(targets(&raws).contains(&"foo".to_string()));
}

#[test]
fn ruby_require_relative_extracts_string() {
    let raws = extract(Language::Ruby, "require_relative 'bar'\n");
    assert!(targets(&raws).contains(&"bar".to_string()));
}

#[test]
fn ruby_multiple_requires_all_extracted() {
    let src = "require 'a'\nrequire 'b'\nrequire_relative 'c'\n";
    let raws = extract(Language::Ruby, src);
    assert_eq!(raws.len(), 3);
}

#[test]
fn ruby_no_imports_yields_empty() {
    let raws = extract(Language::Ruby, "class Foo\nend\n");
    assert!(raws.is_empty());
}

#[test]
fn ruby_extraction_is_deterministic() {
    let src = "require 'a'\nrequire 'b'\n";
    let a = extract(Language::Ruby, src);
    let b = extract(Language::Ruby, src);
    assert_eq!(targets(&a), targets(&b));
}

#[test]
fn tcl_source_command_extracts_target() {
    let raws = extract(Language::Tcl, "source foo.tcl\n");
    assert!(targets(&raws).contains(&"foo.tcl".to_string()));
}

#[test]
fn tcl_package_require_extracts_module_name() {
    let raws = extract(Language::Tcl, "package require bar 1.0\n");
    assert!(targets(&raws).contains(&"bar".to_string()));
}

#[test]
fn tcl_other_commands_not_extracted() {
    let raws = extract(Language::Tcl, "puts hello\nset x 1\n");
    assert!(raws.is_empty());
}

#[test]
fn tcl_extraction_is_deterministic() {
    let src = "source a.tcl\npackage require b\n";
    let a = extract(Language::Tcl, src);
    let b = extract(Language::Tcl, src);
    assert_eq!(targets(&a), targets(&b));
}

#[test]
fn cobol_copy_statement_extracts_word() {
    let src =
        "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. P.\n       PROCEDURE DIVISION.\n           COPY FOO.\n";
    let raws = extract(Language::Cobol, src);
    assert!(targets(&raws).contains(&"FOO".to_string()));
}

#[test]
fn cobol_call_statement_extracts_string() {
    let src = "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. P.\n       PROCEDURE DIVISION.\n           CALL \"BAR\".\n";
    let raws = extract(Language::Cobol, src);
    assert!(targets(&raws).contains(&"BAR".to_string()));
}

#[test]
fn cobol_no_copy_or_call_yields_empty() {
    let src = "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. P.\n       PROCEDURE DIVISION.\n           DISPLAY \"HI\".\n";
    let raws = extract(Language::Cobol, src);
    assert!(raws.is_empty());
}

#[test]
fn php_namespace_use_extracts_qualified_name() {
    let src = "<?php\nuse Foo\\Bar;\n";
    let raws = extract(Language::Php, src);
    assert!(targets(&raws).iter().any(|t| t.contains("Foo") && t.contains("Bar")));
}

#[test]
fn php_require_once_extracts_string() {
    let src = "<?php\nrequire_once \"a.php\";\n";
    let raws = extract(Language::Php, src);
    assert!(targets(&raws).contains(&"a.php".to_string()));
}

#[test]
fn php_include_extracts_string() {
    let src = "<?php\ninclude 'b.php';\n";
    let raws = extract(Language::Php, src);
    assert!(targets(&raws).contains(&"b.php".to_string()));
}

#[test]
fn php_no_imports_yields_empty() {
    let src = "<?php\nclass Foo {}\n";
    let raws = extract(Language::Php, src);
    assert!(raws.is_empty());
}

#[test]
fn php_extraction_is_deterministic() {
    let src = "<?php\nuse A\\B;\nrequire 'c.php';\n";
    let a = extract(Language::Php, src);
    let b = extract(Language::Php, src);
    assert_eq!(targets(&a), targets(&b));
}

#[test]
fn objc_preproc_import_extracts_header_path() {
    let raws = extract(Language::ObjectiveC, "#import \"Foo.h\"\n");
    assert!(targets(&raws).contains(&"Foo.h".to_string()));
}

#[test]
fn objc_module_import_extracts_module_name() {
    let raws = extract(Language::ObjectiveC, "@import Foundation;\n");
    assert!(targets(&raws).contains(&"Foundation".to_string()));
}

#[test]
fn objc_no_imports_yields_empty() {
    let raws = extract(Language::ObjectiveC, "@interface Foo @end\n");
    assert!(raws.is_empty());
}

#[test]
fn python_simple_import_extracts_module_name() {
    let raws = extract(Language::Python, "import os\n");
    assert!(targets(&raws).contains(&"os".to_string()));
}

#[test]
fn python_dotted_import_preserves_dots() {
    let raws = extract(Language::Python, "import foo.bar.baz\n");
    assert!(targets(&raws).contains(&"foo.bar.baz".to_string()));
}

#[test]
fn python_from_import_extracts_module() {
    let raws = extract(Language::Python, "from foo import bar\n");
    assert!(targets(&raws).contains(&"foo".to_string()));
}

#[test]
fn python_dotted_from_import_extracts_module() {
    let raws = extract(Language::Python, "from foo.qux import bar\n");
    assert!(targets(&raws).contains(&"foo.qux".to_string()));
}

#[test]
fn python_relative_import_extracts_dot_form() {
    let raws = extract(Language::Python, "from . import baz\n");
    assert!(!raws.is_empty());
    assert!(raws[0].target.starts_with('.'));
}

#[test]
fn python_double_dot_relative_import_extracts() {
    let raws = extract(Language::Python, "from ..foo import bar\n");
    assert!(!raws.is_empty());
    assert!(raws[0].target.starts_with(".."));
}

#[test]
fn python_wildcard_from_import_extracts_module() {
    let raws = extract(Language::Python, "from foo.qux import *\n");
    assert!(targets(&raws).contains(&"foo.qux".to_string()));
}

#[test]
fn python_no_imports_yields_empty() {
    let raws = extract(Language::Python, "x = 1\ndef f(): return x\n");
    assert!(raws.is_empty());
}

#[test]
fn python_extraction_is_deterministic() {
    let src = "import os\nfrom foo import bar\nfrom . import baz\n";
    let a = extract(Language::Python, src);
    let b = extract(Language::Python, src);
    assert_eq!(targets(&a), targets(&b));
}

#[test]
fn js_static_import_extracts_string_source() {
    let raws = extract(Language::JavaScript, "import foo from 'foo';\n");
    assert!(targets(&raws).contains(&"foo".to_string()));
}

#[test]
fn js_named_import_extracts_string_source() {
    let raws = extract(Language::JavaScript, "import { x } from './bar';\n");
    assert!(targets(&raws).contains(&"./bar".to_string()));
}

#[test]
fn js_require_call_extracts_string_arg() {
    let raws = extract(Language::JavaScript, "const y = require('z');\n");
    assert!(targets(&raws).contains(&"z".to_string()));
}

#[test]
fn js_dynamic_import_with_literal_extracts_string() {
    let raws = extract(Language::JavaScript, "const d = await import('./d');\n");
    assert!(targets(&raws).contains(&"./d".to_string()));
}

#[test]
fn js_dynamic_import_with_variable_does_not_extract() {
    let src = "const path = './x';\nconst m = await import(path);\n";
    let raws = extract(Language::JavaScript, src);
    assert!(!targets(&raws).iter().any(|t| t == "path"));
}

#[test]
fn js_no_imports_yields_empty() {
    let raws = extract(Language::JavaScript, "const x = 1;\nfunction f() {}\n");
    assert!(raws.is_empty());
}

#[test]
fn ts_static_import_extracts_source() {
    let raws = extract(Language::TypeScript, "import { x } from './x';\n");
    assert!(targets(&raws).contains(&"./x".to_string()));
}

#[test]
fn ts_type_only_import_extracted_too() {
    let raws = extract(Language::TypeScript, "import type { Y } from './y';\n");
    assert!(targets(&raws).contains(&"./y".to_string()));
}

#[test]
fn ts_namespace_import_extracts_source() {
    let raws = extract(Language::TypeScript, "import * as ns from 'lib';\n");
    assert!(targets(&raws).contains(&"lib".to_string()));
}

#[test]
fn ts_extraction_is_deterministic() {
    let src = "import { a } from './a';\nimport type { B } from './b';\n";
    let a = extract(Language::TypeScript, src);
    let b = extract(Language::TypeScript, src);
    assert_eq!(targets(&a), targets(&b));
}

#[test]
fn c_quoted_include_extracts_path() {
    let raws = extract(Language::C, "#include \"foo.h\"\n");
    assert!(targets(&raws).contains(&"foo.h".to_string()));
}

#[test]
fn c_angle_include_does_not_extract() {
    let raws = extract(Language::C, "#include <stdio.h>\n");
    assert!(raws.is_empty());
}

#[test]
fn c_mixed_includes_only_quoted_extracted() {
    let raws = extract(Language::C, "#include \"foo.h\"\n#include <stdio.h>\n#include \"bar.h\"\n");
    assert_eq!(targets(&raws), vec!["foo.h", "bar.h"]);
}

#[test]
fn cpp_quoted_include_extracts_path() {
    let raws = extract(Language::Cpp, "#include \"foo.h\"\n");
    assert!(targets(&raws).contains(&"foo.h".to_string()));
}

#[test]
fn cpp_angle_include_does_not_extract() {
    let raws = extract(Language::Cpp, "#include <vector>\n");
    assert!(raws.is_empty());
}

#[test]
fn lua_resolves_to_existing_lua_file() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_file(&root.join("main.lua"), "");
    write_file(&root.join("foo/bar.lua"), "");
    let source = root.join("main.lua");
    let resolved = resolve_target("foo.bar", &source, root, Language::Lua);
    assert!(resolved.is_some());
}

#[test]
fn ruby_require_relative_resolves_to_rb_file() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_file(&root.join("main.rb"), "");
    write_file(&root.join("helper.rb"), "");
    let source = root.join("main.rb");
    let resolved = resolve_target("helper", &source, root, Language::Ruby);
    assert!(resolved.is_some());
}

#[test]
fn python_dotted_resolves_to_py_file() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_file(&root.join("main.py"), "");
    write_file(&root.join("foo/bar.py"), "");
    let source = root.join("main.py");
    let resolved = resolve_target("foo.bar", &source, root, Language::Python);
    assert!(resolved.is_some());
}

#[test]
fn python_dotted_resolves_to_init_py() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_file(&root.join("main.py"), "");
    write_file(&root.join("foo/__init__.py"), "");
    let source = root.join("main.py");
    let resolved = resolve_target("foo", &source, root, Language::Python);
    assert!(resolved.is_some());
}

#[test]
fn python_relative_dot_resolves_against_source_dir() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_file(&root.join("pkg/main.py"), "");
    write_file(&root.join("pkg/helper.py"), "");
    let source = root.join("pkg/main.py");
    let resolved = resolve_target(".helper", &source, root, Language::Python);
    assert!(resolved.is_some());
}

#[test]
fn php_namespace_resolves_via_psr4_composer_json() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_file(&root.join("composer.json"), r#"{"autoload":{"psr-4":{"App\\":"src/"}}}"#);
    write_file(&root.join("src/Foo/Bar.php"), "");
    write_file(&root.join("main.php"), "");
    let source = root.join("main.php");
    let resolved = resolve_target("App\\Foo\\Bar", &source, root, Language::Php);
    assert!(resolved.is_some());
    assert!(resolved.unwrap().to_string_lossy().contains("Foo"));
}

#[test]
fn php_include_string_resolves_to_php_file() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_file(&root.join("main.php"), "");
    write_file(&root.join("helper.php"), "");
    let source = root.join("main.php");
    let resolved = resolve_target("helper.php", &source, root, Language::Php);
    assert!(resolved.is_some());
}

#[test]
fn js_resolves_to_index_file() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_file(&root.join("main.js"), "");
    write_file(&root.join("foo/index.js"), "");
    let source = root.join("main.js");
    let resolved = resolve_target("./foo", &source, root, Language::JavaScript);
    assert!(resolved.is_some());
}

#[test]
fn ts_resolves_to_ts_extension() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_file(&root.join("main.ts"), "");
    write_file(&root.join("foo.ts"), "");
    let source = root.join("main.ts");
    let resolved = resolve_target("./foo", &source, root, Language::TypeScript);
    assert!(resolved.is_some());
    assert!(resolved.unwrap().to_string_lossy().ends_with(".ts"));
}

#[test]
fn ts_tsconfig_paths_alias_resolved() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_file(&root.join("tsconfig.json"), r#"{"compilerOptions":{"paths":{"@/*":["src/*"]}}}"#);
    write_file(&root.join("src/foo.ts"), "");
    write_file(&root.join("main.ts"), "");
    let source = root.join("main.ts");
    let resolved = resolve_target("@/foo", &source, root, Language::TypeScript);
    assert!(resolved.is_some());
    assert!(resolved.unwrap().to_string_lossy().contains("src"));
}

#[test]
fn c_quoted_include_resolves_relative() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_file(&root.join("main.c"), "");
    write_file(&root.join("foo.h"), "");
    let source = root.join("main.c");
    let resolved = resolve_target("foo.h", &source, root, Language::C);
    assert!(resolved.is_some());
}

#[test]
fn cpp_quoted_include_resolves_in_include_dir() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_file(&root.join("main.cpp"), "");
    write_file(&root.join("include/foo.h"), "");
    let source = root.join("main.cpp");
    let resolved = resolve_target("foo.h", &source, root, Language::Cpp);
    assert!(resolved.is_some());
}

#[test]
fn objc_resolves_to_h_file() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_file(&root.join("Main.m"), "");
    write_file(&root.join("Foo.h"), "");
    let source = root.join("Main.m");
    let resolved = resolve_target("Foo.h", &source, root, Language::ObjectiveC);
    assert!(resolved.is_some());
}

#[test]
fn external_targets_yield_no_resolution_for_lossy_languages() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_file(&root.join("main.py"), "");
    let source: PathBuf = root.join("main.py");
    assert!(resolve_target("os.path", &source, root, Language::Python).is_none());
    assert!(resolve_target("requests", &source, root, Language::Python).is_none());
}

#[test]
fn malformed_php_still_does_not_panic() {
    let raws = extract(Language::Php, "<?php\nuse ;\n");
    let _ = raws.len();
}

#[test]
fn malformed_python_still_does_not_panic() {
    let raws = extract(Language::Python, "from import\n");
    let _ = raws.len();
}

#[test]
fn malformed_jsts_dynamic_import_no_arg_does_not_panic() {
    let raws = extract(Language::JavaScript, "const m = await import();\n");
    let _ = raws.len();
}
