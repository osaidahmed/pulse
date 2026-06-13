pub mod priors;
pub mod stats;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use rayon::prelude::*;

use crate::audit::corpus_stats::PerFileFeatures;
use crate::audit::{corpus_stats, record_extraction, vendor_filter, walk_typed_source_files_filtered, IgnoreFilter};
use crate::baselines;
use crate::parse::{self, Language};
use crate::test_detection;
use crate::thresholds::{AuditThresholds, Thresholds};
use crate::walk::{FunctionMetrics, ModuleMetrics};

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
        .filter(|(path, _)| !baselines::is_fixture_file(&path.to_string_lossy()))
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
