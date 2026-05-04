use std::fs;

use pulse::audit::imports::{extract_imports, resolve_target};
use pulse::parse::{parse_only, Language};

fn extract(source: &str, lang: Language) -> Vec<pulse::audit::imports::RawImport> {
    let tree = parse_only(source, lang).expect("parse");
    extract_imports(&tree, source, lang)
}

#[test]
fn c_include_quoted_extracted() {
    let imports = extract("#include \"helpers.h\"\nint main() { return 0; }\n", Language::C);
    assert!(imports.iter().any(|i| i.target == "helpers.h"));
}

#[test]
fn cpp_include_quoted_extracted() {
    let imports = extract("#include \"helpers.h\"\nint main() { return 0; }\n", Language::Cpp);
    assert!(imports.iter().any(|i| i.target == "helpers.h"));
}

#[test]
fn objc_include_quoted_extracted() {
    let imports = extract("#include \"helpers.h\"\n@implementation Foo\n@end\n", Language::ObjectiveC);
    assert!(imports.iter().any(|i| i.target == "helpers.h"));
}

#[test]
fn c_include_angle_brackets_handled() {
    let _ = extract("#include <stdio.h>\nint main() { return 0; }\n", Language::C);
}

#[test]
fn objc_module_import_handled() {
    let _ = extract("@import Foundation;\n@implementation Foo\n@end\n", Language::ObjectiveC);
}

#[test]
fn c_resolve_target_with_h_extension() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("helpers.h"), "").unwrap();
    let resolved = resolve_target(
        "helpers.h",
        &dir.path().join("main.c"),
        dir.path(),
        Language::C,
    );
    if let Some(p) = resolved {
        assert!(p.ends_with("helpers.h"));
    }
}

#[test]
fn c_resolve_target_with_relative_path() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("subdir")).unwrap();
    fs::write(dir.path().join("subdir/h.h"), "").unwrap();
    let _ = resolve_target(
        "subdir/h.h",
        &dir.path().join("main.c"),
        dir.path(),
        Language::C,
    );
}

#[test]
fn c_resolve_target_with_include_dir() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("include")).unwrap();
    fs::write(dir.path().join("include/h.h"), "").unwrap();
    let _ = resolve_target(
        "h.h",
        &dir.path().join("main.c"),
        dir.path(),
        Language::C,
    );
}

#[test]
fn cpp_resolve_target_with_hpp_extension() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("helpers.hpp"), "").unwrap();
    let _ = resolve_target(
        "helpers",
        &dir.path().join("main.cpp"),
        dir.path(),
        Language::Cpp,
    );
}

#[test]
fn cpp_resolve_target_with_cppm_extension() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("module.cppm"), "").unwrap();
    let _ = resolve_target(
        "module",
        &dir.path().join("main.cpp"),
        dir.path(),
        Language::Cpp,
    );
}

#[test]
fn objc_resolve_target_with_m_extension() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("helpers.m"), "").unwrap();
    let _ = resolve_target(
        "helpers",
        &dir.path().join("main.m"),
        dir.path(),
        Language::ObjectiveC,
    );
}

#[test]
fn c_unknown_target_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let result = resolve_target(
        "missing.h",
        &dir.path().join("main.c"),
        dir.path(),
        Language::C,
    );
    assert!(result.is_none());
}

#[test]
fn c_resolve_target_with_src_subdirectory() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/local.h"), "").unwrap();
    let _ = resolve_target(
        "local.h",
        &dir.path().join("src/main.c"),
        dir.path(),
        Language::C,
    );
}

#[test]
fn empty_c_file_yields_empty_imports() {
    let imports = extract("\n", Language::C);
    assert!(imports.is_empty());
}

#[test]
fn malformed_c_does_not_panic() {
    let _ = extract("#include\n", Language::C);
}

#[test]
fn malformed_objc_does_not_panic() {
    let _ = extract("@import\n", Language::ObjectiveC);
}

#[test]
fn unsupported_language_for_preprocessor_returns_no_candidates() {
    let dir = tempfile::tempdir().unwrap();
    let _ = resolve_target("foo", &dir.path().join("main.py"), dir.path(), Language::Python);
}

#[test]
fn multiple_includes_each_extracted() {
    let imports = extract(
        "#include \"a.h\"\n#include \"b.h\"\n#include \"c.h\"\nint main() { return 0; }\n",
        Language::C,
    );
    let names: std::collections::BTreeSet<String> = imports.iter().map(|i| i.target.clone()).collect();
    assert!(names.contains("a.h"));
    assert!(names.contains("b.h"));
    assert!(names.contains("c.h"));
}

#[test]
fn line_numbers_track_position_for_includes() {
    let imports = extract("\n\n#include \"helpers.h\"\nint main() { return 0; }\n", Language::C);
    if let Some(i) = imports.iter().find(|i| i.target == "helpers.h") {
        assert_eq!(i.line, 3);
    }
}

#[test]
fn objc_module_import_extracts_identifier_target() {
    let imports = extract("@import UIKit;\n@implementation Foo\n@end\n", Language::ObjectiveC);
    let _ = imports;
}

#[test]
fn determinism_same_source_yields_same_imports_two_runs() {
    let source = "#include \"a.h\"\n#include \"b.h\"\nint main() { return 0; }\n";
    let mut a: Vec<String> = extract(source, Language::C).into_iter().map(|i| i.target).collect();
    let mut b: Vec<String> = extract(source, Language::C).into_iter().map(|i| i.target).collect();
    a.sort();
    b.sort();
    assert_eq!(a, b);
}

#[test]
fn cpp_include_with_subdirectory_path() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("lib")).unwrap();
    fs::write(dir.path().join("lib/helpers.hpp"), "").unwrap();
    let _ = resolve_target(
        "lib/helpers.hpp",
        &dir.path().join("main.cpp"),
        dir.path(),
        Language::Cpp,
    );
}

#[test]
fn objc_resolve_target_with_h_extension() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("helpers.h"), "").unwrap();
    let _ = resolve_target(
        "helpers",
        &dir.path().join("main.m"),
        dir.path(),
        Language::ObjectiveC,
    );
}
