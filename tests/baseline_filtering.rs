mod common;

use std::io::Write;
use std::path::Path;
use std::process::Command;

use pulse::thresholds::Thresholds;

fn run_pulse(args: &[&str], baseline: &Path, stdin: &str) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(args)
        .env("PULSE_BASELINE_DIR", baseline)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            child.stdin.take().unwrap().write_all(stdin.as_bytes()).unwrap();
            child.wait_with_output()
        })
        .expect("pulse failed");
    String::from_utf8(output.stdout).unwrap()
}

fn hook_json(file_path: &str, old: Option<&str>, new: Option<&str>) -> String {
    match (old, new) {
        (Some(o), Some(n)) => format!(
            r#"{{"tool_input":{{"file_path":"{}","old_string":"{}","new_string":"{}"}}}}"#,
            file_path.replace('\\', "\\\\").replace('"', "\\\""),
            o.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n"),
            n.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n"),
        ),
        _ => format!(
            r#"{{"tool_input":{{"file_path":"{}","content":"written"}}}}"#,
            file_path.replace('\\', "\\\\").replace('"', "\\\""),
        ),
    }
}

struct Env {
    _dir: tempfile::TempDir,
    bl: tempfile::TempDir,
    path: std::path::PathBuf,
}

impl Env {
    fn new(name: &str, content: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let bl = tempfile::tempdir().unwrap();
        let path = dir.path().join(name);
        std::fs::write(&path, content).unwrap();
        Self { _dir: dir, bl, path }
    }

    fn file(&self) -> &str { self.path.to_str().unwrap() }

    fn edit(&self, old: &str, new: &str) -> String {
        run_pulse(&["--hook"], self.bl.path(), &hook_json(self.file(), Some(old), Some(new)))
    }

    fn write_hook(&self) -> String {
        run_pulse(&["--hook"], self.bl.path(), &hook_json(self.file(), None, None))
    }

    fn stop(&self) -> String {
        run_pulse(&["--stop"], self.bl.path(), "{}")
    }
}

// ===========================================================================
// Baseline filtering: pre-existing smells NOT reported
// ===========================================================================

#[test]
fn preexisting_smells_silent_on_noop_edit() {
    // Python excess args
    let e = Env::new("t.py", "def f(a, b, c, d, e, f, g, h):\n    return a\n");
    assert!(e.edit("return a", "return a").is_empty(), "py excess args");
    // Python complex method
    let mut code = String::from("def big(x):\n");
    for i in 0..10 { code.push_str(&format!("    if x > {i}:\n        pass\n")); }
    code.push_str("    y = 1\n    return x\n");
    let e = Env::new("cc.py", &code);
    assert!(e.edit("y = 1", "y = 1").is_empty(), "py complex method");
    // JavaScript excess args
    let e = Env::new("t.js", "function f(a, b, c, d, e, f2, g, h) { return a; }\n");
    assert!(e.edit("return a", "return a").is_empty(), "js excess args");
}

// ===========================================================================
// Baseline filtering: NEW smells ARE reported
// ===========================================================================

#[test]
fn new_excess_args_reported_across_languages() {
    // Python
    let e = Env::new("t.py", "def f(a, b, c, d, e, f, g, h):\n    return a\n");
    let out = e.edit("def f(a, b):\n    return a", "def f(a, b, c, d, e, f, g, h):\n    return a");
    assert!(out.contains("excess arguments"), "Python: {out}");
    // TypeScript
    let e = Env::new("t.ts",
        "function f(a: number, b: number, c: number, d: number, e: number, f2: number, g: number, h: number): number { return a; }\n");
    let out = e.edit(
        "function f(a: number): number { return a; }",
        "function f(a: number, b: number, c: number, d: number, e: number, f2: number, g: number, h: number): number { return a; }");
    assert!(out.contains("excess arguments"), "TypeScript: {out}");
    // Rust
    let e = Env::new("t.rs",
        "fn f(a: i32, b: i32, c: i32, d: i32, e: i32, f2: i32, g: i32, h: i32) -> i32 { a }\n");
    let out = e.edit("fn f(a: i32) -> i32 { a }",
        "fn f(a: i32, b: i32, c: i32, d: i32, e: i32, f2: i32, g: i32, h: i32) -> i32 { a }");
    assert!(out.contains("excess arguments"), "Rust: {out}");
}

#[test]
fn worsened_cc_reported() {
    let mut code = String::from("def f(x):\n");
    for i in 0..10 { code.push_str(&format!("    if x > {i}:\n        pass\n")); }
    code.push_str("    if x > 99:\n        pass\n    return x\n");
    let e = Env::new("t.py", &code);
    let out = e.edit("    return x", "    if x > 99:\n        pass\n    return x");
    assert!(out.contains("complex method"), "worsened cc should be reported, got: {out}");
}

#[test]
fn write_mode_reports_all_new_findings() {
    let e = Env::new("t.py", "def f(a, b, c, d, e, f, g, h):\n    return a\n");
    let out = e.write_hook();
    assert!(out.contains("excess arguments"), "write mode should report all, got: {out}");
}

// ===========================================================================
// Baseline filtering: multi-edit and mixed scenarios
// ===========================================================================

