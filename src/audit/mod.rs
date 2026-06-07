#![allow(dead_code)]

pub mod abstractness;
pub mod arch_smells;
pub mod call_graph;
pub mod call_method_dotted;
pub mod call_path_qualified;
pub mod call_walker;
pub mod calls;
pub mod categorize;
pub mod centrality;
pub mod class_registry;
pub mod community;
pub mod complexity_floor;
pub mod components;
pub mod compound;
pub mod confidence;
pub mod corpus_stats;
pub mod cycle_shapes;
pub mod cycles;
pub mod definitions;
pub mod detector_divergent_change;
pub mod detector_feature_envy;
pub mod detector_god_class;
pub mod detector_parallel_inheritance;
pub mod detector_refused_bequest;
pub mod discovery;
pub mod duplication_clusters;
pub mod expression_filter;
pub mod finding;
pub mod finding_evidence;
pub mod graph;
pub mod import_call_form;
pub mod import_command_form;
pub mod import_jsts;
pub mod import_kinds;
pub mod import_php;
pub mod import_preprocessor;
pub mod import_python;
pub mod imports;
pub mod inheritance;
pub mod lang_kinds;
pub mod martin;
pub mod mdl;
pub mod named_smells;
pub mod output;
pub mod output_advisory;
pub mod output_arch;
pub mod output_clones;
pub mod output_grouped;
pub mod output_helpers;
pub mod output_named_smells;
pub mod output_naturalness;
pub mod output_package_metrics;
pub mod output_sections;
pub mod output_taint;
pub mod output_vuln_clones;
pub mod package_metrics;
pub mod record_extraction;
pub mod remodularization;
pub mod scoring;
pub mod swap_significance;
pub mod taint;
pub mod vendor_filter;
pub mod vuln_clones;
pub mod walker;

use std::path::{Path, PathBuf};

use crate::config::{AuditSuppression, IgnoreMatcher};
use crate::parse::{self, Language};
use crate::test_detection;
use crate::thresholds::AuditThresholds;
use finding::{action_for_kind, finding_confidence, AuditFinding};
use graph::InputEdge;
use package_metrics::ModuleProfile;
use walker::SubtreeRecord;

