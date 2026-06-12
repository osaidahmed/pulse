use std::collections::HashSet;
use std::path::{Path, PathBuf};

use rayon::prelude::*;

use crate::audit::corpus::{Corpus, CorpusFile};
use crate::audit::{corpus_stats, record_extraction, vendor_filter, walk_typed_source_files_filtered, IgnoreFilter};
use crate::baselines;
use crate::parse::{self, Language};
use crate::test_detection;
use crate::thresholds::Thresholds;
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
    let corpus = Corpus::load(&typed);
    let vendored = vendored_paths(&corpus, thresholds);
    let measured: Vec<FileCensus> = corpus.files.par_iter().filter_map(measure).collect();
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

fn vendored_paths(corpus: &Corpus, thresholds: &Thresholds) -> HashSet<PathBuf> {
    let bundle = record_extraction::corpus_bundle_from(corpus, &thresholds.audit);
    let stats = corpus_stats::aggregate_corpus(bundle.features);
    vendor_filter::flagged_paths(&vendor_filter::classify(&stats, &thresholds.audit.pattern_mining.vendor))
}

fn sorted_paths(paths: &HashSet<PathBuf>) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = paths.iter().cloned().collect();
    out.sort();
    out
}

fn measure(file: &CorpusFile) -> Option<FileCensus> {
    let tree = file.tree.as_ref()?;
    let source = file.source.as_ref()?;
    let metrics = parse::walk_guarded_shared(tree, source, file.lang)?;
    Some(FileCensus { path: file.path.clone(), lang: file.lang, functions: metrics.functions, module: metrics.module })
}
