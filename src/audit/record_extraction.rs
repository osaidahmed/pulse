use std::path::{Path, PathBuf};

use crate::parse::Language;
use crate::thresholds::AuditThresholds;

use super::corpus_stats::PerFileFeatures;
use super::walker::{self, KindIndex, SubtreeRecord, WalkOutput};

pub struct CorpusBundle {
    pub subtrees: Vec<SubtreeRecord>,
    pub features: Vec<PerFileFeatures>,
    pub kinds_by_fp: KindIndex,
}

pub fn corpus_bundle(typed: &[(PathBuf, Language)], thresholds: &AuditThresholds) -> CorpusBundle {
    corpus_bundle_from(&super::corpus::Corpus::load(typed), thresholds)
}

pub fn corpus_bundle_from(corpus: &super::corpus::Corpus, thresholds: &AuditThresholds) -> CorpusBundle {
    use rayon::prelude::*;
    let outputs: Vec<Option<WalkOutput>> = corpus.files.par_iter().map(|file| walk_one(file, thresholds)).collect();
    let mut bundle = CorpusBundle { subtrees: Vec::new(), features: Vec::new(), kinds_by_fp: KindIndex::default() };
    for output in outputs.into_iter().flatten() {
        bundle.subtrees.extend(output.subtrees);
        bundle.features.push(output.features);
        for (fp, kinds) in output.kinds_by_fp {
            bundle.kinds_by_fp.entry(fp).or_insert(kinds);
        }
    }
    bundle
}

pub fn records_and_features(
    typed: &[(PathBuf, Language)],
    thresholds: &AuditThresholds,
) -> (Vec<SubtreeRecord>, Vec<PerFileFeatures>) {
    let bundle = corpus_bundle(typed, thresholds);
    (bundle.subtrees, bundle.features)
}

pub fn records_only(typed: &[(PathBuf, Language)], thresholds: &AuditThresholds) -> Vec<SubtreeRecord> {
    records_and_features(typed, thresholds).0
}

pub fn for_dir(root: &Path, lang: Language, thresholds: &AuditThresholds) -> Vec<SubtreeRecord> {
    let typed: Vec<(PathBuf, Language)> =
        super::walk_typed_source_files(root, true).into_iter().filter(|(_, l)| *l == lang).collect();
    records_only(&typed, thresholds)
}

fn walk_one(file: &super::corpus::CorpusFile, thresholds: &AuditThresholds) -> Option<WalkOutput> {
    let (source, tree) = file.parsed()?;
    Some(walker::extract_records(tree, source, file.lang, &file.path, thresholds))
}
