use pulse_audit::categorize::categorize;
use pulse_audit::finding::PatternCategory;

fn kinds(items: &[&str]) -> Vec<Box<str>> {
    items.iter().map(|s| Box::<str>::from(*s)).collect()
}

#[test]
fn categorize_subscript_root_with_two_chained_returns_chained_dict_access() {
    let ks = kinds(&["subscript", "subscript", "identifier"]);
    assert_eq!(categorize(&ks), PatternCategory::ChainedDictAccess);
}

#[test]
fn categorize_subscript_root_with_one_subscript_falls_back_to_primitive_obsession() {
    let ks = kinds(&["subscript", "identifier"]);
    assert_eq!(categorize(&ks), PatternCategory::PrimitiveObsession);
}

#[test]
fn categorize_subscript_root_with_zero_chained_subscripts_is_primitive_obsession() {
    let ks = kinds(&["subscript_expression", "identifier", "string"]);
    assert_eq!(categorize(&ks), PatternCategory::PrimitiveObsession);
}

#[test]
fn categorize_attribute_root_with_value_field_is_enum_value_access() {
    let ks = kinds(&["attribute", "identifier", "value"]);
    assert_eq!(categorize(&ks), PatternCategory::EnumValueAccess);
}

#[test]
fn categorize_attribute_root_without_value_field_is_attribute_chain() {
    let ks = kinds(&["attribute", "identifier", "name"]);
    assert_eq!(categorize(&ks), PatternCategory::AttributeChain);
}

#[test]
fn categorize_literal_root_yields_literal_repetition() {
    assert_eq!(categorize(&kinds(&["string", "identifier"])), PatternCategory::LiteralRepetition);
    assert_eq!(categorize(&kinds(&["integer"])), PatternCategory::LiteralRepetition);
}

#[test]
fn categorize_call_root_yields_method_call() {
    assert_eq!(categorize(&kinds(&["call", "identifier"])), PatternCategory::MethodCall);
    assert_eq!(categorize(&kinds(&["call_expression", "identifier"])), PatternCategory::MethodCall);
}

#[test]
fn categorize_dict_root_yields_dict_literal() {
    assert_eq!(categorize(&kinds(&["dictionary", "pair"])), PatternCategory::DictLiteral);
}

#[test]
fn categorize_list_root_yields_list_literal() {
    assert_eq!(categorize(&kinds(&["list", "integer"])), PatternCategory::ListLiteral);
}

#[test]
fn categorize_unknown_root_falls_through_to_other() {
    assert_eq!(categorize(&kinds(&["totally_unknown_kind"])), PatternCategory::Other);
}

#[test]
fn categorize_empty_kind_list_returns_other() {
    let empty: Vec<Box<str>> = Vec::new();
    assert_eq!(categorize(&empty), PatternCategory::Other);
}

#[test]
fn categorize_subscript_chain_at_two_is_boundary_for_chained() {
    let two = kinds(&["subscript", "subscript"]);
    assert_eq!(categorize(&two), PatternCategory::ChainedDictAccess);

    let one = kinds(&["subscript"]);
    assert_eq!(categorize(&one), PatternCategory::PrimitiveObsession);
}
