mod common;

use std::path::Path;
use std::process::Command;

use pulse::config::{self, IgnoreMatcher, PulseConfig};

// ── helpers ──

fn write_config(dir: &Path, body: &str) {
    std::fs::write(dir.join(".pulse.toml"), body).unwrap();
}

fn write_source(path: &Path, code: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, code).unwrap();
}

fn many_args_py() -> String {
    let args: Vec<String> = (0..common::args_above()).map(|i| format!("a{i}")).collect();
    format!("def foo({}):\n    pass\n", args.join(", "))
}

fn run_check_on(file: &Path) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["check", file.to_str().unwrap()])
        .output()
        .unwrap();
    String::from_utf8(out.stdout).unwrap()
}

fn run_budget_on(file: &Path) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["budget", file.to_str().unwrap()])
        .output()
        .unwrap();
    String::from_utf8(out.stderr).unwrap()
}

fn run_debug_on(file: &Path) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["debug", file.to_str().unwrap()])
        .output()
        .unwrap();
    String::from_utf8(out.stderr).unwrap()
}

fn run_hook_on(file: &Path) -> String {
    use std::io::Write;
    let json = format!(r#"{{"tool_input":{{"file_path":"{}"}}}}"#, file.display());
    let mut child = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["--hook"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(json.as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    String::from_utf8(out.stdout).unwrap()
}

fn run_check_all_in(dir: &Path) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .current_dir(dir)
        .args(["-a"])
        .output()
        .unwrap();
    String::from_utf8(out.stdout).unwrap()
}

fn ignore_only_config(paths: &[&str]) -> String {
    let quoted: Vec<String> = paths.iter().map(|p| format!("\"{p}\"")).collect();
    format!("[ignore]\npaths = [{}]\n", quoted.join(", "))
}

// ── 1. TOML parsing ──

#[test]
fn empty_ignore_section_parses() {
    let cfg: PulseConfig = toml::from_str("[ignore]\n").unwrap();
    assert!(cfg.ignore.paths.is_empty());
}

#[test]
fn ignore_paths_empty_list_parses() {
    let cfg: PulseConfig = toml::from_str("[ignore]\npaths = []\n").unwrap();
    assert!(cfg.ignore.paths.is_empty());
}

#[test]
fn ignore_paths_single_pattern_parses() {
    let cfg: PulseConfig = toml::from_str("[ignore]\npaths = [\"vendor/**\"]\n").unwrap();
    assert_eq!(cfg.ignore.paths, vec!["vendor/**".to_string()]);
}

#[test]
fn ignore_paths_multiple_patterns_parse() {
    let cfg: PulseConfig = toml::from_str(
        "[ignore]\npaths = [\"vendor/**\", \"third_party/**\", \"*.gen.py\"]\n",
    )
    .unwrap();
    assert_eq!(cfg.ignore.paths.len(), 3);
}

#[test]
fn ignore_combines_with_other_sections() {
    let cfg: PulseConfig = toml::from_str(
        r#"
        [thresholds]
        arg_max = 8
        [disable]
        smells = ["primitive_obsession"]
        [ignore]
        paths = ["vendor/**"]
        [languages.python]
        cc_warning = 12
        "#,
    )
    .unwrap();
    assert_eq!(cfg.ignore.paths, vec!["vendor/**".to_string()]);
    assert_eq!(cfg.thresholds.function.arg_max, Some(8));
}

#[test]
fn no_ignore_section_defaults_empty() {
    let cfg: PulseConfig = toml::from_str("[thresholds]\narg_max = 8\n").unwrap();
    assert!(cfg.ignore.paths.is_empty());
}

// ── 2. IgnoreMatcher unit (no FS) ──

#[test]
fn matcher_empty_patterns_matches_nothing() {
    let m = IgnoreMatcher::from_patterns(&[]);
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("a.py");
    std::fs::write(&file, "").unwrap();
    assert!(!m.matches_file(dir.path(), &file));
}

#[test]
fn matcher_exact_filename_matches() {
    let m = IgnoreMatcher::from_patterns(&["foo.py".to_string()]);
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("foo.py");
    std::fs::write(&file, "").unwrap();
    assert!(m.matches_file(dir.path(), &file));
}

#[test]
fn matcher_exact_filename_no_match_other() {
    let m = IgnoreMatcher::from_patterns(&["foo.py".to_string()]);
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("bar.py");
    std::fs::write(&file, "").unwrap();
    assert!(!m.matches_file(dir.path(), &file));
}

#[test]
fn matcher_star_segment() {
    let m = IgnoreMatcher::from_patterns(&["*.gen.py".to_string()]);
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("schema.gen.py");
    std::fs::write(&file, "").unwrap();
    assert!(m.matches_file(dir.path(), &file));
}

#[test]
fn matcher_star_does_not_cross_separator() {
    let m = IgnoreMatcher::from_patterns(&["*.py".to_string()]);
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("sub");
    std::fs::create_dir(&nested).unwrap();
    let file = nested.join("a.py");
    std::fs::write(&file, "").unwrap();
    assert!(!m.matches_file(dir.path(), &file));
}

#[test]
fn matcher_double_star_crosses_separators() {
    let m = IgnoreMatcher::from_patterns(&["**/*.py".to_string()]);
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("a/b/c");
    std::fs::create_dir_all(&nested).unwrap();
    let file = nested.join("x.py");
    std::fs::write(&file, "").unwrap();
    assert!(m.matches_file(dir.path(), &file));
}

#[test]
fn matcher_double_star_in_middle() {
    let m = IgnoreMatcher::from_patterns(&["src/**/generated.rs".to_string()]);
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("src/a/b");
    std::fs::create_dir_all(&nested).unwrap();
    let file = nested.join("generated.rs");
    std::fs::write(&file, "").unwrap();
    assert!(m.matches_file(dir.path(), &file));
}

#[test]
fn matcher_question_mark_single_char() {
    let m = IgnoreMatcher::from_patterns(&["a?.py".to_string()]);
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("a1.py");
    std::fs::write(&file, "").unwrap();
    assert!(m.matches_file(dir.path(), &file));
}

#[test]
fn matcher_char_class() {
    let m = IgnoreMatcher::from_patterns(&["v[0-9].py".to_string()]);
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("v3.py");
    std::fs::write(&file, "").unwrap();
    assert!(m.matches_file(dir.path(), &file));
}

#[test]
fn matcher_alternation_braces() {
    let m = IgnoreMatcher::from_patterns(&["{vendor,third_party}/**".to_string()]);
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("third_party");
    std::fs::create_dir(&nested).unwrap();
    let file = nested.join("lib.py");
    std::fs::write(&file, "").unwrap();
    assert!(m.matches_file(dir.path(), &file));
}

#[test]
fn matcher_bare_folder_matches_contents() {
    let m = IgnoreMatcher::from_patterns(&["vendor".to_string()]);
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("vendor");
    std::fs::create_dir(&nested).unwrap();
    let file = nested.join("lib.py");
    std::fs::write(&file, "").unwrap();
    assert!(m.matches_file(dir.path(), &file));
}

#[test]
fn matcher_trailing_slash_treated_as_folder() {
    let m = IgnoreMatcher::from_patterns(&["vendor/".to_string()]);
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("vendor");
    std::fs::create_dir(&nested).unwrap();
    let file = nested.join("a.py");
    std::fs::write(&file, "").unwrap();
    assert!(m.matches_file(dir.path(), &file));
}

#[test]
fn matcher_empty_string_pattern_ignored() {
    let m = IgnoreMatcher::from_patterns(&String::new().repeat(0).split(',').map(String::from).collect::<Vec<_>>());
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("x.py");
    std::fs::write(&file, "").unwrap();
    assert!(!m.matches_file(dir.path(), &file));
}

#[test]
fn matcher_whitespace_pattern_ignored() {
    let m = IgnoreMatcher::from_patterns(&["   ".to_string()]);
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("x.py");
    std::fs::write(&file, "").unwrap();
    assert!(!m.matches_file(dir.path(), &file));
}

#[test]
fn matcher_case_sensitive() {
    let m = IgnoreMatcher::from_patterns(&["Vendor/**".to_string()]);
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("vendor");
    std::fs::create_dir(&nested).unwrap();
    let file = nested.join("a.py");
    std::fs::write(&file, "").unwrap();
    // Lowercase vendor/ should NOT match Vendor/ pattern (case sensitive)
    assert!(!m.matches_file(dir.path(), &file));
}

#[test]
fn matcher_multiple_patterns_first_matches() {
    let m = IgnoreMatcher::from_patterns(&["vendor/**".to_string(), "build/**".to_string()]);
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("vendor");
    std::fs::create_dir(&nested).unwrap();
    let file = nested.join("a.py");
    std::fs::write(&file, "").unwrap();
    assert!(m.matches_file(dir.path(), &file));
}

#[test]
fn matcher_multiple_patterns_second_matches() {
    let m = IgnoreMatcher::from_patterns(&["vendor/**".to_string(), "build/**".to_string()]);
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("build");
    std::fs::create_dir(&nested).unwrap();
    let file = nested.join("out.py");
    std::fs::write(&file, "").unwrap();
    assert!(m.matches_file(dir.path(), &file));
}

#[test]
fn matcher_multiple_patterns_none_match() {
    let m = IgnoreMatcher::from_patterns(&["vendor/**".to_string(), "build/**".to_string()]);
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("src");
    std::fs::create_dir(&nested).unwrap();
    let file = nested.join("a.py");
    std::fs::write(&file, "").unwrap();
    assert!(!m.matches_file(dir.path(), &file));
}

#[test]
fn matcher_duplicate_patterns_match() {
    let m = IgnoreMatcher::from_patterns(&["vendor/**".to_string(), "vendor/**".to_string()]);
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("vendor");
    std::fs::create_dir(&nested).unwrap();
    let file = nested.join("lib.py");
    std::fs::write(&file, "").unwrap();
    assert!(m.matches_file(dir.path(), &file));
}

#[test]
fn matcher_double_star_recursive_folder() {
    let m = IgnoreMatcher::from_patterns(&["**/generated/**".to_string()]);
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("a/b/generated/c");
    std::fs::create_dir_all(&nested).unwrap();
    let file = nested.join("out.py");
    std::fs::write(&file, "").unwrap();
    assert!(m.matches_file(dir.path(), &file));
}

#[test]
fn matcher_extension_at_any_depth() {
    let m = IgnoreMatcher::from_patterns(&["**/*.gen.rs".to_string()]);
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("src/sub");
    std::fs::create_dir_all(&nested).unwrap();
    let file = nested.join("schema.gen.rs");
    std::fs::write(&file, "").unwrap();
    assert!(m.matches_file(dir.path(), &file));
}

// ── 3. is_ignored_for_file with config root resolution ──

#[test]
fn is_ignored_no_config_returns_false() {
    let cfg = PulseConfig::default();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("a.py");
    std::fs::write(&file, "").unwrap();
    assert!(!config::is_ignored_for_file(&cfg, &file));
}

#[test]
fn is_ignored_empty_paths_returns_false() {
    let cfg: PulseConfig = toml::from_str("[ignore]\npaths = []\n").unwrap();
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path(), "[ignore]\npaths = []\n");
    let file = dir.path().join("a.py");
    std::fs::write(&file, "").unwrap();
    assert!(!config::is_ignored_for_file(&cfg, &file));
}

#[test]
fn is_ignored_match_file_in_config_dir() {
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path(), &ignore_only_config(&["foo.py"]));
    let file = dir.path().join("foo.py");
    std::fs::write(&file, "").unwrap();
    let cfg = config::load_config(&file).unwrap();
    assert!(config::is_ignored_for_file(&cfg, &file));
}

