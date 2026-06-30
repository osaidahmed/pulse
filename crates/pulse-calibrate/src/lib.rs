pub mod emit;
pub mod estimator;
pub mod priors;
pub mod stats;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use rayon::prelude::*;

use pulse_corpus::corpus_stats::PerFileFeatures;
use pulse_corpus::{corpus_stats, record_extraction, vendor_filter, walk_typed_source_files_filtered, IgnoreFilter};

use pulse_syntax::parse::{self, Language};
use pulse_syntax::test_detection;
use pulse_syntax::walk::{FunctionMetrics, ModuleMetrics};
use pulse_thresholds::{AuditThresholds, Thresholds};

pub struct Census {
    pub cpg_enabled: bool,
    pub main: Vec<FileCensus>,
    pub tests: Vec<FileCensus>,
    pub vendored_excluded: Vec<PathBuf>,
}

pub struct FileCensus {
    pub path: PathBuf,
    pub lang: Language,
    pub functions: Vec<FunctionMetrics>,
    pub module: ModuleMetrics,
}

pub fn collect(root: &Path, thresholds: &Thresholds, filter: &IgnoreFilter) -> Census {
    let typed: Vec<(PathBuf, Language)> = walk_typed_source_files_filtered(root, true, filter)
        .into_iter()
        .filter(|(path, _)| !pulse_core::is_fixture_file(&path.to_string_lossy()))
        .collect();
    let streamed: Vec<(FileCensus, PerFileFeatures)> =
        typed.par_iter().filter_map(|(path, lang)| measure_file(path, *lang, &thresholds.audit)).collect();
    let (measured, features): (Vec<FileCensus>, Vec<PerFileFeatures>) = streamed.into_iter().unzip();
    let stats = corpus_stats::aggregate_corpus(features);
    let vendored =
        vendor_filter::flagged_paths(&vendor_filter::classify(&stats, &thresholds.audit.pattern_mining.vendor));
    let mut census = Census {
        cpg_enabled: thresholds.cpg.enabled,
        main: Vec::new(),
        tests: Vec::new(),
        vendored_excluded: sorted_paths(&vendored),
    };
    for file in measured {
        if vendored.contains(&file.path) {
            continue;
        }
        if test_detection::is_test_file(&file.path.to_string_lossy()) {
            census.tests.push(file);
        } else {
            census.main.push(file);
        }
    }
    census.main.sort_by(|a, b| a.path.cmp(&b.path));
    census.tests.sort_by(|a, b| a.path.cmp(&b.path));
    census
}

#[derive(Default)]
pub struct AccumSummary {
    pub main_files: usize,
    pub test_files: usize,
    pub vendored: usize,
}

struct FileContribution {
    path: PathBuf,
    is_test: bool,
    features: PerFileFeatures,
    partial: priors::PriorsBuilder,
}

pub fn accumulate(
    root: &Path,
    thresholds: &Thresholds,
    filter: &IgnoreFilter,
    builder: &mut priors::PriorsBuilder,
) -> AccumSummary {
    let typed: Vec<(PathBuf, Language)> = walk_typed_source_files_filtered(root, true, filter)
        .into_iter()
        .filter(|(path, _)| !pulse_core::is_fixture_file(&path.to_string_lossy()))
        .collect();
    let contributions: Vec<FileContribution> =
        typed.par_iter().filter_map(|(path, lang)| contribution(path, *lang, &thresholds.audit)).collect();
    let stats = corpus_stats::aggregate_corpus(contributions.iter().map(|c| c.features.clone()).collect());
    let vendored =
        vendor_filter::flagged_paths(&vendor_filter::classify(&stats, &thresholds.audit.pattern_mining.vendor));
    let mut summary = AccumSummary { vendored: vendored.len(), ..AccumSummary::default() };
    for c in &contributions {
        if vendored.contains(c.path.as_path()) {
            continue;
        }
        builder.merge(&c.partial);
        if c.is_test {
            summary.test_files += 1;
        } else {
            summary.main_files += 1;
        }
    }
    summary
}

fn contribution(path: &Path, lang: Language, audit: &AuditThresholds) -> Option<FileContribution> {
    let source: std::sync::Arc<str> = std::sync::Arc::from(std::fs::read_to_string(path).ok()?);
    let tree = parse::parse_guarded_shared(&source, lang)?;
    let metrics = parse::walk_guarded_shared(&tree, &source, lang)?;
    let features = record_extraction::file_features(&tree, &source, lang, path, audit)?;
    let is_test = test_detection::is_test_file(&path.to_string_lossy());
    let mut partial = priors::PriorsBuilder::default();
    partial.add_file_metrics(lang, is_test, &metrics);
    Some(FileContribution { path: path.to_path_buf(), is_test, features, partial })
}

fn sorted_paths(paths: &HashSet<PathBuf>) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = paths.iter().cloned().collect();
    out.sort();
    out
}

fn measure_file(path: &Path, lang: Language, audit: &AuditThresholds) -> Option<(FileCensus, PerFileFeatures)> {
    let source: std::sync::Arc<str> = std::sync::Arc::from(std::fs::read_to_string(path).ok()?);
    let tree = parse::parse_guarded_shared(&source, lang)?;
    let metrics = parse::walk_guarded_shared(&tree, &source, lang)?;
    let features = record_extraction::file_features(&tree, &source, lang, path, audit)?;
    let census = FileCensus { path: path.to_path_buf(), lang, functions: metrics.functions, module: metrics.module };
    Some((census, features))
}
