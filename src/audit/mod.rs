pub mod abstractness;
pub mod arch_smells;
pub mod binding;
pub mod binding_cpp;
pub mod binding_csharp;
pub mod binding_d;
pub mod binding_extract;
pub mod binding_go;
pub mod binding_java;
pub mod binding_kotlin;
pub mod binding_objc;
pub mod binding_rust;
pub mod binding_swift;
pub mod binding_typescript;
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
pub mod component_thresholds;
pub mod components;
pub mod compound;
pub mod conceptual_cohesion;
pub mod confidence;
pub mod constraint_smells;
pub mod corpus;
pub mod corpus_stats;
pub mod coverage;
pub mod cycle_shapes;
pub mod cycles;
pub mod definitions;
pub mod deps_reconcile;
pub mod detector_conceptual_cohesion;
pub mod detector_divergent_change;
pub mod detector_feature_envy;
pub mod detector_god_class;
pub mod detector_parallel_inheritance;
pub mod detector_refused_bequest;
pub mod discovery;
pub mod duplication_clusters;
pub mod expression_filter;
pub mod finding;
pub mod finding_confidence;
pub mod finding_evidence;
pub mod fragmentation;
pub mod freshness;
pub mod graph;
pub mod hist_crossval;
pub mod ifdef_density;
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
pub mod method_vocab;
pub mod named_smells;
pub mod output;
pub mod output_advisory;
pub mod output_arch;
pub mod output_clones;
pub mod output_deps;
pub mod output_grouped;
pub mod output_helpers;
pub mod output_named_smells;
pub mod output_naturalness;
pub mod output_package_metrics;
pub mod output_sections;
pub mod output_taint;
pub mod output_vuln_clones;
pub mod package_metrics;
mod passes;
pub mod progress;
pub mod record_extraction;
pub mod reflexion;
pub mod remodularization;
pub mod scoring;
pub mod strictness;
pub mod swap_significance;
pub mod taint;
pub mod vendor_filter;
pub mod vuln_clones;
pub mod vuln_deps;
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
    Deps,
    Taint,
    Clones,
    Naturalness,
    VulnClones,
    IfdefDensity,
    All,
}

const SKIP_DIRS: &[&str] = &["node_modules", "target", "vendor", "build", "dist", "__pycache__"];

pub fn run(opts: &AuditOpts, thresholds: &AuditThresholds) -> Vec<AuditFinding> {
    let empty = IgnoreMatcher::from_patterns(&[]);
    let filter = IgnoreFilter::new(&empty, &opts.root);
    run_with_filter(opts, thresholds, &filter)
}

pub fn run_with_filter(opts: &AuditOpts, thresholds: &AuditThresholds, filter: &IgnoreFilter) -> Vec<AuditFinding> {
    run_with_filter_online(opts, thresholds, filter, false, &std::env::temp_dir())
}

pub fn run_with_filter_online(
    opts: &AuditOpts,
    thresholds: &AuditThresholds,
    filter: &IgnoreFilter,
    online: bool,
    cache_dir: &Path,
) -> Vec<AuditFinding> {
    let typed_files = walk_typed_source_files_filtered(&opts.root, opts.include_tests, filter);
    let shared = corpus::Corpus::load(&typed_files);
    let active = active_passes(opts.pass);
    let ctx =
        passes::PassCtx { shared: &shared, typed_files: &typed_files, root: &opts.root, thresholds, online, cache_dir };
    let mut findings: Vec<AuditFinding> = Vec::new();
    passes::run_selected_passes(&mut findings, &active, &ctx);
    maybe_cross_validate(&mut findings, opts, thresholds, filter, &active);
    populate_action_labels(&mut findings);
    findings.sort_by_key(|f| std::cmp::Reverse(finding_confidence(f)));
    findings
}

fn maybe_cross_validate(
    findings: &mut [AuditFinding],
    opts: &AuditOpts,
    thresholds: &AuditThresholds,
    filter: &IgnoreFilter,
    passes: &[PassChoice],
) {
    if !thresholds.cross_validate_history || !passes.contains(&PassChoice::NamedSmells) {
        return;
    }
    let hist_opts = crate::history::HistoryOpts {
        root: opts.root.clone(),
        include_tests: opts.include_tests,
        since: None,
        max_commits: None,
    };
    let flagged = crate::history::changeshotgun_files(
        &hist_opts,
        &crate::history::thresholds::HistoryThresholds::DEFAULTS,
        filter,
    );
    hist_crossval::apply_crossval(findings, flagged.as_ref());
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
            vec![PassChoice::PatternMining, PassChoice::PackageMetrics, PassChoice::NamedSmells, PassChoice::Deps]
        }
        Some(choice) => vec![choice],
    }
}

const MIN_SUPPORTED_MODULES_FOR_PACKAGE_METRICS: usize = 5;

fn run_package_metrics(
    shared: &corpus::Corpus,
    typed_files: &[(PathBuf, Language)],
    root: &Path,
    thresholds: &AuditThresholds,
) -> Vec<AuditFinding> {
    let supported_count = typed_files.iter().filter(|(_, l)| imports::has_extractor(*l)).count();
    if supported_count < MIN_SUPPORTED_MODULES_FOR_PACKAGE_METRICS {
        return Vec::new();
    }
    let edges = collect_import_edges(shared, typed_files, root);
    package_metrics::run_with_module_count(
        &edges,
        |path| profile_for_path(path, shared),
        thresholds,
        supported_count as u32,
    )
}

fn collect_import_edges(shared: &corpus::Corpus, typed_files: &[(PathBuf, Language)], root: &Path) -> Vec<InputEdge> {
    let typed_set: std::collections::HashSet<PathBuf> = typed_files.iter().map(|(p, _)| p.clone()).collect();
    let mut edges: Vec<InputEdge> = Vec::new();
    for file in &shared.files {
        edges.extend(edges_for_file(file, shared, root, &typed_set));
    }
    edges
}

fn edges_for_file(
    file: &corpus::CorpusFile,
    shared: &corpus::Corpus,
    root: &Path,
    typed_set: &std::collections::HashSet<PathBuf>,
) -> Vec<InputEdge> {
    let lang = file.lang;
    let Some((source, tree)) = file.parsed() else { return Vec::new() };
    let raws = imports::extract_imports(tree, source, lang);
    let mut out: Vec<InputEdge> = Vec::new();
    for raw in raws {
        let Some(resolved) = resolve_via_strategies(&raw.target, &file.path, root, lang, typed_set) else {
            continue;
        };
        let target_lang = shared.get(&resolved).map_or(lang, |f| f.lang);
        out.push(InputEdge { source: file.path.clone(), target: resolved, source_lang: lang, target_lang });
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

fn profile_for_path(path: &Path, shared: &corpus::Corpus) -> ModuleProfile {
    let file = shared.get(path);
    let lang = file.map(|f| f.lang);
    let import_confidence = lang.map_or(finding::ImportConfidence::BestEffort, imports::confidence_for);
    let abstractness = file.map_or_else(
        || martin::AbstractnessRecord { abstractness: None, confidence: finding::ImportConfidence::NaAbstraction },
        |f| abstractness::abstractness_from_parsed(f.tree.as_ref(), f.lang),
    );
    let loc = file.and_then(|f| f.source.as_deref()).map_or(0, |s| s.lines().count() as u32);
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
    if !include_tests {
        let test_roots = crate::buildmeta::declared_test_roots(root);
        if !test_roots.is_empty() {
            files.retain(|(p, _)| !crate::buildmeta::is_under_test_root(p, &test_roots));
        }
    }
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