#[test]
fn is_ignored_no_match_file_in_config_dir() {
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path(), &ignore_only_config(&["foo.py"]));
    let file = dir.path().join("bar.py");
    std::fs::write(&file, "").unwrap();
    let cfg = config::load_config(&file).unwrap();
    assert!(!config::is_ignored_for_file(&cfg, &file));
}

#[test]
fn is_ignored_resolves_relative_to_config_root() {
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path(), &ignore_only_config(&["src/sub/**"]));
    let file = dir.path().join("src/sub/a.py");
    write_source(&file, "");
    let cfg = config::load_config(&file).unwrap();
    assert!(config::is_ignored_for_file(&cfg, &file));
}

#[test]
fn is_ignored_does_not_match_outside_pattern_path() {
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path(), &ignore_only_config(&["src/sub/**"]));
    let file = dir.path().join("src/other/a.py");
    write_source(&file, "");
    let cfg = config::load_config(&file).unwrap();
    assert!(!config::is_ignored_for_file(&cfg, &file));
}

#[test]
fn is_ignored_handles_canonicalized_path() {
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path(), &ignore_only_config(&["sub/**"]));
    let file = dir.path().join("sub").join("a.py");
    write_source(&file, "");
    let cfg = config::load_config(&file).unwrap();
    assert!(config::is_ignored_for_file(&cfg, &file));
}

