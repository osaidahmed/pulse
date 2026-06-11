use crate::common::t;
use pulse::analyze::{self, ScanOptions};
use pulse::baseline_ratchet::{baseline_from_entries, filter_worsened, FunctionBaselineEntry};
use pulse::parse::Language;
use pulse::smells::Smell;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

fn pulse_bin() -> String {
    env!("CARGO_BIN_EXE_pulse").to_string()
}

struct StopEnv {
    baseline_dir: tempfile::TempDir,
    files_dir: tempfile::TempDir,
}

impl StopEnv {
    fn new() -> Self {
        Self { baseline_dir: tempfile::tempdir().unwrap(), files_dir: tempfile::tempdir().unwrap() }
    }

    fn file_path(&self, name: &str) -> PathBuf {
        self.files_dir.path().join(name)
    }

    fn cache_path(&self, file_path: &str) -> PathBuf {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        file_path.hash(&mut hasher);
        self.baseline_dir.path().join(format!("{:016x}.analysis", hasher.finish()))
    }

    fn run_hook(&self, json: &str) -> String {
        let output = Command::new(pulse_bin())
            .args(["--hook"])
            .env("PULSE_BASELINE_DIR", self.baseline_dir.path())
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                child.stdin.take().unwrap().write_all(json.as_bytes()).unwrap();
                child.wait_with_output()
            })
            .expect("failed to run pulse --hook");
        String::from_utf8(output.stdout).unwrap()
    }

    fn run_stop(&self) -> String {
        let output = Command::new(pulse_bin())
            .args(["--stop"])
            .env("PULSE_BASELINE_DIR", self.baseline_dir.path())
            .output()
            .expect("failed to run pulse --stop");
        String::from_utf8(output.stdout).unwrap()
    }

    fn plain_edit_json(path: &std::path::Path) -> String {
        format!(r#"{{"tool_input":{{"file_path":"{}"}}}}"#, path.to_str().unwrap())
    }

    fn scoped_edit_json(path: &std::path::Path, old: &str, new: &str) -> String {
        format!(
            r#"{{"tool_input":{{"file_path":"{}","old_string":"{old}","new_string":"{new}"}}}}"#,
            path.to_str().unwrap()
        )
    }
}

fn big_loop_fn(name: &str) -> String {
    let body_lines = t().analysis.duplication.skeleton_min_loc + 4;
    let mut code = format!("def {name}(items):\n    total = 0\n    for item in items:\n        total = total + item\n");
    for i in 0..body_lines.saturating_sub(5) {
        code.push_str(&format!("    slot_{i} = total + {i}\n"));
    }
    code.push_str("    return total\n\n");
    code
}

fn big_branch_fn(name: &str) -> String {
    let target = t().analysis.duplication.skeleton_min_loc + 4;
    let branches = 4u32;
    let whiles = target.saturating_sub(3 + 2 * branches + 1) / 2;
    let mut code = format!("def {name}(flag, value):\n    result = 0\n");
    for i in 0..branches {
        code.push_str(&format!("    if flag == {i}:\n        result = value + {i}\n"));
    }
    code.push_str("    result = result + flag\n");
    for i in 0..whiles {
        code.push_str(&format!("    while result > {}:\n        result = result - 1\n", 900 + i));
    }
    code.push_str("    return result\n\n");
    code
}

fn distinct_pair_source(marker: u32) -> String {
    format!(
        "{}{}def tiny():\n    marker = {marker}\n    return marker\n",
        big_loop_fn("big_alpha"),
        big_branch_fn("big_beta")
    )
}

fn twin_pair_source(marker: u32) -> String {
    format!(
        "{}{}def tiny():\n    marker = {marker}\n    return marker\n",
        big_loop_fn("clone_one"),
        big_loop_fn("clone_two")
    )
}

fn run_three_edit_lifecycle(env: &StopEnv, path: &std::path::Path, v2: &str, v3: &str) {
    env.run_hook(&StopEnv::plain_edit_json(path));
    std::fs::write(path, v2).unwrap();
    env.run_hook(&StopEnv::plain_edit_json(path));
    std::fs::write(path, v3).unwrap();
    env.run_hook(&StopEnv::scoped_edit_json(path, "marker = 2", "marker = 3"));
}

