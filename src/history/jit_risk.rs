use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::parse::Language;

use super::git::Commit;
use super::jit_thresholds::JitThresholds;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Quintiles {
    pub p20: f64,
    pub p50: f64,
    pub p80: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct JitCalibration {
    pub lt: Option<Quintiles>,
    pub age_days: Option<Quintiles>,
    pub entropy_bits: Option<f64>,
}

pub fn calibrate(
    typed_files: &[(PathBuf, Language)],
    commits: &[Commit],
    now_secs: i64,
    t: JitThresholds,
) -> JitCalibration {
    let lt = t.use_lt.then(|| percentiles(lt_values(typed_files))).flatten();
    let age_days = t.use_age.then(|| percentiles(age_values(typed_files, commits, now_secs))).flatten();
    let entropy_bits = t.use_entropy.then_some(t.entropy_bits);
    JitCalibration { lt, age_days, entropy_bits }
}

fn lt_values(typed_files: &[(PathBuf, Language)]) -> Vec<f64> {
    typed_files.iter().filter_map(|(p, _)| std::fs::read_to_string(p).ok()).map(|s| s.lines().count() as f64).collect()
}

fn age_values(typed_files: &[(PathBuf, Language)], commits: &[Commit], now_secs: i64) -> Vec<f64> {
    let last = last_commit_ts(commits);
    typed_files.iter().filter_map(|(p, _)| last.get(p)).map(|&ts| age_days_since(ts, now_secs)).collect()
}

fn age_days_since(ts: i64, now_secs: i64) -> f64 {
    f64::from(i32::try_from((now_secs - ts).max(0)).unwrap_or(i32::MAX)) / 86_400.0
}

fn last_commit_ts(commits: &[Commit]) -> HashMap<PathBuf, i64> {
    let mut latest: HashMap<PathBuf, i64> = HashMap::new();
    for c in commits {
        for f in &c.files {
            latest.entry(f.clone()).and_modify(|ts| *ts = (*ts).max(c.timestamp)).or_insert(c.timestamp);
        }
    }
    latest
}

pub fn percentiles(mut values: Vec<f64>) -> Option<Quintiles> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    Some(Quintiles { p20: quantile(&values, 0.20), p50: quantile(&values, 0.50), p80: quantile(&values, 0.80) })
}

fn quantile(sorted: &[f64], q: f64) -> f64 {
    let rank = (q * sorted.len() as f64).ceil() as usize;
    sorted[rank.saturating_sub(1).min(sorted.len() - 1)]
}

pub fn calib_path(repo_root: &Path) -> PathBuf {
    pulse_core::analytics_dir().join("jit").join(format!("{}.json", repo_key(repo_root)))
}

fn repo_key(repo_root: &Path) -> String {
    let anchor = super::git::repo_toplevel(repo_root)
        .or_else(|| std::fs::canonicalize(repo_root).ok())
        .unwrap_or_else(|| repo_root.to_path_buf());
    format!("{:016x}", xxhash_rust::xxh3::xxh3_64(anchor.to_string_lossy().as_bytes()))
}

pub fn write_calibration(repo_root: &Path, calib: &JitCalibration) -> std::io::Result<()> {
    let path = calib_path(repo_root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(calib).unwrap_or_default())
}

pub fn read_calibration(repo_root: &Path) -> Option<JitCalibration> {
    serde_json::from_str(&std::fs::read_to_string(calib_path(repo_root)).ok()?).ok()
}

pub fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

pub fn hook_advisory(source: &str, file_path: &Path) -> Option<String> {
    if !pulse_core::analytics_dir().join("jit").is_dir() {
        return None;
    }
    let dir = file_path.parent()?;
    let calib = read_calibration(dir)?;
    let lt = calib.lt.and_then(|q| lt_band_message(source.lines().count(), &q));
    let age = calib.age_days.and_then(|q| {
        band(file_age_days(file_path, now_secs()), |v| v < q.p20, |v| {
            format!("[risk] file age is below the repo's 20th percentile (AGE {v:.0}d < p20 {:.0}d); recently changed code churns and is defect-prone — change carefully", q.p20)
        })
    });
    let entropy = calib.entropy_bits.and_then(|bits| {
        band(working_tree_entropy(dir), |h| h >= bits, |h| {
            format!("[risk] this session's edits are spread across the working tree (change entropy {h:.1} bits ≥ {bits:.1}); scattered edits across many files are defect-prone — prefer a focused, cohesive change")
        })
    });
    join_advisory([lt, age, entropy])
}

fn band(value: Option<f64>, flagged: impl Fn(f64) -> bool, message: impl Fn(f64) -> String) -> Option<String> {
    value.filter(|&v| flagged(v)).map(message)
}

fn join_advisory(parts: [Option<String>; 3]) -> Option<String> {
    let lines: Vec<String> = parts.into_iter().flatten().collect();
    (!lines.is_empty()).then(|| lines.join("\n"))
}

fn working_tree_entropy(dir: &Path) -> Option<f64> {
    diffusion_entropy(&super::git::working_tree_numstat(dir)?)
}

fn diffusion_entropy(line_changes: &[u64]) -> Option<f64> {
    let nonzero: Vec<u64> = line_changes.iter().copied().filter(|&c| c > 0).collect();
    if nonzero.len() < 2 {
        return None;
    }
    let total = nonzero.iter().sum::<u64>() as f64;
    Some(nonzero.iter().map(|&c| c as f64 / total).map(|p| -p * p.log2()).sum())
}

fn file_age_days(file_path: &Path, now_secs: i64) -> Option<f64> {
    let ts = super::git::last_commit_unix(file_path.parent()?, Path::new(file_path.file_name()?))?;
    (ts <= now_secs).then(|| age_days_since(ts, now_secs))
}

fn lt_band_message(file_lt: usize, lt: &Quintiles) -> Option<String> {
    let value = f64::from(u32::try_from(file_lt).unwrap_or(u32::MAX));
    if value < lt.p20 {
        Some(format!(
            "[risk] file length is below the repo's 20th percentile (LT {file_lt} < p20 {:.0}); small dense files churn — change carefully",
            lt.p20
        ))
    } else if value > lt.p80 {
        Some(format!(
            "[risk] file length is above the repo's 80th percentile (LT {file_lt} > p80 {:.0}); a maintainability hotspot — prefer extracting over adding",
            lt.p80
        ))
    } else {
        None
    }
}
