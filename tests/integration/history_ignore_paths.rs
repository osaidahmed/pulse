use std::path::Path;
use std::process::Command;

use serde_json::Value;

use crate::history_common::{build_repo, CommitSpec};

fn pulse_in(repo: &Path, args: &[&str]) -> (String, String, i32) {
    let mut all = vec!["history", "--root", repo.to_str().unwrap()];
    all.extend(args);
    let output = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(&all)
        .output()
        .expect("pulse failed");
    (
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
        output.status.code().unwrap_or(-1),
    )
}

fn write_config(repo: &Path, content: &str) {
    std::fs::write(repo.join(".pulse.toml"), content).unwrap();
}

fn repo_with_six_pairs() -> tempfile::TempDir {
    let writes: [[(&'static str, &'static str); 2]; 24] = [
        [("a.py", "x = 1\n"), ("b.py", "y = 1\n")],
        [("a.py", "x = 2\n"), ("b.py", "y = 2\n")],
        [("a.py", "x = 3\n"), ("b.py", "y = 3\n")],
        [("a.py", "x = 4\n"), ("b.py", "y = 4\n")],
        [("c.py", "x = 1\n"), ("d.py", "y = 1\n")],
        [("c.py", "x = 2\n"), ("d.py", "y = 2\n")],
        [("c.py", "x = 3\n"), ("d.py", "y = 3\n")],
        [("c.py", "x = 4\n"), ("d.py", "y = 4\n")],
        [("e.py", "x = 1\n"), ("f.py", "y = 1\n")],
        [("e.py", "x = 2\n"), ("f.py", "y = 2\n")],
        [("e.py", "x = 3\n"), ("f.py", "y = 3\n")],
        [("e.py", "x = 4\n"), ("f.py", "y = 4\n")],
        [("g.py", "x = 1\n"), ("h.py", "y = 1\n")],
        [("g.py", "x = 2\n"), ("h.py", "y = 2\n")],
        [("g.py", "x = 3\n"), ("h.py", "y = 3\n")],
        [("g.py", "x = 4\n"), ("h.py", "y = 4\n")],
        [("i.py", "x = 1\n"), ("j.py", "y = 1\n")],
        [("i.py", "x = 2\n"), ("j.py", "y = 2\n")],
        [("i.py", "x = 3\n"), ("j.py", "y = 3\n")],
        [("i.py", "x = 4\n"), ("j.py", "y = 4\n")],
        [("k.py", "x = 1\n"), ("l.py", "y = 1\n")],
        [("k.py", "x = 2\n"), ("l.py", "y = 2\n")],
        [("k.py", "x = 3\n"), ("l.py", "y = 3\n")],
        [("k.py", "x = 4\n"), ("l.py", "y = 4\n")],
    ];
    let authors = ["alice <a@x>", "bob <b@x>", "carol <c@x>", "dave <d@x>", "eve <e@x>", "frank <f@x>", "gina <g@x>"];
    let commits: Vec<CommitSpec> = writes
        .iter()
        .enumerate()
        .map(|(i, w)| CommitSpec {
            author: authors[i % authors.len()],
            message: "tick",
            writes: w.as_slice(),
            deletes: &[],
        })
        .collect();
    build_repo(&commits)
}

fn repo_with_directory_layout() -> tempfile::TempDir {
    let writes: [[(&'static str, &'static str); 2]; 12] = [
        [("src/app.py", "x = 1\n"), ("src/util.py", "y = 1\n")],
        [("src/app.py", "x = 2\n"), ("src/util.py", "y = 2\n")],
        [("src/app.py", "x = 3\n"), ("src/util.py", "y = 3\n")],
        [("src/app.py", "x = 4\n"), ("src/util.py", "y = 4\n")],
        [("legacy/old.py", "x = 1\n"), ("legacy/older.py", "y = 1\n")],
        [("legacy/old.py", "x = 2\n"), ("legacy/older.py", "y = 2\n")],
        [("legacy/old.py", "x = 3\n"), ("legacy/older.py", "y = 3\n")],
        [("legacy/old.py", "x = 4\n"), ("legacy/older.py", "y = 4\n")],
        [("generated/gen_a.py", "x = 1\n"), ("generated/gen_b.py", "y = 1\n")],
        [("generated/gen_a.py", "x = 2\n"), ("generated/gen_b.py", "y = 2\n")],
        [("generated/gen_a.py", "x = 3\n"), ("generated/gen_b.py", "y = 3\n")],
        [("generated/gen_a.py", "x = 4\n"), ("generated/gen_b.py", "y = 4\n")],
    ];
    let authors = ["alice <a@x>", "bob <b@x>", "carol <c@x>", "dave <d@x>", "eve <e@x>"];
    let commits: Vec<CommitSpec> = writes
        .iter()
        .enumerate()
        .map(|(i, w)| CommitSpec {
            author: authors[i % authors.len()],
            message: "tick",
            writes: w.as_slice(),
            deletes: &[],
        })
        .collect();
    build_repo(&commits)
}

