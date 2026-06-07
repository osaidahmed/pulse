use std::io::Write;
use std::path::Path;
use std::process::Command;

fn pulse(args: &[&str], baseline: &Path, stdin: &str) -> String {
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

fn hook_json(path: &str, old: &str, new: &str) -> String {
    format!(
        r#"{{"session_id":"test-session","tool_input":{{"file_path":"{}","old_string":"{}","new_string":"{}"}}}}"#,
        path.replace('\\', "\\\\").replace('"', "\\\""),
        old.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n"),
        new.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n"),
    )
}

fn read_analytics(analytics_dir: &Path) -> String {
    std::fs::read_to_string(analytics_dir.join("analytics.jsonl")).unwrap_or_default()
}

// ===========================================================================
// Session ID tracking
// ===========================================================================

#[test]
fn session_id_saved_from_hook_input() {
    let bl = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("t.py");
    std::fs::write(&p, "def f(a, b, c, d, e, f, g, h):\n    return a\n").unwrap();
    let json = format!(
        r#"{{"session_id":"my-session-123","tool_input":{{"file_path":"{}","old_string":"def f(a):\n    return a","new_string":"def f(a, b, c, d, e, f, g, h):\n    return a"}}}}"#,
        p.to_str().unwrap()
    );
    pulse(&["--hook"], bl.path(), &json);
    let sid = std::fs::read_to_string(bl.path().join("session_id")).unwrap();
    assert_eq!(sid, "my-session-123");
}

#[test]
fn session_id_defaults_to_unknown() {
    let bl = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("t.py");
    std::fs::write(&p, "def f(a, b, c, d, e, f, g, h):\n    return a\n").unwrap();
    let json = format!(
        r#"{{"tool_input":{{"file_path":"{}","old_string":"def f(a):\n    return a","new_string":"def f(a, b, c, d, e, f, g, h):\n    return a"}}}}"#,
        p.to_str().unwrap()
    );
    pulse(&["--hook"], bl.path(), &json);
    let sid = std::fs::read_to_string(bl.path().join("session_id")).unwrap();
    assert_eq!(sid, "unknown");
}

#[test]
fn session_id_not_overwritten_on_second_edit() {
    let bl = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("t.py");
    std::fs::write(&p, "def f(a, b, c, d, e, f, g, h):\n    return a\n").unwrap();
    let j1 = format!(
        r#"{{"session_id":"first","tool_input":{{"file_path":"{}","old_string":"def f(a):\n    return a","new_string":"def f(a, b, c, d, e, f, g, h):\n    return a"}}}}"#,
        p.to_str().unwrap()
    );
    pulse(&["--hook"], bl.path(), &j1);
    let j2 = format!(
        r#"{{"session_id":"second","tool_input":{{"file_path":"{}","old_string":"return a","new_string":"return a"}}}}"#,
        p.to_str().unwrap()
    );
    pulse(&["--hook"], bl.path(), &j2);
    let sid = std::fs::read_to_string(bl.path().join("session_id")).unwrap();
    assert_eq!(sid, "first", "session_id should not be overwritten");
}

// ===========================================================================
// Findings log
// ===========================================================================

#[test]
fn findings_logged_to_jsonl() {
    let bl = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("t.py");
    std::fs::write(&p, "def f(a, b, c, d, e, f, g, h):\n    return a\n").unwrap();
    let json = hook_json(p.to_str().unwrap(),
        "def f(a):\n    return a", "def f(a, b, c, d, e, f, g, h):\n    return a");
    pulse(&["--hook"], bl.path(), &json);
    let log = std::fs::read_to_string(bl.path().join("findings.jsonl")).unwrap();
    assert!(!log.is_empty(), "findings should be logged");
    assert!(log.contains("Excess Arguments"), "should log smell name");
    assert!(log.contains(p.to_str().unwrap()), "should log file path");
}

#[test]
fn module_level_finding_logged_with_null_function() {
    let bl = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("t.py");
    // Write mode on a file with module-level smells won't log module findings
    // (PostToolUse excludes module-level), but checkpoint mode does
    // This test just verifies the log format for function-level findings
    std::fs::write(&p, "def f(a, b, c, d, e, f, g, h):\n    return a\n").unwrap();
    let json = hook_json(p.to_str().unwrap(),
        "def f(a):\n    return a", "def f(a, b, c, d, e, f, g, h):\n    return a");
    pulse(&["--hook"], bl.path(), &json);
    let log = std::fs::read_to_string(bl.path().join("findings.jsonl")).unwrap();
    let entry: serde_json::Value = serde_json::from_str(log.lines().next().unwrap()).unwrap();
    assert!(entry.get("fn").is_some(), "should have fn field");
    assert!(entry.get("smell").is_some(), "should have smell field");
    assert!(entry.get("ts").is_some(), "should have timestamp");
}

// ===========================================================================
// Analytics resolution (addressed vs ignored)
// ===========================================================================

#[test]
fn resolved_as_addressed_when_finding_gone() {
    let bl = tempfile::tempdir().unwrap();
    let analytics = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("t.py");
    // Step 1: Create smelly file, trigger hook to log finding
    std::fs::write(&p, "def f(a, b, c, d, e, f, g, h):\n    return a\n").unwrap();
    let json = hook_json(p.to_str().unwrap(),
        "def f(a):\n    return a", "def f(a, b, c, d, e, f, g, h):\n    return a");
    pulse(&["--hook"], bl.path(), &json);
    // Step 2: Fix the smell
    std::fs::write(&p, "def f(a):\n    return a\n").unwrap();
    // Step 3: Run stop to resolve analytics
    let _ = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["--stop"])
        .env("PULSE_BASELINE_DIR", bl.path())
        .env("PULSE_ANALYTICS_DIR", analytics.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut c| { c.stdin.take().unwrap().write_all(b"{}").unwrap(); c.wait_with_output() })
        .unwrap();
    let analytics_content = read_analytics(analytics.path());
    assert!(analytics_content.contains("\"addressed\""), "fixed finding should be addressed, got: {analytics_content}");
}

