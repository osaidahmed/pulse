use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use super::finding::{FragmentationEvidence, HistoryFinding, HistoryKind};
use super::git::Commit;
use super::thresholds::HistoryThresholds;

pub fn rank(commits: &[Commit], typed_paths: &HashSet<PathBuf>, t: &HistoryThresholds) -> Vec<HistoryFinding> {
    let per_file = author_counts_per_file(commits, typed_paths);
    let mut findings: Vec<HistoryFinding> =
        per_file.into_iter().filter_map(|(file, counts)| build_fragmentation(file, counts, t)).collect();
    sort_fragmentation_findings(&mut findings);
    findings.truncate(t.contributors.max_findings_reported as usize);
    findings
}

pub fn author_counts_per_file(
    commits: &[Commit],
    typed_paths: &HashSet<PathBuf>,
) -> HashMap<PathBuf, HashMap<String, u32>> {
    let mut map: HashMap<PathBuf, HashMap<String, u32>> = HashMap::new();
    for c in commits {
        for f in &c.files {
            if !typed_paths.contains(f) {
                continue;
            }
            *map.entry(f.clone()).or_default().entry(c.author.clone()).or_insert(0) += 1;
        }
    }
    map
}

fn build_fragmentation(file: PathBuf, counts: HashMap<String, u32>, t: &HistoryThresholds) -> Option<HistoryFinding> {
    let total: u32 = counts.values().sum();
    if total < t.contributors.min_total_commits {
        return None;
    }
    let total_authors = counts.len();
    let mut author_counts: Vec<(String, u32)> = counts.into_iter().collect();
    author_counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let pct_threshold = t.contributors.minor_contributor_pct / 100.0;
    let total_f = f64::from(total);
    let minors: Vec<&(String, u32)> =
        author_counts.iter().filter(|(_, count)| f64::from(*count) / total_f < pct_threshold).collect();
    let minor_count = u32::try_from(minors.len()).unwrap_or(u32::MAX);
    if minor_count < t.contributors.min_minor_authors {
        return None;
    }
    let top_minor_authors: Vec<String> = minors.iter().take(5).map(|(a, _)| a.clone()).collect();
    let total_contributors = u32::try_from(total_authors).unwrap_or(u32::MAX);
    let observed_pct =
        if total_contributors == 0 { 0.0 } else { f64::from(minor_count) / f64::from(total_contributors) };
    Some(HistoryFinding {
        kind: HistoryKind::KnowledgeFragmentation(FragmentationEvidence {
            file,
            total_contributors,
            minor_contributor_count: minor_count,
            minor_contributor_pct: observed_pct,
            top_minor_authors,
        }),
        action_label: None,
    })
}

fn sort_fragmentation_findings(findings: &mut [HistoryFinding]) {
    findings.sort_by(|a, b| {
        let HistoryKind::KnowledgeFragmentation(ea) = &a.kind else {
            return std::cmp::Ordering::Equal;
        };
        let HistoryKind::KnowledgeFragmentation(eb) = &b.kind else {
            return std::cmp::Ordering::Equal;
        };
        eb.minor_contributor_count.cmp(&ea.minor_contributor_count).then_with(|| ea.file.cmp(&eb.file))
    });
}
