use std::collections::HashMap;
use std::path::Path;

use crate::applicability::{self, Surface};
use crate::config;
use crate::parse::{self, Language};
use crate::smells::{self, Finding, Location};
use crate::thresholds::Thresholds;
use crate::walk::FileMetrics;

pub struct AnalysisResultFull {
    pub findings: Vec<Finding>,
    pub filename: String,
    pub metrics: FileMetrics,
    pub thresholds: Thresholds,
}

#[derive(Debug, Clone, Copy)]
pub struct ScanOptions {
    pub edit_byte_range: Option<(usize, usize)>,
    pub surface: Surface,
}

impl ScanOptions {
    pub fn hook(edit_byte_range: Option<(usize, usize)>) -> Self {
        Self { edit_byte_range, surface: Surface::Hook }
    }

    pub fn check() -> Self {
        Self { edit_byte_range: None, surface: Surface::Check }
    }
}

impl AnalysisResultFull {
    pub fn fn_count(&self) -> u32 {
        self.metrics.functions.len() as u32
    }
    pub fn total_loc(&self) -> u32 {
        self.metrics.module.total_loc
    }
    pub fn sum_cc(&self) -> u32 {
        self.metrics.module.sum_cc
    }
}

pub fn read_source_unchecked(file_path: &str) -> Option<(Language, String)> {
    let path = Path::new(file_path);
    let lang = parse::detect_language(path)?;
    let source = std::fs::read_to_string(path).ok()?;
    Some((lang, source))
}

pub fn analyze_source(
    file_path: &str,
    source: &str,
    lang: Language,
    cfg: Option<&config::PulseConfig>,
    opts: ScanOptions,
) -> Option<AnalysisResultFull> {
    let thresholds = config::resolve_thresholds(cfg, lang);
    let metrics =
        parse::parse_and_walk_scoped(source, lang, opts.edit_byte_range, thresholds.cpg.enabled)?;
    let disabled = config::resolve_disabled(cfg);
    let mut findings = smells::detect(&metrics, source, &thresholds);
    config::filter_disabled(&mut findings, &disabled);
    applicability::filter_by_applicability(&mut findings, opts.surface);
    let filename = Path::new(file_path)
        .file_name()?
        .to_string_lossy()
        .into_owned();
    Some(AnalysisResultFull {
        findings,
        filename,
        metrics,
        thresholds,
    })
}

pub fn analyze_file(
    file_path: &str,
    cfg: Option<&config::PulseConfig>,
    opts: ScanOptions,
) -> Option<AnalysisResultFull> {
    let path = Path::new(file_path);
    if cfg.is_some_and(|c| config::is_ignored_for_file(c, path)) {
        return None;
    }
    let (lang, source) = read_source_unchecked(file_path)?;
    analyze_source(file_path, &source, lang, cfg, opts)
}

pub fn module_regressions(
    result: &AnalysisResultFull,
    baseline: &HashMap<String, usize>,
) -> Vec<Finding> {
    let mut current_counts: HashMap<smells::Smell, usize> = HashMap::new();
    for f in result
        .findings
        .iter()
        .filter(|f| matches!(f.location, Location::Module))
    {
        *current_counts.entry(f.smell).or_default() += 1;
    }
    result
        .findings
        .iter()
        .filter(|f| matches!(f.location, Location::Module))
        .filter(|f| {
            let baseline_count = baseline.get(f.smell.as_str()).copied().unwrap_or(0);
            current_counts.get(&f.smell).copied().unwrap_or(0) > baseline_count
        })
        .cloned()
        .collect()
}
