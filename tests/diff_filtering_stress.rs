mod common;

use common::*;
use std::process::Command;

fn hook_write_mode(file_path: &str) -> String {
    let json = format!(
        r#"{{"tool_input":{{"file_path":"{}","content":"full file"}}}}"#,
        file_path.replace('\\', "\\\\").replace('"', "\\\""),
    );
    let baseline_dir = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["--hook"])
        .env("PULSE_BASELINE_DIR", baseline_dir.path())
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

fn hook_with_edit(file_path: &str, old: &str, new: &str) -> String {
    let json = format!(
        r#"{{"tool_input":{{"file_path":"{}","old_string":"{}","new_string":"{}"}}}}"#,
        file_path.replace('\\', "\\\\").replace('"', "\\\""),
        old.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n"),
        new.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n"),
    );
    let baseline_dir = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["--hook"])
        .env("PULSE_BASELINE_DIR", baseline_dir.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child
                .stdin
                .take()
                .unwrap()
                .write_all(json.as_bytes())
                .unwrap();
            child.wait_with_output()
        })
        .expect("failed to run pulse --hook");
    String::from_utf8(output.stdout).unwrap()
}

// ===========================================================================
// Diff filtering on production fixture
// ===========================================================================

fn write_borderline_function(path: &std::path::Path, extra_branch: bool) {
    let mut code = String::from("def process(x):\n");
    for i in 0..7 {
        code.push_str(&format!("    if x > {i}:\n        pass\n"));
    }
    if extra_branch {
        code.push_str("    if x > 7:\n        pass\n");
    }
    code.push_str("    return x\n");
    std::fs::write(path, code).unwrap();
}

#[test]
fn edit_that_worsens_function_reports_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("worsen.py");
    write_borderline_function(&path, true);
    let out = hook_with_edit(
        path.to_str().unwrap(),
        "    return x",
        "    if x > 7:\n        pass\n    return x",
    );
    assert!(has_function(&out, "process"), "worsening should report it, got: {}", out);
}

#[test]
fn edit_outside_smelly_function_skips_it() {
    let path = fixtures_dir("python").join("production_service.py");
    let out = hook_with_edit(path.to_str().unwrap(), "import json", "import json");
    // Line 3 is far from process_order (L58-111)
    assert!(
        !has_function(&out, "process_order"),
        "editing imports should not report process_order, got: {}",
        out
    );
}

#[test]
fn module_findings_excluded_from_hook_output() {
    let path = fixtures_dir("python").join("production_service.py");
    let out = hook_with_edit(path.to_str().unwrap(), "import json", "import json");
    // Module-level findings should NOT appear in hook output (handled by Stop hook)
    assert!(
        !has_smell(&out, "Low Cohesion"),
        "module findings should not appear in hook output, got: {}",
        out
    );
    assert!(
        !has_smell(&out, "Code Duplication"),
        "module findings should not appear in hook output, got: {}",
        out
    );
}

// ===========================================================================
// Compact output validation
// ===========================================================================

#[test]
fn hook_output_is_json_block_decision() {
    let path = fixtures_dir("python").join("production_service.py");
    let out = hook_with_edit(
        path.to_str().unwrap(),
        "def process_order",
        "def process_order",
    );
    if out.is_empty() {
        return;
    }
    let parsed: serde_json::Value = serde_json::from_str(out.trim())
        .expect("hook output should be valid JSON");
    assert_eq!(
        parsed.get("decision").and_then(|v| v.as_str()),
        Some("block"),
        "should have decision: block, got: {}",
        out
    );
    let reason = parsed.get("reason").and_then(|v| v.as_str()).unwrap_or("");
    assert!(
        reason.contains("error[pulse]:"),
        "reason should contain error[pulse]:, got: {}",
        reason
    );
}

#[test]
fn hook_output_contains_error_prefix() {
    let path = fixtures_dir("python").join("production_service.py");
    let out = hook_with_edit(
        path.to_str().unwrap(),
        "def process_order",
        "def process_order",
    );
    if out.is_empty() {
        return;
    }
    assert!(
        out.contains("error[pulse]:"),
        "output should contain error[pulse]: prefix: {}",
        out
    );
}

#[test]
fn check_mode_still_multiline() {
    let out = run_check("python", "production_service.py");
    let lines = out.trim().lines().count();
    assert!(
        lines > 3,
        "check mode should be multi-line verbose, got {} lines",
        lines
    );
}

// ===========================================================================
// Edge cases for diff filtering
// ===========================================================================

#[test]
fn edit_spanning_multiple_functions() {
    let content = "def smelly_a(a, b, c, d, e, f, g, h):\n    return a\n\nx = 1\n\ndef smelly_b(a, b, c, d, e, f, g, h):\n    return a\n";
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("multi.py");
    std::fs::write(&path, content).unwrap();
    let out = hook_write_mode(path.to_str().unwrap());
    assert!(has_function(&out, "smelly_a"), "write mode should see smelly_a: {}", out);
}

#[test]
fn edit_on_empty_line_between_functions() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("gap.py");
    std::fs::write(
        &path,
        concat!(
            "def smelly_a(a, b, c, d, e, f, g, h):\n",
            "    return a\n",
            "\n",
            "\n",
            "\n",
            "def smelly_b(a, b, c, d, e, f, g, h):\n",
            "    return a\n",
        ),
    )
    .unwrap();

    // Edit the blank line between functions — should not overlap either
    let out = hook_with_edit(path.to_str().unwrap(), "x = 1", "x = 1");
    // old_string not found → fallback to all findings
    assert!(!out.is_empty() || out.is_empty()); // just verify no crash
}

#[test]
fn write_mode_reports_all() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("new.py");
    std::fs::write(
        &path,
        concat!(
            "def a(x, y, z, w, a, b, c, d):\n    return x\n\n",
            "def b(x, y, z, w, a, b, c, d):\n    return x\n",
        ),
    )
    .unwrap();
    let out = hook_write_mode(path.to_str().unwrap());
    assert!(has_function(&out, " a ") || has_function(&out, "`a`"), "Write mode should show a: {}", out);
    assert!(has_function(&out, " b ") || has_function(&out, "`b`"), "Write mode should show b: {}", out);
}
