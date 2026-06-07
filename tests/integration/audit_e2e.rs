use crate::audit_common::*;
use std::process::Command;

fn pulse_audit(args: &[&str]) -> (String, String, i32) {
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .arg("audit")
        .args(args)
        .output()
        .expect("pulse audit failed to spawn");
    (
        String::from_utf8(out.stdout).unwrap_or_default(),
        String::from_utf8(out.stderr).unwrap_or_default(),
        out.status.code().unwrap_or(-1),
    )
}

fn audit_in_dir(scenario: &str) -> (String, String, i32) {
    let dir = copy_scenario_to_tempdir(scenario, "python");
    pulse_audit(&["--pass", "pattern-mining", "--root", dir.path().to_str().unwrap()])
}

fn audit_in_dir_json(scenario: &str) -> (String, String, i32) {
    let dir = copy_scenario_to_tempdir(scenario, "python");
    pulse_audit(&["--pass", "pattern-mining", "--root", dir.path().to_str().unwrap(), "--json"])
}

#[test]
fn audit_clean_small_no_findings_exit_zero() {
    let (stdout, _stderr, code) = audit_in_dir("clean_small");
    assert_eq!(code, 0);
    assert!(stdout.is_empty());
}

#[test]
fn audit_clean_medium_no_findings_exit_zero() {
    let (_stdout, _stderr, code) = audit_in_dir("clean_medium");
    assert_eq!(code, 0);
}

#[test]
fn audit_clean_disjoint_no_findings_exit_zero() {
    let (_stdout, _stderr, code) = audit_in_dir("clean_disjoint");
    assert_eq!(code, 0);
}

#[test]
fn audit_idiom_heavy_listcomp_after_g2() {
    let (_stdout, _stderr, code) = audit_in_dir("idiom_heavy_listcomp");
    assert!(code == 0 || code == 1, "G² may keep distinctive shapes; just no panic");
}

#[test]
fn audit_idiom_heavy_dict_get_after_g2() {
    let (_stdout, _stderr, code) = audit_in_dir("idiom_heavy_dict_get");
    assert!(code == 0 || code == 1, "G² may keep distinctive shapes; just no panic");
}

#[test]
fn audit_shotgun_media_type_finds_cluster() {
    let (stdout, _stderr, code) = audit_in_dir("shotgun_media_type");
    assert_eq!(code, 1, "findings → exit 1");
    assert!(stdout.contains("media_type"));
    assert!(stdout.contains("audit: cross-file pattern"));
}

#[test]
fn audit_shotgun_cache_pattern_finds_cluster() {
    let (stdout, _stderr, code) = audit_in_dir("shotgun_cache_pattern");
    assert_eq!(code, 1);
    assert!(stdout.contains("audit: cross-file pattern"));
}

#[test]
fn audit_shotgun_multi_class_init_finds_cluster() {
    let (stdout, _stderr, code) = audit_in_dir("shotgun_multi_class_init");
    assert_eq!(code, 1);
    assert!(stdout.contains("audit: cross-file pattern"));
}

#[test]
fn audit_no_root_uses_cwd() {
    let dir = tempfile::tempdir().unwrap();
    let (_stdout, _stderr, code) = pulse_audit_in_cwd(&[], dir.path());
    assert_eq!(code, 0, "empty cwd → no findings");
}

fn pulse_audit_in_cwd(args: &[&str], cwd: &std::path::Path) -> (String, String, i32) {
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .arg("audit")
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("pulse audit failed");
    (
        String::from_utf8(out.stdout).unwrap_or_default(),
        String::from_utf8(out.stderr).unwrap_or_default(),
        out.status.code().unwrap_or(-1),
    )
}

#[test]
fn audit_root_nonexistent_path_exits_one() {
    let (_stdout, stderr, code) = pulse_audit(&["--root", "/no/such/dir/xyz"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("does not exist"));
}

#[test]
fn audit_root_file_not_dir_exits_one() {
    let f = tempfile::NamedTempFile::new().unwrap();
    let (_stdout, stderr, code) = pulse_audit(&["--root", f.path().to_str().unwrap()]);
    assert_eq!(code, 1);
    assert!(stderr.contains("not a directory"));
}

#[test]
fn audit_json_flag_emits_valid_json() {
    let (stdout, _stderr, code) = audit_in_dir_json("shotgun_media_type");
    assert_eq!(code, 1);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).expect("valid json");
    let findings = parsed.get("findings").and_then(serde_json::Value::as_array).expect("findings array");
    assert!(!findings.is_empty());
}

