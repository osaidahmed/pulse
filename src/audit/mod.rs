#![allow(dead_code)]

pub mod discovery;
pub mod finding;
pub mod lang_kinds;
pub mod output;
pub mod scoring;
pub mod walker;

use std::path::{Path, PathBuf};

use crate::parse::{self, Language};
use crate::thresholds::AuditThresholds;
use finding::AuditFinding;
use walker::SubtreeRecord;

pub struct AuditOpts {
    pub root: PathBuf,
    pub layer: Option<u8>,
    pub json: bool,
}

const SKIP_DIRS: &[&str] = &[
    "node_modules",
    "target",
    "vendor",
    "build",
    "dist",
    "__pycache__",
];

pub fn run(opts: &AuditOpts, thresholds: &AuditThresholds) -> Vec<AuditFinding> {
    let typed_files = walk_typed_source_files(&opts.root);
    let total_files = typed_files.len();
    let records = extract_records_from_typed_files(&typed_files, thresholds);
    let clusters = discovery::freqt_mine(&records, thresholds);
    scoring::apply_idf(clusters, total_files, thresholds)
}

pub fn extract_subtrees_for_dir(
    root: &Path,
    lang: Language,
    thresholds: &AuditThresholds,
) -> Vec<SubtreeRecord> {
    let typed: Vec<(PathBuf, Language)> = walk_typed_source_files(root)
        .into_iter()
        .filter(|(_, l)| *l == lang)
        .collect();
    extract_records_from_typed_files(&typed, thresholds)
}

pub fn walk_typed_source_files(root: &Path) -> Vec<(PathBuf, Language)> {
    if !root.exists() {
        return Vec::new();
    }
    let mut files = Vec::new();
    walk_typed_dir(root, &mut files);
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files
}

fn walk_typed_dir(dir: &Path, files: &mut Vec<(PathBuf, Language)>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        descend_or_collect(entry.path(), files);
    }
}

fn descend_or_collect(path: PathBuf, files: &mut Vec<(PathBuf, Language)>) {
    if path.is_dir() {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.starts_with('.') || SKIP_DIRS.contains(&name) {
            return;
        }
        walk_typed_dir(&path, files);
        return;
    }
    if let Some(lang) = parse::detect_language(&path) {
        files.push((path, lang));
    }
}

fn extract_records_from_typed_files(
    typed: &[(PathBuf, Language)],
    thresholds: &AuditThresholds,
) -> Vec<SubtreeRecord> {
    let mut out = Vec::new();
    for (path, lang) in typed {
        if let Some(records) = extract_records_for_file(path, *lang, thresholds) {
            out.extend(records);
        }
    }
    out
}

fn extract_records_for_file(
    path: &Path,
    lang: Language,
    thresholds: &AuditThresholds,
) -> Option<Vec<SubtreeRecord>> {
    let source = std::fs::read_to_string(path).ok()?;
    let tree = parse::parse_only(&source, lang)?;
    Some(walker::extract_subtrees(&tree, &source, lang, path, thresholds))
}
