use pulse_audit::import;
use pulse_audit::imports::{extract_imports, RawImport};
use pulse_syntax::parse::{parse_only, Language};

fn extract(source: &str, lang: Language) -> Vec<RawImport> {
    let tree = parse_only(source, lang).expect("parse");
    extract_imports(&tree, source, lang)
}

#[test]
fn candidates_unsupported_language_returns_empty() {
    let dir = tempfile::tempdir().unwrap();
    let source_file = dir.path().join("main.rs");
    let result = import::preprocessor::candidates("foo", &source_file, dir.path(), Language::Rust);
    assert!(result.is_empty());
}

#[test]
fn candidates_python_language_returns_empty() {
    let dir = tempfile::tempdir().unwrap();
    let source_file = dir.path().join("main.py");
    let result = import::preprocessor::candidates("foo", &source_file, dir.path(), Language::Python);
    assert!(result.is_empty());
}

#[test]
fn c_empty_quoted_include_uses_strip_fallback() {
    let imports = extract("#include \"\"\nint main() { return 0; }\n", Language::C);
    assert!(imports.iter().any(|i| i.target.is_empty()));
}

#[test]
fn cpp_empty_quoted_include_does_not_panic() {
    let _ = extract("#include \"\"\nint main() { return 0; }\n", Language::Cpp);
}

#[test]
fn objc_empty_quoted_include_does_not_panic() {
    let _ = extract("#include \"\"\n@implementation Foo\n@end\n", Language::ObjectiveC);
}
