use pulse::langkinds::{block_kinds, control_flow_kinds, function_kinds};
use pulse::parse::Language;

#[test]
fn every_language_returns_a_slice_without_panicking() {
    for lang in Language::ALL {
        let _ = function_kinds(lang);
        let _ = control_flow_kinds(lang);
        let _ = block_kinds(lang);
    }
}

#[test]
fn legacy_extract_control_flow_kinds_are_unchanged() {
    assert_eq!(
        control_flow_kinds(Language::Rust),
        ["if_expression", "for_expression", "while_expression", "loop_expression", "match_expression"].as_slice()
    );
    assert_eq!(
        control_flow_kinds(Language::Python),
        ["if_statement", "for_statement", "while_statement", "with_statement"].as_slice()
    );
    assert_eq!(
        control_flow_kinds(Language::TypeScript),
        ["if_statement", "for_statement", "for_in_statement", "while_statement", "switch_statement"].as_slice()
    );
}

#[test]
fn legacy_refmine_block_kinds_are_unchanged() {
    assert_eq!(block_kinds(Language::Rust), ["block"].as_slice(), "rust stays a single block kind, not match_block");
    assert_eq!(block_kinds(Language::Python), ["block"].as_slice());
    assert_eq!(block_kinds(Language::TypeScript), ["statement_block"].as_slice());
}

#[test]
fn legacy_naturalness_function_kinds_are_unchanged() {
    assert_eq!(function_kinds(Language::Python), ["function_definition"].as_slice());
    assert_eq!(function_kinds(Language::Rust), ["function_item"].as_slice());
}

#[test]
fn newly_covered_languages_have_kind_tables() {
    for lang in [Language::Go, Language::Java, Language::CSharp, Language::Php] {
        assert!(!function_kinds(lang).is_empty(), "{lang:?} needs function kinds");
        assert!(!control_flow_kinds(lang).is_empty(), "{lang:?} needs control-flow kinds");
        assert!(!block_kinds(lang).is_empty(), "{lang:?} needs a block kind");
    }
}
