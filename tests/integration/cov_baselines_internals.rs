use crate::common::*;
use crate::history_common::{build_repo, CommitSpec};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use pulse::baselines;
use pulse::parse::MAX_LINE_BYTES;
use pulse::smells::{Finding, Location, Smell};

fn pulse_bin() -> String {
    env!("CARGO_BIN_EXE_pulse").to_string()
}

fn hash_path(file_path: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    file_path.hash(&mut hasher);
    hasher.finish()
}

fn baseline_json_path(baseline_dir: &Path, file_path: &str) -> PathBuf {
    baseline_dir.join(format!("{:016x}.json", hash_path(file_path)))
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")
}

fn edit_json(file_path: &str, old: &str, new: &str) -> String {
    format!(
        r#"{{"tool_input":{{"file_path":"{}","old_string":"{}","new_string":"{}"}}}}"#,
        escape(file_path),
        escape(old),
        escape(new),
    )
}

fn write_json(file_path: &str) -> String {
    format!(r#"{{"tool_input":{{"file_path":"{}","content":"..."}}}}"#, escape(file_path))
}

fn run_hook(baseline_dir: &Path, json: &str) {
    let mut child = Command::new(pulse_bin())
        .args(["--hook"])
        .env("PULSE_BASELINE_DIR", baseline_dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn pulse --hook");
    child.stdin.take().unwrap().write_all(json.as_bytes()).unwrap();
    child.wait_with_output().expect("wait pulse --hook");
}

// ── line 20: session_baseline_dir returns the env-var dir verbatim ──
// Reachable only by a direct call: init_session_dir gates session_baseline_dir
// behind the same env check, so this branch is only exercised here.
#[test]
fn session_baseline_dir_uses_env_var() {
    let dir = tempfile::tempdir().unwrap();
    std::env::set_var("PULSE_BASELINE_DIR", dir.path());
    let got = baselines::session_baseline_dir("any-session-id-1234567890");
    std::env::remove_var("PULSE_BASELINE_DIR");
    assert_eq!(got, dir.path(), "session_baseline_dir must echo PULSE_BASELINE_DIR when set");
}

#[test]
fn filter_worsened_keeps_module_findings_untouched() {
    let baseline = pulse::baseline_ratchet::baseline_from_entries(Vec::new());
    let module_finding =
        Finding { smell: Smell::FileTooLarge, location: Location::Module, detail: "irrelevant".to_string() };
    let metrics = pulse::parse::parse_and_walk_guarded("def f():\n    pass\n", pulse::parse::Language::Python).unwrap();
    let kept = pulse::baseline_ratchet::filter_worsened(vec![module_finding], &baseline, &metrics);
    assert_eq!(kept.len(), 1, "module-located findings pass through the function ratchet untouched");
}

// ── line 137: compute_baseline bails when parse_and_walk_guarded returns None ──
// A single line longer than MAX_LINE_BYTES trips parse's size cap, so the
// supported-language file parses to None and compute_baseline writes an empty
// baseline. Driven through `pulse --hook` (Edit mode) so old/new are present,
// keeping reconstruct_pre_edit on the in-place replacen branch.
#[test]
fn compute_baseline_oversized_source_writes_empty_baseline() {
    let baseline_dir = tempfile::tempdir().unwrap();
    let files = tempfile::tempdir().unwrap();
    let path = files.path().join("oversized.py");

    let giant_line: String = "A".repeat(MAX_LINE_BYTES + 1000);
    let post_edit = format!("x = \"{giant_line}\"\nPAD = 2\n");
    std::fs::write(&path, &post_edit).unwrap();

    let json = edit_json(path.to_str().unwrap(), "PAD = 1", "PAD = 2");
    run_hook(baseline_dir.path(), &json);

    let bp = baseline_json_path(baseline_dir.path(), path.to_str().unwrap());
    assert!(bp.exists(), "an (empty) baseline file should still be created: {bp:?}");
    let json_text = std::fs::read_to_string(&bp).unwrap();
    let counts: std::collections::HashMap<String, usize> = serde_json::from_str(&json_text).unwrap_or_default();
    assert!(counts.is_empty(), "oversized source must yield an empty baseline: {counts:?}");
}

// ── lines 181-187: git_show_head reconstructs pre-edit source from HEAD ──
// Write mode (no old_string/new_string) sends reconstruct_pre_edit down the
// git fallback. The file lives in a real repo and is committed, so
// rev-parse --show-toplevel and `git show HEAD:<rel>` both succeed and the
// committed (smelly) content drives the baseline.
#[test]
fn git_show_head_reconstructs_baseline_from_committed_head() {
    let fn_count = t().module.file_function_count as usize + 5;
    let mut committed = String::new();
    for i in 0..fn_count {
        committed.push_str(&format!("def fn_{i}():\n    return {i}\n\n"));
    }

    let leaked: &'static str = Box::leak(committed.clone().into_boxed_str());
    let writes: &'static [(&'static str, &'static str)] = Box::leak(vec![("bigmod.py", leaked)].into_boxed_slice());
    let repo = build_repo(&[CommitSpec { author: "alice <alice@x>", message: "seed", writes, deletes: &[] }]);

    let file_path = repo.path().join("bigmod.py");
    assert!(file_path.exists(), "committed working-tree file should exist");

    let baseline_dir = tempfile::tempdir().unwrap();
    let json = write_json(file_path.to_str().unwrap());
    run_hook(baseline_dir.path(), &json);

    let bp = baseline_json_path(baseline_dir.path(), file_path.to_str().unwrap());
    assert!(bp.exists(), "baseline file should be created via git HEAD reconstruction: {bp:?}");
    let json_text = std::fs::read_to_string(&bp).unwrap();
    let counts: std::collections::HashMap<String, usize> = serde_json::from_str(&json_text).unwrap_or_default();
    assert!(
        counts.contains_key("Too Many Functions"),
        "baseline rebuilt from committed HEAD must capture the module smell: {counts:?}"
    );
}
