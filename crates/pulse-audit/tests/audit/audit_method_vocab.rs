use pulse_audit::definitions::definitions_from;
use pulse_audit::method_vocab::method_vocab_from;
use pulse_syntax::parse::Language;

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
fn ruby_singleton_methods_are_keyed_to_their_definition_lines() {
    let src = "class Reporting\n  def self.daily_summary(account_ledger)\n  end\n  def self.weekly_rollup(invoice_totals)\n  end\nend\n";
    let (_d, corpus) = one_source(src, "reporting.rb", Language::Ruby);
    let file = corpus.files.first().unwrap();
    let vocab = method_vocab_from(file);
    let defs = definitions_from(file);
    let methods: Vec<_> = defs.iter().filter(|d| d.identity.class.is_some()).collect();
    assert!(methods.len() >= 2, "two singleton methods extracted: {defs:?}");
    for m in &methods {
        assert!(
            vocab.contains_key(&(m.identity.file.clone(), m.identity.line)),
            "def self.{} vocab is keyed to its definition line",
            m.identity.name
        );
    }
}

#[test]
fn swift_initializers_are_keyed_to_their_definition_lines() {
    let src =
        "class Account {\n  init(ownerName: String) {}\n  init(accountNumber: Int) {}\n  func closeAccount() {}\n}\n";
    let (_d, corpus) = one_source(src, "Account.swift", Language::Swift);
    let file = corpus.files.first().unwrap();
    let vocab = method_vocab_from(file);
    let defs = definitions_from(file);
    let methods: Vec<_> = defs.iter().filter(|d| d.identity.class.is_some()).collect();
    assert!(methods.len() >= 2, "initializers and methods extracted: {defs:?}");
    for m in &methods {
        assert!(
            vocab.contains_key(&(m.identity.file.clone(), m.identity.line)),
            "method/init at line {} has a vocab key",
            m.identity.line
        );
    }
}

#[test]
fn vocab_is_empty_for_a_file_without_methods() {
    let (_d, corpus) = one_source("x = 1\ny = 2\n", "mod.py", Language::Python);
    let vocab = method_vocab_from(corpus.files.first().unwrap());
    assert!(vocab.is_empty(), "top-level statements are not methods: {vocab:?}");
}
