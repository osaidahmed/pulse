use pulse::audit::definitions::definitions_from;
use pulse::audit::method_vocab::method_vocab_from;
use pulse::parse::Language;

use crate::binding_common::one_source;

#[test]
fn vocab_keys_align_with_definition_lines_and_split_camel_case() {
    let src = "class Invoice {\n  void computeTotalAmount() {\n    int lineItemPrice = unitPrice * quantity;\n  }\n  void renderHtmlTemplate() {\n    String markupOutput = buildDomNode();\n  }\n}\n";
    let (_d, corpus) = one_source(src, "Invoice.java", Language::Java);
    let file = corpus.files.first().unwrap();
    let vocab = method_vocab_from(file);
    let defs = definitions_from(file);

    let methods: Vec<_> = defs.iter().filter(|d| d.identity.class.is_some() && !d.is_constructor).collect();
    assert!(methods.len() >= 2, "expected two class methods: {defs:?}");
    for m in &methods {
        let key = (m.identity.file.clone(), m.identity.line);
        let tokens = vocab.get(&key).unwrap_or_else(|| panic!("no vocab keyed to {} at {key:?}", m.identity.name));
        assert!(!tokens.is_empty(), "method {} has identifier tokens", m.identity.name);
    }

    let all: Vec<&str> = vocab.values().flatten().map(String::as_str).collect();
    assert!(all.contains(&"total"), "camelCase split produced `total`: {all:?}");
    assert!(all.contains(&"amount"), "camelCase split produced `amount`: {all:?}");
    assert!(all.contains(&"template"), "second method vocabulary present: {all:?}");
    assert!(!all.contains(&"int"), "short tokens below the min length are dropped: {all:?}");
}

#[test]
fn vocab_is_empty_for_a_file_without_methods() {
    let (_d, corpus) = one_source("x = 1\ny = 2\n", "mod.py", Language::Python);
    let vocab = method_vocab_from(corpus.files.first().unwrap());
    assert!(vocab.is_empty(), "top-level statements are not methods: {vocab:?}");
}