#[test]
fn is_ignored_when_no_config_file_present() {
    let cfg: PulseConfig = toml::from_str(&ignore_only_config(&["**"])).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("a.py");
    std::fs::write(&file, "").unwrap();
    // Without a discoverable .pulse.toml, is_ignored_for_file returns false.
    assert!(!config::is_ignored_for_file(&cfg, &file));
}

#[test]
fn is_ignored_via_parent_config() {
    let outer = tempfile::tempdir().unwrap();
    write_config(outer.path(), &ignore_only_config(&["src/legacy/**"]));
    let file = outer.path().join("src/legacy/old.py");
    write_source(&file, "");
    let cfg = config::load_config(&file).unwrap();
    assert!(config::is_ignored_for_file(&cfg, &file));
}

#[test]
fn closer_config_overrides_outer_for_ignore() {
    let outer = tempfile::tempdir().unwrap();
    write_config(outer.path(), &ignore_only_config(&["**"]));
    let inner = outer.path().join("project");
    std::fs::create_dir(&inner).unwrap();
    write_config(&inner, "[ignore]\npaths = []\n");
    let file = inner.join("a.py");
    std::fs::write(&file, "").unwrap();
    let cfg = config::load_config(&file).unwrap();
    assert!(!config::is_ignored_for_file(&cfg, &file));
}

