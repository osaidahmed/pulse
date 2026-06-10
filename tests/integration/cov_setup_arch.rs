use std::path::Path;
use std::process::Command;

fn pulse_setup(home: &Path, extra: &[&str]) -> (String, String) {
    let mut args = vec!["setup"];
    args.extend_from_slice(extra);
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(&args)
        .env("HOME", home)
        .output()
        .expect("failed to run pulse setup");
    (String::from_utf8(out.stdout).unwrap(), String::from_utf8(out.stderr).unwrap())
}

fn write_settings(home: &Path, value: &serde_json::Value) {
    let claude = home.join(".claude");
    std::fs::create_dir_all(&claude).unwrap();
    std::fs::write(claude.join("settings.json"), serde_json::to_string_pretty(value).unwrap()).unwrap();
}

fn read_settings(home: &Path) -> serde_json::Value {
    let content = std::fs::read_to_string(home.join(".claude/settings.json")).unwrap();
    serde_json::from_str(&content).unwrap()
}

fn event_groups<'a>(settings: &'a serde_json::Value, event: &str) -> &'a Vec<serde_json::Value> {
    settings["hooks"][event].as_array().unwrap()
}

fn group_has_command(group: &serde_json::Value, command: &str) -> bool {
    group["hooks"].as_array().unwrap().iter().any(|h| h["command"].as_str() == Some(command))
}

fn count_command(settings: &serde_json::Value, event: &str, command: &str) -> usize {
    event_groups(settings, event)
        .iter()
        .flat_map(|g| g["hooks"].as_array().unwrap())
        .filter(|h| h["command"].as_str() == Some(command))
        .count()
}

// Covers line 52: root is an object but `hooks` is a non-object (an array here),
// so it must be normalized to a fresh object before merging.
#[test]
fn setup_normalizes_non_object_hooks_field() {
    let dir = tempfile::tempdir().unwrap();
    write_settings(dir.path(), &serde_json::json!({ "hooks": [] }));

    let (_, err) = pulse_setup(dir.path(), &[]);
    assert!(!err.contains("panic"), "must not panic when hooks is a non-object: {err}");

    let settings = read_settings(dir.path());
    assert!(settings["hooks"].is_object(), "hooks coerced to an object");
    assert_eq!(count_command(&settings, "PostToolUse", "pulse --hook"), 1);
    assert_eq!(count_command(&settings, "Stop", "pulse --stop"), 1);
    assert_eq!(count_command(&settings, "SessionStart", "pulse --cleanup"), 1);
}

// Covers lines 93-97 + 135-137: an existing group whose matcher already equals the
// wanted matcher (Edit|Write|MultiEdit for PostToolUse) but which lacks the pulse
// command — the pulse hook is pushed into that existing group via g_hooks_mut.
#[test]
fn setup_merges_into_group_with_matching_matcher() {
    let dir = tempfile::tempdir().unwrap();
    write_settings(
        dir.path(),
        &serde_json::json!({
            "hooks": {
                "PostToolUse": [{
                    "matcher": "Edit|Write|MultiEdit",
                    "hooks": [{ "type": "command", "command": "foreign-formatter" }]
                }]
            }
        }),
    );

    pulse_setup(dir.path(), &[]);

    let settings = read_settings(dir.path());
    let groups = event_groups(&settings, "PostToolUse");
    let matching: Vec<&serde_json::Value> =
        groups.iter().filter(|g| g["matcher"].as_str() == Some("Edit|Write|MultiEdit")).collect();
    assert_eq!(matching.len(), 1, "pulse reuses the existing matching group rather than creating a new one");
    let group = matching[0];
    assert!(group_has_command(group, "pulse --hook"), "pulse hook pushed into existing group");
    assert!(group_has_command(group, "foreign-formatter"), "foreign hook preserved in same group");
}

