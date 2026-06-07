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
}

pub fn calibrate(
    typed_files: &[(PathBuf, Language)],
    commits: &[Commit],
    now_secs: i64,
    t: JitThresholds,
) -> JitCalibration {
    let lt = t.use_lt.then(|| percentiles(lt_values(typed_files))).flatten();
    let age_days = t.use_age.then(|| percentiles(age_values(typed_files, commits, now_secs))).flatten();
    JitCalibration { lt, age_days }
}

fn lt_values(typed_files: &[(PathBuf, Language)]) -> Vec<f64> {
    typed_files.iter().filter_map(|(p, _)| std::fs::read_to_string(p).ok()).map(|s| s.lines().count() as f64).collect()
}

fn age_values(typed_files: &[(PathBuf, Language)], commits: &[Commit], now_secs: i64) -> Vec<f64> {
    let last = last_commit_ts(commits);
    typed_files
        .iter()
        .filter_map(|(p, _)| last.get(p))
        .map(|&ts| f64::from(i32::try_from((now_secs - ts).max(0)).unwrap_or(i32::MAX)) / 86_400.0)
        .collect()
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
    crate::analytics::analytics_dir().join("jit").join(format!("{}.json", repo_key(repo_root)))
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
