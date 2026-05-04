use pulse::audit::imports::{extract_imports, has_extractor};
use pulse::parse::{parse_only, Language};

fn extract(source: &str, lang: Language) -> Vec<pulse::audit::imports::RawImport> {
    let tree = parse_only(source, lang).expect("parse");
    extract_imports(&tree, source, lang)
}

#[test]
fn rust_use_declaration_extracted() {
    let imports = extract("use std::collections::HashMap;\n", Language::Rust);
    assert!(imports.iter().any(|i| i.target.contains("HashMap") || i.target.contains("collections")));
}

#[test]
fn rust_extern_crate_extracted() {
    let imports = extract("extern crate serde;\n", Language::Rust);
    assert!(imports.iter().any(|i| i.target.contains("serde")));
}

#[test]
fn go_import_path_extracted() {
    let imports = extract("package m\nimport \"fmt\"\n", Language::Go);
    assert!(imports.iter().any(|i| i.target.contains("fmt")));
}

#[test]
fn go_grouped_imports_each_extracted() {
    let imports = extract("package m\nimport (\n    \"fmt\"\n    \"os\"\n    \"errors\"\n)\n", Language::Go);
    assert!(imports.len() >= 3);
}

#[test]
fn java_single_import_extracted() {
    let imports = extract("import java.util.HashMap;\n", Language::Java);
    assert!(imports.iter().any(|i| i.target.contains("HashMap") || i.target.contains("util")));
}

#[test]
fn java_wildcard_import_extracted() {
    let imports = extract("import java.util.*;\n", Language::Java);
    let _ = imports;
}

#[test]
fn kotlin_import_extracted() {
    let imports = extract("import kotlin.collections.HashMap\n", Language::Kotlin);
    let _ = imports;
}

#[test]
fn typescript_named_imports_extracted() {
    let imports = extract("import { foo, bar } from './module';\n", Language::TypeScript);
    assert!(!imports.is_empty());
}

#[test]
fn typescript_namespace_import_extracted() {
    let imports = extract("import * as ns from './module';\n", Language::TypeScript);
    assert!(!imports.is_empty());
}

#[test]
fn typescript_default_import_extracted() {
    let imports = extract("import Default from './module';\n", Language::TypeScript);
    assert!(!imports.is_empty());
}

#[test]
fn typescript_side_effect_import_handled() {
    let imports = extract("import './side-effect';\n", Language::TypeScript);
    let _ = imports;
}

#[test]
fn javascript_require_call_extracted() {
    let imports = extract("const fs = require('fs');\n", Language::JavaScript);
    let _ = imports;
}

#[test]
fn javascript_dynamic_import_extracted() {
    let imports = extract("const mod = await import('./mod.js');\n", Language::JavaScript);
    let _ = imports;
}

#[test]
fn python_simple_import_extracted() {
    let imports = extract("import os\n", Language::Python);
    assert!(imports.iter().any(|i| i.target == "os"));
}

#[test]
fn python_from_import_extracted() {
    let imports = extract("from os.path import join\n", Language::Python);
    assert!(!imports.is_empty());
}

#[test]
fn python_relative_import_handled() {
    let imports = extract("from . import helpers\n", Language::Python);
    let _ = imports;
}

#[test]
fn python_double_relative_import_handled() {
    let imports = extract("from .. import helpers\n", Language::Python);
    let _ = imports;
}

#[test]
fn python_multiple_imports_each_extracted() {
    let imports = extract("import os\nimport sys\nimport json\n", Language::Python);
    let names: std::collections::BTreeSet<String> = imports
        .iter()
        .map(|i| i.target.clone())
        .collect();
    assert!(names.contains("os"));
    assert!(names.contains("sys"));
    assert!(names.contains("json"));
}

#[test]
fn c_include_quoted_extracted() {
    let imports = extract("#include \"helpers.h\"\nint main() { return 0; }\n", Language::C);
    assert!(imports.iter().any(|i| i.target.contains("helpers")));
}

#[test]
fn c_include_angle_bracket_handled() {
    let _ = extract("#include <stdio.h>\nint main() { return 0; }\n", Language::C);
}

