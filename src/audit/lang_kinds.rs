use crate::parse::Language;

const SKIPPABLE_TABLE: [&[&str]; 22] =
    [&[], &[], &[], &[], &[], &[], &[], &[], &[], &[], &[], &[], &[], &[], &[], &[], &[], &[], &[], &[], &[], &[]];

const IDENTIFIER_TABLE: [&[&str]; 22] = [
    // Python
    &["identifier"],
    // TypeScript
    &["identifier", "property_identifier", "shorthand_property_identifier", "type_identifier"],
    // JavaScript
    &["identifier", "property_identifier", "shorthand_property_identifier"],
    // Rust
    &["identifier", "type_identifier", "field_identifier", "shorthand_field_identifier"],
    // C
    &["identifier", "field_identifier", "type_identifier"],
    // Cpp
    &["identifier", "field_identifier", "type_identifier", "namespace_identifier"],
    // Java
    &["identifier", "type_identifier"],
    // CSharp
    &["identifier"],
    // Go
    &["identifier", "field_identifier", "type_identifier", "package_identifier"],
    // Swift
    &["simple_identifier"],
    // Zig
    &["identifier"],
    // Ruby
    &["identifier", "constant", "instance_variable", "class_variable", "global_variable"],
    // ObjectiveC
    &["identifier", "field_identifier"],
    // Tcl
    &["simple_word"],
    // Kotlin
    &["simple_identifier", "type_identifier"],
    // Haskell
    &["variable", "constructor"],
    // Lua
    &["identifier"],
    // R
    &["identifier"],
    // Php
    &["name", "variable_name"],
    // Cobol
    &["WORD", "_word"],
    // D
    &["identifier"],
    // Groovy
    &["identifier"],
];

pub fn skippable_root_kinds(lang: Language) -> &'static [&'static str] {
    SKIPPABLE_TABLE[lang as usize]
}

pub fn is_skippable_root(lang: Language, kind: &str) -> bool {
    skippable_root_kinds(lang).contains(&kind)
}

pub fn identifier_kinds(lang: Language) -> &'static [&'static str] {
    IDENTIFIER_TABLE[lang as usize]
}

pub fn is_identifier_kind(lang: Language, kind: &str) -> bool {
    identifier_kinds(lang).contains(&kind)
}
