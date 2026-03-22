mod common;

use common::*;
use std::process::Command;

const LANG: &str = "python";

// ===========================================================================
// Low Cohesion (LCOM4)
// ===========================================================================

#[test]
fn low_cohesion_detected() {
    let output = run_check(LANG, "low_cohesion.py");
    assert!(
        has_smell(&output, "Low Cohesion"),
        "should detect low cohesion in KitchenSink, got: {}",
        output
    );
}

#[test]
fn low_cohesion_reports_cluster_count() {
    let output = run_check(LANG, "low_cohesion.py");
    assert!(
        output.contains("LCOM4=4"),
        "should report 4 disconnected clusters, got: {}",
        output
    );
}

#[test]
fn cohesive_class_not_flagged() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cohesive.py");
    std::fs::write(
        &path,
        r#"
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
"#,
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        !has_smell(&stdout, "Low Cohesion"),
        "bridged clusters should not trigger, got: {}",
        stdout
    );
}

// ===========================================================================
// Code Duplication (implemented)
// ===========================================================================

#[test]
fn code_duplication_detected() {
    let output = run_check(LANG, "code_duplication.py");
    assert!(
        has_smell(&output, "Code Duplication"),
        "should detect duplicated report functions, got: {}",
        output
    );
}

#[test]
fn code_duplication_identifies_all_clones() {
    let output = run_check(LANG, "code_duplication.py");
    assert!(has_function(&output, "process_user_report"));
    assert!(has_function(&output, "process_admin_report"));
    assert!(has_function(&output, "process_vendor_report"));
}

#[test]
fn non_duplicate_similar_functions_not_flagged() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("similar.py");
    std::fs::write(
        &path,
        r#"
def process_users(users):
    return [{"id": u.id, "name": u.name, "type": "user"} for u in users]

def process_orders(orders):
    return [{"id": o.id, "total": o.total, "type": "order"} for o in orders]
"#,
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!has_smell(&stdout, "Code Duplication"));
}

#[test]
fn test_file_duplication_not_flagged() {
    let output = run_check(LANG, "test_smells.py");
    assert!(
        !has_smell(&output, "Code Duplication"),
        "test methods with identical structure should NOT be flagged as code duplication, got: {}",
        output
    );
}

#[test]
fn small_duplicate_functions_not_flagged() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("small_dupes.py");
    std::fs::write(
        &path,
        r#"
def get_x(obj):
    return obj.x

def get_y(obj):
    return obj.y

def get_z(obj):
    return obj.z
"#,
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        !has_smell(&stdout, "Code Duplication"),
        "3-line duplicate functions should be below min_loc threshold, got: {}",
        stdout
    );
}

// ===========================================================================
// Primitive Obsession
// ===========================================================================

#[test]
fn primitive_obsession_detected() {
    let output = run_check(LANG, "primitive_obsession.py");
    assert!(
        has_smell(&output, "Primitive Obsession"),
        "should detect primitive obsession in create_invoice, got: {}",
        output
    );
}

#[test]
fn domain_typed_params_not_flagged() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("typed.py");
    std::fs::write(
        &path,
        r#"
from dataclasses import dataclass

@dataclass
class Address:
    street: str
    city: str

def create_user(name: str, address: Address):
    pass
"#,
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        !has_smell(&stdout, "Primitive Obsession"),
        "domain-typed params should not trigger, got: {}",
        stdout
    );
}

// ===========================================================================
// Excessive Declarations
// ===========================================================================

#[test]
fn excessive_declarations_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("many_types.py");
    let mut content = String::new();
    for i in 0..25 {
        content.push_str(&format!("class Type{}:\n    pass\n\n", i));
    }
    std::fs::write(&path, content).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        has_smell(&stdout, "Declarations"),
        "25 classes in one file should be flagged, got: {}",
        stdout
    );
}

// ===========================================================================
// Overall Function Size (aggregate)
// ===========================================================================

#[test]
fn overall_function_size_pattern_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("many_large.py");
    let mut content = String::new();
    for i in 0..5 {
        content.push_str(&format!("def func_{}():\n", i));
        for j in 0..55 {
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
    assert!(
        has_smell(&stdout, "Overall Function Size"),
        "pattern of large functions should be flagged, got: {}",
        stdout
    );
}

// ===========================================================================
// Large Assertion Blocks (test-specific)
// ===========================================================================

#[test]
fn large_assertion_block_detected() {
    let output = run_check(LANG, "test_smells.py");
    assert!(
        has_smell(&output, "Large Assertion Block"),
        "should detect large assertion blocks, got: {}",
        output
    );
}

#[test]
fn duplicated_assertion_blocks_detected() {
    let output = run_check(LANG, "test_smells.py");
    assert!(
        has_smell(&output, "Duplicated Assertion"),
        "should detect duplicated assertion blocks, got: {}",
        output
    );
}

#[test]
fn small_assertion_block_not_flagged() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("small_test.py");
    std::fs::write(
        &path,
        "def test_simple():\n    assert 1 + 1 == 2\n    assert True\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        !has_smell(&stdout, "Assertion Block"),
        "small assertion groups should not trigger, got: {}",
        stdout
    );
}

// ===========================================================================
// Deep Global Nested Complexity
// ===========================================================================

#[test]
fn deep_global_nesting_detected() {
    let output = run_check(LANG, "global_conditionals.py");
    assert!(
        has_smell(&output, "Deep Global Nesting"),
        "should detect deep nesting at module scope, got: {}",
        output
    );
}

// ===========================================================================
// Missing Argument Abstractions — deferred, requires cross-function analysis
// ===========================================================================

#[test]
#[ignore = "future: missing argument abstractions requires cross-function parameter group analysis"]
fn missing_argument_abstractions_detected() {
    let output = run_check(LANG, "primitive_obsession.py");
    assert!(has_smell(&output, "Missing Argument"));
}