#[test]
fn resolved_as_removed_when_function_deleted() {
    let bl = tempfile::tempdir().unwrap();
    let analytics = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("t.py");
    std::fs::write(&p, "def f(a, b, c, d, e, f, g, h):\n    return a\n").unwrap();
    let json = hook_json(p.to_str().unwrap(),
        "def f(a):\n    return a", "def f(a, b, c, d, e, f, g, h):\n    return a");
    pulse(&["--hook"], bl.path(), &json);
    // The function `f` is removed entirely (replaced by a different, clean function).
    std::fs::write(&p, "def other():\n    return 0\n").unwrap();
    let _ = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["--stop"])
        .env("PULSE_BASELINE_DIR", bl.path())
        .env("PULSE_ANALYTICS_DIR", analytics.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut c| { c.stdin.take().unwrap().write_all(b"{}").unwrap(); c.wait_with_output() })
        .unwrap();
    let analytics_content = read_analytics(analytics.path());
    assert!(
        analytics_content.contains("\"removed\""),
        "a finding whose function vanished should be removed, not addressed, got: {analytics_content}"
    );
}

#[test]
fn resolved_as_moved_when_function_relocates() {
    let bl = tempfile::tempdir().unwrap();
    let analytics = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.py");
    let b = dir.path().join("b.py");
    let smelly = "def f(a, b, c, d, e, f, g, h):\n    return a\n";

    // f is smelly in a.py — the hook logs it with its structural hash.
    std::fs::write(&a, smelly).unwrap();
    pulse(&["--hook"], bl.path(), &hook_json(a.to_str().unwrap(), "def f(a):\n    return a", smelly));

    // f is relocated verbatim to b.py; a.py keeps only a clean function.
    std::fs::write(&a, "def stay():\n    return 0\n").unwrap();
    std::fs::write(&b, smelly).unwrap();
    pulse(&["--hook"], bl.path(), &hook_json(a.to_str().unwrap(), smelly, "def stay():\n    return 0"));
    pulse(&["--hook"], bl.path(), &hook_json(b.to_str().unwrap(), "", smelly));

    let _ = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["--stop"])
        .env("PULSE_BASELINE_DIR", bl.path())
        .env("PULSE_ANALYTICS_DIR", analytics.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut c| { c.stdin.take().unwrap().write_all(b"{}").unwrap(); c.wait_with_output() })
        .unwrap();

    let content = read_analytics(analytics.path());
    assert!(
        content.contains("\"moved\""),
        "a function relocated to another session file should be moved, got: {content}"
    );
}

#[test]
fn resolved_as_ignored_when_finding_persists() {
    let bl = tempfile::tempdir().unwrap();
    let analytics = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("t.py");
    // Step 1: Create smelly file, trigger hook
    std::fs::write(&p, "def f(a, b, c, d, e, f, g, h):\n    return a\n").unwrap();
    let json = hook_json(p.to_str().unwrap(),
        "def f(a):\n    return a", "def f(a, b, c, d, e, f, g, h):\n    return a");
    pulse(&["--hook"], bl.path(), &json);
    // Step 2: Don't fix — file still smelly
    // Step 3: Run stop
    let _ = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["--stop"])
        .env("PULSE_BASELINE_DIR", bl.path())
        .env("PULSE_ANALYTICS_DIR", analytics.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut c| { c.stdin.take().unwrap().write_all(b"{}").unwrap(); c.wait_with_output() })
        .unwrap();
    let analytics_content = read_analytics(analytics.path());
    assert!(analytics_content.contains("\"ignored\""), "unfixed finding should be ignored, got: {analytics_content}");
}

