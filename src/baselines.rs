use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::hook::HookInput;
use crate::smells::{Finding, Location};
use crate::{parse, smells, thresholds};

pub fn baseline_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("PULSE_BASELINE_DIR") {
        return PathBuf::from(dir);
    }
    PathBuf::from("/tmp/pulse-baselines")
}

pub fn cache_baseline(hook: &HookInput) {
    let bp = baseline_path(&hook.file_path);
    if bp.exists() {
        return;
    }

    let counts = compute_baseline_counts(hook);
    write_baseline(&bp, &counts);
    append_manifest(&hook.file_path);
}

pub fn load_baseline(file_path: &str) -> HashMap<String, usize> {
    let bp = baseline_path(file_path);
    let Ok(json) = std::fs::read_to_string(&bp) else {
        return HashMap::new();
    };
    serde_json::from_str(&json).unwrap_or_default()
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
    let manifest = baseline_dir().join("manifest.txt");
    let existing = std::fs::read_to_string(&manifest).unwrap_or_default();
    if existing.lines().any(|l| l == file_path) {
        return;
    }
    let _ = std::fs::create_dir_all(baseline_dir());
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&manifest)
    {
        let _ = writeln!(f, "{file_path}");
    }
}

fn baseline_path(file_path: &str) -> PathBuf {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    file_path.hash(&mut hasher);
    baseline_dir().join(format!("{:016x}.json", hasher.finish()))
}

fn compute_baseline_counts(hook: &HookInput) -> HashMap<String, usize> {
    let source = match reconstruct_pre_edit(hook) {
        Some(s) if !s.is_empty() => s,
        _ => return HashMap::new(),
    };
    let Some(lang) = parse::detect_language(Path::new(&hook.file_path)) else {
        return HashMap::new();
    };
    let Some(metrics) = parse::parse_and_walk(&source, lang) else {
        return HashMap::new();
    };
    let t = thresholds::Thresholds::default();
    let findings = smells::detect(&metrics, &source, &t);
    count_module_findings(&findings)
}

fn write_baseline(path: &Path, counts: &HashMap<String, usize>) {
    let _ = std::fs::create_dir_all(baseline_dir());
    let json = serde_json::to_string(counts).unwrap_or_default();
    let _ = std::fs::write(path, json);
}

fn reconstruct_pre_edit(hook: &HookInput) -> Option<String> {
    if let (Some(old_str), Some(new_str)) = (&hook.old_string, &hook.new_string) {
        let current = std::fs::read_to_string(&hook.file_path).ok()?;
        return Some(current.replacen(new_str, old_str, 1));
    }

    let output = std::process::Command::new("git")
        .args(["show", &format!("HEAD:{}", &hook.file_path)])
        .output()
        .ok()?;
    if output.status.success() {
        return Some(String::from_utf8_lossy(&output.stdout).into_owned());
    }

    None
}
