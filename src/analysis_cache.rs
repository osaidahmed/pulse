use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::analyze::AnalysisResultFull;
use crate::baselines;
use crate::smells::{smell_from_snake_case, Finding, Location};

#[derive(Serialize, Deserialize)]
struct CachedFinding {
    smell: String,
    function: Option<(String, u32, u32)>,
    detail: String,
}

#[derive(Serialize, Deserialize)]
pub struct CachedAnalysis {
    source_hash: u64,
    findings: Vec<CachedFinding>,
    pub functions: Vec<(String, u64)>,
}

impl CachedAnalysis {
    pub fn findings(&self) -> Vec<Finding> {
        self.findings.iter().filter_map(decode_finding).collect()
    }
}

pub fn store(file_path: &str, source: &str, analysis: &AnalysisResultFull) {
    let cached = CachedAnalysis {
        source_hash: xxhash_rust::xxh3::xxh3_64(source.as_bytes()),
        findings: analysis.findings.iter().map(encode_finding).collect(),
        functions: analysis.metrics.functions.iter().map(|f| (f.name.clone(), f.structural_hash)).collect(),
    };
    let Ok(json) = serde_json::to_string(&cached) else { return };
    let _ = std::fs::create_dir_all(baselines::baseline_dir());
    let _ = std::fs::write(cache_path(file_path), json);
}

pub fn load(file_path: &str) -> Option<CachedAnalysis> {
    let source = std::fs::read_to_string(file_path).ok()?;
    let raw = std::fs::read_to_string(cache_path(file_path)).ok()?;
    let cached: CachedAnalysis = serde_json::from_str(&raw).ok()?;
    (cached.source_hash == xxhash_rust::xxh3::xxh3_64(source.as_bytes())).then_some(cached)
}

fn encode_finding(f: &Finding) -> CachedFinding {
    let function = match &f.location {
        Location::Function { name, start_line, end_line } => Some((name.clone(), *start_line, *end_line)),
        Location::Module => None,
    };
    CachedFinding { smell: f.smell.snake_name().to_string(), function, detail: f.detail.clone() }
}

fn decode_finding(c: &CachedFinding) -> Option<Finding> {
    let smell = smell_from_snake_case(&c.smell)?;
    let location = match &c.function {
        Some((name, start, end)) => Location::Function { name: name.clone(), start_line: *start, end_line: *end },
        None => Location::Module,
    };
    Some(Finding { smell, location, detail: c.detail.clone() })
}

fn cache_path(file_path: &str) -> PathBuf {
    baselines::baseline_dir().join(format!("{}.analysis", baselines::file_key(file_path)))
}
