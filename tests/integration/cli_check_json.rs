use crate::common::*;
use std::process::Command;

fn check_json(path: &str) -> (String, bool) {
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", "--json", path])
        .output()
        .expect("failed to run pulse");
    (String::from_utf8(out.stdout).unwrap(), out.status.success())
}

#[test]
fn check_json_emits_a_structured_array_of_findings() {
    let path = fixtures_dir("rust").join("production_api_service.rs");
    let (stdout, success) = check_json(path.to_str().unwrap());
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("stdout must be valid json");
    let arr = v.as_array().expect("top-level value must be an array");
    assert!(!arr.is_empty(), "expected findings in the production fixture, got {stdout}");
    for item in arr {
        assert!(item.get("file").and_then(serde_json::Value::as_str).is_some());
        assert!(item.get("smell").and_then(serde_json::Value::as_str).is_some());
        assert!(item.get("scope").and_then(serde_json::Value::as_str).is_some());
        assert!(item.get("detail").is_some());
    }
    assert!(!success, "a file with findings should exit nonzero");
}

#[test]
fn check_json_on_clean_code_is_an_empty_array_and_exits_zero() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("clean.rs");
    std::fs::write(&path, "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n").unwrap();
    let (stdout, success) = check_json(path.to_str().unwrap());
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("valid json even when empty");
    assert!(v.as_array().unwrap().is_empty(), "clean code should yield [], got {stdout}");
    assert!(success, "clean code should exit zero");
}
