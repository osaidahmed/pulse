#![allow(clippy::assigning_clones)]

pub mod abstractness;
pub mod arch_smells;
pub mod binding;
pub mod call_graph;
pub mod call_method_dotted;
pub mod call_path_qualified;
pub mod call_walker;
pub mod calls;
pub mod categorize;
pub mod centrality;
pub mod class_evidence;
pub mod class_registry;
pub mod community;
pub mod complexity_floor;
pub mod component_thresholds;
pub mod components;
pub mod compound;
pub mod conceptual_cohesion;
pub mod confidence;
pub mod constraint_smells;
pub mod corpus_df;
pub mod coverage;
pub mod cycle_shapes;
pub mod cycles;
pub mod definitions;
pub mod deps_reconcile;
pub mod detector;
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
pub mod import;
pub mod imports;
pub mod inheritance;
pub mod martin;
pub mod mcd;
pub mod mdl;
pub mod method_vocab;
pub mod named_smells;
pub mod naturalness;
pub mod output;
pub mod package_metrics;
mod passes;
pub mod progress;
pub mod reflexion;
pub mod remodularization;
pub mod scoring;
pub mod strictness;
pub mod suppression;
pub mod swap_significance;
pub mod taint;
pub mod vuln_clones;
pub mod vuln_deps;

use std::path::{Path, PathBuf};

use finding::{action_for_kind, finding_confidence, AuditFinding};
use graph::InputEdge;
use package_metrics::ModuleProfile;
use pulse_config::IgnoreMatcher;
use pulse_corpus::walker::SubtreeRecord;
use pulse_syntax::parse::Language;
use pulse_thresholds::AuditThresholds;
use suppression::AuditSuppression;

pub struct AuditOpts {
    pub root: PathBuf,
    pub pass: Option<PassChoice>,
    pub json: bool,
    pub include_tests: bool,
    pub show_noise: bool,
    pub suppression: AuditSuppression,
}

pub use pulse_corpus::{
    audit_traversal_cap_hit, corpus, corpus_stats, lang_kinds, record_extraction, vendor_filter,
    walk_typed_source_files, walk_typed_source_files_filtered, walker, IgnoreFilter, MAX_FILES,
};

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

pub fn run(opts: &AuditOpts, thresholds: &AuditThresholds) -> Vec<AuditFinding> {
    let empty = IgnoreMatcher::from_patterns(&[]);
    let filter = IgnoreFilter::new(&empty, &opts.root);
    run_with_filter(opts, thresholds, &filter)
}

pub type CrossValidator<'a> = &'a dyn Fn(&Path, bool, &IgnoreFilter) -> Option<std::collections::HashSet<PathBuf>>;

pub struct RunCtx<'a> {
    pub online: bool,
    pub cache_dir: &'a Path,
    pub cross_validator: Option<CrossValidator<'a>>,
}

pub fn run_with_filter(opts: &AuditOpts, thresholds: &AuditThresholds, filter: &IgnoreFilter) -> Vec<AuditFinding> {
    let tmp = std::env::temp_dir();
    let run = RunCtx { online: false, cache_dir: &tmp, cross_validator: None };
    run_with_filter_online(opts, thresholds, filter, &run)
}

pub fn run_with_filter_online(
    opts: &AuditOpts,
    thresholds: &AuditThresholds,
    filter: &IgnoreFilter,
    run: &RunCtx,
) -> Vec<AuditFinding> {
    let typed_files = walk_typed_source_files_filtered(&opts.root, opts.include_tests, filter);
    let shared = corpus::Corpus::load(&typed_files);
    let active = active_passes(opts.pass);
    let ctx = passes::PassCtx {
        shared: &shared,
        typed_files: &typed_files,
        root: &opts.root,
        thresholds,
        online: run.online,
        cache_dir: run.cache_dir,
    };
    let mut findings: Vec<AuditFinding> = Vec::new();
    passes::run_selected_passes(&mut findings, &active, &ctx);
    if thresholds.cross_validate_history && active.contains(&PassChoice::NamedSmells) {
        if let Some(cross_validate) = run.cross_validator {
            let flagged = cross_validate(&opts.root, opts.include_tests, filter);
            hist_crossval::apply_crossval(&mut findings, flagged.as_ref());
        }
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