// Covers the SessionStart (None matcher) variant of the push-into-existing-group path:
// group_matcher_matches (None, None) => true (line 124) and g_hooks_mut.
#[test]
fn setup_merges_into_matcherless_group_for_sessionstart() {
    let dir = tempfile::tempdir().unwrap();
    write_settings(
        dir.path(),
        &serde_json::json!({
            "hooks": {
                "SessionStart": [{
                    "hooks": [{ "type": "command", "command": "foreign-session-init" }]
                }]
            }
        }),
    );

    pulse_setup(dir.path(), &[]);

    let settings = read_settings(dir.path());
    let groups = event_groups(&settings, "SessionStart");
    let matcherless: Vec<&serde_json::Value> = groups.iter().filter(|g| g.get("matcher").is_none()).collect();
    assert_eq!(matcherless.len(), 1, "pulse reuses the matcherless group");
    assert!(group_has_command(matcherless[0], "pulse --cleanup"));
    assert!(group_has_command(matcherless[0], "foreign-session-init"));
}

// Covers lines 103-113 + 118 (migrate_matcher Some arm) and line 125 (_ => false):
// a group already carries `pulse --hook` but under a stale matcher; the matcher is
// migrated to the canonical Edit|Write|MultiEdit value.
#[test]
fn setup_migrates_stale_matcher_on_existing_pulse_hook() {
    let dir = tempfile::tempdir().unwrap();
    write_settings(
        dir.path(),
        &serde_json::json!({
            "hooks": {
                "PostToolUse": [{
                    "matcher": "Edit|Write",
                    "hooks": [{ "type": "command", "command": "pulse --hook" }]
                }]
            }
        }),
    );

    pulse_setup(dir.path(), &[]);

    let settings = read_settings(dir.path());
    let groups = event_groups(&settings, "PostToolUse");
    let pulse_groups: Vec<&serde_json::Value> =
        groups.iter().filter(|g| group_has_command(g, "pulse --hook")).collect();
    assert_eq!(pulse_groups.len(), 1, "no duplicate pulse group created");
    assert_eq!(
        pulse_groups[0]["matcher"].as_str(),
        Some("Edit|Write|MultiEdit"),
        "stale matcher migrated to canonical value"
    );
    assert_eq!(count_command(&settings, "PostToolUse", "pulse --hook"), 1);
}

// Covers lines 103-110 + 114-116 + 118 (migrate_matcher None arm): a group carries
// `pulse --cleanup` but has a spurious matcher; SessionStart wants no matcher, so the
// matcher key is removed.
#[test]
fn setup_strips_spurious_matcher_on_sessionstart_pulse_hook() {
    let dir = tempfile::tempdir().unwrap();
    write_settings(
        dir.path(),
        &serde_json::json!({
            "hooks": {
                "SessionStart": [{
                    "matcher": "*",
                    "hooks": [{ "type": "command", "command": "pulse --cleanup" }]
                }]
            }
        }),
    );

    pulse_setup(dir.path(), &[]);

    let settings = read_settings(dir.path());
    let groups = event_groups(&settings, "SessionStart");
    let pulse_groups: Vec<&serde_json::Value> =
        groups.iter().filter(|g| group_has_command(g, "pulse --cleanup")).collect();
    assert_eq!(pulse_groups.len(), 1, "no duplicate cleanup group created");
    assert!(
        pulse_groups[0].get("matcher").is_none(),
        "spurious matcher removed so SessionStart group matches the desired (None) shape"
    );
}

// Covers line 206 (locate_legacy_md early return None): a legacy `# Pulse` heading that
// carries the legacy signature but is immediately preceded by a non-newline character
// and is not at offset 0, so it is NOT recognized as the pulse-authored block and is
// preserved; pulse appends its own marked block instead.
#[test]
fn setup_preserves_inline_pulse_heading_with_signature() {
    let dir = tempfile::tempdir().unwrap();
    let claude = dir.path().join(".claude");
    std::fs::create_dir_all(&claude).unwrap();
    let original =
        "# My Project\n\nprefix-text# Pulse\n\nPulse fires as a PostToolUse hook on every file edit. user note.\n";
    std::fs::write(claude.join("CLAUDE.md"), original).unwrap();

    pulse_setup(dir.path(), &[]);

    let md = std::fs::read_to_string(claude.join("CLAUDE.md")).unwrap();
    assert!(
        md.contains("prefix-text# Pulse"),
        "an inline '# Pulse' heading not preceded by a newline is not treated as the legacy block"
    );
    assert!(md.contains("user note."), "user content preserved");
    assert!(md.contains("<!-- pulse:setup"), "pulse appends its own marked block instead");
}