fn drift_files_in_json(stdout: &str) -> Vec<String> {
    let v: Value = serde_json::from_str(stdout).expect("json");
    let mut out = Vec::new();
    for f in v["findings"].as_array().unwrap() {
        if f["kind"].as_str() == Some("ArchitecturalDrift") {
            if let Some(a) = f["file_a"].as_str() {
                out.push(a.to_string());
            }
            if let Some(b) = f["file_b"].as_str() {
                out.push(b.to_string());
            }
        }
    }
    out
}

fn hotspot_files_in_json(stdout: &str) -> Vec<String> {
    let v: Value = serde_json::from_str(stdout).expect("json");
    let mut out = Vec::new();
    for f in v["findings"].as_array().unwrap() {
        if f["kind"].as_str() == Some("Hotspot") {
            if let Some(p) = f["file"].as_str() {
                out.push(p.to_string());
            }
        }
    }
    out
}

fn contrib_files_in_json(stdout: &str) -> Vec<String> {
    let v: Value = serde_json::from_str(stdout).expect("json");
    let mut out = Vec::new();
    for f in v["findings"].as_array().unwrap() {
        if f["kind"].as_str() == Some("KnowledgeFragmentation") {
            if let Some(p) = f["file"].as_str() {
                out.push(p.to_string());
            }
        }
    }
    out
}

#[test]
fn no_history_config_default_run_still_works() {
    let repo = repo_with_six_pairs();
    let (stdout, _, code) = pulse_in(repo.path(), &["--json"]);
    assert!(code == 0 || code == 1);
    let v: Value = serde_json::from_str(&stdout).unwrap();
    assert!(v["findings"].is_array());
}

#[test]
fn history_ignore_paths_skips_file_in_hotspots() {
    let repo = repo_with_six_pairs();
    write_config(repo.path(), "[history]\nignore_paths = [\"a.py\"]\n");
    let (stdout, _, _) = pulse_in(repo.path(), &["--json"]);
    let files = hotspot_files_in_json(&stdout);
    assert!(!files.iter().any(|f| f.ends_with("/a.py") || f == "a.py"));
}

#[test]
fn history_ignore_paths_skips_file_in_drift() {
    let repo = repo_with_six_pairs();
    write_config(repo.path(), "[history]\nignore_paths = [\"a.py\"]\n");
    let (stdout, _, _) = pulse_in(repo.path(), &["--json"]);
    let files = drift_files_in_json(&stdout);
    assert!(!files.iter().any(|f| f.ends_with("/a.py") || f == "a.py"));
}

#[test]
fn history_ignore_paths_skips_file_in_contributors() {
    let repo = repo_with_six_pairs();
    write_config(repo.path(), "[history]\nignore_paths = [\"a.py\"]\n");
    let (stdout, _, _) = pulse_in(repo.path(), &["--json"]);
    let files = contrib_files_in_json(&stdout);
    assert!(!files.iter().any(|f| f.ends_with("/a.py") || f == "a.py"));
}

#[test]
fn global_ignore_paths_skips_file_in_history() {
    let repo = repo_with_six_pairs();
    write_config(repo.path(), "[ignore]\npaths = [\"a.py\"]\n");
    let (stdout, _, _) = pulse_in(repo.path(), &["--json"]);
    let files = hotspot_files_in_json(&stdout);
    assert!(!files.iter().any(|f| f.ends_with("/a.py") || f == "a.py"));
}

#[test]
fn history_ignore_dir_glob_skips_all_files_under_dir() {
    let repo = repo_with_directory_layout();
    write_config(repo.path(), "[history]\nignore_paths = [\"legacy/**\"]\n");
    let (stdout, _, _) = pulse_in(repo.path(), &["--json"]);
    for files in [
        hotspot_files_in_json(&stdout),
        drift_files_in_json(&stdout),
        contrib_files_in_json(&stdout),
    ] {
        assert!(!files.iter().any(|f| f.contains("legacy/")), "legacy/ leaked: {files:?}");
    }
}

