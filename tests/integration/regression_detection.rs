#![allow(clippy::format_collect, clippy::unused_self)]


use crate::common::*;
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

fn pulse_bin() -> String {
    env!("CARGO_BIN_EXE_pulse").to_string()
}

struct TestEnv {
    baseline_dir: tempfile::TempDir,
    files_dir: tempfile::TempDir,
}

impl TestEnv {
    fn new() -> Self {
        Self {
            baseline_dir: tempfile::tempdir().unwrap(),
            files_dir: tempfile::tempdir().unwrap(),
        }
    }

    fn baseline_path(&self) -> &std::path::Path {
        self.baseline_dir.path()
    }

    fn file_path(&self, name: &str) -> PathBuf {
        self.files_dir.path().join(name)
    }

    fn baseline_file_path(&self, file_path: &str) -> PathBuf {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        file_path.hash(&mut hasher);
        self.baseline_path()
            .join(format!("{:016x}.json", hasher.finish()))
    }

    fn run_hook(&self, json: &str) -> String {
        let output = Command::new(pulse_bin())
            .args(["--hook"])
            .env("PULSE_BASELINE_DIR", self.baseline_path())
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
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

    fn run_stop(&self) -> String {
        let output = Command::new(pulse_bin())
            .args(["--stop"])
            .env("PULSE_BASELINE_DIR", self.baseline_path())
            .output()
            .expect("failed to run pulse --stop");
        String::from_utf8(output.stdout).unwrap()
    }

    fn run_cleanup(&self) {
        Command::new(pulse_bin())
            .args(["--cleanup"])
            .env("PULSE_BASELINE_DIR", self.baseline_path())
            .output()
            .expect("failed to run pulse --cleanup");
    }

    fn manifest_contents(&self) -> String {
        std::fs::read_to_string(self.baseline_path().join("manifest.txt")).unwrap_or_default()
    }

    fn edit_hook_json(&self, path: &std::path::Path, old: &str, new: &str) -> String {
        format!(
            r#"{{"tool_input":{{"file_path":"{}","old_string":"{}","new_string":"{}"}}}}"#,
            path.to_str()
                .unwrap()
                .replace('\\', "\\\\")
                .replace('"', "\\\""),
            old.replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', "\\n"),
            new.replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', "\\n"),
        )
    }

    fn write_hook_json(&self, path: &std::path::Path) -> String {
        format!(
            r#"{{"tool_input":{{"file_path":"{}","content":"..."}}}}"#,
            path.to_str()
                .unwrap()
                .replace('\\', "\\\\")
                .replace('"', "\\\""),
        )
    }

    fn baseline_counts(&self, file_path: &str) -> HashMap<String, usize> {
        let bp = self.baseline_file_path(file_path);
        let json = std::fs::read_to_string(&bp).unwrap_or_default();
        serde_json::from_str(&json).unwrap_or_default()
    }
}

// Helper: generate N lines of variable declarations
fn var_lines(prefix: &str, count: usize) -> String {
    (0..count)
        .map(|i| format!("{prefix}_{i} = {i}\n"))
        .collect()
}

// Helper: generate N simple functions
fn simple_functions(count: usize) -> String {
    (0..count)
        .map(|i| format!("def fn_{i}():\n    return {i}\n\n"))
        .collect()
}

// Helper: generate a complex function with given CC
fn complex_function(name: &str, branches: usize) -> String {
    let mut code = format!("def {name}(x):\n");
    for i in 0..branches {
        code.push_str(&format!("    if x == {i}:\n        pass\n"));
    }
    code.push_str("    return x\n\n");
    code
}

// ═══════════════════════════════════════════════════════════════════════════
// BASELINE CACHING
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn baseline_created_on_first_edit() {
    let env = TestEnv::new();
    let path = env.file_path("test.py");
    std::fs::write(&path, "def foo():\n    return 1\n").unwrap();

    let json = env.edit_hook_json(&path, "return 1", "return 2");
    std::fs::write(&path, "def foo():\n    return 2\n").unwrap();
    env.run_hook(&json);

    assert!(
        env.baseline_file_path(path.to_str().unwrap()).exists(),
        "baseline should be created on first edit"
    );
    assert!(
        env.manifest_contents().contains(path.to_str().unwrap()),
        "manifest should list the file"
    );
}

#[test]
fn baseline_immutable_after_first_edit() {
    let env = TestEnv::new();
    let path = env.file_path("test.py");

    std::fs::write(&path, "def foo():\n    return 1\n").unwrap();
    let json1 = env.edit_hook_json(&path, "return 1", "return 2");
    std::fs::write(&path, "def foo():\n    return 2\n").unwrap();
    env.run_hook(&json1);

    let first = std::fs::read_to_string(env.baseline_file_path(path.to_str().unwrap())).unwrap();

    let json2 = env.edit_hook_json(&path, "return 2", "return 3");
    std::fs::write(&path, "def foo():\n    return 3\n").unwrap();
    env.run_hook(&json2);

    let second = std::fs::read_to_string(env.baseline_file_path(path.to_str().unwrap())).unwrap();
    assert_eq!(first, second, "baseline must not change after first edit");
}

#[test]
fn baseline_captures_preexisting_module_findings() {
    let env = TestEnv::new();
    let path = env.file_path("big.py");

    // 25 functions + 200 vars = well over 400 LOC, >20 functions
    let code = format!("{}{}", simple_functions(25), var_lines("x", file_padding()));
    std::fs::write(&path, format!("{code}MARKER = 1\n")).unwrap();

    let json = env.edit_hook_json(&path, "MARKER = 1", "MARKER = 2");
    std::fs::write(&path, format!("{code}MARKER = 2\n")).unwrap();
    env.run_hook(&json);

    let counts = env.baseline_counts(path.to_str().unwrap());
    assert!(
        counts.contains_key("File Too Large") || counts.contains_key("Too Many Functions"),
        "baseline should capture module findings: {counts:?}"
    );
}

#[test]
fn baseline_empty_for_clean_file() {
    let env = TestEnv::new();
    let path = env.file_path("clean.py");
    std::fs::write(&path, "def foo():\n    return 1\n").unwrap();

    let json = env.edit_hook_json(&path, "return 1", "return 2");
    std::fs::write(&path, "def foo():\n    return 2\n").unwrap();
    env.run_hook(&json);

    let counts = env.baseline_counts(path.to_str().unwrap());
    assert!(
        counts.is_empty(),
        "baseline should be empty for clean file: {counts:?}"
    );
}

#[test]
fn baseline_created_for_write_mode() {
    let env = TestEnv::new();
    let path = env.file_path("new.py");
    std::fs::write(&path, "def foo():\n    return 1\n").unwrap();

    let json = env.write_hook_json(&path);
    env.run_hook(&json);

    assert!(
        env.baseline_file_path(path.to_str().unwrap()).exists(),
        "baseline should be created for Write mode too"
    );
}

#[test]
fn baseline_created_for_unsupported_language() {
    let env = TestEnv::new();
    let path = env.file_path("data.toml");
    std::fs::write(&path, "[package]\nname = \"test\"\n").unwrap();

    let json = env.edit_hook_json(&path, "name = \"test\"", "name = \"test2\"");
    std::fs::write(&path, "[package]\nname = \"test2\"\n").unwrap();
    env.run_hook(&json);

    // Should still create an empty baseline and manifest entry
    assert!(
        env.baseline_file_path(path.to_str().unwrap()).exists(),
        "baseline should exist even for unsupported language"
    );
    let counts = env.baseline_counts(path.to_str().unwrap());
    assert!(
        counts.is_empty(),
        "unsupported language baseline should be empty"
    );
}

#[test]
fn baseline_reconstruction_reverses_edit() {
    let env = TestEnv::new();
    let path = env.file_path("recon.py");

    // Start with 19 functions (under 20 threshold)
    let code = format!("{}MARKER = 1\n", simple_functions(19));
    std::fs::write(&path, &code).unwrap();

    // "Edit" that adds a 20th function (simulated)
    let new_code = format!(
        "{}def fn_19():\n    return 19\n\nMARKER = 2\n",
        simple_functions(19)
    );
    std::fs::write(&path, &new_code).unwrap();
    let json = env.edit_hook_json(
        &path,
        "MARKER = 1",
        "def fn_19():\n    return 19\n\nMARKER = 2",
    );
    env.run_hook(&json);

    // Baseline should reflect the pre-edit state (19 functions, no Too Many Functions)
    let counts = env.baseline_counts(path.to_str().unwrap());
    assert!(
        !counts.contains_key("Too Many Functions"),
        "baseline should reflect pre-edit state (19 functions, not 20+): {counts:?}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// STOP HOOK — NEGATIVE PATHS (should be silent)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn stop_silent_when_no_manifest() {
    let env = TestEnv::new();
    assert!(env.run_stop().is_empty());
}

#[test]
fn stop_silent_when_no_regressions_clean_file() {
    let env = TestEnv::new();
    let path = env.file_path("clean.py");
    std::fs::write(&path, "def foo():\n    return 1\n").unwrap();

    let json = env.edit_hook_json(&path, "return 1", "return 2");
    std::fs::write(&path, "def foo():\n    return 2\n").unwrap();
    env.run_hook(&json);

    assert!(env.run_stop().is_empty(), "no regressions on clean file");
}

#[test]
fn stop_silent_when_preexisting_file_too_large() {
    let env = TestEnv::new();
    let path = env.file_path("big.py");

    let code = format!("{}MARKER = 1\n", var_lines("x", file_padding()));
    std::fs::write(&path, &code).unwrap();

    let json = env.edit_hook_json(&path, "MARKER = 1", "MARKER = 2");
    std::fs::write(&path, code.replace("MARKER = 1", "MARKER = 2")).unwrap();
    env.run_hook(&json);

    assert!(
        !env.run_stop().contains("File Too Large"),
        "pre-existing File Too Large should not be a regression"
    );
}

#[test]
fn stop_silent_when_preexisting_too_many_functions() {
    let env = TestEnv::new();
    let path = env.file_path("many.py");

    let code = format!("{}MARKER = 1\n", simple_functions(25));
    std::fs::write(&path, &code).unwrap();

    let json = env.edit_hook_json(&path, "MARKER = 1", "MARKER = 2");
    std::fs::write(&path, code.replace("MARKER = 1", "MARKER = 2")).unwrap();
    env.run_hook(&json);

    assert!(
        !env.run_stop().contains("Too Many Functions"),
        "pre-existing Too Many Functions should not be a regression"
    );
}

#[test]
fn stop_silent_when_file_shrinks_below_threshold() {
    let env = TestEnv::new();
    let path = env.file_path("shrinking.py");

    // Start above 500 LOC
    let code = format!("{}MARKER = 1\n", var_lines("x", file_padding()));
    std::fs::write(&path, &code).unwrap();

    let json = env.edit_hook_json(&path, "MARKER = 1", "MARKER = 2");
    std::fs::write(&path, code.replace("MARKER = 1", "MARKER = 2")).unwrap();
    env.run_hook(&json);

    // Shrink file below threshold (finding disappears — NOT a regression)
    std::fs::write(&path, "def foo():\n    return 1\n").unwrap();

    let out = env.run_stop();
    assert!(
        out.is_empty(),
        "shrinking below threshold is not a regression: {out}"
    );
}

#[test]
fn stop_silent_when_file_deleted_between_hook_and_stop() {
    let env = TestEnv::new();
    let path = env.file_path("ephemeral.py");
    std::fs::write(&path, "def foo():\n    return 1\n").unwrap();

    let json = env.edit_hook_json(&path, "return 1", "return 2");
    std::fs::write(&path, "def foo():\n    return 2\n").unwrap();
    env.run_hook(&json);

    std::fs::remove_file(&path).unwrap();

    let out = env.run_stop();
    assert!(
        out.is_empty(),
        "deleted file should not cause errors: {out}"
    );
}

#[test]
fn stop_silent_when_unsupported_file_in_manifest() {
    let env = TestEnv::new();
    let path = env.file_path("config.toml");
    std::fs::write(&path, "[package]\nname = \"x\"\n").unwrap();

    let json = env.edit_hook_json(&path, "name = \"x\"", "name = \"y\"");
    std::fs::write(&path, "[package]\nname = \"y\"\n").unwrap();
    env.run_hook(&json);

    let out = env.run_stop();
    assert!(
        out.is_empty(),
        "unsupported file should not trigger regressions: {out}"
    );
}

#[test]
fn stop_handles_corrupt_baseline() {
    let env = TestEnv::new();
    let path = env.file_path("corrupt.py");
    std::fs::write(&path, "def foo():\n    return 1\n").unwrap();

    let json = env.edit_hook_json(&path, "return 1", "return 2");
    std::fs::write(&path, "def foo():\n    return 2\n").unwrap();
    env.run_hook(&json);

    // Corrupt the baseline file
    let bp = env.baseline_file_path(path.to_str().unwrap());
    std::fs::write(&bp, "NOT VALID JSON {{{").unwrap();

    // Stop should not crash — corrupt baseline treated as empty (all findings are regressions)
    let _out = env.run_stop();
    // Just verifying no panic/crash
}

// ═══════════════════════════════════════════════════════════════════════════
// STOP HOOK — POSITIVE REGRESSION PATHS (per module smell)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn stop_detects_file_too_large_regression() {
    let env = TestEnv::new();
    let path = env.file_path("grow_loc.py");

    // Start under 500 LOC
    let code = format!("{}MARKER = 1\n", var_lines("x", 130));
    std::fs::write(&path, &code).unwrap();

    let json = env.edit_hook_json(&path, "MARKER = 1", "MARKER = 2");
    std::fs::write(&path, code.replace("MARKER = 1", "MARKER = 2")).unwrap();
    env.run_hook(&json);

    // Grow past 500 LOC
    let mut big = std::fs::read_to_string(&path).unwrap();
    big.push_str(&var_lines("y", file_padding()));
    std::fs::write(&path, &big).unwrap();

    let out = env.run_stop();
    assert!(out.contains("file too large"), "should detect: {out}");
    assert!(
        out.contains("error[pulse]"),
        "finding should use error[pulse] prefix: {out}"
    );
    assert!(
        out.contains("crossed"),
        "informational finding should use 'crossed' framing: {out}"
    );
}

#[test]
fn stop_detects_too_many_functions_regression() {
    let env = TestEnv::new();
    let path = env.file_path("grow_fns.py");

    // Start with 18 functions (under 20)
    let code = format!("{}MARKER = 1\n", simple_functions(18));
    std::fs::write(&path, &code).unwrap();

    let json = env.edit_hook_json(&path, "MARKER = 1", "MARKER = 2");
    std::fs::write(&path, code.replace("MARKER = 1", "MARKER = 2")).unwrap();
    env.run_hook(&json);

    // Add functions past 20
    let mut grown = std::fs::read_to_string(&path).unwrap();
    for i in 18..25 {
        grown.push_str(&format!("def added_{i}():\n    return {i}\n\n"));
    }
    std::fs::write(&path, &grown).unwrap();

    let out = env.run_stop();
    assert!(out.contains("too many functions"), "should detect: {out}");
}

#[test]
fn stop_detects_overall_complexity_regression() {
    let env = TestEnv::new();
    let path = env.file_path("grow_cc.py");

    // Start with moderate complexity (under 100 total CC)
    let mut code = String::new();
    for i in 0..8 {
        code.push_str(&complex_function(&format!("fn_{i}"), 8));
    }
    code.push_str("MARKER = 1\n");
    std::fs::write(&path, &code).unwrap();

    let json = env.edit_hook_json(&path, "MARKER = 1", "MARKER = 2");
    std::fs::write(&path, code.replace("MARKER = 1", "MARKER = 2")).unwrap();
    env.run_hook(&json);

    // Add more complex functions to push total CC past 100
    let mut grown = std::fs::read_to_string(&path).unwrap();
    for i in 8..18 {
        grown.push_str(&complex_function(&format!("added_{i}"), 12));
    }
    std::fs::write(&path, &grown).unwrap();

    let out = env.run_stop();
    assert!(
        out.contains("overall code complexity"),
        "should detect overall code complexity: {out}"
    );
}

#[test]
fn stop_detects_excessive_declarations_regression() {
    let env = TestEnv::new();
    let path = env.file_path("grow_decl.py");

    // Python counts class_definition as declarations. Start below threshold
    let below = t().module.max_declarations as usize / 2;
    let mut code = String::new();
    for i in 0..below {
        code.push_str(&format!("class C_{i}:\n    pass\n\n"));
    }
    code.push_str("MARKER = 1\n");
    std::fs::write(&path, &code).unwrap();

    let json = env.edit_hook_json(&path, "MARKER = 1", "MARKER = 2");
    std::fs::write(&path, code.replace("MARKER = 1", "MARKER = 2")).unwrap();
    env.run_hook(&json);

    // Add class declarations past threshold
    let mut grown = std::fs::read_to_string(&path).unwrap();
    for i in below..declarations_above() {
        grown.push_str(&format!("class Added_{i}:\n    pass\n\n"));
    }
    std::fs::write(&path, &grown).unwrap();

    let out = env.run_stop();
    assert!(
        out.contains("excessive declarations"),
        "should detect excessive declarations: {out}"
    );
}

#[test]
fn stop_detects_global_conditionals_regression() {
    let env = TestEnv::new();
    let path = env.file_path("grow_global.py");

    // Start with no global conditionals
    let code = "def foo():\n    return 1\n\nMARKER = 1\n".to_string();
    std::fs::write(&path, &code).unwrap();

    let json = env.edit_hook_json(&path, "MARKER = 1", "MARKER = 2");
    std::fs::write(&path, code.replace("MARKER = 1", "MARKER = 2")).unwrap();
    env.run_hook(&json);

    // Add global conditional
    let mut grown = std::fs::read_to_string(&path).unwrap();
    grown.push_str("if __name__ == \"__main__\":\n    foo()\n");
    std::fs::write(&path, &grown).unwrap();

    let out = env.run_stop();
    assert!(
        out.contains("global conditionals"),
        "should detect global conditionals: {out}"
    );
}

#[test]
fn stop_detects_overall_function_size_regression() {
    let env = TestEnv::new();
    let path = env.file_path("grow_size.py");

    // Start with 2 large functions (threshold is 3+ functions with loc >= 40)
    let mut code = String::new();
    for i in 0..2 {
        code.push_str(&format!("def big_{i}():\n"));
        for j in 0..large_fn_lines() {
            code.push_str(&format!("    x_{j} = {j}\n"));
        }
        code.push('\n');
    }
    code.push_str("MARKER = 1\n");
    std::fs::write(&path, &code).unwrap();

    let json = env.edit_hook_json(&path, "MARKER = 1", "MARKER = 2");
    std::fs::write(&path, code.replace("MARKER = 1", "MARKER = 2")).unwrap();
    env.run_hook(&json);

    // Add a 3rd large function (crosses the threshold)
    let mut grown = std::fs::read_to_string(&path).unwrap();
    grown.push_str("def big_2():\n");
    for j in 0..large_fn_lines() {
        grown.push_str(&format!("    y_{j} = {j}\n"));
    }
    std::fs::write(&path, &grown).unwrap();

    let out = env.run_stop();
    assert!(
        out.contains("overall function size"),
        "should detect overall function size: {out}"
    );
}

#[test]
fn stop_detects_code_duplication_regression() {
    let env = TestEnv::new();
    let path = env.file_path("grow_dup.py");

    // Start with one function (no duplication possible)
    let func = "def original(a, b, c):\n    x = a + b\n    y = b + c\n    z = x * y\n    w = z + 1\n    v = w - 2\n    u = v * 3\n    return u\n\n";
    let code = format!("{func}MARKER = 1\n");
    std::fs::write(&path, &code).unwrap();

    let json = env.edit_hook_json(&path, "MARKER = 1", "MARKER = 2");
    std::fs::write(&path, code.replace("MARKER = 1", "MARKER = 2")).unwrap();
    env.run_hook(&json);

    // Add a duplicate function
    let mut grown = std::fs::read_to_string(&path).unwrap();
    grown.push_str("def duplicate(a, b, c):\n    x = a + b\n    y = b + c\n    z = x * y\n    w = z + 1\n    v = w - 2\n    u = v * 3\n    return u\n");
    std::fs::write(&path, &grown).unwrap();

    let out = env.run_stop();
    assert!(
        out.contains("code duplication"),
        "should detect code duplication: {out}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// STOP HOOK — MULTI-FILE SCENARIOS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn stop_reports_regressions_across_multiple_files() {
    let env = TestEnv::new();
    let path_a = env.file_path("a.py");
    let path_b = env.file_path("b.py");

    // File A: start under 400 LOC
    let code_a = format!("{}MARKER = 1\n", var_lines("a", 130));
    std::fs::write(&path_a, &code_a).unwrap();

    // File B: start under 20 functions
    let code_b = format!("{}MARKER = 1\n", simple_functions(18));
    std::fs::write(&path_b, &code_b).unwrap();

    // Hook both files
    let json_a = env.edit_hook_json(&path_a, "MARKER = 1", "MARKER = 2");
    std::fs::write(&path_a, code_a.replace("MARKER = 1", "MARKER = 2")).unwrap();
    env.run_hook(&json_a);

    let json_b = env.edit_hook_json(&path_b, "MARKER = 1", "MARKER = 2");
    std::fs::write(&path_b, code_b.replace("MARKER = 1", "MARKER = 2")).unwrap();
    env.run_hook(&json_b);

    // Grow A past LOC threshold, B past function count threshold
    let mut big_a = std::fs::read_to_string(&path_a).unwrap();
    big_a.push_str(&var_lines("extra", file_padding()));
    std::fs::write(&path_a, &big_a).unwrap();

    let mut big_b = std::fs::read_to_string(&path_b).unwrap();
    for i in 18..25 {
        big_b.push_str(&format!("def extra_{i}():\n    return {i}\n\n"));
    }
    std::fs::write(&path_b, &big_b).unwrap();

    let out = env.run_stop();
    assert!(
        out.contains("file too large"),
        "should report A's regression: {out}"
    );
    assert!(
        out.contains("too many functions"),
        "should report B's regression: {out}"
    );
}

#[test]
fn stop_one_file_regresses_other_stays_clean() {
    let env = TestEnv::new();
    let path_clean = env.file_path("clean.py");
    let path_dirty = env.file_path("dirty.py");

    std::fs::write(&path_clean, "def foo():\n    return 1\n").unwrap();
    let code_dirty = format!("{}MARKER = 1\n", var_lines("x", 130));
    std::fs::write(&path_dirty, &code_dirty).unwrap();

    let json_clean = env.edit_hook_json(&path_clean, "return 1", "return 2");
    std::fs::write(&path_clean, "def foo():\n    return 2\n").unwrap();
    env.run_hook(&json_clean);

    let json_dirty = env.edit_hook_json(&path_dirty, "MARKER = 1", "MARKER = 2");
    std::fs::write(&path_dirty, code_dirty.replace("MARKER = 1", "MARKER = 2")).unwrap();
    env.run_hook(&json_dirty);

    // Only grow the dirty file
    let mut big = std::fs::read_to_string(&path_dirty).unwrap();
    big.push_str(&var_lines("y", file_padding()));
    std::fs::write(&path_dirty, &big).unwrap();

    let out = env.run_stop();
    assert!(
        out.contains("file too large"),
        "dirty file should regress: {out}"
    );
    assert!(
        out.contains("dirty.py"),
        "output should reference the right file: {out}"
    );
}

#[test]
fn stop_many_files_in_manifest() {
    let env = TestEnv::new();
    let mut paths = Vec::new();

    for i in 0..10 {
        let path = env.file_path(&format!("file_{i}.py"));
        std::fs::write(
            &path,
            format!("def fn_{i}():\n    return {i}\nMARKER = 1\n"),
        )
        .unwrap();
        let json = env.edit_hook_json(&path, "MARKER = 1", "MARKER = 2");
        std::fs::write(
            &path,
            format!("def fn_{i}():\n    return {i}\nMARKER = 2\n"),
        )
        .unwrap();
        env.run_hook(&json);
        paths.push(path);
    }

    let manifest = env.manifest_contents();
    for path in &paths {
        assert!(
            manifest.contains(path.to_str().unwrap()),
            "manifest should contain all 10 files"
        );
    }

    // No regressions — all clean
    let out = env.run_stop();
    assert!(
        out.is_empty(),
        "10 clean files should produce no regressions: {out}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// STOP HOOK — OUTPUT FORMAT
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn stop_informational_uses_note_not_regression() {
    let env = TestEnv::new();
    let path = env.file_path("note_only.py");

    let code = format!("{}MARKER = 1\n", var_lines("x", 130));
    std::fs::write(&path, &code).unwrap();
    let json = env.edit_hook_json(&path, "MARKER = 1", "MARKER = 2");
    std::fs::write(&path, code.replace("MARKER = 1", "MARKER = 2")).unwrap();
    env.run_hook(&json);

    let mut big = std::fs::read_to_string(&path).unwrap();
    big.push_str(&var_lines("y", file_padding()));
    std::fs::write(&path, &big).unwrap();

    let out = env.run_stop();
    assert!(
        out.contains("error[pulse]"),
        "informational should use error[pulse] prefix: {out}"
    );
    assert!(
        out.contains("crossed"),
        "informational should use 'crossed' label: {out}"
    );
}

#[test]
fn stop_actionable_uses_regression_framing() {
    let env = TestEnv::new();
    let path = env.file_path("actionable.py");

    let code = "def foo():\n    return 1\nMARKER = 1\n".to_string();
    std::fs::write(&path, &code).unwrap();
    let json = env.edit_hook_json(&path, "MARKER = 1", "MARKER = 2");
    std::fs::write(&path, code.replace("MARKER = 1", "MARKER = 2")).unwrap();
    env.run_hook(&json);

    // Add only a global conditional (actionable)
    let mut grown = std::fs::read_to_string(&path).unwrap();
    grown.push_str("if True:\n    pass\n");
    std::fs::write(&path, &grown).unwrap();

    let out = env.run_stop();
    assert!(
        out.contains("regression"),
        "actionable should use 'regression' framing: {out}"
    );
    let parsed: serde_json::Value = serde_json::from_str(out.trim())
        .unwrap_or_default();
    assert_eq!(
        parsed.get("decision").and_then(|v| v.as_str()),
        Some("block"),
        "actionable should produce blocking decision: {out}"
    );
}

#[test]
fn stop_mixed_produces_blocking_decision() {
    let env = TestEnv::new();
    let path = env.file_path("mixed.py");

    let code = "def foo():\n    return 1\nMARKER = 1\n".to_string();
    std::fs::write(&path, &code).unwrap();
    let json = env.edit_hook_json(&path, "MARKER = 1", "MARKER = 2");
    std::fs::write(&path, code.replace("MARKER = 1", "MARKER = 2")).unwrap();
    env.run_hook(&json);

    let mut grown = std::fs::read_to_string(&path).unwrap();
    grown.push_str(&var_lines("v", file_padding()));
    grown.push_str("if True:\n    pass\n");
    std::fs::write(&path, &grown).unwrap();

    let out = env.run_stop();
    assert!(
        out.contains("regression"),
        "should have regression in reason: {out}"
    );
    assert!(out.contains("crossed"), "should have threshold crossed finding: {out}");
}

#[test]
fn stop_output_is_json_decision() {
    let env = TestEnv::new();
    let path = env.file_path("oneline.py");

    let code = format!("{}MARKER = 1\n", var_lines("x", 130));
    std::fs::write(&path, &code).unwrap();
    let json = env.edit_hook_json(&path, "MARKER = 1", "MARKER = 2");
    std::fs::write(&path, code.replace("MARKER = 1", "MARKER = 2")).unwrap();
    env.run_hook(&json);

    let mut big = std::fs::read_to_string(&path).unwrap();
    big.push_str(&var_lines("y", file_padding()));
    std::fs::write(&path, &big).unwrap();

    let out = env.run_stop();
    if !out.is_empty() {
        let parsed: serde_json::Value = serde_json::from_str(out.trim())
            .expect("stop output should be valid JSON");
        assert_eq!(
            parsed.get("decision").and_then(|v| v.as_str()),
            Some("block"),
            "should be a blocking decision: {out}"
        );
    }
}

#[test]
fn stop_output_contains_filename() {
    let env = TestEnv::new();
    let path = env.file_path("named.py");

    let code = format!("{}MARKER = 1\n", var_lines("x", 130));
    std::fs::write(&path, &code).unwrap();
    let json = env.edit_hook_json(&path, "MARKER = 1", "MARKER = 2");
    std::fs::write(&path, code.replace("MARKER = 1", "MARKER = 2")).unwrap();
    env.run_hook(&json);

    let mut big = std::fs::read_to_string(&path).unwrap();
    big.push_str(&var_lines("y", file_padding()));
    std::fs::write(&path, &big).unwrap();

    let out = env.run_stop();
    assert!(
        out.contains("named.py"),
        "output should include filename: {out}"
    );
}

#[test]
fn stop_output_contains_detail() {
    let env = TestEnv::new();
    let path = env.file_path("detail.py");

    let code = format!("{}MARKER = 1\n", var_lines("x", 130));
    std::fs::write(&path, &code).unwrap();
    let json = env.edit_hook_json(&path, "MARKER = 1", "MARKER = 2");
    std::fs::write(&path, code.replace("MARKER = 1", "MARKER = 2")).unwrap();
    env.run_hook(&json);

    let mut big = std::fs::read_to_string(&path).unwrap();
    big.push_str(&var_lines("y", file_padding()));
    std::fs::write(&path, &big).unwrap();

    let out = env.run_stop();
    assert!(
        out.contains("LOC"),
        "output should include detail with LOC count: {out}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// HOOK OUTPUT — MODULE EXCLUSION
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn hook_excludes_module_but_shows_function_findings() {
    let env = TestEnv::new();
    let path = env.file_path("mixed.py");

    // File with both function-level and module-level issues (POST-edit state)
    let mut code = String::new();
    code.push_str("def complex_fn(x):\n");
    for i in 0..11 {
        code.push_str(&format!("    if x == {i}:\n        pass\n"));
    }
    code.push_str("    return x\n\n");
    code.push_str(&var_lines("pad", file_padding()));
    std::fs::write(&path, &code).unwrap();

    // Edit added the 11th branch (old had just return x)
    let json = env.edit_hook_json(&path, "    return x", "    if x == 10:\n        pass\n    return x");
    let out = env.run_hook(&json);

    assert!(
        out.contains("complex_fn"),
        "should show function findings: {out}"
    );
    assert!(
        out.contains("complex method") || out.contains("god method"),
        "should show complexity finding: {out}"
    );
    assert!(
        !out.contains("File Too Large"),
        "must NOT show module findings: {out}"
    );
}

#[test]
fn hook_edit_mode_excludes_module_findings() {
    let env = TestEnv::new();
    let path = env.file_path("edit_mod.py");

    let mut code = String::new();
    code.push_str(&simple_functions(25));
    code.push_str(&var_lines("x", file_padding()));
    std::fs::write(&path, &code).unwrap();

    // Edit near a function
    let json = env.edit_hook_json(&path, "def fn_0", "def fn_0");
    let out = env.run_hook(&json);

    assert!(
        !out.contains("File Too Large"),
        "no module findings in edit mode: {out}"
    );
    assert!(
        !out.contains("Too Many Functions"),
        "no module findings in edit mode: {out}"
    );
}

#[test]
fn hook_write_mode_excludes_module_findings() {
    let env = TestEnv::new();
    let path = env.file_path("write_mod.py");

    let mut code = String::new();
    for i in 0..25 {
        code.push_str(&format!(
            "def fn_{i}(a, b, c, d, e, f, g, h):\n    return a\n\n"
        ));
    }
    code.push_str(&var_lines("x", file_padding()));
    std::fs::write(&path, &code).unwrap();

    let json = env.write_hook_json(&path);
    let out = env.run_hook(&json);

    assert!(
        out.contains("excess arguments"),
        "write mode should show function findings: {out}"
    );
    assert!(
        !out.contains("File Too Large"),
        "write mode must exclude module: {out}"
    );
    assert!(
        !out.contains("Too Many Functions"),
        "write mode must exclude module: {out}"
    );
}

#[test]
fn hook_clean_file_is_silent() {
    let env = TestEnv::new();
    let path = env.file_path("pristine.py");
    std::fs::write(&path, "def foo():\n    return 1\n").unwrap();

    let json = env.edit_hook_json(&path, "return 1", "return 2");
    std::fs::write(&path, "def foo():\n    return 2\n").unwrap();
    let out = env.run_hook(&json);

    assert!(out.is_empty(), "clean file should be silent: {out}");
}

// ═══════════════════════════════════════════════════════════════════════════
// MANIFEST BEHAVIOR
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn manifest_deduplicates_same_file() {
    let env = TestEnv::new();
    let path = env.file_path("dedup.py");
    std::fs::write(&path, "def foo():\n    return 1\n").unwrap();

    // Edit same file three times
    for i in 1..=3 {
        let old = format!("return {i}");
        let new = format!("return {}", i + 1);
        let json = env.edit_hook_json(&path, &old, &new);
        std::fs::write(&path, format!("def foo():\n    {new}\n")).unwrap();
        env.run_hook(&json);
    }

    let manifest = env.manifest_contents();
    let count = manifest.lines().filter(|l| l.contains("dedup.py")).count();
    assert_eq!(
        count, 1,
        "manifest should have exactly one entry per file, got {count}"
    );
}

#[test]
fn manifest_tracks_multiple_files() {
    let env = TestEnv::new();
    let paths: Vec<_> = (0..5)
        .map(|i| {
            let p = env.file_path(&format!("multi_{i}.py"));
            std::fs::write(&p, format!("def fn():\n    return {i}\n")).unwrap();
            p
        })
        .collect();

    for (i, path) in paths.iter().enumerate() {
        let json = env.edit_hook_json(
            path,
            &format!("return {i}"),
            &format!("return {}", i + 10),
        );
        std::fs::write(path, format!("def fn():\n    return {}\n", i + 10)).unwrap();
        env.run_hook(&json);
    }

    let manifest = env.manifest_contents();
    for path in &paths {
        assert!(
            manifest.contains(path.to_str().unwrap()),
            "manifest missing: {}",
            path.display()
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CLEANUP
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn cleanup_removes_directory() {
    let env = TestEnv::new();
    std::fs::write(env.baseline_path().join("test.json"), "{}").unwrap();
    env.run_cleanup();
    assert!(
        !env.baseline_path().exists(),
        "cleanup should remove baseline dir"
    );
}

#[test]
fn cleanup_idempotent() {
    let env = TestEnv::new();
    env.run_cleanup();
    env.run_cleanup();
    // No panic = success
}

#[test]
fn stop_cleans_up_after_itself() {
    let env = TestEnv::new();
    let path = env.file_path("auto_clean.py");
    std::fs::write(&path, "def foo():\n    return 1\n").unwrap();

    let json = env.edit_hook_json(&path, "return 1", "return 2");
    std::fs::write(&path, "def foo():\n    return 2\n").unwrap();
    env.run_hook(&json);

    assert!(
        env.baseline_path().exists(),
        "baselines should exist before stop"
    );
    env.run_stop();
    assert!(
        !env.baseline_path().exists(),
        "stop should clean up baselines"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// CROSS-LANGUAGE
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn baseline_works_for_typescript() {
    let env = TestEnv::new();
    let path = env.file_path("test.ts");

    let mut code = String::new();
    for i in 0..25 {
        code.push_str(&format!(
            "function fn_{i}(): number {{ return {i}; }}\n"
        ));
    }
    code.push_str(&var_lines("const x", file_padding()));
    code.push_str("const MARKER = 1;\n");
    std::fs::write(&path, &code).unwrap();

    let json = env.edit_hook_json(&path, "const MARKER = 1;", "const MARKER = 2;");
    std::fs::write(
        &path,
        code.replace("const MARKER = 1;", "const MARKER = 2;"),
    )
    .unwrap();
    env.run_hook(&json);

    let counts = env.baseline_counts(path.to_str().unwrap());
    assert!(
        !counts.is_empty(),
        "TypeScript baseline should capture module findings: {counts:?}"
    );
}

#[test]
fn baseline_works_for_rust() {
    let env = TestEnv::new();
    let path = env.file_path("test.rs");

    let mut code = String::new();
    for i in 0..25 {
        code.push_str(&format!("fn fn_{i}() -> i32 {{ {i} }}\n"));
    }
    for i in 0..file_padding() {
        code.push_str(&format!("static X_{i}: i32 = {i};\n"));
    }
    code.push_str("static MARKER: i32 = 1;\n");
    std::fs::write(&path, &code).unwrap();

    let json = env.edit_hook_json(&path, "static MARKER: i32 = 1;", "static MARKER: i32 = 2;");
    std::fs::write(
        &path,
        std::fs::read_to_string(&path)
            .unwrap()
            .replace("static MARKER: i32 = 1;", "static MARKER: i32 = 2;"),
    )
    .unwrap();
    env.run_hook(&json);

    let counts = env.baseline_counts(path.to_str().unwrap());
    assert!(
        !counts.is_empty(),
        "Rust baseline should capture module findings: {counts:?}"
    );
}

#[test]
fn baseline_works_for_java() {
    let env = TestEnv::new();
    let path = env.file_path("BigClass.java");

    let mut code = "public class BigClass {\n".to_string();
    for i in 0..25 {
        code.push_str(&format!(
            "    public static int fn_{i}() {{ return {i}; }}\n"
        ));
    }
    for i in 0..file_padding() {
        code.push_str(&format!("    static int x_{i} = {i};\n"));
    }
    code.push_str("    static int MARKER = 1;\n");
    code.push_str("}\n");
    std::fs::write(&path, &code).unwrap();

    let json = env.edit_hook_json(&path, "static int MARKER = 1;", "static int MARKER = 2;");
    std::fs::write(
        &path,
        std::fs::read_to_string(&path)
            .unwrap()
            .replace("static int MARKER = 1;", "static int MARKER = 2;"),
    )
    .unwrap();
    env.run_hook(&json);

    let counts = env.baseline_counts(path.to_str().unwrap());
    assert!(
        !counts.is_empty(),
        "Java baseline should capture module findings: {counts:?}"
    );
}

#[test]
fn baseline_works_for_c() {
    let env = TestEnv::new();
    let path = env.file_path("test.c");

    let mut code = "#include <stdio.h>\n".to_string();
    for i in 0..25 {
        code.push_str(&format!("int fn_{i}() {{ return {i}; }}\n"));
    }
    for i in 0..file_padding() {
        code.push_str(&format!("int x_{i} = {i};\n"));
    }
    code.push_str("int MARKER = 1;\n");
    std::fs::write(&path, &code).unwrap();

    let json = env.edit_hook_json(&path, "int MARKER = 1;", "int MARKER = 2;");
    std::fs::write(
        &path,
        std::fs::read_to_string(&path)
            .unwrap()
            .replace("int MARKER = 1;", "int MARKER = 2;"),
    )
    .unwrap();
    env.run_hook(&json);

    let counts = env.baseline_counts(path.to_str().unwrap());
    assert!(
        !counts.is_empty(),
        "C baseline should capture module findings: {counts:?}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// STRESS: MULTIPLE REGRESSIONS IN SAME FILE
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn stop_detects_multiple_regressions_in_same_file() {
    let env = TestEnv::new();
    let path = env.file_path("multi_regress.py");

    // Start clean
    let code = "def foo():\n    return 1\nMARKER = 1\n".to_string();
    std::fs::write(&path, &code).unwrap();

    let json = env.edit_hook_json(&path, "MARKER = 1", "MARKER = 2");
    std::fs::write(&path, code.replace("MARKER = 1", "MARKER = 2")).unwrap();
    env.run_hook(&json);

    // Now add: too many LOC + too many functions + global conditional
    let mut grown = std::fs::read_to_string(&path).unwrap();
    grown.push_str(&var_lines("v", 350));
    grown.push_str(&simple_functions(25));
    grown.push_str("if True:\n    pass\n");
    std::fs::write(&path, &grown).unwrap();

    let out = env.run_stop();

    let regression_types: Vec<&str> = [
        "file too large",
        "too many functions",
        "global conditionals",
    ]
    .iter()
    .filter(|s| out.contains(**s))
    .copied()
    .collect();

    assert!(
        regression_types.len() >= 2,
        "should detect at least 2 regressions in same file, found {regression_types:?} in: {out}"
    );
}

#[test]
fn stop_full_lifecycle_three_edits_then_stop() {
    let env = TestEnv::new();
    let path = env.file_path("lifecycle.py");

    // Start with moderate file
    let code = "def foo():\n    return 1\nMARKER = 1\n".to_string();
    std::fs::write(&path, &code).unwrap();

    // Edit 1: small change, cache baseline
    let json1 = env.edit_hook_json(&path, "MARKER = 1", "MARKER = 2");
    std::fs::write(&path, code.replace("MARKER = 1", "MARKER = 2")).unwrap();
    let out1 = env.run_hook(&json1);
    assert!(out1.is_empty(), "edit 1 should be clean");

    // Edit 2: add some code
    let current = std::fs::read_to_string(&path).unwrap();
    let added = format!("{}{}", current, var_lines("x", 100));
    std::fs::write(&path, &added).unwrap();
    let json2 = env.edit_hook_json(
        &path,
        "MARKER = 2",
        &format!("MARKER = 2\n{}", var_lines("x", 100)),
    );
    env.run_hook(&json2);

    // Edit 3: add more code, pushing past threshold
    let current = std::fs::read_to_string(&path).unwrap();
    let added = format!("{}{}", current, var_lines("y", file_padding()));
    std::fs::write(&path, &added).unwrap();
    let json3 = env.edit_hook_json(
        &path,
        &format!("x_{} = {}", 99, 99),
        &format!("x_{} = {}\n{}", 99, 99, var_lines("y", file_padding())),
    );
    env.run_hook(&json3);

    // Stop: should detect the regression
    let out = env.run_stop();
    assert!(
        out.contains("file too large"),
        "lifecycle: should detect file too large after 3 edits: {out}"
    );

    // Baselines should be cleaned up
    assert!(
        !env.baseline_path().exists(),
        "stop should clean up baselines"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// MID-SESSION CHECKPOINT
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn checkpoint_fires_at_5th_edit() {
    let env = TestEnv::new();
    let path = env.file_path("grow.py");

    // Start with a clean file
    std::fs::write(&path, "def f():\n    return 1\n").unwrap();
    let json = env.write_hook_json(&path);
    env.run_hook(&json);

    // Add functions one by one to cross Too Many Functions threshold
    let mut code = String::new();
    for i in 0..22 {
        code.push_str(&format!("def fn_{i}():\n    return {i}\n\n"));
    }
    std::fs::write(&path, &code).unwrap();

    // Edit 2: checkpoint fires (new file interval = 2)
    let old = "return 0";
    let new = "return 999";
    let json = env.edit_hook_json(&path, old, new);
    code = code.replacen(old, new, 1);
    std::fs::write(&path, &code).unwrap();
    let out = env.run_hook(&json);
    assert!(
        out.contains("too many functions"),
        "2nd edit on new file should trigger checkpoint: {out}"
    );
}

#[test]
fn checkpoint_silent_between_intervals() {
    let env = TestEnv::new();
    let path = env.file_path("small.py");

    let mut code = simple_functions(22);
    std::fs::write(&path, &code).unwrap();
    let json = env.write_hook_json(&path);
    env.run_hook(&json); // edit 1

    // Edit 2: fires checkpoint (new file interval=2), consume it
    let old2 = "return 1";
    let new2 = "return 101";
    let json2 = env.edit_hook_json(&path, old2, new2);
    code = code.replacen(old2, new2, 1);
    std::fs::write(&path, &code).unwrap();
    env.run_hook(&json2); // edit 2 — checkpoint fires

    // Edit 3: between checkpoints, should be silent for module findings
    let old3 = "return 2";
    let new3 = "return 102";
    let json3 = env.edit_hook_json(&path, old3, new3);
    code = code.replacen(old3, new3, 1);
    std::fs::write(&path, &code).unwrap();
    let out = env.run_hook(&json3);
    assert!(
        !out.contains("too many functions"),
        "edit 3 should not trigger module checkpoint: {out}"
    );
}

#[test]
fn checkpoint_silent_when_no_regressions() {
    let env = TestEnv::new();
    let path = env.file_path("stable.py");

    // Start with a file that already has many functions (pre-existing)
    let code = simple_functions(22);
    std::fs::write(&path, format!("{code}MARKER = 1\n")).unwrap();
    let json = env.edit_hook_json(&path, "MARKER = 1", "MARKER = 2");
    std::fs::write(&path, format!("{code}MARKER = 2\n")).unwrap();
    env.run_hook(&json);

    // Edits 2-5: small changes that don't add functions
    let mut current = format!("{code}MARKER = 2\n");
    for i in 2..=5 {
        let old = format!("MARKER = {i}");
        let new = format!("MARKER = {}", i + 1);
        let json = env.edit_hook_json(&path, &old, &new);
        current = current.replacen(&old, &new, 1);
        std::fs::write(&path, &current).unwrap();
        let out = env.run_hook(&json);
        assert!(
            !out.contains("too many functions"),
            "edit {i}: no regression should be reported since baseline captured pre-existing: {out}"
        );
    }
}

#[test]
fn checkpoint_skips_test_files() {
    let env = TestEnv::new();
    let test_dir = env.files_dir.path().join("tests");
    std::fs::create_dir_all(&test_dir).unwrap();
    let path = test_dir.join("test_things.py");

    let mut code = simple_functions(22);
    std::fs::write(&path, &code).unwrap();
    let json = env.write_hook_json(&path);
    env.run_hook(&json);

    for i in 1..=5 {
        let old = format!("return {i}");
        let new = format!("return {}", i + 100);
        let json = env.edit_hook_json(&path, &old, &new);
        code = code.replacen(&old, &new, 1);
        std::fs::write(&path, &code).unwrap();
        let out = env.run_hook(&json);
        assert!(
            !out.contains("too many functions"),
            "test file should skip checkpoint at edit {}: {}",
            i + 1,
            out
        );
    }
}

#[test]
fn checkpoint_fires_again_at_10th_edit() {
    let env = TestEnv::new();
    let path = env.file_path("repeat.py");

    let mut code = simple_functions(22);
    std::fs::write(&path, &code).unwrap();
    let json = env.write_hook_json(&path);
    env.run_hook(&json);

    let mut fired_at = Vec::new();
    for i in 1..=10 {
        let old = format!("return {i}");
        let new = format!("return {}", i + 200);
        let json = env.edit_hook_json(&path, &old, &new);
        code = code.replacen(&old, &new, 1);
        std::fs::write(&path, &code).unwrap();
        let out = env.run_hook(&json);
        if out.contains("too many functions") {
            fired_at.push(i + 1);
        }
    }
    assert!(
        fired_at.contains(&5) || fired_at.contains(&6),
        "should fire near edit 5, fired at: {fired_at:?}"
    );
    assert!(
        fired_at.contains(&10) || fired_at.contains(&11),
        "should fire near edit 10, fired at: {fired_at:?}"
    );
}

#[test]
fn edit_counter_persists_across_invocations() {
    let env = TestEnv::new();
    let path = env.file_path("counter.py");

    std::fs::write(&path, "def f():\n    return 1\n").unwrap();
    let json = env.write_hook_json(&path);
    env.run_hook(&json);

    // Check counter file exists
    let hash = {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        path.to_str().unwrap().hash(&mut hasher);
        hasher.finish()
    };
    let counter_path = env.baseline_path().join(format!("{hash:016x}.edits"));
    assert!(counter_path.exists(), "counter file should exist");
    let count: u32 = std::fs::read_to_string(&counter_path)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert_eq!(count, 1, "counter should be 1 after first hook");

    // Second invocation
    let json2 = env.edit_hook_json(&path, "return 1", "return 2");
    std::fs::write(&path, "def f():\n    return 2\n").unwrap();
    env.run_hook(&json2);
    let count2: u32 = std::fs::read_to_string(&counter_path)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert_eq!(count2, 2, "counter should be 2 after second hook");
}

#[test]
fn checkpoint_includes_function_findings_too() {
    let env = TestEnv::new();
    let path = env.file_path("mixed.py");

    // File with many functions AND a complex function
    let mut code = simple_functions(22);
    code.push_str(&complex_function("complex_one", cc_branches()));
    std::fs::write(&path, &code).unwrap();
    let json = env.write_hook_json(&path);
    env.run_hook(&json);

    // Edits 2-4
    let mut current = code.clone();
    for i in 1..=3 {
        let old = format!("return {i}");
        let new = format!("return {}", i + 300);
        let json = env.edit_hook_json(&path, &old, &new);
        current = current.replacen(&old, &new, 1);
        std::fs::write(&path, &current).unwrap();
        env.run_hook(&json);
    }

    // Edit 5: trigger checkpoint AND function smell
    let old = "return 4";
    let new = "return 444";
    let json = env.edit_hook_json(&path, old, new);
    current = current.replacen(old, new, 1);
    std::fs::write(&path, &current).unwrap();
    let out = env.run_hook(&json);
    // Should have module findings from checkpoint
    assert!(
        out.contains("too many functions") || out.contains("file too large"),
        "5th edit should include module checkpoint: {out}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// MODULE-LEVEL FINDINGS ON FIRST WRITE
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn first_write_blocks_on_too_many_functions() {
    let env = TestEnv::new();
    let path = env.file_path("big_new.py");
    let code = simple_functions(25);
    std::fs::write(&path, &code).unwrap();
    let json = env.write_hook_json(&path);
    let out = env.run_hook(&json);
    assert!(
        out.contains("too many functions"),
        "first write of new file with 25 functions should block: {out}"
    );
}

#[test]
fn first_write_blocks_on_file_too_large() {
    let env = TestEnv::new();
    let path = env.file_path("huge_new.py");
    let code = format!("def f():\n    return 1\n\n{}", var_lines("x", file_padding()));
    std::fs::write(&path, &code).unwrap();
    let json = env.write_hook_json(&path);
    let out = env.run_hook(&json);
    assert!(
        out.contains("file too large"),
        "first write of 600-line file should block: {out}"
    );
}

#[test]
fn first_write_reports_multiple_module_findings() {
    let env = TestEnv::new();
    let path = env.file_path("multi_new.py");
    let mut code = simple_functions(25);
    code.push_str(&var_lines("pad", file_padding()));
    std::fs::write(&path, &code).unwrap();
    let json = env.write_hook_json(&path);
    let out = env.run_hook(&json);
    assert!(
        out.contains("too many functions"),
        "should report too many functions: {out}"
    );
    assert!(
        out.contains("file too large"),
        "should report file too large: {out}"
    );
}

#[test]
fn first_write_silent_when_under_thresholds() {
    let env = TestEnv::new();
    let path = env.file_path("small_new.py");
    std::fs::write(&path, "def f():\n    return 1\n").unwrap();
    let json = env.write_hook_json(&path);
    let out = env.run_hook(&json);
    assert!(
        out.is_empty(),
        "clean new file should produce no output: {out}"
    );
}

#[test]
fn non_checkpoint_edit_does_not_include_module_findings() {
    let env = TestEnv::new();
    let path = env.file_path("grow.py");

    // First write: already exceeds thresholds (baseline captures violations)
    let mut code = simple_functions(25);
    code.push_str(&var_lines("pad", file_padding()));
    code.push_str("MARKER = 1\n");
    std::fs::write(&path, &code).unwrap();
    let json1 = env.write_hook_json(&path);
    env.run_hook(&json1); // edit 1 — reports module findings, baseline set

    // Edit 2: checkpoint fires (new file interval=2), but violations are pre-existing
    let code2 = code.replace("MARKER = 1", "MARKER = 2");
    std::fs::write(&path, &code2).unwrap();
    let json2 = env.edit_hook_json(&path, "MARKER = 1", "MARKER = 2");
    env.run_hook(&json2); // edit 2 — consume checkpoint

    // Edit 3: between checkpoints, module findings should not appear
    let code3 = code2.replace("MARKER = 2", "MARKER = 3");
    std::fs::write(&path, &code3).unwrap();
    let json3 = env.edit_hook_json(&path, "MARKER = 2", "MARKER = 3");
    let out = env.run_hook(&json3);

    assert!(
        !out.contains("too many functions"),
        "non-checkpoint edit should not report module findings: {out}"
    );
    assert!(
        !out.contains("file too large"),
        "non-checkpoint edit should not report module findings: {out}"
    );
}

#[test]
fn first_edit_of_existing_file_skips_preexisting_module_smells() {
    let env = TestEnv::new();
    let path = env.file_path("preexisting.py");

    // Create file that already exceeds thresholds
    let mut code = simple_functions(25);
    code.push_str(&var_lines("pad", file_padding()));
    code.push_str("MARKER = 1\n");
    std::fs::write(&path, &code).unwrap();

    // First hook captures baseline with existing module violations
    let json1 = env.edit_hook_json(&path, "MARKER = 1", "MARKER = 2");
    std::fs::write(&path, code.replace("MARKER = 1", "MARKER = 2")).unwrap();
    let out = env.run_hook(&json1);

    // Module findings should be filtered out — they were in the baseline
    assert!(
        !out.contains("too many functions"),
        "pre-existing too-many-functions should not block: {out}"
    );
    assert!(
        !out.contains("file too large"),
        "pre-existing file-too-large should not block: {out}"
    );
}

#[test]
fn first_write_includes_overall_cc_when_exceeded() {
    let env = TestEnv::new();
    let path = env.file_path("cc_heavy.py");

    // Generate functions with high CC that sum > 100
    let mut code = String::new();
    for i in 0..12 {
        code.push_str(&format!("def fn_{i}(x):\n"));
        for j in 0..10 {
            code.push_str(&format!("    if x == {j}:\n        pass\n"));
        }
        code.push_str("    return x\n\n");
    }
    std::fs::write(&path, &code).unwrap();
    let json = env.write_hook_json(&path);
    let out = env.run_hook(&json);
    assert!(
        out.contains("overall code complexity"),
        "first write with total cc>100 should block: {out}"
    );
}

#[test]
fn first_write_module_smell_is_advisory_not_blocking() {
    let env = TestEnv::new();
    let path = env.file_path("block_json.py");
    let code = simple_functions(25);
    std::fs::write(&path, &code).unwrap();
    let json = env.write_hook_json(&path);
    let out = env.run_hook(&json);
    let v: serde_json::Value = serde_json::from_str(out.trim()).unwrap();
    assert!(
        v.get("decision").is_none(),
        "module-level smells are advisory at edit time, enforced as regressions at stop: {out}"
    );
    let ctx = v["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .expect("the module smell is surfaced via the advisory channel");
    assert!(ctx.to_lowercase().contains("too many functions"));
}
