mod common;

use common::*;
use std::process::Command;

const LANG: &str = "python";

// ===========================================================================
// FUTURE: Low Cohesion (LCOM4)
// ===========================================================================

#[test]
#[ignore = "future: LCOM4 cohesion not yet implemented"]
fn low_cohesion_detected() {
    let output = run_check(LANG, "low_cohesion.py");
    assert!(has_smell(&output, "Low Cohesion"), "should detect low cohesion in KitchenSink");
}

#[test]
#[ignore = "future: LCOM4 cohesion not yet implemented"]
fn low_cohesion_identifies_disconnected_clusters() {
    let debug = run_debug(LANG, "low_cohesion.py");
    // KitchenSink has 4 clusters: users, orders, logs, config
    assert!(
        debug.contains("lcom4=4") || debug.contains("clusters=4"),
        "should identify 4 disconnected method clusters"
    );
}

#[test]
#[ignore = "future: LCOM4 cohesion not yet implemented"]
fn potentially_low_cohesion_near_threshold() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("borderline.py");
    std::fs::write(&path, r#"
class Service:
    def __init__(self):
        self.data = []
        self.cache = {}

    def add(self, item):
        self.data.append(item)

    def get(self, key):
        return self.cache.get(key)

    def process(self):
        for d in self.data:
            self.cache[d.id] = d
"#).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!has_smell(&stdout, "Low Cohesion"), "bridged clusters should not trigger");
}

// ===========================================================================
// FUTURE: Code Duplication
// ===========================================================================

#[test]
#[ignore = "future: code duplication not yet implemented"]
fn code_duplication_detected() {
    let output = run_check(LANG, "code_duplication.py");
    assert!(has_smell(&output, "Code Duplication"), "should detect duplicated report functions");
}

#[test]
#[ignore = "future: code duplication not yet implemented"]
fn code_duplication_identifies_all_clones() {
    let output = run_check(LANG, "code_duplication.py");
    assert!(has_function(&output, "process_user_report"));
    assert!(has_function(&output, "process_admin_report"));
    assert!(has_function(&output, "process_vendor_report"));
}

#[test]
#[ignore = "future: code duplication not yet implemented"]
fn non_duplicate_similar_functions_not_flagged() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("similar.py");
    std::fs::write(&path, r#"
def process_users(users):
    return [{"id": u.id, "name": u.name, "type": "user"} for u in users]

def process_orders(orders):
    return [{"id": o.id, "total": o.total, "type": "order"} for o in orders]
"#).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!has_smell(&stdout, "Code Duplication"));
}

// ===========================================================================
// FUTURE: Primitive Obsession
// ===========================================================================

#[test]
#[ignore = "future: primitive obsession not yet implemented"]
fn primitive_obsession_detected() {
    let output = run_check(LANG, "primitive_obsession.py");
    assert!(
        has_smell(&output, "Primitive Obsession") || has_smell(&output, "String Heavy"),
        "should detect primitive obsession in create_invoice"
    );
}

#[test]
#[ignore = "future: primitive obsession not yet implemented"]
fn domain_typed_params_not_flagged() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("typed.py");
    std::fs::write(&path, r#"
from dataclasses import dataclass

@dataclass
class Address:
    street: str
    city: str

def create_user(name: str, address: Address):
    pass
"#).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!has_smell(&stdout, "Primitive Obsession"));
}

// ===========================================================================
// FUTURE: Lines of Declarations
// ===========================================================================

#[test]
#[ignore = "future: declaration count not yet implemented"]
fn excessive_declarations_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("many_types.py");
    let mut content = String::new();
    for i in 0..30 {
        content.push_str(&format!("class Type{}:\n    pass\n\n", i));
    }
    std::fs::write(&path, content).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(has_smell(&stdout, "Declarations"));
}

// ===========================================================================
// FUTURE: Overall Function Size (aggregate)
// ===========================================================================

#[test]
#[ignore = "future: overall function size aggregate not yet implemented"]
fn overall_function_size_pattern_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("many_large.py");
    let mut content = String::new();
    for i in 0..5 {
        content.push_str(&format!("def func_{}():\n", i));
        for j in 0..45 {
            content.push_str(&format!("    x_{} = {}\n", j, j));
        }
        content.push_str("    return None\n\n");
    }
    std::fs::write(&path, content).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(has_smell(&stdout, "Overall Function Size"));
}

// ===========================================================================
// FUTURE: Large Assertion Blocks (test-specific)
// ===========================================================================

#[test]
#[ignore = "future: test-specific smells not yet implemented"]
fn large_assertion_block_detected() {
    let output = run_check(LANG, "test_smells.py");
    assert!(has_smell(&output, "Large Assertion Block"));
}

#[test]
#[ignore = "future: test-specific smells not yet implemented"]
fn duplicated_assertion_blocks_detected() {
    let output = run_check(LANG, "test_smells.py");
    assert!(has_smell(&output, "Duplicated Assertion"));
}

#[test]
#[ignore = "future: test-specific smells not yet implemented"]
fn small_assertion_block_not_flagged() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("small_test.py");
    std::fs::write(&path, "def test_simple():\n    assert 1 + 1 == 2\n    assert True\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!has_smell(&stdout, "Assertion Block"));
}

// ===========================================================================
// FUTURE: Missing Argument Abstractions
// ===========================================================================

#[test]
#[ignore = "future: missing argument abstractions not yet implemented"]
fn missing_argument_abstractions_detected() {
    let output = run_check(LANG, "primitive_obsession.py");
    assert!(has_smell(&output, "Missing Argument"));
}

// ===========================================================================
// FUTURE: Deep Global Nested Complexity
// ===========================================================================

#[test]
#[ignore = "future: deep global nesting threshold tuning"]
fn deep_global_nesting_detected() {
    let output = run_check(LANG, "global_conditionals.py");
    assert!(has_smell(&output, "Deep Global Nesting"));
}