#[test]
fn scoped_edit_does_not_fabricate_duplication_at_stop() {
    let env = StopEnv::new();
    let path = env.file_path("svc.py");
    std::fs::write(&path, distinct_pair_source(1)).unwrap();
    run_three_edit_lifecycle(&env, &path, &distinct_pair_source(2), &distinct_pair_source(3));
    let out = env.run_stop();
    assert!(
        !out.to_lowercase().contains("code duplication"),
        "structurally distinct functions must not become clones via a scoped edit: {out}"
    );
}

#[test]
fn scoped_edit_does_not_rewrite_the_analysis_cache() {
    let env = StopEnv::new();
    let path = env.file_path("svc.py");
    std::fs::write(&path, distinct_pair_source(1)).unwrap();
    env.run_hook(&StopEnv::plain_edit_json(&path));
    std::fs::write(&path, distinct_pair_source(2)).unwrap();
    env.run_hook(&StopEnv::plain_edit_json(&path));
    let cache = env.cache_path(path.to_str().unwrap());
    let before = std::fs::read(&cache).expect("unscoped edit stores the analysis cache");
    std::fs::write(&path, distinct_pair_source(3)).unwrap();
    env.run_hook(&StopEnv::scoped_edit_json(&path, "marker = 2", "marker = 3"));
    let after = std::fs::read(&cache).expect("cache file survives a scoped edit");
    assert_eq!(before, after, "a scoped analysis must not overwrite the cache");
}

#[test]
fn real_duplication_introduced_mid_session_still_reported_at_stop() {
    let env = StopEnv::new();
    let path = env.file_path("svc.py");
    std::fs::write(&path, distinct_pair_source(1)).unwrap();
    run_three_edit_lifecycle(&env, &path, &twin_pair_source(2), &twin_pair_source(3));
    let out = env.run_stop();
    assert!(
        out.to_lowercase().contains("code duplication"),
        "genuinely identical functions introduced mid-session must still surface: {out}"
    );
}

#[test]
fn out_of_scope_zero_hashes_never_group_as_clones() {
    let source = distinct_pair_source(1);
    let len = source.len();
    let analysis = analyze::analyze_source(
        "svc.py",
        &source,
        Language::Python,
        None,
        ScanOptions::hook(Some((len + 10, len + 11))),
    )
    .expect("analysis");
    let dup = analysis.findings.iter().any(|f| f.smell == Smell::CodeDuplication);
    assert!(!dup, "zero sentinel hashes must not form clone groups: {:?}", analysis.findings);
}

#[test]
fn real_twins_still_group_in_unscoped_analysis() {
    let source = twin_pair_source(1);
    let analysis =
        analyze::analyze_source("svc.py", &source, Language::Python, None, ScanOptions::hook(None)).expect("analysis");
    let dup = analysis.findings.iter().any(|f| f.smell == Smell::CodeDuplication);
    assert!(dup, "identical functions must still be detected unscoped: {:?}", analysis.findings);
}

#[test]
fn ratchet_hash_fallback_ignores_zero_sentinel() {
    let cc_target = t().function.cc_warning + 1;
    let mut source = String::from("def new_fn(x):\n");
    for i in 0..cc_target - 1 {
        source.push_str(&format!("    if x == {i}:\n        return {i}\n"));
    }
    source.push_str("    return x\n");
    let len = source.len();
    let analysis = analyze::analyze_source(
        "svc.py",
        &source,
        Language::Python,
        None,
        ScanOptions::hook(Some((len + 10, len + 11))),
    )
    .expect("analysis");
    assert_eq!(analysis.metrics.functions[0].structural_hash, 0, "out-of-scope function must carry the sentinel");

    let baseline = baseline_from_entries(vec![FunctionBaselineEntry {
        name: "old_fn".to_string(),
        hash: 0,
        smell: Smell::ComplexMethod.snake_name().to_string(),
        mags: [cc_target + 20, cc_target + 20, 0],
        count: 1,
    }]);
    let complex: Vec<_> = analysis.findings.iter().filter(|f| f.smell == Smell::ComplexMethod).cloned().collect();
    assert!(!complex.is_empty(), "fixture must trip complex method: {:?}", analysis.findings);
    let kept = filter_worsened(complex, &baseline, &analysis.metrics);
    assert!(
        !kept.is_empty(),
        "a vanished zero-hash baseline entry must not suppress findings on other zero-hash functions"
    );
}