#[test]
fn second_edit_uses_original_baseline() {
    let e = Env::new("t.py", "def f(a, b, c, d, e, f, g, h):\n    return a\n");
    // Overwrite with post-edit content, baseline captures pre-edit via old_string
    let out1 = e.edit(
        "def f(a, b):\n    return a + b", "def f(a, b, c, d, e, f, g, h):\n    return a");
    assert!(!out1.is_empty(), "first edit introducing smell should report");
    let out2 = e.edit("return a", "return a");
    assert!(!out2.is_empty(), "second edit should still report (baseline was clean)");
}

#[test]
fn only_newly_smelly_function_in_mixed_file() {
    let e = Env::new("t.py", concat!(
        "def func_a(a, b, c, d, e, f, g, h):\n    return a\n\n",
        "def func_b(a, b, c, d, e, f, g, h):\n    return a\n",
    ));
    let out = e.edit(
        "def func_b(a, b):\n    return a", "def func_b(a, b, c, d, e, f, g, h):\n    return a");
    assert!(out.contains("func_b"), "new func_b smell should appear, got: {out}");
    assert!(!out.contains("func_a"), "preexisting func_a should NOT appear, got: {out}");
}

// ===========================================================================
// JSON output format
// ===========================================================================

#[test]
fn json_output_format_correct() {
    // Smelly file: valid JSON block with error prefix, trailer, no note
    let e = Env::new("t.py", "def f(a, b, c, d, e, f, g, h):\n    return a\n");
    let out = e.write_hook();
    let v: serde_json::Value = serde_json::from_str(out.trim()).expect("should be JSON");
    assert_eq!(v["decision"], "block");
    let reason = v["reason"].as_str().unwrap();
    assert!(reason.contains("error[pulse]:"), "should have error prefix");
    assert!(reason.contains("Fix all issues above"), "should have trailer");
    assert!(!reason.contains("note[pulse]"), "should never use note");
    // Clean file: no output
    let e = Env::new("clean.py", "def add(a, b):\n    return a + b\n");
    assert!(e.write_hook().is_empty(), "clean file should be silent");
}

// ===========================================================================
// PULSE_DISABLE
// ===========================================================================

#[test]
fn pulse_disable_silences_hook() {
    let e = Env::new("t.py", "def f(a, b, c, d, e, f, g, h):\n    return a\n");
    let json = hook_json(e.file(), None, None);
    let output = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["--hook"])
        .env("PULSE_DISABLE", "1")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            child.stdin.take().unwrap().write_all(json.as_bytes()).unwrap();
            child.wait_with_output()
        })
        .expect("failed");
    assert!(String::from_utf8(output.stdout).unwrap().is_empty());
}

// ===========================================================================
// Edge cases
// ===========================================================================

#[test]
fn silent_on_bad_files() {
    let bl = tempfile::tempdir().unwrap();
    // Nonexistent file
    let json = hook_json("/no/such/file.py", Some("x"), Some("x"));
    assert!(run_pulse(&["--hook"], bl.path(), &json).is_empty(), "nonexistent");
    // Unsupported extension
    let e = Env::new("t.toml", "[config]\nkey = \"val\"\n");
    assert!(e.write_hook().is_empty(), "unsupported ext");
    // Empty file
    let e = Env::new("t.py", "");
    assert!(e.write_hook().is_empty(), "empty file");
    // Binary content
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("t.py");
    std::fs::write(&p, b"\x00\x01\x02\xff").unwrap();
    let json = hook_json(p.to_str().unwrap(), None, None);
    assert!(run_pulse(&["--hook"], bl.path(), &json).is_empty(), "binary");
}

#[test]
fn silent_on_bad_input() {
    let bl = tempfile::tempdir().unwrap();
    assert!(run_pulse(&["--hook"], bl.path(), "").is_empty(), "empty stdin");
    assert!(run_pulse(&["--hook"], bl.path(), "{bad").is_empty(), "malformed json");
    assert!(run_pulse(&["--hook"], bl.path(), r#"{"session_id":"x"}"#).is_empty(), "no tool_input");
    assert!(run_pulse(&["--hook"], bl.path(), r#"{"tool_input":{"content":"x"}}"#).is_empty(), "no file_path");
}

// ===========================================================================
// Stop hook
// ===========================================================================

#[test]
fn stop_silent_when_clean() {
    // No edits at all
    let bl = tempfile::tempdir().unwrap();
    assert!(run_pulse(&["--stop"], bl.path(), "{}").is_empty(), "no edits");
    // Edit but no regression
    let e = Env::new("t.py", "def f():\n    return 1\n");
    e.edit("return 1", "return 2");
    std::fs::write(&e.path, "def f():\n    return 2\n").unwrap();
    assert!(e.stop().is_empty(), "no regression");
}

#[test]
fn stop_blocks_on_module_regression() {
    let e = Env::new("g.py", "def f():\n    return 1\nM = 1\n");
    e.edit("M = 1", "M = 2");
    let mut big = String::from("def f():\n    return 1\nM = 2\n");
    for i in 0..(Thresholds::default().module.file_loc_warning as usize + 20) { big.push_str(&format!("x_{i} = {i}\n")); }
    std::fs::write(&e.path, &big).unwrap();
    let out = e.stop();
    assert!(out.contains("file too large"), "should report regression, got: {out}");
    let v: serde_json::Value = serde_json::from_str(out.trim()).unwrap();
    assert_eq!(v["decision"], "block");
}