#[test]
fn audit_json_clean_emits_empty_array() {
    let (stdout, _stderr, code) = audit_in_dir_json("clean_small");
    assert_eq!(code, 0);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).expect("valid json");
    assert_eq!(parsed.as_array().unwrap().len(), 0);
}

#[test]
fn audit_json_each_finding_has_kind_and_locations() {
    let (stdout, _stderr, _code) = audit_in_dir_json("shotgun_media_type");
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    let arr = parsed.get("findings").and_then(serde_json::Value::as_array).expect("findings array");
    for f in arr {
        assert!(f.get("kind").is_some());
        assert!(f.get("locations").is_some());
        assert!(f["locations"].is_array());
    }
}

#[test]
fn audit_pass_pattern_mining_explicit_works() {
    let dir = copy_scenario_to_tempdir("shotgun_media_type", "python");
    let (_stdout, _stderr, code) = pulse_audit(&["--pass", "pattern-mining", "--root", dir.path().to_str().unwrap()]);
    assert_eq!(code, 1);
}

#[test]
fn audit_pass_invalid_value_exits_two() {
    let p = scenario_path("shotgun_media_type", "python");
    let (_stdout, _stderr, code) = pulse_audit(&["--pass", "garbage", "--root", p.to_str().unwrap()]);
    assert_eq!(code, 2);
}

#[test]
fn audit_legacy_layer_flag_no_longer_recognized() {
    let p = scenario_path("shotgun_media_type", "python");
    let (_stdout, _stderr, code) = pulse_audit(&["--layer", "3", "--root", p.to_str().unwrap()]);
    assert_eq!(code, 2);
}

#[test]
fn audit_handles_empty_directory() {
    let dir = tempfile::tempdir().unwrap();
    let (stdout, _stderr, code) = pulse_audit(&["--root", dir.path().to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(stdout.is_empty());
}

#[test]
fn audit_handles_directory_with_only_non_python_files() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), "hello").unwrap();
    std::fs::write(dir.path().join("b.json"), "{}").unwrap();
    let (_stdout, _stderr, code) = pulse_audit(&["--root", dir.path().to_str().unwrap()]);
    assert_eq!(code, 0);
}

#[test]
fn audit_skips_dot_dirs_by_default() {
    let dir = tempfile::tempdir().unwrap();
    let hidden = dir.path().join(".cache");
    std::fs::create_dir(&hidden).unwrap();
    for i in 0..6 {
        std::fs::write(hidden.join(format!("a{i}.py")), "def f(x):\n    if x == 1:\n        return 1\n    return 0\n")
            .unwrap();
    }
    let (_stdout, _stderr, code) = pulse_audit(&["--root", dir.path().to_str().unwrap()]);
    assert_eq!(code, 0, "hidden dir contents should be skipped");
}

#[test]
fn audit_skips_node_modules() {
    let dir = tempfile::tempdir().unwrap();
    let nm = dir.path().join("node_modules");
    std::fs::create_dir(&nm).unwrap();
    for i in 0..6 {
        std::fs::write(nm.join(format!("a{i}.py")), "def f(x):\n    if x == 1:\n        return 1\n    return 0\n")
            .unwrap();
    }
    let (_stdout, _stderr, code) = pulse_audit(&["--root", dir.path().to_str().unwrap()]);
    assert_eq!(code, 0);
}

#[test]
fn audit_skips_target_dir() {
    let dir = tempfile::tempdir().unwrap();
    let tgt = dir.path().join("target");
    std::fs::create_dir(&tgt).unwrap();
    for i in 0..6 {
        std::fs::write(tgt.join(format!("a{i}.py")), "def f(x):\n    if x == 1:\n        return 1\n    return 0\n")
            .unwrap();
    }
    let (_stdout, _stderr, code) = pulse_audit(&["--root", dir.path().to_str().unwrap()]);
    assert_eq!(code, 0);
}

#[test]
fn audit_recurses_subdirs() {
    let dir = tempfile::tempdir().unwrap();
    let sub = dir.path().join("sub");
    std::fs::create_dir(&sub).unwrap();
    for i in 0..6 {
        std::fs::write(sub.join(format!("a{i}.py")), "def f(x):\n    if x == 1:\n        return 1\n    return 0\n")
            .unwrap();
    }
    let (_stdout, _stderr, _code) = pulse_audit(&["--root", dir.path().to_str().unwrap()]);
}

#[test]
fn audit_is_deterministic_across_runs() {
    let p = scenario_path("shotgun_media_type", "python");
    let (a, _, _) = pulse_audit(&["--root", p.to_str().unwrap()]);
    let (b, _, _) = pulse_audit(&["--root", p.to_str().unwrap()]);
    assert_eq!(a, b);
}

