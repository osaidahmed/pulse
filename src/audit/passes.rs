use std::path::{Path, PathBuf};

use crate::parse::Language;
use crate::thresholds::AuditThresholds;

use super::finding::AuditFinding;
use super::{
    complexity_floor, constraint_smells, corpus, corpus_stats, dead_variability, deps_reconcile, discovery,
    duplication_clusters, expression_filter, ifdef_density, mdl, named_smells, record_extraction, reflexion, scoring,
    strictness, taint, vendor_filter, vuln_clones, PassChoice,
};

pub(super) struct PassCtx<'a> {
    pub shared: &'a corpus::Corpus,
    pub typed_files: &'a [(PathBuf, Language)],
    pub root: &'a Path,
    pub thresholds: &'a AuditThresholds,
}

type PassRunner = fn(&PassCtx) -> Vec<AuditFinding>;

const RUNNERS: &[(PassChoice, PassRunner)] = &[
    (PassChoice::PatternMining, |ctx| run_pattern_mining(ctx.shared, ctx.thresholds)),
    (PassChoice::PackageMetrics, |ctx| {
        super::run_package_metrics(ctx.shared, ctx.typed_files, ctx.root, ctx.thresholds)
    }),
    (PassChoice::NamedSmells, |ctx| named_smells::run_from(ctx.shared, ctx.root, ctx.thresholds)),
    (PassChoice::Deps, |ctx| {
        let mut found = deps_reconcile::run_from(ctx.shared, ctx.root, ctx.thresholds);
        found.extend(constraint_smells::run_from(ctx.root, ctx.thresholds));
        found.extend(reflexion::run_from(ctx.shared, ctx.root, ctx.thresholds));
        found.extend(strictness::run_from(ctx.shared, ctx.root, ctx.thresholds));
        found
    }),
    (PassChoice::Taint, |ctx| taint::run_from(ctx.shared, ctx.thresholds)),
    (PassChoice::Clones, |ctx| duplication_clusters::run_from(ctx.shared, ctx.thresholds)),
    (PassChoice::Naturalness, |ctx| crate::naturalness::run_from(ctx.shared, ctx.thresholds)),
    (PassChoice::VulnClones, |ctx| vuln_clones::run_from(ctx.shared, ctx.thresholds)),
    (PassChoice::IfdefDensity, |ctx| ifdef_density::run_from(ctx.shared, ctx.thresholds)),
    (PassChoice::DeadVariability, |ctx| dead_variability::run_from(ctx.shared, ctx.root, ctx.thresholds)),
];

pub(super) fn run_selected_passes(findings: &mut Vec<AuditFinding>, passes: &[PassChoice], ctx: &PassCtx) {
    for (choice, runner) in RUNNERS {
        if passes.contains(choice) {
            findings.extend(runner(ctx));
        }
    }
}

fn run_pattern_mining(shared: &corpus::Corpus, thresholds: &AuditThresholds) -> Vec<AuditFinding> {
    let bundle = record_extraction::corpus_bundle_from(shared, thresholds);
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