// ── 4. CLI integration: check ──

#[test]
fn check_emits_findings_without_ignore() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("a.py");
    write_source(&file, &many_args_py());
    assert!(run_check_on(&file).contains("Excess Arguments"));
}

#[test]
fn check_skips_ignored_file() {
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path(), &ignore_only_config(&["**"]));
    let file = dir.path().join("a.py");
    write_source(&file, &many_args_py());
    let out = run_check_on(&file);
    assert!(!out.contains("Excess Arguments"), "output: {out}");
}

#[test]
fn check_skips_file_under_ignored_folder() {
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path(), &ignore_only_config(&["vendor/**"]));
    let file = dir.path().join("vendor/lib.py");
    write_source(&file, &many_args_py());
    let out = run_check_on(&file);
    assert!(out.is_empty(), "output: {out}");
}

#[test]
fn check_emits_for_non_matching_sibling_file() {
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path(), &ignore_only_config(&["vendor/**"]));
    let ignored = dir.path().join("vendor/lib.py");
    let kept = dir.path().join("src/app.py");
    write_source(&ignored, &many_args_py());
    write_source(&kept, &many_args_py());
    assert!(run_check_on(&ignored).is_empty());
    assert!(run_check_on(&kept).contains("Excess Arguments"));
}

#[test]
fn check_skips_glob_extension_match() {
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path(), &ignore_only_config(&["**/*.gen.py"]));
    let file = dir.path().join("src/schema.gen.py");
    write_source(&file, &many_args_py());
    assert!(run_check_on(&file).is_empty());
}

#[test]
fn check_handles_multiple_patterns_match_one() {
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path(), &ignore_only_config(&["vendor/**", "build/**"]));
    let file = dir.path().join("build/out.py");
    write_source(&file, &many_args_py());
    assert!(run_check_on(&file).is_empty());
}

#[test]
fn check_findings_when_no_pattern_matches() {
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path(), &ignore_only_config(&["vendor/**"]));
    let file = dir.path().join("src/app.py");
    write_source(&file, &many_args_py());
    assert!(run_check_on(&file).contains("Excess Arguments"));
}

// ── 5. CLI integration: -a / --all ──

#[test]
fn check_all_skips_ignored_folder() {
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path(), &ignore_only_config(&["legacy/**"]));
    let ignored = dir.path().join("legacy/old.py");
    let kept = dir.path().join("src/app.py");
    write_source(&ignored, &many_args_py());
    write_source(&kept, &many_args_py());
    let out = run_check_all_in(dir.path());
    assert!(out.contains("app.py"), "should report kept file: {out}");
    assert!(!out.contains("old.py"), "should not report ignored: {out}");
}

#[test]
fn check_all_with_no_ignore_runs_all() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.py");
    let b = dir.path().join("sub/b.py");
    write_source(&a, &many_args_py());
    write_source(&b, &many_args_py());
    let out = run_check_all_in(dir.path());
    assert!(out.contains("a.py") && out.contains("b.py"), "{out}");
}

