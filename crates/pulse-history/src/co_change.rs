use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use pulse_audit::graph::ImportGraph;

use super::edges::directly_linked;
use super::finding::{DriftEvidence, HistoryFinding, HistoryKind};
use super::git::Commit;
use super::thresholds::HistoryThresholds;

pub struct CoChangeAgg {
    pub support: u32,
    pub last_seen: i64,
    pub authors: HashSet<String>,
}

pub struct RevisionScope {
    pub per_file: HashMap<PathBuf, u32>,
    pub commits: u32,
}

fn in_scope(commit: &Commit, t: &HistoryThresholds) -> bool {
    commit.files.len() <= t.max_commit_files as usize
}

pub fn mine(commits: &[Commit], t: &HistoryThresholds) -> HashMap<(PathBuf, PathBuf), CoChangeAgg> {
    let mut pairs: HashMap<(PathBuf, PathBuf), CoChangeAgg> = HashMap::new();
    for commit in commits {
        if !in_scope(commit, t) {
            continue;
        }
        record_commit_pairs(commit, &mut pairs);
    }
    pairs
}

pub fn revisions_in_scope(commits: &[Commit], t: &HistoryThresholds) -> RevisionScope {
    let mut per_file: HashMap<PathBuf, u32> = HashMap::new();
    let mut commit_count: u32 = 0;
    for commit in commits {
        if !in_scope(commit, t) {
            continue;
        }
        commit_count += 1;
        for f in &commit.files {
            *per_file.entry(f.clone()).or_insert(0) += 1;
        }
    }
    RevisionScope { per_file, commits: commit_count }
}

fn record_commit_pairs(commit: &Commit, pairs: &mut HashMap<(PathBuf, PathBuf), CoChangeAgg>) {
    for i in 0..commit.files.len() {
        for j in (i + 1)..commit.files.len() {
            let (lo, hi) = canonical_pair(&commit.files[i], &commit.files[j]);
            if lo == hi {
                continue;
            }
            let agg = pairs.entry((lo, hi)).or_insert_with(|| CoChangeAgg {
                support: 0,
                last_seen: 0,
                authors: HashSet::new(),
            });
            agg.support += 1;
            agg.last_seen = agg.last_seen.max(commit.timestamp);
            agg.authors.insert(commit.author.clone());
        }
    }
}

fn canonical_pair(a: &Path, b: &Path) -> (PathBuf, PathBuf) {
    if a < b {
        (a.to_path_buf(), b.to_path_buf())
    } else {
        (b.to_path_buf(), a.to_path_buf())
    }
}

struct DriftCtx<'a> {
    scope: &'a RevisionScope,
    graph: &'a ImportGraph,
    typed_paths: &'a HashSet<PathBuf>,
    t: &'a HistoryThresholds,
}

struct DriftCounts {
    support: u32,
    rev_a: u32,
    rev_b: u32,
    commits: u32,
}

pub fn rank_drift(
    pairs: HashMap<(PathBuf, PathBuf), CoChangeAgg>,
    scope: &RevisionScope,
    graph: &ImportGraph,
    typed_paths: &HashSet<PathBuf>,
    t: &HistoryThresholds,
) -> Vec<HistoryFinding> {
    let ctx = DriftCtx { scope, graph, typed_paths, t };
    let mut findings: Vec<HistoryFinding> =
        pairs.into_iter().filter_map(|((a, b), agg)| build_drift(a, b, &agg, &ctx)).collect();
    sort_drift_findings(&mut findings);
    findings.truncate(t.co_change.max_findings_reported as usize);
    findings
}

fn build_drift(a: PathBuf, b: PathBuf, agg: &CoChangeAgg, ctx: &DriftCtx) -> Option<HistoryFinding> {
    if agg.support < ctx.t.co_change.min_support
        || !ctx.typed_paths.contains(&a)
        || !ctx.typed_paths.contains(&b)
        || directly_linked(ctx.graph, &a, &b)
    {
        return None;
    }
    let rev_a = ctx.scope.per_file.get(&a).copied().unwrap_or(agg.support).max(agg.support);
    let rev_b = ctx.scope.per_file.get(&b).copied().unwrap_or(agg.support).max(agg.support);
    let counts = DriftCounts { support: agg.support, rev_a, rev_b, commits: ctx.scope.commits };
    let (confidence, lift, jaccard) = drift_metrics(&counts);
    if confidence < ctx.t.co_change.min_confidence || lift <= ctx.t.co_change.min_lift {
        return None;
    }
    let evidence = DriftEvidence {
        file_a: a,
        file_b: b,
        support: agg.support,
        commits: ctx.scope.commits,
        confidence,
        lift,
        jaccard,
        last_seen_unix: agg.last_seen,
        distinct_authors: u32::try_from(agg.authors.len()).unwrap_or(u32::MAX),
    };
    Some(HistoryFinding { kind: HistoryKind::ArchitecturalDrift(evidence), action_label: None })
}

fn drift_metrics(c: &DriftCounts) -> (f64, f64, f64) {
    let s = f64::from(c.support);
    let ra = f64::from(c.rev_a);
    let rb = f64::from(c.rev_b);
    let n = f64::from(c.commits);
    let confidence = (s / ra).min(s / rb);
    let lift = (s * n) / (ra * rb);
    let union = ra + rb - s;
    let jaccard = if union > 0.0 { s / union } else { 0.0 };
    (confidence, lift, jaccard)
}

fn sort_drift_findings(findings: &mut [HistoryFinding]) {
    findings.sort_by(|a, b| {
        let HistoryKind::ArchitecturalDrift(ea) = &a.kind else { return std::cmp::Ordering::Equal };
        let HistoryKind::ArchitecturalDrift(eb) = &b.kind else { return std::cmp::Ordering::Equal };
        eb.confidence
            .total_cmp(&ea.confidence)
            .then_with(|| eb.lift.total_cmp(&ea.lift))
            .then_with(|| (&ea.file_a, &ea.file_b).cmp(&(&eb.file_a, &eb.file_b)))
    });
}
