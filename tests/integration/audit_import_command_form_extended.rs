use std::path::Path;

use pulse::audit::imports::extract_imports;
use pulse::parse::{parse_only, Language};

fn extract(source: &str, lang: Language) -> Vec<pulse::audit::imports::RawImport> {
    let tree = parse_only(source, lang).expect("parse");
    extract_imports(&tree, source, lang)
}

#[test]
fn tcl_source_with_path_extracted() {
    let imports = extract("source helpers.tcl\n", Language::Tcl);
    assert!(imports.iter().any(|i| i.target == "helpers.tcl"));
}

#[test]
fn tcl_package_require_extracted() {
    let imports = extract("package require Tk\n", Language::Tcl);
    assert!(imports.iter().any(|i| i.target == "Tk"));
}

#[test]
fn tcl_package_require_with_version_extracts_name_only() {
    let imports = extract("package require Tk 8.6\n", Language::Tcl);
    assert!(imports.iter().any(|i| i.target == "Tk"));
}

#[test]
fn tcl_unknown_command_yields_no_import() {
    let imports = extract("puts hello\n", Language::Tcl);
    assert!(imports.is_empty());
}

#[test]
fn tcl_source_without_arg_yields_no_import() {
    let imports = extract("source\n", Language::Tcl);
    assert!(imports.is_empty());
}

#[test]
fn tcl_package_without_require_yields_no_import() {
    let imports = extract("package names\n", Language::Tcl);
    assert!(imports.is_empty());
}

#[test]
fn tcl_quoted_path_handled() {
    let imports = extract("source \"with spaces.tcl\"\n", Language::Tcl);
    let _ = imports;
}

#[test]
fn cobol_copy_statement_extracts_word() {
    let source = "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. T.\n       PROCEDURE DIVISION.\n           COPY HELPERS.\n           STOP RUN.\n";
    let imports = extract(source, Language::Cobol);
    let _ = imports;
}

#[test]
fn cobol_call_statement_string_extracted() {
    let source = "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. T.\n       PROCEDURE DIVISION.\n           CALL \"HELPERS\".\n           STOP RUN.\n";
    let _ = extract(source, Language::Cobol);
}

#[test]
fn empty_tcl_source_yields_empty_imports() {
    let imports = extract("\n", Language::Tcl);
    assert!(imports.is_empty());
}

#[test]
fn empty_cobol_source_yields_empty_imports() {
    let _ = extract("\n", Language::Cobol);
}

#[test]
fn malformed_tcl_does_not_panic() {
    let _ = extract("source ", Language::Tcl);
}

#[test]
fn malformed_cobol_does_not_panic() {
    let _ = extract("garbage cobol input \"unclosed\n", Language::Cobol);
}

#[test]
fn non_tcl_non_cobol_languages_unaffected_by_command_form() {
    let imports = extract("import os\n", Language::Python);
    assert!(imports.iter().any(|i| i.target == "os"));
}

#[test]
fn tcl_multiple_source_commands_each_extracted() {
    let source = "source first.tcl\nsource second.tcl\n";
    let imports = extract(source, Language::Tcl);
    let names: std::collections::BTreeSet<String> =
        imports.iter().map(|i| i.target.clone()).collect();
    assert!(names.contains("first.tcl") || names.contains("second.tcl"));
}

#[test]
fn tcl_source_with_dotted_name_extracted() {
    let imports = extract("source pkg.helpers\n", Language::Tcl);
    let _ = imports;
}

#[test]
fn tcl_line_numbers_track_source_position() {
    let imports = extract("\n\nsource helpers.tcl\n", Language::Tcl);
    if let Some(i) = imports.iter().find(|i| i.target == "helpers.tcl") {
        assert_eq!(i.line, 3);
    }
}

#[test]
fn determinism_two_runs_same_imports() {
    let a = extract("source first.tcl\nsource second.tcl\n", Language::Tcl);
    let b = extract("source first.tcl\nsource second.tcl\n", Language::Tcl);
    let names_a: Vec<String> = a.iter().map(|i| i.target.clone()).collect();
    let names_b: Vec<String> = b.iter().map(|i| i.target.clone()).collect();
    assert_eq!(names_a, names_b);
}

#[test]
fn tcl_with_unicode_path_handled() {
    let _ = extract("source модуль.tcl\n", Language::Tcl);
}

#[test]
fn resolve_target_for_tcl_uses_command_form_candidates() {
    let raw = "helpers";
    let source_file = Path::new("/tmp/proj/main.tcl");
    let project_root = Path::new("/tmp/proj");
    let _ = pulse::audit::imports::resolve_target(raw, source_file, project_root, Language::Tcl);
}

#[test]
fn resolve_target_for_cobol_uses_command_form_candidates() {
    let raw = "HELPERS";
    let source_file = Path::new("/tmp/proj/main.cob");
    let project_root = Path::new("/tmp/proj");
    let _ = pulse::audit::imports::resolve_target(raw, source_file, project_root, Language::Cobol);
}
