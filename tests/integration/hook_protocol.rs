use pulse::hook::{ClaudeCodeProtocol, GenericProtocol, HookProtocol};
use std::io::Write;
use std::process::Command;

fn smelly_ruby() -> &'static str {
    concat!("def f(x)\n", "  begin\n", "    Integer(x)\n", "  rescue\n", "  end\n", "end\n",)
}

#[test]
fn claude_parse_reads_tool_input_envelope() {
    let raw = r#"{"session_id":"s1","tool_input":{"file_path":"/tmp/a.py","old_string":"x","new_string":"y"}}"#;
    let h = ClaudeCodeProtocol.parse(raw).expect("parse");
    assert_eq!(h.file_path, "/tmp/a.py");
    assert_eq!(h.session_id.as_deref(), Some("s1"));
    assert_eq!(h.old_string.as_deref(), Some("x"));
    assert_eq!(h.new_string.as_deref(), Some("y"));
}

#[test]
fn claude_render_block_carries_decision_schema() {
    let out = ClaudeCodeProtocol.render_block("too complex", Some("note"));
    let v: serde_json::Value = serde_json::from_str(&out).expect("json");
    assert_eq!(v["decision"], "block");
    assert_eq!(v["reason"], "too complex");
    assert_eq!(v["hookSpecificOutput"]["hookEventName"], "PostToolUse");
    assert_eq!(v["hookSpecificOutput"]["additionalContext"], "note");
}

#[test]
fn claude_render_advisory_has_no_decision() {
    let out = ClaudeCodeProtocol.render_advisory("heads up");
    let v: serde_json::Value = serde_json::from_str(&out).expect("json");
    assert!(v.get("decision").is_none());
    assert_eq!(v["hookSpecificOutput"]["additionalContext"], "heads up");
}

#[test]
fn generic_parse_reads_flat_payload() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.py");
    std::fs::write(&path, "line one\nline two\nline three\n").unwrap();
    let raw =
        format!(r#"{{"file_path":"{}","session_id":"g1","original_file":"old","edit_range":[2,3]}}"#, path.display());
    let h = GenericProtocol.parse(&raw).expect("parse");
    assert_eq!(h.session_id.as_deref(), Some("g1"));
    assert_eq!(h.original_file.as_deref(), Some("old"));
    assert_eq!(h.edit_range, Some((2, 3)));
    assert!(h.edit_byte_range.is_some());
}

#[test]
fn generic_parse_falls_back_to_string_search() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.py");
    std::fs::write(&path, "alpha\nbeta\ngamma\n").unwrap();
    let raw = format!(r#"{{"file_path":"{}","old_string":"old beta","new_string":"beta"}}"#, path.display());
    let h = GenericProtocol.parse(&raw).expect("parse");
    assert_eq!(h.edit_range, Some((2, 2)));
}

#[test]
fn generic_render_block_uses_status_schema() {
    let out = GenericProtocol.render_block("reason", None);
    let v: serde_json::Value = serde_json::from_str(&out).expect("json");
    assert_eq!(v["status"], "block");
    assert_eq!(v["reason"], "reason");
    assert!(v.get("decision").is_none());
}

fn run_hook_with_env(payload: &str, env: &[(&str, &str)]) -> String {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_pulse"));
    cmd.args(["--hook"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    for (k, v) in env {
        cmd.env(k, v);
    }
    let output = cmd
        .spawn()
        .and_then(|mut child| {
            child.stdin.take().unwrap().write_all(payload.as_bytes()).unwrap();
            child.wait_with_output()
        })
        .expect("failed to run pulse --hook");
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn generic_protocol_end_to_end_emits_status_block() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("svc.rb");
    std::fs::write(&path, smelly_ruby()).unwrap();
    let payload = format!(r#"{{"file_path":"{}"}}"#, path.display());
    let out = run_hook_with_env(&payload, &[("PULSE_PROTOCOL", "generic")]);
    let v: serde_json::Value = serde_json::from_str(&out).expect("json output: {out}");
    assert_eq!(v["status"], "block", "got: {out}");
    assert!(v.get("decision").is_none(), "generic schema must not carry claude fields: {out}");
}

#[test]
fn default_protocol_remains_claude_code() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("svc.rb");
    std::fs::write(&path, smelly_ruby()).unwrap();
    let payload = format!(r#"{{"tool_input":{{"file_path":"{}"}}}}"#, path.display());
    let out = run_hook_with_env(&payload, &[]);
    let v: serde_json::Value = serde_json::from_str(&out).expect("json output: {out}");
    assert_eq!(v["decision"], "block", "got: {out}");
}
