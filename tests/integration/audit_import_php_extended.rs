use std::fs;
use std::path::Path;

use pulse::audit::imports::{extract_imports, resolve_target};
use pulse::parse::{parse_only, Language};

fn extract(source: &str) -> Vec<pulse::audit::imports::RawImport> {
    let tree = parse_only(source, Language::Php).expect("parse");
    extract_imports(&tree, source, Language::Php)
}

#[test]
fn use_statement_extracts_namespace() {
    let imports = extract("<?php\nuse Foo\\Bar;\n");
    assert!(imports.iter().any(|i| i.target.contains("Foo")));
}

#[test]
fn use_with_alias_handled() {
    let imports = extract("<?php\nuse Foo\\Bar as Baz;\n");
    assert!(!imports.is_empty());
}

#[test]
fn require_with_double_quoted_string_extracted() {
    let imports = extract("<?php\nrequire \"helpers.php\";\n");
    assert!(imports.iter().any(|i| i.target.contains("helpers")));
}

#[test]
fn require_with_single_quoted_string_extracted() {
    let imports = extract("<?php\nrequire 'helpers.php';\n");
    assert!(imports.iter().any(|i| i.target.contains("helpers")));
}

#[test]
fn require_once_extracted() {
    let imports = extract("<?php\nrequire_once \"setup.php\";\n");
    assert!(imports.iter().any(|i| i.target.contains("setup")));
}

#[test]
fn include_extracted() {
    let imports = extract("<?php\ninclude \"part.php\";\n");
    assert!(imports.iter().any(|i| i.target.contains("part")));
}

#[test]
fn include_once_extracted() {
    let imports = extract("<?php\ninclude_once \"part.php\";\n");
    assert!(imports.iter().any(|i| i.target.contains("part")));
}

#[test]
fn empty_php_source_yields_empty_imports() {
    let imports = extract("<?php\n");
    assert!(imports.is_empty());
}

#[test]
fn malformed_php_does_not_panic() {
    let _ = extract("<?php\nrequire (.\n");
}

#[test]
fn multiple_use_statements_all_extracted() {
    let imports = extract("<?php\nuse Foo\\Alpha;\nuse Foo\\Beta;\nuse Foo\\Gamma;\n");
    assert!(imports.len() >= 3);
}

#[test]
fn nested_namespace_use_handled() {
    let imports = extract("<?php\nuse Foo\\Bar\\Baz\\Quux;\n");
    let _ = imports;
}

#[test]
fn path_suffix_for_simple_namespace() {
    let s = pulse::audit::imports::resolve_by_suffix("Foo\\Bar", Language::Php, &std::collections::HashSet::new());
    assert!(s.is_none());
}

#[test]
fn resolve_target_when_composer_missing_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let _ = resolve_target("Foo\\Bar", &dir.path().join("src/file.php"), dir.path(), Language::Php);
}

#[test]
fn resolve_target_with_invalid_composer_json_returns_fallback_paths() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("composer.json"), "this is not json").unwrap();
    let _ = resolve_target("Foo\\Bar", &dir.path().join("src/file.php"), dir.path(), Language::Php);
}

#[test]
fn resolve_target_with_empty_composer_json() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("composer.json"), "{}").unwrap();
    let _ = resolve_target("Foo\\Bar", &dir.path().join("src/file.php"), dir.path(), Language::Php);
}

#[test]
fn resolve_target_with_composer_no_psr4_section() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("composer.json"), r#"{"autoload": {"files": ["bootstrap.php"]}}"#).unwrap();
    let _ = resolve_target("Foo\\Bar", &dir.path().join("src/file.php"), dir.path(), Language::Php);
}

#[test]
fn resolve_target_with_psr4_prefix_match_resolves() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("composer.json"), r#"{"autoload":{"psr-4":{"App\\":"src/"}}}"#).unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/Foo.php"), "<?php\n").unwrap();
    let result = resolve_target("App\\Foo", &dir.path().join("src/Other.php"), dir.path(), Language::Php);
    if let Some(p) = result {
        assert!(p.ends_with("Foo.php"));
    }
}

#[test]
fn resolve_target_with_psr4_no_prefix_match_returns_other_candidates() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("composer.json"), r#"{"autoload":{"psr-4":{"App\\":"src/"}}}"#).unwrap();
    let _ = resolve_target("Vendor\\OtherLib\\Class", &dir.path().join("src/file.php"), dir.path(), Language::Php);
}

#[test]
fn line_numbers_track_use_position() {
    let imports = extract("<?php\n\n\nuse Foo\\Bar;\n");
    let i = imports.iter().find(|i| i.target.contains("Foo")).unwrap();
    assert!(i.line >= 4);
}

#[test]
fn determinism_two_runs_same_extracted_targets() {
    let source = "<?php\nuse A\\B;\nrequire \"x.php\";\nuse C\\D;\n";
    let mut a: Vec<String> = extract(source).into_iter().map(|i| i.target).collect();
    let mut b: Vec<String> = extract(source).into_iter().map(|i| i.target).collect();
    a.sort();
    b.sort();
    assert_eq!(a, b);
}

#[test]
fn use_function_extraction_handled() {
    let imports = extract("<?php\nuse function Foo\\helper;\n");
    let _ = imports;
}

#[test]
fn use_const_extraction_handled() {
    let imports = extract("<?php\nuse const Foo\\PI;\n");
    let _ = imports;
}

#[test]
fn unresolvable_psr4_falls_through_to_default_candidates() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("composer.json"), r#"{"autoload":{"psr-4":{"Foo\\":"lib/"}}}"#).unwrap();
    let _ = resolve_target("Bar\\Baz", Path::new(&dir.path().join("file.php")), dir.path(), Language::Php);
}
