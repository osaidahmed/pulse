use std::process::Command;

fn run_setup(home: &std::path::Path) -> (String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .arg("setup")
        .env("HOME", home)
        .output()
        .expect("failed to run pulse setup");
    (
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

fn read_settings(home: &std::path::Path) -> serde_json::Value {
    let path = home.join(".claude/settings.json");
    let content = std::fs::read_to_string(path).unwrap();
    serde_json::from_str(&content).unwrap()
}

fn read_claude_md(home: &std::path::Path) -> String {
    std::fs::read_to_string(home.join(".claude/CLAUDE.md")).unwrap()
}

fn has_command_in_event(settings: &serde_json::Value, event: &str, command: &str) -> bool {
    settings["hooks"][event]
        .as_array()
        .unwrap()
        .iter()
        .any(|group| {
            group["hooks"]
                .as_array()
                .unwrap()
                .iter()
                .any(|h| h["command"].as_str() == Some(command))
        })
}

// ── Settings from scratch ──

#[test]
fn setup_creates_settings_from_scratch() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".claude")).unwrap();

    run_setup(dir.path());

    let settings = read_settings(dir.path());
    assert!(has_command_in_event(&settings, "PostToolUse", "pulse --hook"));
    assert!(has_command_in_event(&settings, "Stop", "pulse --stop"));
    assert!(has_command_in_event(&settings, "SessionStart", "pulse --cleanup"));
}

// ── Merge with existing hooks ──

#[test]
fn setup_merges_with_existing_hooks() {
    let dir = tempfile::tempdir().unwrap();
    let claude = dir.path().join(".claude");
    std::fs::create_dir_all(&claude).unwrap();

    let existing = serde_json::json!({
        "hooks": {
            "PostToolUse": [{
                "matcher": "Edit|Write",
                "hooks": [{"type": "command", "command": "other-tool --check", "async": true}]
            }]
        }
    });
    std::fs::write(
        claude.join("settings.json"),
        serde_json::to_string_pretty(&existing).unwrap(),
    )
    .unwrap();

    run_setup(dir.path());

    let settings = read_settings(dir.path());
    assert!(has_command_in_event(&settings, "PostToolUse", "pulse --hook"));
    assert!(has_command_in_event(&settings, "PostToolUse", "other-tool --check"));
    assert!(has_command_in_event(&settings, "Stop", "pulse --stop"));
}

// ── Idempotent hooks ──

#[test]
fn setup_idempotent_hooks() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".claude")).unwrap();

    run_setup(dir.path());
    let (_, stderr) = run_setup(dir.path());

    assert!(stderr.contains("already configured"), "second run should say already configured: {stderr}");

    let settings = read_settings(dir.path());
    let post_hooks: Vec<&serde_json::Value> = settings["hooks"]["PostToolUse"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|g| g["hooks"].as_array().unwrap())
        .filter(|h| h["command"].as_str() == Some("pulse --hook"))
        .collect();
    assert_eq!(post_hooks.len(), 1, "should not duplicate pulse hooks");
}

// ── CLAUDE.md from scratch ──

#[test]
fn setup_creates_claude_md_from_scratch() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".claude")).unwrap();

    run_setup(dir.path());

    let md = read_claude_md(dir.path());
    assert!(md.contains("# Pulse"));
    assert!(md.contains("pulse budget"));
    assert!(md.contains("error[pulse]"));
}

// ── CLAUDE.md appends to existing ──

#[test]
fn setup_appends_to_existing_claude_md() {
    let dir = tempfile::tempdir().unwrap();
    let claude = dir.path().join(".claude");
    std::fs::create_dir_all(&claude).unwrap();

    std::fs::write(claude.join("CLAUDE.md"), "# My Project\n\nExisting content.\n").unwrap();

    run_setup(dir.path());

    let md = read_claude_md(dir.path());
    assert!(md.starts_with("# My Project"), "original content preserved");
    assert!(md.contains("Existing content."), "original content preserved");
    assert!(md.contains("# Pulse"), "pulse section added");
}

// ── CLAUDE.md idempotent ──

#[test]
fn setup_idempotent_claude_md() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".claude")).unwrap();

    run_setup(dir.path());
    run_setup(dir.path());

    let md = read_claude_md(dir.path());
    let pulse_count = md.matches("\n# Pulse").count() + usize::from(md.starts_with("# Pulse"));
    assert_eq!(pulse_count, 1, "should have exactly one Pulse section");
}

// ── Threshold reference matches defaults ──

#[test]
fn setup_threshold_reference_matches_defaults() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".claude")).unwrap();

    run_setup(dir.path());

    let md = read_claude_md(dir.path());
    let t = pulse::thresholds::Thresholds::default();
    assert!(md.contains(&format!("cc>={} warning", t.cc_warning)));
    assert!(md.contains(&format!("args>{}", t.arg_max)));
    assert!(md.contains(&format!("functions>{}", t.file_function_count)));
    assert!(md.contains(&format!("struct_fields>{}", t.max_struct_fields)));
}

// ── Settings is valid JSON ──

#[test]
fn setup_settings_valid_json() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".claude")).unwrap();

    run_setup(dir.path());

    let content = std::fs::read_to_string(dir.path().join(".claude/settings.json")).unwrap();
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&content);
    assert!(parsed.is_ok(), "settings.json should be valid JSON: {content}");
}

// ── Complete flow ──

#[test]
fn setup_complete_flow() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".claude")).unwrap();

    let (_, stderr1) = run_setup(dir.path());
    assert!(stderr1.contains("configured"), "first run should configure");

    let settings = read_settings(dir.path());
    assert!(has_command_in_event(&settings, "PostToolUse", "pulse --hook"));
    assert!(has_command_in_event(&settings, "Stop", "pulse --stop"));
    assert!(has_command_in_event(&settings, "SessionStart", "pulse --cleanup"));

    let md = read_claude_md(dir.path());
    assert!(md.contains("# Pulse"));

    let (_, stderr2) = run_setup(dir.path());
    assert!(stderr2.contains("already configured"), "second run should be no-op");
}