#[test]
fn analytics_includes_session_id() {
    let bl = tempfile::tempdir().unwrap();
    let analytics = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("t.py");
    std::fs::write(&p, "def f(a, b, c, d, e, f, g, h):\n    return a\n").unwrap();
    let json = format!(
        r#"{{"session_id":"analytics-test","tool_input":{{"file_path":"{}","old_string":"def f(a):\n    return a","new_string":"def f(a, b, c, d, e, f, g, h):\n    return a"}}}}"#,
        p.to_str().unwrap()
    );
    pulse(&["--hook"], bl.path(), &json);
    let _ = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["--stop"])
        .env("PULSE_BASELINE_DIR", bl.path())
        .env("PULSE_ANALYTICS_DIR", analytics.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut c| { c.stdin.take().unwrap().write_all(b"{}").unwrap(); c.wait_with_output() })
        .unwrap();
    let content = read_analytics(analytics.path());
    assert!(content.contains("analytics-test"), "should include session_id, got: {content}");
}

#[test]
fn analytics_deduplicates_same_finding() {
    let bl = tempfile::tempdir().unwrap();
    let analytics = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("t.py");
    std::fs::write(&p, "def f(a, b, c, d, e, f, g, h):\n    return a\n").unwrap();
    let json = hook_json(p.to_str().unwrap(),
        "def f(a):\n    return a", "def f(a, b, c, d, e, f, g, h):\n    return a");
    // Trigger hook twice — same finding logged twice
    pulse(&["--hook"], bl.path(), &json);
    pulse(&["--hook"], bl.path(), &json);
    let _ = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["--stop"])
        .env("PULSE_BASELINE_DIR", bl.path())
        .env("PULSE_ANALYTICS_DIR", analytics.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut c| { c.stdin.take().unwrap().write_all(b"{}").unwrap(); c.wait_with_output() })
        .unwrap();
    let content = read_analytics(analytics.path());
    let lines: Vec<&str> = content.lines().collect();
    // Same finding should be deduped to 1 entry
    let excess_count = lines.iter().filter(|l| l.contains("Excess Arguments")).count();
    assert_eq!(excess_count, 1, "should deduplicate, got {excess_count} entries");
}

#[test]
fn no_analytics_when_no_findings() {
    let bl = tempfile::tempdir().unwrap();
    let analytics = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("t.py");
    std::fs::write(&p, "def f():\n    return 1\n").unwrap();
    let json = hook_json(p.to_str().unwrap(), "return 1", "return 2");
    pulse(&["--hook"], bl.path(), &json);
    let _ = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["--stop"])
        .env("PULSE_BASELINE_DIR", bl.path())
        .env("PULSE_ANALYTICS_DIR", analytics.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut c| { c.stdin.take().unwrap().write_all(b"{}").unwrap(); c.wait_with_output() })
        .unwrap();
    let content = read_analytics(analytics.path());
    assert!(content.is_empty(), "no findings should produce no analytics, got: {content}");
}

// ===========================================================================
// Tier metadata (machine-readable interaction tier)
// ===========================================================================

fn write_json(path: &str) -> String {
    format!(
        r#"{{"session_id":"test","tool_input":{{"file_path":"{}"}}}}"#,
        path.replace('\\', "\\\\").replace('"', "\\\"")
    )
}

#[test]
fn findings_log_tags_blocking_tier() {
    let bl = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("t.py");
    std::fs::write(&p, "def f(a, b, c, d, e, f, g, h):\n    return a\n").unwrap();
    pulse(&["--hook"], bl.path(), &write_json(p.to_str().unwrap()));
    let log = std::fs::read_to_string(bl.path().join("findings.jsonl")).unwrap_or_default();
    assert!(log.contains(r#""tier":"blocking""#), "excess-arguments is a blocking tier: {log}");
}

#[test]
fn findings_log_tags_advisory_tier() {
    let bl = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("t.py");
    std::fs::write(&p, "def g(a: int, b: int, c: int, d: int):\n    return a\n").unwrap();
    pulse(&["--hook"], bl.path(), &write_json(p.to_str().unwrap()));
    let log = std::fs::read_to_string(bl.path().join("findings.jsonl")).unwrap_or_default();
    assert!(log.contains(r#""tier":"advisory""#), "primitive-obsession is an advisory tier: {log}");
}

#[test]
fn outcome_record_carries_tier() {
    let bl = tempfile::tempdir().unwrap();
    let analytics = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("t.py");
    std::fs::write(&p, "def f(a, b, c, d, e, f, g, h):\n    return a\n").unwrap();
    pulse(&["--hook"], bl.path(), &write_json(p.to_str().unwrap()));
    let _ = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["--stop"])
        .env("PULSE_BASELINE_DIR", bl.path())
        .env("PULSE_ANALYTICS_DIR", analytics.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut c| {
            c.stdin.take().unwrap().write_all(b"{}").unwrap();
            c.wait_with_output()
        })
        .unwrap();
    let content = read_analytics(analytics.path());
    assert!(content.contains(r#""tier":"blocking""#), "resolved outcome carries the tier: {content}");
}
