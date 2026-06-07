use std::path::{Path, PathBuf};
use std::process::Command;

const MD_START: &str = "<!-- pulse:setup";

fn pulse_setup(home: &Path, extra: &[&str]) -> (String, String) {
    let mut args = vec!["setup"];
    args.extend_from_slice(extra);
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(&args)
        .env("HOME", home)
        .output()
        .expect("failed to run pulse setup");
    (
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

fn settings_path(home: &Path) -> PathBuf {
    home.join(".claude/settings.json")
}

fn md_path(home: &Path) -> PathBuf {
    home.join(".claude/CLAUDE.md")
}

fn read(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap_or_default()
}

#[test]
fn setup_writes_marked_block_and_version_sentinel() {
    let dir = tempfile::tempdir().unwrap();
    pulse_setup(dir.path(), &[]);

    let md = read(&md_path(dir.path()));
    assert_eq!(md.matches(MD_START).count(), 1, "exactly one marked block");
    assert!(md.contains("# Pulse"));

    let settings = read(&settings_path(dir.path()));
    assert!(
        settings.contains("_pulse_version"),
        "settings.json carries the version sentinel"
    );
}

#[test]
fn setup_idempotent_leaves_files_byte_identical() {
    let dir = tempfile::tempdir().unwrap();
    pulse_setup(dir.path(), &[]);
    let md_first = read(&md_path(dir.path()));
    let settings_first = read(&settings_path(dir.path()));

    let (_, stderr) = pulse_setup(dir.path(), &[]);

    assert_eq!(read(&md_path(dir.path())), md_first, "md unchanged on re-run");
    assert_eq!(
        read(&settings_path(dir.path())),
        settings_first,
        "settings unchanged on re-run"
    );
    assert!(
        stderr.contains("already current") && stderr.contains("already configured"),
        "second run reports no-op: {stderr}"
    );
}

#[test]
fn setup_rewrites_drifted_block_without_duplication() {
    let dir = tempfile::tempdir().unwrap();
    pulse_setup(dir.path(), &[]);
    let canonical = read(&md_path(dir.path()));

    let drifted = canonical.replace("# Pulse", "# Pulse\nDRIFT_MARKER injected by hand");
    std::fs::write(md_path(dir.path()), drifted).unwrap();

    pulse_setup(dir.path(), &[]);
    let restored = read(&md_path(dir.path()));

    assert!(!restored.contains("DRIFT_MARKER"), "hand edit overwritten");
    assert_eq!(restored.matches(MD_START).count(), 1, "still one block");
    assert_eq!(restored, canonical, "restored to the canonical block");
}

#[test]
fn setup_migrates_legacy_unmarked_block() {
    let dir = tempfile::tempdir().unwrap();
    let claude = dir.path().join(".claude");
    std::fs::create_dir_all(&claude).unwrap();
    std::fs::write(
        claude.join("CLAUDE.md"),
        "# My Project\n\nNotes.\n\n# Pulse\n\nPulse fires as a PostToolUse hook on every file edit. Old stale text.\n",
    )
    .unwrap();

    pulse_setup(dir.path(), &[]);
    let md = read(&md_path(dir.path()));

    assert!(md.starts_with("# My Project"), "user content preserved");
    assert!(md.contains("Notes."), "user content preserved");
    assert!(!md.contains("Old stale text"), "legacy block replaced");
    assert_eq!(md.matches(MD_START).count(), 1, "exactly one marked block");
    assert_eq!(md.matches("# Pulse").count(), 1, "no duplicate Pulse heading");
}

#[test]
fn setup_does_not_clobber_user_pulse_heading_without_signature() {
    let dir = tempfile::tempdir().unwrap();
    let claude = dir.path().join(".claude");
    std::fs::create_dir_all(&claude).unwrap();
    std::fs::write(
        claude.join("CLAUDE.md"),
        "# My Project\n\n# Pulse\n\nMy own hand-written notes about the pulse heartbeat feature.\n",
    )
    .unwrap();

    pulse_setup(dir.path(), &[]);
    let md = read(&md_path(dir.path()));

    assert!(
        md.contains("My own hand-written notes about the pulse heartbeat feature."),
        "a user-authored '# Pulse' section without the pulse signature must be preserved"
    );
    assert!(md.contains(MD_START), "pulse still appends its own marked block");
}

#[test]
fn setup_does_not_duplicate_on_unterminated_marker() {
    let dir = tempfile::tempdir().unwrap();
    let claude = dir.path().join(".claude");
    std::fs::create_dir_all(&claude).unwrap();
    std::fs::write(
        claude.join("CLAUDE.md"),
        "# My Project\n\n<!-- pulse:setup v=0.0.0 hash=dead -->\n# Pulse\n\ntruncated, no end marker\n",
    )
    .unwrap();

    pulse_setup(dir.path(), &[]);
    let md = read(&md_path(dir.path()));

    assert_eq!(md.matches(MD_START).count(), 1, "dangling marker does not duplicate");
    assert!(md.contains("<!-- /pulse:setup -->"), "block is now terminated");
    assert!(md.starts_with("# My Project"), "preceding user content preserved");
}

#[test]
fn setup_and_uninstall_survive_non_object_settings() {
    let dir = tempfile::tempdir().unwrap();
    let claude = dir.path().join(".claude");
    std::fs::create_dir_all(&claude).unwrap();
    std::fs::write(claude.join("settings.json"), "[]").unwrap();

    let (_, err) = pulse_setup(dir.path(), &[]);
    assert!(!err.contains("panic"), "setup must not panic on non-object settings: {err}");
    let settings: serde_json::Value =
        serde_json::from_str(&read(&settings_path(dir.path()))).unwrap();
    assert!(settings.is_object(), "settings.json normalized to an object");
    assert!(read(&settings_path(dir.path())).contains("pulse --hook"));

    let (_, err2) = pulse_setup(dir.path(), &["--uninstall"]);
    assert!(!err2.contains("panic"), "uninstall must not panic: {err2}");
}

#[test]
fn uninstall_removes_all_pulse_regions() {
    let dir = tempfile::tempdir().unwrap();
    pulse_setup(dir.path(), &[]);
    let (_, stderr) = pulse_setup(dir.path(), &["--uninstall"]);

    let settings = read(&settings_path(dir.path()));
    assert!(!settings.contains("pulse --hook"));
    assert!(!settings.contains("pulse --stop"));
    assert!(!settings.contains("pulse --cleanup"));
    assert!(!settings.contains("_pulse_version"));

    let md = read(&md_path(dir.path()));
    assert!(!md.contains(MD_START));
    assert!(!md.contains("# Pulse"));

    assert!(stderr.contains("removed"), "stderr: {stderr}");
}

#[test]
fn uninstall_preserves_foreign_hooks() {
    let dir = tempfile::tempdir().unwrap();
    let claude = dir.path().join(".claude");
    std::fs::create_dir_all(&claude).unwrap();
    let existing = serde_json::json!({
        "hooks": {
            "PostToolUse": [{
                "matcher": "Edit|Write",
                "hooks": [{"type": "command", "command": "other-tool --check"}]
            }]
        }
    });
    std::fs::write(
        claude.join("settings.json"),
        serde_json::to_string_pretty(&existing).unwrap(),
    )
    .unwrap();

    pulse_setup(dir.path(), &[]);
    pulse_setup(dir.path(), &["--uninstall"]);

    let settings = read(&settings_path(dir.path()));
    assert!(settings.contains("other-tool --check"), "foreign hook preserved");
    assert!(!settings.contains("pulse --hook"), "pulse hook removed");
    assert!(!settings.contains("_pulse_version"));
}

#[test]
fn uninstall_is_noop_when_not_installed() {
    let dir = tempfile::tempdir().unwrap();
    let claude = dir.path().join(".claude");
    std::fs::create_dir_all(&claude).unwrap();
    let foreign = serde_json::json!({
        "hooks": {"PostToolUse": [{"matcher": "Edit", "hooks": [{"type": "command", "command": "other"}]}]}
    });
    let foreign_str = serde_json::to_string_pretty(&foreign).unwrap();
    std::fs::write(claude.join("settings.json"), &foreign_str).unwrap();
    std::fs::write(claude.join("CLAUDE.md"), "# My Project\n").unwrap();

    let (_, stderr) = pulse_setup(dir.path(), &["--uninstall"]);

    assert!(stderr.contains("nothing to remove"), "stderr: {stderr}");
    assert_eq!(read(&settings_path(dir.path())), foreign_str, "settings untouched");
    assert_eq!(read(&md_path(dir.path())), "# My Project\n", "md untouched");
}

#[test]
fn uninstall_restores_claude_md_byte_identical() {
    let dir = tempfile::tempdir().unwrap();
    let claude = dir.path().join(".claude");
    std::fs::create_dir_all(&claude).unwrap();
    let original = "# My Project\n\nExisting content.\n";
    std::fs::write(claude.join("CLAUDE.md"), original).unwrap();

    pulse_setup(dir.path(), &[]);
    assert!(read(&md_path(dir.path())).contains(MD_START), "block was added");

    pulse_setup(dir.path(), &["--uninstall"]);
    assert_eq!(
        read(&md_path(dir.path())),
        original,
        "CLAUDE.md byte-identical after uninstall"
    );
}

#[test]
fn uninstall_on_missing_files_does_not_panic() {
    let dir = tempfile::tempdir().unwrap();
    let (_, stderr) = pulse_setup(dir.path(), &["--uninstall"]);
    assert!(stderr.contains("nothing to remove"), "stderr: {stderr}");
}
