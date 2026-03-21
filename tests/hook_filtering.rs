mod common;

use common::*;
use std::process::Command;

fn run_hook_with_json(json: &str) -> String {
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
// Edit range filtering
// ===========================================================================

#[test]
fn edit_that_introduces_smell_shows_only_near_function() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.py");
    // After edit: func_a has too many args (smelly), func_b is clean far away
    std::fs::write(
        &path,
        concat!(
            "def func_a(a, b, c, d, e, f, g, h):\n",
            "    return a\n",
            "\n",
            "def clean():\n",
            "    return 1\n",
        ),
    )
    .unwrap();

    // Edit introduced func_a's excess args (old had fewer params)
    let json = format!(
        r#"{{"tool_input":{{"file_path":"{}","old_string":"def func_a(a, b):\n    return a","new_string":"def func_a(a, b, c, d, e, f, g, h):\n    return a"}}}}"#,
        path.to_str().unwrap()
    );
    let output = run_hook_with_json(&json);
    assert!(
        output.contains("func_a"),
        "should see func_a after introducing smell, got: {}",
        output
    );
}

#[test]
fn edit_far_from_functions_shows_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.py");
    std::fs::write(
        &path,
        concat!(
            "import os\n",
            "\n",
            "\n",
            "\n",
            "\n",
            "def smelly(a, b, c, d, e, f, g, h):\n",
            "    return a\n",
        ),
    )
    .unwrap();

    // Edit at line 1 (import) — function is at line 6, no overlap
    let json = format!(
        r#"{{"tool_input":{{"file_path":"{}","old_string":"import os","new_string":"import os"}}}}"#,
        path.to_str().unwrap()
    );
    let output = run_hook_with_json(&json);
    assert!(
        output.is_empty(),
        "edit at line 1 should not show function at line 6, got: {}",
        output
    );
}

#[test]
fn write_mode_shows_all_findings() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.py");
    std::fs::write(
        &path,
        concat!(
            "def func_a(a, b, c, d, e, f, g, h):\n",
            "    return a\n",
            "\n",
            "def func_b(a, b, c, d, e, f, g, h):\n",
            "    return a\n",
        ),
    )
    .unwrap();

    // Write mode — content field, no old_string/new_string → None edit range → all findings
    let json = format!(
        r#"{{"tool_input":{{"file_path":"{}","content":"full file"}}}}"#,
        path.to_str().unwrap()
    );
    let output = run_hook_with_json(&json);
    assert!(
        output.contains("func_a"),
        "Write mode should show all: {}",
        output
    );
    assert!(
        output.contains("func_b"),
        "Write mode should show all: {}",
        output
    );
}

#[test]
fn malformed_json_produces_no_output() {
    let output = run_hook_with_json("not json");
    assert!(output.is_empty());
}

#[test]
fn missing_file_path_produces_no_output() {
    let output = run_hook_with_json(r#"{"tool_input":{"content":"hello"}}"#);
    assert!(output.is_empty());
}

// ===========================================================================
// Compact output format
// ===========================================================================

#[test]
fn hook_output_is_json_blocking_decision() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.py");
    std::fs::write(&path, "def smelly(a, b, c, d, e, f, g, h):\n    return a\n").unwrap();

    // Edit introduces the excess args (old had 2 params)
    let json = format!(
        r#"{{"tool_input":{{"file_path":"{}","old_string":"def smelly(a, b):\n    return a","new_string":"def smelly(a, b, c, d, e, f, g, h):\n    return a"}}}}"#,
        path.to_str().unwrap()
    );
    let output = run_hook_with_json(&json);
    let parsed: serde_json::Value = serde_json::from_str(output.trim())
        .expect("hook output should be valid JSON");
    assert_eq!(
        parsed.get("decision").and_then(|v| v.as_str()),
        Some("block"),
        "should have decision: block"
    );
}

#[test]
fn hook_output_reason_contains_findings() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.py");
    std::fs::write(&path, "def smelly(a, b, c, d, e, f, g, h):\n    return a\n").unwrap();

    // Edit introduces the excess args (old had 2 params)
    let json = format!(
        r#"{{"tool_input":{{"file_path":"{}","old_string":"def smelly(a, b):\n    return a","new_string":"def smelly(a, b, c, d, e, f, g, h):\n    return a"}}}}"#,
        path.to_str().unwrap()
    );
    let output = run_hook_with_json(&json);
    let parsed: serde_json::Value = serde_json::from_str(output.trim()).unwrap();
    let reason = parsed.get("reason").and_then(|v| v.as_str()).unwrap_or("");
    assert!(
        reason.contains("error[pulse]:"),
        "reason should contain error[pulse]:, got: {}",
        reason
    );
}

#[test]
fn check_mode_still_uses_verbose_format() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.py");
    std::fs::write(&path, "def smelly(a, b, c, d, e, f, g, h):\n    return a\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(output.stdout).unwrap();
    // Check mode should use multi-line format with indented findings
    assert!(
        stdout.contains("\n  "),
        "check mode should have indented findings: {}",
        stdout
    );
}

// ===========================================================================
// Module-level findings excluded from hook mode (handled by Stop hook)
// ===========================================================================

#[test]
fn module_findings_excluded_from_hook_output() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.py");
    // Generate a file that's too large (> 400 LOC) with too many functions
    let mut code = "import os\n\n".to_string();
    for i in 0..25 {
        code.push_str(&format!("def fn_{}():\n    return {}\n\n", i, i));
    }
    for i in 0..200 {
        code.push_str(&format!("VAR_{} = {}\n", i, i));
    }
    std::fs::write(&path, &code).unwrap();

    // Edit at line 1 — module findings should NOT appear in hook output
    let json = format!(
        r#"{{"tool_input":{{"file_path":"{}","old_string":"import os","new_string":"import os"}}}}"#,
        path.to_str().unwrap()
    );
    let output = run_hook_with_json(&json);
    assert!(
        !output.contains("File Too Large"),
        "Module findings should not appear in hook output: {}",
        output
    );
    assert!(
        !output.contains("Too Many Functions"),
        "Module findings should not appear in hook output: {}",
        output
    );
}

// ===========================================================================
// Clean file still silent
// ===========================================================================

#[test]
fn hook_on_clean_file_still_silent() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.py");
    std::fs::write(&path, "def add(a, b):\n    return a + b\n").unwrap();

    let json = format!(
        r#"{{"tool_input":{{"file_path":"{}","old_string":"def add","new_string":"def add"}}}}"#,
        path.to_str().unwrap()
    );
    let output = run_hook_with_json(&json);
    assert!(
        output.is_empty(),
        "clean file should be silent, got: {}",
        output
    );
}

#[test]
fn hook_on_unsupported_file_still_silent() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "[package]\nname = \"test\"\n").unwrap();

    let json = format!(
        r#"{{"tool_input":{{"file_path":"{}","old_string":"name","new_string":"name"}}}}"#,
        path.to_str().unwrap()
    );
    let output = run_hook_with_json(&json);
    assert!(output.is_empty());
}
