use std::process::Command;

fn cc_fn(name: &str, ifs: u32) -> String {
    let mut s = format!("def {name}(flag):\n");
    for _ in 0..ifs {
        s.push_str("    if flag:\n        x = 0\n");
    }
    s
}

fn padded_fn(name: &str, ifs: u32, loc: u32) -> String {
    let mut s = cc_fn(name, ifs);
    let used = 1 + 2 * ifs;
    for i in used..loc {
        s.push_str(&format!("    pad{i} = 0\n"));
    }
    s
}

fn cmp_fn(name: &str, op: &str) -> String {
    let mut s = format!("def {name}(a, b):\n");
    for _ in 0..12 {
        s.push_str(&format!("    if a {op} b:\n        a = 0\n"));
    }
    s
}

fn run_hook_edit(content: &str, old: &str, new: &str) -> String {
    let dir = tempfile::tempdir().unwrap();
    let baseline_dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("lib.py");
    std::fs::write(&path, content).unwrap();
    let input = serde_json::json!({
        "tool_input": {"file_path": path.to_str().unwrap(), "old_string": old, "new_string": new}
    })
    .to_string();
    let output = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["--hook"])
        .env("PULSE_BASELINE_DIR", baseline_dir.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
            child.wait_with_output()
        })
        .expect("failed to run pulse --hook");
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn partial_improvement_stays_suppressed() {
    let old = cc_fn("worker", 13);
    let new = cc_fn("worker", 11);
    let out = run_hook_edit(&new, &old, &new);
    assert!(!out.to_lowercase().contains("complex method"), "improving cc must not re-fire: {out}");
}

#[test]
fn worsening_pre_existing_smell_refires() {
    let old = cc_fn("worker", 13);
    let new = cc_fn("worker", 15);
    let out = run_hook_edit(&new, &old, &new);
    assert!(out.to_lowercase().contains("complex method"), "worsening cc must re-fire: {out}");
}

#[test]
fn unchanged_pre_existing_smell_stays_suppressed() {
    let body = cc_fn("worker", 13);
    let content = format!("import os\n\n{body}");
    let out = run_hook_edit(&content, "import os", "import os");
    assert!(!out.to_lowercase().contains("complex method"), "unchanged smell must stay suppressed: {out}");
}

#[test]
fn rename_without_change_stays_suppressed() {
    let new = cc_fn("renamed_worker", 13);
    let out = run_hook_edit(&new, "def worker", "def renamed_worker");
    assert!(!out.to_lowercase().contains("complex method"), "rename with identical body must stay suppressed: {out}");
}

#[test]
fn rename_and_worsen_fires() {
    let old = cc_fn("worker", 13);
    let new = cc_fn("renamed_worker", 15);
    let out = run_hook_edit(&new, &old, &new);
    assert!(out.to_lowercase().contains("complex method"), "renamed and worsened function must fire: {out}");
}

#[test]
fn rename_with_structural_body_change_refires() {
    let old = cmp_fn("worker", ">");
    let new = cmp_fn("renamed_worker", "<");
    let out = run_hook_edit(&new, &old, &new);
    assert!(
        out.to_lowercase().contains("complex method"),
        "a renamed function whose body structure changed no longer matches its grandfathered baseline, so the still-present finding re-surfaces: {out}"
    );
}

#[test]
fn newly_introduced_smell_fires() {
    let body = cc_fn("worker", 13);
    let new_region = format!("anchor = 1\n\n{body}");
    let content = new_region.clone();
    let out = run_hook_edit(&content, "anchor = 1\n", &new_region);
    assert!(out.to_lowercase().contains("complex method"), "new smelly function must fire: {out}");
}

#[test]
fn god_method_shrunk_to_complex_method_stays_suppressed() {
    let old = padded_fn("worker", 13, 70);
    let new = padded_fn("worker", 13, 30);
    let out = run_hook_edit(&new, &old, &new);
    assert!(
        !out.to_lowercase().contains("complex method") && !out.to_lowercase().contains("god method"),
        "shrinking a god method to a complex method is an improvement: {out}"
    );
}

#[test]
fn empty_handler_count_increase_refires() {
    let old = "def worker(flag):\n    try:\n        x = 1\n    except ValueError:\n        pass\n";
    let new = "def worker(flag):\n    try:\n        x = 1\n    except ValueError:\n        pass\n    try:\n        y = 2\n    except KeyError:\n        pass\n";
    let out = run_hook_edit(new, old, new);
    assert!(out.to_lowercase().contains("empty error handler"), "second empty handler must fire: {out}");
}

#[test]
fn empty_handler_count_unchanged_stays_suppressed() {
    let body = "def worker(flag):\n    try:\n        x = 1\n    except ValueError:\n        pass\n";
    let content = format!("import os\n\n{body}");
    let out = run_hook_edit(&content, "import os", "import os");
    assert!(!out.to_lowercase().contains("empty error handler"), "unchanged handler count must stay suppressed: {out}");
}

fn god_fn_with_nesting(name: &str, loc: u32) -> String {
    let mut s = format!("def {name}(flag):\n");
    for d in 0..8 {
        s.push_str(&"    ".repeat(d + 1));
        s.push_str("if flag:\n");
    }
    s.push_str(&"    ".repeat(9));
    s.push_str("x = 0\n");
    for i in 10..loc {
        s.push_str(&format!("    pad{i} = 0\n"));
    }
    s
}

#[test]
fn god_method_subsumes_structural_findings_in_hook_output() {
    let body = god_fn_with_nesting("monster", 70);
    let new_region = format!("anchor = 1\n\n{body}");
    let out = run_hook_edit(&new_region, "anchor = 1\n", &new_region).to_lowercase();
    assert!(out.contains("god method"), "god method must fire: {out}");
    assert!(!out.contains("deep nested complexity"), "nesting is subsumed by god method: {out}");
    assert!(!out.contains("nested conditional chunks"), "chunks are subsumed by god method: {out}");
}

#[test]
fn structural_findings_survive_without_god_method() {
    let mut body = String::from("def deep(flag):\n");
    for d in 0..5 {
        body.push_str(&"    ".repeat(d + 1));
        body.push_str("if flag:\n");
    }
    body.push_str(&"    ".repeat(6));
    body.push_str("x = 0\n");
    let new_region = format!("anchor = 1\n\n{body}");
    let out = run_hook_edit(&new_region, "anchor = 1\n", &new_region).to_lowercase();
    assert!(out.contains("deep nested complexity"), "nesting fires when no god method subsumes it: {out}");
    assert!(!out.contains("god method"), "fixture must not be a god method: {out}");
}

#[test]
fn complex_method_grown_into_god_method_refires() {
    let old = padded_fn("worker", 13, 30);
    let new = padded_fn("worker", 13, 70);
    let out = run_hook_edit(&new, &old, &new);
    assert!(out.to_lowercase().contains("god method"), "growing into a god method must re-fire: {out}");
}

#[test]
fn god_method_does_not_subsume_findings_in_other_functions() {
    let god = god_fn_with_nesting("monster", 70);
    let nested = god_fn_with_nesting("nester", 12);
    let new_region = format!("anchor = 1\n\n{god}\n{nested}");
    let out = run_hook_edit(&new_region, "anchor = 1\n", &new_region).to_lowercase();
    assert!(out.contains("god method"), "god method must fire for monster: {out}");
    assert!(
        out.contains("deep nested complexity") || out.contains("nested conditional chunks"),
        "structural findings on a different function must not be subsumed: {out}"
    );
}