pub struct AuditOpts {
    pub root: PathBuf,
    pub pass: Option<PassChoice>,
    pub json: bool,
    pub include_tests: bool,
    pub show_noise: bool,
    pub suppression: AuditSuppression,
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum PassChoice {
    PatternMining,
    PackageMetrics,
    NamedSmells,
    Taint,
    Clones,
    Naturalness,
    VulnClones,
    All,
}

const SKIP_DIRS: &[&str] = &["node_modules", "target", "vendor", "build", "dist", "__pycache__"];

pub fn run(opts: &AuditOpts, thresholds: &AuditThresholds) -> Vec<AuditFinding> {
    let empty = IgnoreMatcher::from_patterns(&[]);
    let filter = IgnoreFilter::new(&empty, &opts.root);
    run_with_filter(opts, thresholds, &filter)
}

pub fn run_with_filter(opts: &AuditOpts, thresholds: &AuditThresholds, filter: &IgnoreFilter) -> Vec<AuditFinding> {
    let typed_files = walk_typed_source_files_filtered(&opts.root, opts.include_tests, filter);
    let passes = active_passes(opts.pass);
    let mut findings: Vec<AuditFinding> = Vec::new();
    if passes.contains(&PassChoice::PatternMining) {
        findings.extend(run_pattern_mining(&typed_files, thresholds));
    }
    if passes.contains(&PassChoice::PackageMetrics) {
        findings.extend(run_package_metrics(&typed_files, &opts.root, thresholds));
    }
    if passes.contains(&PassChoice::NamedSmells) {
        findings.extend(named_smells::run(&typed_files, &opts.root, thresholds));
    }
    if passes.contains(&PassChoice::Taint) {
        findings.extend(taint::run(&typed_files, thresholds));
    }
    if passes.contains(&PassChoice::Clones) {
        findings.extend(duplication_clusters::run(&typed_files, thresholds));
    }
    if passes.contains(&PassChoice::Naturalness) {
        findings.extend(crate::naturalness::run(&typed_files, thresholds));
    }
    if passes.contains(&PassChoice::VulnClones) {
        findings.extend(vuln_clones::run(&typed_files, thresholds));
    }
    populate_action_labels(&mut findings);
    findings.sort_by_key(|f| std::cmp::Reverse(finding_confidence(f)));
    findings
}

fn populate_action_labels(findings: &mut [AuditFinding]) {
    for f in findings {
        if f.action_label.is_some() {
            continue;
        }
        f.action_label = Some(action_for_kind(&f.kind, f.pattern_category));
    }
}

pub fn active_passes(pass: Option<PassChoice>) -> Vec<PassChoice> {
    match pass {
        Some(PassChoice::All) | None => {
            vec![PassChoice::PatternMining, PassChoice::PackageMetrics, PassChoice::NamedSmells]
        }
        Some(choice) => vec![choice],
    }
}

fn run_pattern_mining(typed_files: &[(PathBuf, Language)], thresholds: &AuditThresholds) -> Vec<AuditFinding> {
    let bundle = record_extraction::corpus_bundle(typed_files, thresholds);
    let stats = corpus_stats::aggregate_corpus(bundle.features);
    let flagged = vendor_filter::flagged_paths(&vendor_filter::classify(&stats, &thresholds.pattern_mining.vendor));
    let filtered: Vec<_> = bundle.subtrees.into_iter().filter(|r| !flagged.contains(&r.file)).collect();
    let expression_fps: std::collections::HashSet<u64> = bundle
        .kinds_by_fp
        .keys()
        .copied()
        .filter(|fp| expression_filter::is_expression_level(*fp, &bundle.kinds_by_fp))
        .collect();
    let clusters = discovery::closed_mine(&filtered, &expression_fps, thresholds);
    let shapes = complexity_floor::shape_index(&filtered);
    let trimmed = complexity_floor::filter_clusters(clusters, &shapes, thresholds.pattern_mining.complexity);
    let expression_only = expression_filter::keep_expression_clusters(trimmed, &bundle.kinds_by_fp);
    let vocab = bundle.kinds_by_fp.values().flatten().collect::<std::collections::HashSet<_>>().len();
    let size_by_fp: std::collections::HashMap<u64, u32> =
        filtered.iter().map(|r| (r.fingerprint, r.named_node_count)).collect();
    let ctx = scoring::ScoringCtx {
        kinds_by_fp: &bundle.kinds_by_fp,
        size_by_fp: &size_by_fp,
        corpus: mdl::CorpusScale { vocab, total_occurrences: filtered.len() as u64 },
        floor: thresholds.pattern_mining.mdl.compression_floor_bits,
        max_findings: thresholds.pattern_mining.max_findings_reported,
    };
    scoring::build_findings(expression_only, &ctx)
}

const MIN_SUPPORTED_MODULES_FOR_PACKAGE_METRICS: usize = 5;

fn run_package_metrics(
    typed_files: &[(PathBuf, Language)],
    root: &Path,
    thresholds: &AuditThresholds,
) -> Vec<AuditFinding> {
    let supported_count = typed_files.iter().filter(|(_, l)| imports::has_extractor(*l)).count();
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
    let typed_set: std::collections::HashSet<PathBuf> = typed_files.iter().map(|(p, _)| p.clone()).collect();
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
    let Some(tree) = parse::parse_guarded(&source, lang) else { return Vec::new() };
    let raws = imports::extract_imports(&tree, &source, lang);
    let mut out: Vec<InputEdge> = Vec::new();
    for raw in raws {
        let Some(resolved) = resolve_via_strategies(&raw.target, path, root, lang, typed_set) else {
            continue;
        };
        let target_lang = typed_files.iter().find(|(p, _)| p == &resolved).map_or(lang, |(_, l)| *l);
        out.push(InputEdge { source: path.to_path_buf(), target: resolved, source_lang: lang, target_lang });
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

fn profile_for_path(path: &Path, lang_by_path: &std::collections::HashMap<PathBuf, Language>) -> ModuleProfile {
    let lang = lang_by_path.get(path).copied();
    let import_confidence = lang.map_or(finding::ImportConfidence::BestEffort, imports::confidence_for);
    let abstractness = lang.map_or_else(
        || martin::AbstractnessRecord { abstractness: 0.0, confidence: finding::ImportConfidence::NaAbstraction },
        |l| abstractness::abstractness_for_file(path, l),
    );
    let loc = std::fs::read_to_string(path).map_or(0, |s| s.lines().count() as u32);
    ModuleProfile { abstractness, import_confidence, loc }
}

pub fn extract_subtrees_for_dir(root: &Path, lang: Language, thresholds: &AuditThresholds) -> Vec<SubtreeRecord> {
    record_extraction::for_dir(root, lang, thresholds)
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
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files
}

pub const MAX_FILES: usize = 100_000;

#[doc(hidden)]
pub fn audit_traversal_cap_hit(file_count: usize) -> bool {
    file_count >= MAX_FILES
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
