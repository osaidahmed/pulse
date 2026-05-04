#![allow(dead_code)]

pub mod cycles;
pub mod discovery;
pub mod finding;
pub mod graph;
pub mod import_call_form;
pub mod import_command_form;
pub mod import_jsts;
pub mod import_kinds;
pub mod import_php;
pub mod import_preprocessor;
pub mod import_python;
pub mod imports;
pub mod lang_kinds;
pub mod martin;
pub mod output;
pub mod package_metrics;
pub mod scoring;
pub mod walker;

use std::path::{Path, PathBuf};

use crate::parse::{self, Language};
use crate::test_detection;
use crate::thresholds::AuditThresholds;
use finding::AuditFinding;
use graph::InputEdge;
use package_metrics::ModuleProfile;
use walker::SubtreeRecord;

pub struct AuditOpts {
    pub root: PathBuf,
    pub layer: Option<u8>,
    pub json: bool,
    pub include_tests: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditPass {
    PatternMining,
    PackageMetrics,
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
    let typed_files = walk_typed_source_files(&opts.root, opts.include_tests);
    let passes = active_passes(opts.layer);
    let mut findings: Vec<AuditFinding> = Vec::new();
    if passes.contains(&AuditPass::PatternMining) {
        findings.extend(run_pattern_mining(&typed_files, thresholds));
    }
    if passes.contains(&AuditPass::PackageMetrics) {
        findings.extend(run_package_metrics(&typed_files, &opts.root, thresholds));
    }
    findings
}

pub fn active_passes(layer: Option<u8>) -> Vec<AuditPass> {
    match layer {
        Some(3) => vec![AuditPass::PatternMining],
        Some(5) => vec![AuditPass::PackageMetrics],
        _ => vec![AuditPass::PatternMining, AuditPass::PackageMetrics],
    }
}

fn run_pattern_mining(
    typed_files: &[(PathBuf, Language)],
    thresholds: &AuditThresholds,
) -> Vec<AuditFinding> {
    let total_files = typed_files.len();
    let records = extract_records_from_typed_files(typed_files, thresholds);
    let clusters = discovery::freqt_mine(&records, thresholds);
    scoring::apply_idf(clusters, total_files, thresholds)
}

const MIN_SUPPORTED_MODULES_FOR_PACKAGE_METRICS: usize = 5;

fn run_package_metrics(
    typed_files: &[(PathBuf, Language)],
    root: &Path,
    thresholds: &AuditThresholds,
) -> Vec<AuditFinding> {
    let supported_count = typed_files
        .iter()
        .filter(|(_, l)| imports::has_extractor(*l))
        .count();
    if supported_count < MIN_SUPPORTED_MODULES_FOR_PACKAGE_METRICS {
        return Vec::new();
    }
    let edges = collect_import_edges(typed_files, root);
    let lang_by_path = build_lang_by_path(typed_files);
    package_metrics::run_with_module_count(
        &edges,
        |path| profile_for_path(path, &lang_by_path),
        thresholds,
        supported_count as u32,
    )
}

fn collect_import_edges(typed_files: &[(PathBuf, Language)], root: &Path) -> Vec<InputEdge> {
    let typed_set: std::collections::HashSet<PathBuf> =
        typed_files.iter().map(|(p, _)| p.clone()).collect();
    let mut edges: Vec<InputEdge> = Vec::new();
    for (path, lang) in typed_files {
        edges.extend(edges_for_file(path, *lang, root, &typed_set, typed_files));
    }
    edges
}

fn edges_for_file(
    path: &Path,
    lang: Language,
    root: &Path,
    typed_set: &std::collections::HashSet<PathBuf>,
    typed_files: &[(PathBuf, Language)],
) -> Vec<InputEdge> {
    let Ok(source) = std::fs::read_to_string(path) else { return Vec::new() };
    let Some(tree) = parse::parse_only(&source, lang) else { return Vec::new() };
    let raws = imports::extract_imports(&tree, &source, lang);
    let mut out: Vec<InputEdge> = Vec::new();
    for raw in raws {
        let Some(resolved) = resolve_via_strategies(&raw.target, path, root, lang, typed_set) else {
            continue;
        };
        let target_lang = typed_files
            .iter()
            .find(|(p, _)| p == &resolved)
            .map_or(lang, |(_, l)| *l);
        out.push(InputEdge {
            source: path.to_path_buf(),
            target: resolved,
            source_lang: lang,
            target_lang,
        });
    }
    out
}

fn resolve_via_strategies(
    raw: &str,
    source_file: &Path,
    root: &Path,
    lang: Language,
    typed_set: &std::collections::HashSet<PathBuf>,
) -> Option<PathBuf> {
    if let Some(p) = imports::resolve_target(raw, source_file, root, lang) {
        if typed_set.contains(&p) {
            return Some(p);
        }
    }
    imports::resolve_by_suffix(raw, lang, typed_set)
}

fn build_lang_by_path(typed_files: &[(PathBuf, Language)]) -> std::collections::HashMap<PathBuf, Language> {
    typed_files.iter().map(|(p, l)| (p.clone(), *l)).collect()
}

fn profile_for_path(
    path: &Path,
    lang_by_path: &std::collections::HashMap<PathBuf, Language>,
) -> ModuleProfile {
    let lang = lang_by_path.get(path).copied();
    let import_confidence = lang.map_or(finding::ImportConfidence::BestEffort, imports::confidence_for);
    ModuleProfile {
        abstractness: martin::AbstractnessRecord {
            abstractness: 0.0,
            confidence: finding::ImportConfidence::NaAbstraction,
        },
        import_confidence,
    }
}

#[allow(dead_code)]
pub fn run_package_metrics_from_edges(
    edges: &[InputEdge],
    profile_lookup: impl Fn(&Path) -> ModuleProfile,
    thresholds: &AuditThresholds,
) -> Vec<AuditFinding> {
    package_metrics::run_from_edges(edges, profile_lookup, thresholds)
}

pub fn extract_subtrees_for_dir(
    root: &Path,
    lang: Language,
    thresholds: &AuditThresholds,
) -> Vec<SubtreeRecord> {
    let typed: Vec<(PathBuf, Language)> = walk_typed_source_files(root, true)
        .into_iter()
        .filter(|(_, l)| *l == lang)
        .collect();
    extract_records_from_typed_files(&typed, thresholds)
}

pub fn walk_typed_source_files(root: &Path, include_tests: bool) -> Vec<(PathBuf, Language)> {
    if !root.exists() {
        return Vec::new();
    }
    let mut files = Vec::new();
    walk_typed_dir(root, include_tests, &mut files);
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files
}

fn walk_typed_dir(dir: &Path, include_tests: bool, files: &mut Vec<(PathBuf, Language)>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        descend_or_collect(entry.path(), include_tests, files);
    }
}

fn descend_or_collect(path: PathBuf, include_tests: bool, files: &mut Vec<(PathBuf, Language)>) {
    if path.is_dir() {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.starts_with('.') || SKIP_DIRS.contains(&name) {
            return;
        }
        walk_typed_dir(&path, include_tests, files);
        return;
    }
    if !include_tests && test_detection::is_test_file(&path.to_string_lossy()) {
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
