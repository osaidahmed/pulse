#![allow(dead_code)]

pub mod finding;
pub mod walker;

use std::path::{Path, PathBuf};

use crate::parse::{self, Language};
use crate::thresholds::AuditThresholds;
use walker::SubtreeRecord;

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
    let mut out = Vec::new();
    if !root.exists() {
        return out;
    }
    for path in walk_python_files(root, lang) {
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Some(tree) = parse::parse_only(&source, lang) else {
            continue;
        };
        out.extend(walker::extract_subtrees(&tree, &source, lang, &path, thresholds));
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