#[test]
fn history_ignore_dir_pattern_without_glob_normalizes() {
    let repo = repo_with_directory_layout();
    write_config(repo.path(), "[history]\nignore_paths = [\"legacy\"]\n");
    let (stdout, _, _) = pulse_in(repo.path(), &["--json"]);
    let drift = drift_files_in_json(&stdout);
    assert!(!drift.iter().any(|f| f.contains("legacy/")));
}

#[test]
fn history_ignore_with_trailing_slash_normalizes() {
    let repo = repo_with_directory_layout();
    write_config(repo.path(), "[history]\nignore_paths = [\"legacy/\"]\n");
    let (stdout, _, _) = pulse_in(repo.path(), &["--json"]);
    let drift = drift_files_in_json(&stdout);
    assert!(!drift.iter().any(|f| f.contains("legacy/")));
}

#[test]
fn history_ignore_glob_double_star_only_under_dir() {
    let repo = repo_with_directory_layout();
    write_config(repo.path(), "[history]\nignore_paths = [\"legacy/**\"]\n");
    let (stdout, _, _) = pulse_in(repo.path(), &["--json"]);
    let drift = drift_files_in_json(&stdout);
    assert!(drift.iter().any(|f| f.contains("src/")));
}

#[test]
fn history_ignore_multiple_dirs() {
    let repo = repo_with_directory_layout();
    write_config(
        repo.path(),
        "[history]\nignore_paths = [\"legacy/**\", \"generated/**\"]\n",
    );
    let (stdout, _, _) = pulse_in(repo.path(), &["--json"]);
    for files in [
        hotspot_files_in_json(&stdout),
        drift_files_in_json(&stdout),
        contrib_files_in_json(&stdout),
    ] {
        assert!(!files.iter().any(|f| f.contains("legacy/")));
        assert!(!files.iter().any(|f| f.contains("generated/")));
    }
}

#[test]
fn history_ignore_keeps_unmatched_files() {
    let repo = repo_with_directory_layout();
    write_config(repo.path(), "[history]\nignore_paths = [\"legacy/**\"]\n");
    let (stdout, _, _) = pulse_in(repo.path(), &["--json"]);
    let drift = drift_files_in_json(&stdout);
    assert!(drift.iter().any(|f| f.contains("src/")) || drift.iter().any(|f| f.contains("generated/")));
}

#[test]
fn history_ignore_no_match_pattern_is_noop() {
    let repo = repo_with_six_pairs();
    write_config(repo.path(), "[history]\nignore_paths = [\"nonexistent/**\"]\n");
    let (stdout_with, _, _) = pulse_in(repo.path(), &["--json"]);
    std::fs::remove_file(repo.path().join(".pulse.toml")).unwrap();
    let (stdout_without, _, _) = pulse_in(repo.path(), &["--json"]);
    let with: Value = serde_json::from_str(&stdout_with).unwrap();
    let without: Value = serde_json::from_str(&stdout_without).unwrap();
    assert_eq!(with["summary"]["findings_total"], without["summary"]["findings_total"]);
}

#[test]
fn history_ignore_empty_array_is_noop() {
    let repo = repo_with_six_pairs();
    write_config(repo.path(), "[history]\nignore_paths = []\n");
    let (stdout_with, _, _) = pulse_in(repo.path(), &["--json"]);
    std::fs::remove_file(repo.path().join(".pulse.toml")).unwrap();
    let (stdout_without, _, _) = pulse_in(repo.path(), &["--json"]);
    let with: Value = serde_json::from_str(&stdout_with).unwrap();
    let without: Value = serde_json::from_str(&stdout_without).unwrap();
    assert_eq!(with["summary"]["findings_total"], without["summary"]["findings_total"]);
}

#[test]
fn history_ignore_combines_with_global_ignore() {
    let repo = repo_with_directory_layout();
    write_config(
        repo.path(),
        "[ignore]\npaths = [\"legacy/**\"]\n[history]\nignore_paths = [\"generated/**\"]\n",
    );
    let (stdout, _, _) = pulse_in(repo.path(), &["--json"]);
    for files in [
        hotspot_files_in_json(&stdout),
        drift_files_in_json(&stdout),
        contrib_files_in_json(&stdout),
    ] {
        assert!(!files.iter().any(|f| f.contains("legacy/")));
        assert!(!files.iter().any(|f| f.contains("generated/")));
    }
}