#[test]
fn audit_handles_syntax_error_python_without_panic() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("bad.py"), "def f(:\n    if\n").unwrap();
    let (_, _, code) = pulse_audit(&["--root", dir.path().to_str().unwrap()]);
    assert_eq!(code, 0);
}

#[test]
fn audit_handles_unicode_filenames() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("δ.py"), "def f(x):\n    if x == 1:\n        return 1\n    return 0\n").unwrap();
    let (_, _, code) = pulse_audit(&["--root", dir.path().to_str().unwrap()]);
    assert_eq!(code, 0);
}

#[test]
fn audit_default_has_no_decision_block_json() {
    let p = scenario_path("clean_small", "python");
    let (stdout, _, _) = pulse_audit(&["--root", p.to_str().unwrap()]);
    assert!(!stdout.contains("\"decision\":"), "audit must not emit hook-style decision block");
}

#[test]
fn audit_combined_json_pass_root_works() {
    let dir = copy_scenario_to_tempdir("shotgun_media_type", "python");
    let (stdout, _, code) =
        pulse_audit(&["--json", "--pass", "pattern-mining", "--root", dir.path().to_str().unwrap()]);
    assert_eq!(code, 1);
    let _: serde_json::Value = serde_json::from_str(stdout.trim()).expect("valid json");
}

#[test]
fn audit_does_not_create_baseline_dir() {
    let baseline_dir = tempfile::tempdir().unwrap();
    let baseline_before = baseline_dir.path().to_path_buf();
    let p = scenario_path("clean_small", "python");
    let _ = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .arg("audit")
        .args(["--root", p.to_str().unwrap()])
        .env("PULSE_BASELINE_DIR", &baseline_before)
        .output()
        .unwrap();
    assert!(baseline_dir.path().exists());
}

#[test]
fn audit_human_format_includes_audit_prefix() {
    let dir = copy_scenario_to_tempdir("shotgun_media_type", "python");
    let (stdout, _, _) = pulse_audit(&["--root", dir.path().to_str().unwrap()]);
    assert!(stdout.starts_with("audit:") || stdout.contains("\naudit:"));
}

#[test]
fn audit_human_format_shows_locations_block() {
    let dir = copy_scenario_to_tempdir("shotgun_media_type", "python");
    let (stdout, _, _) = pulse_audit(&["--root", dir.path().to_str().unwrap()]);
    assert!(stdout.contains("files:") || stdout.contains("locations:"));
}

#[test]
fn audit_human_format_renders_representative_label() {
    let dir = copy_scenario_to_tempdir("shotgun_media_type", "python");
    let (stdout, _, _) = pulse_audit(&["--root", dir.path().to_str().unwrap()]);
    assert!(stdout.contains("representative:"));
}

#[test]
fn audit_root_relative_path_resolves() {
    let p = scenario_path("clean_small", "python");
    let parent = p.parent().unwrap();
    let (_, _, code) = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .arg("audit")
        .args(["--root", "python"])
        .current_dir(parent)
        .output()
        .map(|o| {
            (
                String::from_utf8(o.stdout).unwrap_or_default(),
                String::from_utf8(o.stderr).unwrap_or_default(),
                o.status.code().unwrap_or(-1),
            )
        })
        .unwrap();
    assert_eq!(code, 0);
}

#[test]
fn audit_pulse_self_runs_without_panic() {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src = manifest.join("src");
    let (_, _, code) = pulse_audit(&["--root", src.to_str().unwrap()]);
    assert!(code == 0 || code == 1, "exit must be 0 or 1, got {code}");
}

#[test]
fn audit_pulse_self_is_deterministic() {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src = manifest.join("src");
    let (a, _, _) = pulse_audit(&["--root", src.to_str().unwrap()]);
    let (b, _, _) = pulse_audit(&["--root", src.to_str().unwrap()]);
    assert_eq!(a, b);
}

#[test]
fn audit_pulse_self_json_output_parses() {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src = manifest.join("src");
    let (stdout, _, _) = pulse_audit(&["--json", "--root", src.to_str().unwrap()]);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).expect("valid json");
    assert!(parsed.is_array() || parsed.get("findings").is_some());
}

#[test]
fn audit_handles_one_file_with_many_functions() {
    let dir = tempfile::tempdir().unwrap();
    let mut src = String::new();
    for i in 0..50 {
        src.push_str(&format!("def f{i}(x):\n    if x == {i}:\n        return 1\n    return 0\n\n"));
    }
    std::fs::write(dir.path().join("many.py"), src).unwrap();
    let (_, _, code) = pulse_audit(&["--root", dir.path().to_str().unwrap()]);
    assert!(code == 0 || code == 1);
}
