#![allow(dead_code)]

pub mod discovery;
pub mod finding;
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

pub fn run(opts: &AuditOpts, thresholds: &AuditThresholds) -> Vec<AuditFinding> {
    let lang = Language::Python;
    let files = walk_python_files(&opts.root, lang);
    let total_files = files.len();
    let records = extract_subtrees_from_files(&files, lang, thresholds);
    let clusters = discovery::freqt_mine(&records, thresholds);
    scoring::apply_idf(clusters, total_files, thresholds)
}

const SKIP_DIRS: &[&str] = &[
    "node_modules",
    "target",
    "vendor",
    "build",
    "dist",
    "__pycache__",
];

pub fn extract_subtrees_for_dir(
    root: &Path,
    lang: Language,
    thresholds: &AuditThresholds,
) -> Vec<SubtreeRecord> {
    if !root.exists() {
        return Vec::new();
    }
    let files = walk_python_files(root, lang);
    extract_subtrees_from_files(&files, lang, thresholds)
}

fn extract_subtrees_from_files(
    files: &[PathBuf],
    lang: Language,
    thresholds: &AuditThresholds,
) -> Vec<SubtreeRecord> {
    let mut out = Vec::new();
    for path in files {
        let Ok(source) = std::fs::read_to_string(path) else {
            continue;
        };
        let Some(tree) = parse::parse_only(&source, lang) else {
            continue;
        };
        out.extend(walker::extract_subtrees(&tree, &source, lang, path, thresholds));
    }
    out
}

fn walk_python_files(root: &Path, lang: Language) -> Vec<PathBuf> {
    let mut files = Vec::new();
    walk_dir(root, lang, &mut files);
    files.sort();
    files
}

fn walk_dir(dir: &Path, lang: Language, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.starts_with('.') || SKIP_DIRS.contains(&name) {
                continue;
            }
            walk_dir(&path, lang, files);
        } else if matches_language(&path, lang) {
            files.push(path);
        }
    }
}

fn matches_language(path: &Path, lang: Language) -> bool {
    parse::detect_language(path) == Some(lang)
}