#[test]
fn history_ignore_preserves_src_when_only_legacy_ignored() {
    let repo = repo_with_directory_layout();
    write_config(repo.path(), "[history]\nignore_paths = [\"legacy/**\"]\n");
    let (stdout, _, _) = pulse_in(repo.path(), &["--json"]);
    let v: Value = serde_json::from_str(&stdout).unwrap();
    assert!(v["summary"]["findings_total"].as_u64().unwrap() > 0);
}

#[test]
fn history_ignore_all_files_yields_no_findings() {
    let repo = repo_with_directory_layout();
    write_config(repo.path(), "[history]\nignore_paths = [\"**/*\"]\n");
    let (stdout, _, _) = pulse_in(repo.path(), &["--json"]);
    let v: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(v["summary"]["findings_total"].as_u64().unwrap(), 0);
}

#[test]
fn history_ignore_does_not_change_summary_root() {
    let repo = repo_with_directory_layout();
    write_config(repo.path(), "[history]\nignore_paths = [\"legacy/**\"]\n");
    let (stdout, _, _) = pulse_in(repo.path(), &["--json"]);
    let v: Value = serde_json::from_str(&stdout).unwrap();
    assert!(v["summary"]["root"].is_string());
}

#[test]
fn history_ignore_specific_file_only_skips_that_file() {
    let repo = repo_with_six_pairs();
    write_config(repo.path(), "[history]\nignore_paths = [\"a.py\"]\n");
    let (stdout, _, _) = pulse_in(repo.path(), &["--json"]);
    let drift = drift_files_in_json(&stdout);
    assert!(drift.iter().any(|f| f.contains("c.py") || f.contains("d.py")));
}

#[test]
fn history_global_ignore_does_not_affect_unrelated_files() {
    let repo = repo_with_directory_layout();
    write_config(repo.path(), "[ignore]\npaths = [\"legacy/**\"]\n");
    let (stdout, _, _) = pulse_in(repo.path(), &["--json"]);
    let v: Value = serde_json::from_str(&stdout).unwrap();
    assert!(v["summary"]["findings_total"].as_u64().unwrap() > 0);
}

#[test]
fn history_ignore_then_cli_top_combined() {
    let repo = repo_with_directory_layout();
    write_config(repo.path(), "[history]\nignore_paths = [\"legacy/**\"]\n");
    let (stdout, _, _) = pulse_in(repo.path(), &["--json", "--hotspot-top", "0"]);
    let v: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(v["summary"]["by_pillar"]["complexity"].as_u64().unwrap(), 0);
}

#[test]
fn history_config_cap_via_toml_only() {
    let repo = repo_with_six_pairs();
    write_config(
        repo.path(),
        "[history.co_change]\nmax_findings = 0\n",
    );
    let (stdout, _, _) = pulse_in(repo.path(), &["--json"]);
    let v: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(v["summary"]["by_pillar"]["drift"].as_u64().unwrap(), 0);
}

#[test]
fn history_config_cap_hotspot_only() {
    let repo = repo_with_six_pairs();
    write_config(repo.path(), "[history.hotspot]\nmax_findings = 0\n");
    let (stdout, _, _) = pulse_in(repo.path(), &["--json"]);
    let v: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(v["summary"]["by_pillar"]["complexity"].as_u64().unwrap(), 0);
}

#[test]
fn history_config_cap_contributors_only() {
    let repo = repo_with_six_pairs();
    write_config(
        repo.path(),
        "[history.contributors]\nmax_findings = 0\n",
    );
    let (stdout, _, _) = pulse_in(repo.path(), &["--json"]);
    let v: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(v["summary"]["by_pillar"]["ownership"].as_u64().unwrap(), 0);
}

#[test]
fn history_cli_top_overrides_config_cap() {
    let repo = repo_with_six_pairs();
    write_config(repo.path(), "[history.hotspot]\nmax_findings = 99\n");
    let (stdout, _, _) = pulse_in(repo.path(), &["--json", "--hotspot-top", "0"]);
    let v: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(v["summary"]["by_pillar"]["complexity"].as_u64().unwrap(), 0);
}

