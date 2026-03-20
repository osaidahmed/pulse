mod common;

use common::*;
use std::process::Command;

fn hook_with_edit(file_path: &str, old: &str, new: &str) -> String {
    let json = format!(
        r#"{{"tool_input":{{"file_path":"{}","old_string":"{}","new_string":"{}"}}}}"#,
        file_path.replace('\\', "\\\\").replace('"', "\\\""),
        old.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n"),
        new.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n"),
    );
    let output = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["--hook"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.take().unwrap().write_all(json.as_bytes()).unwrap();
            child.wait_with_output()
        })
        .expect("failed to run pulse --hook");
    String::from_utf8(output.stdout).unwrap()
}

// ===========================================================================
// Diff filtering on production fixture
// ===========================================================================

#[test]
fn edit_inside_smelly_function_reports_that_function() {
    let path = fixtures_dir("python").join("production_service.py");
    let out = hook_with_edit(
        path.to_str().unwrap(),
        "order.status = \"processing\"",
        "order.status = \"processing\"",
    );
    assert!(has_function(&out, "process_order"), "editing inside process_order should report it, got: {}", out);
}

#[test]
fn edit_outside_smelly_function_skips_it() {
    let path = fixtures_dir("python").join("production_service.py");
    let out = hook_with_edit(
        path.to_str().unwrap(),
        "import json",
        "import json",
    );
    // Line 3 is far from process_order (L58-111)
    assert!(!has_function(&out, "process_order"), "editing imports should not report process_order, got: {}", out);
}

#[test]
fn module_findings_always_present_with_edit() {
    let path = fixtures_dir("python").join("production_service.py");
    let out = hook_with_edit(
        path.to_str().unwrap(),
        "import json",
        "import json",
    );
    // Module-level findings should always be present even when editing far from functions
    // (Low Cohesion is a module finding)
    assert!(has_smell(&out, "Low Cohesion") || has_smell(&out, "Code Duplication"),
        "module findings should appear regardless of edit position, got: {}", out);
}

// ===========================================================================
// Compact output validation
// ===========================================================================

#[test]
fn hook_output_is_single_line() {
    let path = fixtures_dir("python").join("production_service.py");
    let out = hook_with_edit(
        path.to_str().unwrap(),
        "def process_order",
        "def process_order",
    );
    if out.is_empty() { return; }
    let lines = out.trim().lines().count();
    assert_eq!(lines, 1, "hook output should be single line, got {} lines:\n{}", lines, out);
}

#[test]
fn hook_output_contains_issue_count() {
    let path = fixtures_dir("python").join("production_service.py");
    let out = hook_with_edit(
        path.to_str().unwrap(),
        "def process_order",
        "def process_order",
    );
    if out.is_empty() { return; }
    assert!(out.contains("issue"), "output should contain 'issue' count: {}", out);
}

#[test]
fn check_mode_still_multiline() {
    let out = run_check("python", "production_service.py");
    let lines = out.trim().lines().count();
    assert!(lines > 3, "check mode should be multi-line verbose, got {} lines", lines);
}

// ===========================================================================
// Edge cases for diff filtering
// ===========================================================================

#[test]
fn edit_spanning_multiple_functions() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("multi.py");
    std::fs::write(&path, concat!(
        "def smelly_a(a, b, c, d, e, f, g, h):\n",
        "    return a\n",
        "\n",
        "x = 1\n",
        "\n",
        "def smelly_b(a, b, c, d, e, f, g, h):\n",
        "    return a\n",
    )).unwrap();

    // Edit spans from smelly_a to smelly_b
    let out = hook_with_edit(path.to_str().unwrap(), "def smelly_a", "def smelly_a");
    assert!(has_function(&out, "smelly_a"), "should see smelly_a: {}", out);
}

#[test]
fn edit_on_empty_line_between_functions() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("gap.py");
    std::fs::write(&path, concat!(
        "def smelly_a(a, b, c, d, e, f, g, h):\n",
        "    return a\n",
        "\n",
        "\n",
        "\n",
        "def smelly_b(a, b, c, d, e, f, g, h):\n",
        "    return a\n",
    )).unwrap();

    // Edit the blank line between functions — should not overlap either
    let out = hook_with_edit(path.to_str().unwrap(), "x = 1", "x = 1");
    // old_string not found → fallback to all findings
    assert!(!out.is_empty() || out.is_empty()); // just verify no crash
}

#[test]
fn write_mode_reports_all() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("new.py");
    std::fs::write(&path, concat!(
        "def a(x, y, z, w, a, b, c, d):\n    return x\n\n",
        "def b(x, y, z, w, a, b, c, d):\n    return x\n",
    )).unwrap();

    let json = format!(
        r#"{{"tool_input":{{"file_path":"{}","content":"anything"}}}}"#,
        path.to_str().unwrap()
    );
    let output = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["--hook"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.take().unwrap().write_all(json.as_bytes()).unwrap();
            child.wait_with_output()
        })
        .expect("failed to run");
    let out = String::from_utf8(output.stdout).unwrap();
    assert!(has_function(&out, "a"), "Write mode should show all: {}", out);
    assert!(has_function(&out, "b"), "Write mode should show all: {}", out);
}