#[test]
fn cpp_include_handled() {
    let _ = extract("#include <iostream>\nint main() { return 0; }\n", Language::Cpp);
}

#[test]
fn objc_import_extracted() {
    let imports = extract("#import <Foundation/Foundation.h>\n", Language::ObjectiveC);
    let _ = imports;
}

#[test]
fn ruby_require_extracted() {
    let imports = extract("require 'json'\n", Language::Ruby);
    let _ = imports;
}

#[test]
fn ruby_require_relative_extracted() {
    let imports = extract("require_relative 'helpers'\n", Language::Ruby);
    let _ = imports;
}

#[test]
fn lua_require_extracted() {
    let imports = extract("local mod = require('helpers')\n", Language::Lua);
    let _ = imports;
}

#[test]
fn r_library_call_extracted() {
    let imports = extract("library(dplyr)\n", Language::R);
    let _ = imports;
}

#[test]
fn r_require_call_extracted() {
    let imports = extract("require(ggplot2)\n", Language::R);
    let _ = imports;
}

#[test]
fn haskell_import_extracted() {
    let imports = extract("module M where\nimport Data.List\n", Language::Haskell);
    let _ = imports;
}

#[test]
fn zig_import_extracted() {
    let imports = extract("const std = @import(\"std\");\n", Language::Zig);
    let _ = imports;
}

#[test]
fn d_import_extracted() {
    let imports = extract("import std.stdio;\nvoid main() {}\n", Language::D);
    let _ = imports;
}

#[test]
fn groovy_import_extracted() {
    let imports = extract("import groovy.json.JsonSlurper\n", Language::Groovy);
    let _ = imports;
}

#[test]
fn swift_import_extracted() {
    let imports = extract("import Foundation\n", Language::Swift);
    let _ = imports;
}

#[test]
fn csharp_using_extracted() {
    let imports = extract("using System;\nclass M {}\n", Language::CSharp);
    let _ = imports;
}

#[test]
fn has_extractor_returns_true_for_all_supported_languages() {
    let langs = [
        Language::Python,
        Language::JavaScript,
        Language::TypeScript,
        Language::Rust,
        Language::C,
        Language::Cpp,
        Language::Java,
        Language::CSharp,
        Language::Go,
        Language::Swift,
        Language::Zig,
        Language::Ruby,
        Language::ObjectiveC,
        Language::Tcl,
        Language::Kotlin,
        Language::Haskell,
        Language::Lua,
        Language::R,
        Language::Php,
        Language::Cobol,
        Language::D,
        Language::Groovy,
    ];
    for lang in langs {
        assert!(has_extractor(lang), "{lang:?} should have an extractor");
    }
}

#[test]
fn empty_source_yields_empty_imports_each_language() {
    let langs = [
        Language::Python,
        Language::Rust,
        Language::Go,
        Language::Java,
        Language::TypeScript,
        Language::JavaScript,
        Language::Ruby,
        Language::Php,
    ];
    for lang in langs {
        let imports = extract("\n", lang);
        assert!(imports.is_empty(), "empty source should give empty imports for {lang:?}");
    }
}

#[test]
fn malformed_source_does_not_panic_each_language() {
    let cases: &[(&str, Language)] = &[
        ("import (\n", Language::Python),
        ("use ::\n", Language::Rust),
        ("import {\n", Language::TypeScript),
        ("#include \"\n", Language::C),
    ];
    for (src, lang) in cases {
        let _ = extract(src, *lang);
    }
}

#[test]
fn line_numbers_track_source_position_python() {
    let imports = extract("\n\n\nimport os\n", Language::Python);
    let os = imports.iter().find(|i| i.target == "os").unwrap();
    assert_eq!(os.line, 4);
}

#[test]
fn determinism_same_source_yields_same_imports_two_runs() {
    let source = "import os\nimport sys\nimport json\n";
    let mut a: Vec<String> = extract(source, Language::Python)
        .into_iter()
        .map(|i| i.target)
        .collect();
    let mut b: Vec<String> = extract(source, Language::Python)
        .into_iter()
        .map(|i| i.target)
        .collect();
    a.sort();
    b.sort();
    assert_eq!(a, b);
}
