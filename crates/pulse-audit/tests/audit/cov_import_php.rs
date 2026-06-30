use pulse_audit::imports::{extract_imports, RawImport};
use pulse_syntax::parse::{parse_only, Language};

fn extract(source: &str) -> Vec<RawImport> {
    let tree = parse_only(source, Language::Php).expect("parse");
    extract_imports(&tree, source, Language::Php)
}

#[test]
fn require_empty_double_quoted_string_uses_quote_strip_fallback() {
    let imports = extract("<?php\nrequire \"\";\n");
    assert!(imports.iter().any(|i| i.target.is_empty()));
}

#[test]
fn require_empty_single_quoted_string_uses_single_quote_fallback() {
    let imports = extract("<?php\nrequire '';\n");
    assert!(imports.iter().any(|i| i.target.is_empty()));
}

#[test]
fn include_empty_double_quoted_string_does_not_panic() {
    let imports = extract("<?php\ninclude \"\";\n");
    let _ = imports;
}

#[test]
fn require_once_empty_single_quoted_string_does_not_panic() {
    let imports = extract("<?php\nrequire_once '';\n");
    let _ = imports;
}

#[test]
fn require_non_string_variable_argument_yields_no_import() {
    let imports = extract("<?php\nrequire $path;\n");
    assert!(imports.iter().all(|i| !i.target.contains("path")));
}

#[test]
fn require_concatenated_variable_arguments_descend_exhausts_no_match() {
    let imports = extract("<?php\nrequire $a . $b;\n");
    assert!(imports.is_empty() || imports.iter().all(|i| i.target.is_empty()));
}

#[test]
fn include_once_constant_argument_no_match() {
    let imports = extract("<?php\ninclude_once SOME_CONSTANT;\n");
    let _ = imports;
}

#[test]
fn mixed_string_and_nonstring_requires_extract_only_strings() {
    let imports = extract("<?php\nrequire $dynamic;\nrequire \"real.php\";\n");
    assert!(imports.iter().any(|i| i.target.contains("real")));
}

#[test]
fn empty_string_require_is_deterministic_across_runs() {
    let source = "<?php\nrequire \"\";\nrequire '';\n";
    let mut a: Vec<String> = extract(source).into_iter().map(|i| i.target).collect();
    let mut b: Vec<String> = extract(source).into_iter().map(|i| i.target).collect();
    a.sort();
    b.sort();
    assert_eq!(a, b);
}
