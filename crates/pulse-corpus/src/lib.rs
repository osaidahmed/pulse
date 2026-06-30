pub mod corpus;
pub mod corpus_stats;
pub mod lang_kinds;
pub mod record_extraction;
pub mod vendor_filter;
pub mod walker;

use std::path::{Path, PathBuf};

use pulse_config::IgnoreMatcher;
use pulse_syntax::parse::{self, Language};
use pulse_syntax::test_detection;

pub struct IgnoreFilter<'a> {
    matcher: &'a IgnoreMatcher,
    base: &'a Path,
}

impl<'a> IgnoreFilter<'a> {
    pub fn new(matcher: &'a IgnoreMatcher, base: &'a Path) -> Self {
        Self { matcher, base }
    }

    pub fn matches(&self, path: &Path) -> bool {
        self.matcher.matches_file(self.base, path)
    }
}

const SKIP_DIRS: &[&str] = &["node_modules", "target", "vendor", "build", "dist", "__pycache__"];

pub const MAX_FILES: usize = 100_000;

#[doc(hidden)]
pub fn audit_traversal_cap_hit(file_count: usize) -> bool {
    file_count >= MAX_FILES
}

pub fn walk_typed_source_files(root: &Path, include_tests: bool) -> Vec<(PathBuf, Language)> {
    let empty = IgnoreMatcher::from_patterns(&[]);
    let filter = IgnoreFilter::new(&empty, root);
    walk_typed_source_files_filtered(root, include_tests, &filter)
}

pub fn walk_typed_source_files_filtered(
    root: &Path,
    include_tests: bool,
    filter: &IgnoreFilter,
) -> Vec<(PathBuf, Language)> {
    if !root.exists() {
        return Vec::new();
    }
    let mut files = Vec::new();
    walk_typed_dir(root, include_tests, filter, &mut files);
    if !include_tests {
        let test_roots = pulse_buildmeta::declared_test_roots(root);
        if !test_roots.is_empty() {
            files.retain(|(p, _)| !pulse_buildmeta::is_under_test_root(p, &test_roots));
        }
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files
}

fn walk_typed_dir(dir: &Path, include_tests: bool, filter: &IgnoreFilter, files: &mut Vec<(PathBuf, Language)>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        if files.len() >= MAX_FILES {
            return;
        }
        descend_or_collect(entry.path(), include_tests, filter, files);
    }
}

fn descend_or_collect(path: PathBuf, include_tests: bool, filter: &IgnoreFilter, files: &mut Vec<(PathBuf, Language)>) {
    if filter.matches(&path) {
        return;
    }
    if path.is_dir() {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.starts_with('.') || SKIP_DIRS.contains(&name) {
            return;
        }
        walk_typed_dir(&path, include_tests, filter, files);
        return;
    }
    if !include_tests && test_detection::is_test_file(&path.to_string_lossy()) {
        return;
    }
    if let Some(lang) = parse::detect_language(&path) {
        files.push((path, lang));
    }
}
