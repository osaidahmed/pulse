use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::audit::graph::ImportGraph;

use super::edges::directly_linked;
use super::finding::{DriftEvidence, HistoryFinding, HistoryKind};
use super::git::Commit;
use super::thresholds::HistoryThresholds;

pub struct CoChangeAgg {
    pub support: u32,
    pub last_seen: i64,
    pub authors: HashSet<String>,
}

pub fn mine(commits: &[Commit], t: &HistoryThresholds) -> HashMap<(PathBuf, PathBuf), CoChangeAgg> {
    let mut pairs: HashMap<(PathBuf, PathBuf), CoChangeAgg> = HashMap::new();
    for commit in commits {
        if commit.files.len() > t.max_commit_files as usize {
            continue;
        }
        record_commit_pairs(commit, &mut pairs);
    }
    pairs
}

fn record_commit_pairs(
    commit: &Commit,
    pairs: &mut HashMap<(PathBuf, PathBuf), CoChangeAgg>,
) {
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

pub fn rank_drift(
    pairs: HashMap<(PathBuf, PathBuf), CoChangeAgg>,
    graph: &ImportGraph,
    typed_paths: &HashSet<PathBuf>,
    total_commits: u32,
    t: &HistoryThresholds,
) -> Vec<HistoryFinding> {
    let mut findings: Vec<HistoryFinding> = pairs
        .into_iter()
        .filter(|((a, b), agg)| {
            agg.support >= t.co_change.min_support
                && typed_paths.contains(a)
                && typed_paths.contains(b)
                && !directly_linked(graph, a, b)
        })
        .map(|((a, b), agg)| {
            let evidence = DriftEvidence {
                file_a: a,
                file_b: b,
                support: agg.support,
                commits: total_commits,
                last_seen_unix: agg.last_seen,
                distinct_authors: u32::try_from(agg.authors.len()).unwrap_or(u32::MAX),
            };
            HistoryFinding {
                kind: HistoryKind::ArchitecturalDrift(evidence),
                action_label: None,
            }
        })
        .collect();
    sort_drift_findings(&mut findings);
    findings.truncate(t.co_change.max_findings_reported as usize);
    findings
}

fn sort_drift_findings(findings: &mut [HistoryFinding]) {
    findings.sort_by(|a, b| {
        let HistoryKind::ArchitecturalDrift(ea) = &a.kind else { return std::cmp::Ordering::Equal };
        let HistoryKind::ArchitecturalDrift(eb) = &b.kind else { return std::cmp::Ordering::Equal };
        eb.support
            .cmp(&ea.support)
            .then_with(|| (&ea.file_a, &ea.file_b).cmp(&(&eb.file_a, &eb.file_b)))
    });
}
