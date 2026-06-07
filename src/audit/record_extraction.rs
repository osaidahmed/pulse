use std::path::{Path, PathBuf};

use crate::parse::{self, Language};
use crate::thresholds::AuditThresholds;

use super::corpus_stats::PerFileFeatures;
use super::walker::{self, KindIndex, SubtreeRecord, WalkOutput};

pub struct CorpusBundle {
    pub subtrees: Vec<SubtreeRecord>,
    pub features: Vec<PerFileFeatures>,
    pub kinds_by_fp: KindIndex,
}

pub fn corpus_bundle(typed: &[(PathBuf, Language)], thresholds: &AuditThresholds) -> CorpusBundle {
    let mut bundle = CorpusBundle { subtrees: Vec::new(), features: Vec::new(), kinds_by_fp: KindIndex::default() };
    for (path, lang) in typed {
        if let Some(output) = walk_one(path, *lang, thresholds) {
            bundle.subtrees.extend(output.subtrees);
            bundle.features.push(output.features);
            for (fp, kinds) in output.kinds_by_fp {
                bundle.kinds_by_fp.entry(fp).or_insert(kinds);
            }
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

fn walk_one(path: &Path, lang: Language, thresholds: &AuditThresholds) -> Option<WalkOutput> {
    let source = std::fs::read_to_string(path).ok()?;
    let tree = parse::parse_guarded(&source, lang)?;
    Some(walker::extract_records(&tree, &source, lang, path, thresholds))
}
