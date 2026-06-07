use std::io::Write;
use std::process::{Command, Stdio};

use serde_json::Value;

use crate::common::cc_branches;

fn run_hook_isolated(content: &str, name: &str) -> Value {
    let dir = tempfile::tempdir().unwrap();
    let baseline = tempfile::tempdir().unwrap();
    let path = dir.path().join(name);
    std::fs::write(&path, content).unwrap();
    let input = format!(r#"{{"tool_input":{{"file_path":"{}"}}}}"#, path.to_str().unwrap());
    let output = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["--hook"])
        .env("PULSE_BASELINE_DIR", baseline.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
            child.wait_with_output()
        })
        .expect("run pulse --hook");
    let stdout = String::from_utf8(output.stdout).unwrap();
    serde_json::from_str(&stdout).unwrap_or(Value::Null)
}

fn complex_fn(params: &str, guard_var: &str) -> String {
    let mut src = format!("def handle({params}):\n    x = 0\n");
    for i in 0..cc_branches() {
        src.push_str(&format!("    if {guard_var} > {i}:\n        x += 1\n"));
    }
    src.push_str("    return x\n");
    src
}

const PRIMITIVE_ONLY: &str =
    "def make(name: str, email: str, city: str, country: str):\n    return [name, email, city, country]\n";

#[test]
fn advisory_only_finding_emits_context_without_blocking() {
    let v = run_hook_isolated(PRIMITIVE_ONLY, "adv.py");
    assert!(v.get("decision").is_none(), "an advisory-only edit must not block: {v}");
    let ctx = v["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .expect("advisory finding surfaced as additionalContext");
    assert!(ctx.contains("advisory[pulse]"));
    assert!(ctx.to_lowercase().contains("primitive obsession"));
}

#[test]
fn blocking_finding_carries_advisory_context_separately() {
    let src = complex_fn("a: int, b: int, c: int, d: int", "a");
    let v = run_hook_isolated(&src, "both.py");
    assert_eq!(v["decision"], "block");
    let reason = v["reason"].as_str().expect("block reason present");
    assert!(reason.contains("error[pulse]"), "blocking smell in the reason");
    assert!(
        !reason.to_lowercase().contains("primitive obsession"),
        "advisory smell stays out of the block reason"
    );
    let ctx = v["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .expect("advisory context alongside the block");
    assert!(ctx.to_lowercase().contains("primitive obsession"));
}

#[test]
fn blocking_only_finding_has_no_advisory_context() {
    let src = complex_fn("items", "items");
    let v = run_hook_isolated(&src, "blk.py");
    assert_eq!(v["decision"], "block");
    assert!(v.get("hookSpecificOutput").is_none(), "no advisory smell means no additionalContext");
}
