use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::config;
use crate::hook::HookInput;
use crate::smells::{Finding, Location};
use crate::{parse, smells};

pub fn baseline_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("PULSE_BASELINE_DIR") {
        return PathBuf::from(dir);
    }
    PathBuf::from("/tmp/pulse-baselines")
}

pub fn session_baseline_dir(session_id: &str) -> PathBuf {
    if let Ok(dir) = std::env::var("PULSE_BASELINE_DIR") {
        return PathBuf::from(dir);
    }
    let truncated = &session_id[..session_id.len().min(16)];
    PathBuf::from(format!("/tmp/pulse-baselines-{truncated}"))
}

pub fn init_session_dir(session_id: Option<&str>) {
    if std::env::var("PULSE_BASELINE_DIR").is_ok() {
        return;
    }
    if let Some(sid) = session_id {
        let dir = session_baseline_dir(sid);
        std::env::set_var("PULSE_BASELINE_DIR", dir.as_os_str());
    }
}

pub fn is_fixture_file(path: &str) -> bool {
    path.replace('\\', "/").contains("/fixtures/")
}

pub fn cache_baseline(hook: &HookInput, cfg: Option<&config::PulseConfig>, current_source: &str) {
    let dominated = is_fixture_file(&hook.file_path) || baseline_path(&hook.file_path).exists();
    if dominated {
        return;
    }
    create_baseline_files(hook, cfg, current_source);
}

fn create_baseline_files(
    hook: &HookInput,
    cfg: Option<&config::PulseConfig>,
    current_source: &str,
) {
    let bp = baseline_path(&hook.file_path);
    let (counts, func_findings) = compute_baseline(hook, cfg, current_source);
    write_baseline(&bp, &counts);
    write_function_baseline(&hook.file_path, &func_findings);
    append_manifest(&hook.file_path);
}

pub fn load_baseline(file_path: &str) -> HashMap<String, usize> {
    let bp = baseline_path(file_path);
    let Ok(json) = std::fs::read_to_string(&bp) else {
        return HashMap::new();
    };
    serde_json::from_str(&json).unwrap_or_default()
}

pub fn load_function_baseline(file_path: &str) -> HashSet<String> {
    std::fs::read_to_string(baseline_dir().join(format!("{:016x}.funcs", hash_path(file_path))))
        .map(|content| content.lines().map(String::from).collect())
        .unwrap_or_default()
}

pub fn is_preexisting_finding(f: &Finding, baseline: &HashSet<String>) -> bool {
    match &f.location {
        Location::Function { name, .. } => {
            baseline.contains(&format!("{}:{}:{}", name, f.smell, f.detail))
        }
        Location::Module => false,
    }
}

pub fn count_module_findings(findings: &[Finding]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for f in findings {
        if matches!(f.location, Location::Module) {
            *counts.entry(f.smell.to_string()).or_default() += 1;
        }
    }
    counts
}

pub fn append_manifest(file_path: &str) {
    let dir = baseline_dir();
    let manifest = dir.join("manifest.txt");
    let already = std::fs::read_to_string(&manifest).unwrap_or_default();
    already.lines().all(|l| l != file_path).then(|| {
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&manifest)
            .map(|mut f| writeln!(f, "{file_path}"));
    });
}

fn hash_path(file_path: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    file_path.hash(&mut hasher);
    hasher.finish()
}

fn baseline_path(file_path: &str) -> PathBuf {
    baseline_dir().join(format!("{:016x}.json", hash_path(file_path)))
}

pub fn increment_edit_count(file_path: &str) -> u32 {
    let path = baseline_dir().join(format!("{:016x}.edits", hash_path(file_path)));
    let current: u32 = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);
    let next = current + 1;
    let tmp = baseline_dir().join(format!(
        "{:016x}.edits.tmp.{}",
        hash_path(file_path),
        std::process::id()
    ));
    if std::fs::write(&tmp, next.to_string()).is_ok() {
        let _ = std::fs::rename(&tmp, &path);
    }
    next
}

fn compute_baseline(
    hook: &HookInput,
    cfg: Option<&config::PulseConfig>,
    current_source: &str,
) -> (HashMap<String, usize>, Vec<String>) {
    let source = match reconstruct_pre_edit(hook, current_source) {
        Some(s) if !s.is_empty() => s,
        _ => return (HashMap::new(), Vec::new()),
    };
    let Some(lang) = parse::detect_language(Path::new(&hook.file_path)) else {
        return (HashMap::new(), Vec::new());
    };
    let Some(metrics) = parse::parse_and_walk(&source, lang) else {
        return (HashMap::new(), Vec::new());
    };
    let t = config::resolve_thresholds(cfg, lang);
    let disabled = config::resolve_disabled(cfg);
    let mut findings = smells::detect(&metrics, &source, &t);
    config::filter_disabled(&mut findings, &disabled);
    let module_counts = count_module_findings(&findings);
    let func_keys: Vec<String> = findings
        .iter()
        .filter_map(|f| match &f.location {
            Location::Function { name, .. } => Some(format!("{}:{}:{}", name, f.smell, f.detail)),
            Location::Module => None,
        })
        .collect();
    (module_counts, func_keys)
}

fn write_baseline(path: &Path, counts: &HashMap<String, usize>) {
    let _ = std::fs::create_dir_all(baseline_dir());
    let json = serde_json::to_string(counts).unwrap_or_default();
    let _ = std::fs::write(path, json);
}

fn write_function_baseline(file_path: &str, keys: &[String]) {
    let path = baseline_dir().join(format!("{:016x}.funcs", hash_path(file_path)));
    let _ = std::fs::create_dir_all(baseline_dir());
    let _ = std::fs::write(path, keys.join("\n"));
}

fn reconstruct_pre_edit(hook: &HookInput, current_source: &str) -> Option<String> {
    let (Some(old_str), Some(new_str)) = (&hook.old_string, &hook.new_string) else {
        return git_show_head(&hook.file_path);
    };
    Some(current_source.replacen(new_str, old_str, 1))
}

fn git_show_head(file_path: &str) -> Option<String> {
    let abs = std::fs::canonicalize(file_path).ok()?;
    let dir = abs.parent()?;
    let toplevel = std::process::Command::new("git")
        .args(["-C", dir.to_str()?, "rev-parse", "--show-toplevel"])
        .output()
        .ok()?;
    if !toplevel.status.success() {
        return None;
    }
    let root = String::from_utf8_lossy(&toplevel.stdout).trim().to_string();
    let root = std::fs::canonicalize(&root).ok()?;
    let rel = abs
        .strip_prefix(&root)
        .ok()?
        .to_string_lossy()
        .replace('\\', "/");
    let output = std::process::Command::new("git")
        .args(["-C", root.to_str()?, "show", &format!("HEAD:{rel}")])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}