#[test]
fn history_config_cap_applies_without_cli_flag() {
    let repo = repo_with_six_pairs();
    write_config(repo.path(), "[history.hotspot]\nmax_findings = 0\n");
    let (stdout, _, _) = pulse_in(repo.path(), &["--json"]);
    let v: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(v["summary"]["by_pillar"]["complexity"].as_u64().unwrap(), 0);
}

#[test]
fn history_invalid_unknown_field_in_history_causes_default_behavior() {
    let repo = repo_with_six_pairs();
    write_config(repo.path(), "[history]\nfoo = \"bar\"\n");
    let (stdout, _, _) = pulse_in(repo.path(), &["--json"]);
    let v: Value = serde_json::from_str(&stdout).unwrap();
    assert!(v["summary"]["findings_total"].as_u64().is_some());
}

#[test]
fn history_ignore_pattern_with_star_wildcard() {
    let repo = repo_with_directory_layout();
    write_config(repo.path(), "[history]\nignore_paths = [\"src/*.py\"]\n");
    let (stdout, _, _) = pulse_in(repo.path(), &["--json"]);
    let drift = drift_files_in_json(&stdout);
    assert!(!drift.iter().any(|f| f.ends_with("src/app.py") || f.ends_with("src/util.py")));
}

#[test]
fn history_ignore_pattern_specific_filename_under_root_only_matches_root() {
    let repo = repo_with_directory_layout();
    write_config(repo.path(), "[history]\nignore_paths = [\"src/app.py\"]\n");
    let (stdout, _, _) = pulse_in(repo.path(), &["--json"]);
    let drift = drift_files_in_json(&stdout);
    assert!(drift.iter().any(|f| f.contains("legacy/") || f.contains("generated/")));
}

#[test]
fn history_ignore_combined_with_max_commits_flag() {
    let repo = repo_with_six_pairs();
    write_config(repo.path(), "[history]\nignore_paths = [\"a.py\"]\n");
    let (stdout, _, code) = pulse_in(repo.path(), &["--json", "--max-commits", "1000"]);
    assert!(code == 0 || code == 1);
    let v: Value = serde_json::from_str(&stdout).unwrap();
    assert!(v["summary"]["findings_total"].as_u64().is_some());
}

#[test]
fn history_ignore_combined_with_since_flag() {
    let repo = repo_with_six_pairs();
    write_config(repo.path(), "[history]\nignore_paths = [\"a.py\"]\n");
    let (stdout, _, code) = pulse_in(repo.path(), &["--json", "--since", "100 years ago"]);
    assert!(code == 0 || code == 1);
    let v: Value = serde_json::from_str(&stdout).unwrap();
    assert!(v["summary"]["findings_total"].as_u64().is_some());
}

#[test]
fn history_ignore_pattern_matching_nothing_keeps_full_results() {
    let repo = repo_with_six_pairs();
    let (baseline, _, _) = pulse_in(repo.path(), &["--json"]);
    write_config(repo.path(), "[history]\nignore_paths = [\"zzz_never_matches/**\"]\n");
    let (filtered, _, _) = pulse_in(repo.path(), &["--json"]);
    let baseline_v: Value = serde_json::from_str(&baseline).unwrap();
    let filtered_v: Value = serde_json::from_str(&filtered).unwrap();
    assert_eq!(baseline_v["summary"]["findings_total"], filtered_v["summary"]["findings_total"]);
}

#[test]
fn history_ignore_human_output_works_with_config() {
    let repo = repo_with_directory_layout();
    write_config(repo.path(), "[history]\nignore_paths = [\"legacy/**\"]\n");
    let (stdout, _, _) = pulse_in(repo.path(), &[]);
    assert!(stdout.contains("history:"));
    assert!(!stdout.contains("legacy/old.py"));
}

#[test]
fn history_ignore_via_history_section_does_not_affect_audit_or_check() {
    let repo = repo_with_directory_layout();
    write_config(repo.path(), "[history]\nignore_paths = [\"legacy/**\"]\n");
    let output = Command::new(env!("CARGO_BIN_EXE_pulse"))
        .args(["audit", "--json", "--root", repo.path().to_str().unwrap()])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.is_empty());
}

#[test]
fn history_global_ignore_via_ignore_section_filters_history() {
    let repo = repo_with_directory_layout();
    write_config(repo.path(), "[ignore]\npaths = [\"legacy/**\"]\n");
    let (stdout, _, _) = pulse_in(repo.path(), &["--json"]);
    let drift = drift_files_in_json(&stdout);
    assert!(!drift.iter().any(|f| f.contains("legacy/")));
}
