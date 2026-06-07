use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use super::finding::{BlobEvidence, HistoryFinding, HistoryKind};
use super::git::Commit;
use super::thresholds::HistoryThresholds;

pub fn rank(
    commits: &[Commit],
    typed_paths: &HashSet<PathBuf>,
    t: &HistoryThresholds,
) -> Vec<HistoryFinding> {
    if !t.hist.enabled {
        return Vec::new();
    }
    let (per_file, total) = multi_file_participation(commits, t);
    if total == 0 {
        return Vec::new();
    }
    let mut findings: Vec<HistoryFinding> = per_file
        .into_iter()
        .filter(|(file, _)| typed_paths.contains(*file))
        .filter_map(|(file, count)| blob_finding(file, count, total, t))
        .collect();
    sort_blob_findings(&mut findings);
    findings.truncate(t.hist.max_findings_reported as usize);
    findings
}

fn multi_file_participation<'a>(
    commits: &'a [Commit],
    t: &HistoryThresholds,
) -> (HashMap<&'a Path, u32>, u32) {
    let mut per_file: HashMap<&Path, u32> = HashMap::new();
    let mut total = 0u32;
    for commit in commits {
        let n = commit.files.len();
        if n < 2 || n > t.max_commit_files as usize {
            continue;
        }
        total += 1;
        for file in &commit.files {
            *per_file.entry(file.as_path()).or_insert(0) += 1;
        }
    }
    (per_file, total)
}

fn blob_finding(file: &Path, count: u32, total: u32, t: &HistoryThresholds) -> Option<HistoryFinding> {
    let ratio = f64::from(count) / f64::from(total);
    if ratio <= t.hist.blob_commit_pct {
        return None;
    }
    Some(HistoryFinding {
        kind: HistoryKind::FileBlob(BlobEvidence {
            file: file.to_path_buf(),
            multi_file_commits: count,
            total_multi_file_commits: total,
            blob_ratio: ratio,
        }),
        action_label: None,
    })
}

fn sort_blob_findings(findings: &mut [HistoryFinding]) {
    findings.sort_by(|a, b| {
        let HistoryKind::FileBlob(ea) = &a.kind else { return std::cmp::Ordering::Equal };
        let HistoryKind::FileBlob(eb) = &b.kind else { return std::cmp::Ordering::Equal };
        eb.blob_ratio.total_cmp(&ea.blob_ratio).then_with(|| ea.file.cmp(&eb.file))
    });
}