#[test]
fn check_all_multiple_ignore_patterns() {
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path(), &ignore_only_config(&["vendor/**", "build/**"]));
    write_source(&dir.path().join("vendor/v.py"), &many_args_py());
    write_source(&dir.path().join("build/b.py"), &many_args_py());
    write_source(&dir.path().join("src/keep.py"), &many_args_py());
    let out = run_check_all_in(dir.path());
    assert!(out.contains("keep.py"), "{out}");
    assert!(!out.contains("v.py"), "{out}");
    assert!(!out.contains("b.py"), "{out}");
}

#[test]
fn check_all_ignore_extension_glob() {
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path(), &ignore_only_config(&["**/*.gen.py"]));
    write_source(&dir.path().join("src/schema.gen.py"), &many_args_py());
    write_source(&dir.path().join("src/keep.py"), &many_args_py());
    let out = run_check_all_in(dir.path());
    assert!(!out.contains("schema.gen.py"), "{out}");
    assert!(out.contains("keep.py"), "{out}");
}

#[test]
fn check_all_empty_paths_runs_all() {
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path(), &ignore_only_config(&[]));
    write_source(&dir.path().join("a.py"), &many_args_py());
    let out = run_check_all_in(dir.path());
    assert!(out.contains("a.py"), "{out}");
}

// ── 6. CLI integration: budget / debug ──

#[test]
fn budget_reports_ignored() {
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path(), &ignore_only_config(&["legacy/**"]));
    let file = dir.path().join("legacy/old.py");
    write_source(&file, &many_args_py());
    let out = run_budget_on(&file);
    assert!(out.contains("ignored by .pulse.toml"), "{out}");
}

#[test]
fn budget_runs_for_non_ignored() {
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path(), &ignore_only_config(&["legacy/**"]));
    let file = dir.path().join("src/a.py");
    write_source(&file, &many_args_py());
    let out = run_budget_on(&file);
    assert!(!out.contains("ignored by"), "{out}");
    assert!(out.contains("budget:"), "{out}");
}

#[test]
fn debug_reports_ignored() {
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path(), &ignore_only_config(&["legacy/**"]));
    let file = dir.path().join("legacy/old.py");
    write_source(&file, &many_args_py());
    let out = run_debug_on(&file);
    assert!(out.contains("ignored by .pulse.toml"), "{out}");
}

#[test]
fn debug_runs_for_non_ignored() {
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path(), &ignore_only_config(&["legacy/**"]));
    let file = dir.path().join("src/a.py");
    write_source(&file, &many_args_py());
    let out = run_debug_on(&file);
    assert!(!out.contains("ignored by"), "{out}");
    assert!(out.contains("Module:"), "{out}");
}

// ── 7. CLI integration: --hook ──

#[test]
fn hook_emits_block_for_non_ignored() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("a.py");
    write_source(&file, &many_args_py());
    let out = run_hook_on(&file);
    assert!(out.contains("\"decision\":\"block\""), "{out}");
}

#[test]
fn hook_silent_for_ignored_file() {
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path(), &ignore_only_config(&["**"]));
    let file = dir.path().join("a.py");
    write_source(&file, &many_args_py());
    let out = run_hook_on(&file);
    assert!(out.is_empty(), "expected silent, got: {out}");
}

#[test]
fn hook_silent_for_ignored_subfolder() {
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path(), &ignore_only_config(&["legacy/**"]));
    let file = dir.path().join("legacy/old.py");
    write_source(&file, &many_args_py());
    let out = run_hook_on(&file);
    assert!(out.is_empty(), "expected silent, got: {out}");
}

#[test]
fn hook_active_for_unmatched_path() {
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path(), &ignore_only_config(&["legacy/**"]));
    let file = dir.path().join("src/a.py");
    write_source(&file, &many_args_py());
    let out = run_hook_on(&file);
    assert!(out.contains("\"decision\":\"block\""), "{out}");
}

// ── 8. Composition with other config sections ──

#[test]
fn ignore_takes_precedence_over_disable() {
    let dir = tempfile::tempdir().unwrap();
    let toml = "[disable]\nsmells = [\"primitive_obsession\"]\n[ignore]\npaths = [\"**\"]\n";
    write_config(dir.path(), toml);
    let file = dir.path().join("a.py");
    write_source(&file, &many_args_py());
    assert!(run_check_on(&file).is_empty());
}

#[test]
fn ignore_takes_precedence_over_language_overrides() {
    let dir = tempfile::tempdir().unwrap();
    let toml = "[languages.python]\narg_max = 1\n[ignore]\npaths = [\"**\"]\n";
    write_config(dir.path(), toml);
    let file = dir.path().join("a.py");
    write_source(&file, &many_args_py());
    assert!(run_check_on(&file).is_empty());
}

#[test]
fn ignore_only_section_works() {
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path(), "[ignore]\npaths = [\"**\"]\n");
    let file = dir.path().join("a.py");
    write_source(&file, &many_args_py());
    assert!(run_check_on(&file).is_empty());
}

// ── 9. Edge cases ──

#[test]
fn ignore_pattern_already_has_double_star_suffix() {
    let m = IgnoreMatcher::from_patterns(&["vendor/**".to_string()]);
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("vendor/sub");
    std::fs::create_dir_all(&nested).unwrap();
    let file = nested.join("a.py");
    std::fs::write(&file, "").unwrap();
    assert!(m.matches_file(dir.path(), &file));
}

#[test]
fn ignore_pattern_double_star_only_matches_anywhere() {
    let m = IgnoreMatcher::from_patterns(&["**".to_string()]);
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("a/b/c");
    std::fs::create_dir_all(&nested).unwrap();
    let file = nested.join("x.py");
    std::fs::write(&file, "").unwrap();
    assert!(m.matches_file(dir.path(), &file));
}

#[test]
fn ignore_root_file_when_pattern_targets_subdir() {
    let m = IgnoreMatcher::from_patterns(&["sub/**".to_string()]);
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("a.py");
    std::fs::write(&file, "").unwrap();
    assert!(!m.matches_file(dir.path(), &file));
}

#[test]
fn ignore_pattern_with_redundant_trailing_slashes() {
    let m = IgnoreMatcher::from_patterns(&["vendor///".to_string()]);
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("vendor");
    std::fs::create_dir(&nested).unwrap();
    let file = nested.join("a.py");
    std::fs::write(&file, "").unwrap();
    assert!(m.matches_file(dir.path(), &file));
}

#[test]
fn ignore_does_not_match_partial_segment() {
    let m = IgnoreMatcher::from_patterns(&["vendor".to_string()]);
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("vendored");
    std::fs::create_dir(&nested).unwrap();
    let file = nested.join("a.py");
    std::fs::write(&file, "").unwrap();
    assert!(!m.matches_file(dir.path(), &file));
}

#[test]
fn ignore_load_config_with_root_returns_dir_of_toml() {
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path(), &ignore_only_config(&["a"]));
    let file = dir.path().join("a.py");
    std::fs::write(&file, "").unwrap();
    let (_cfg, root) = config::load_config_with_root(&file).unwrap();
    let canon_dir = std::fs::canonicalize(dir.path()).unwrap();
    let canon_root = std::fs::canonicalize(&root).unwrap();
    assert_eq!(canon_root, canon_dir);
}

#[test]
fn ignore_load_config_with_root_none_when_absent() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("a.py");
    std::fs::write(&file, "").unwrap();
    assert!(config::load_config_with_root(&file).is_none());
}

#[test]
fn ignore_pattern_at_root_level_only() {
    let m = IgnoreMatcher::from_patterns(&["a.py".to_string()]);
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("sub");
    std::fs::create_dir(&nested).unwrap();
    let file = nested.join("a.py");
    std::fs::write(&file, "").unwrap();
    // a.py without **/ prefix should not match nested files
    assert!(!m.matches_file(dir.path(), &file));
}

#[test]
fn ignore_recursive_filename_pattern() {
    let m = IgnoreMatcher::from_patterns(&["**/a.py".to_string()]);
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("sub/deep");
    std::fs::create_dir_all(&nested).unwrap();
    let file = nested.join("a.py");
    std::fs::write(&file, "").unwrap();
    assert!(m.matches_file(dir.path(), &file));
}

#[test]
fn ignore_pattern_applies_after_canonicalization() {
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path(), &ignore_only_config(&["sub/**"]));
    let nested = dir.path().join("sub");
    std::fs::create_dir(&nested).unwrap();
    let file = nested.join("a.py");
    std::fs::write(&file, "").unwrap();
    let cfg = config::load_config(&file).unwrap();
    assert!(config::is_ignored_for_file(&cfg, &file));
}
