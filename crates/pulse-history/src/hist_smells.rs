use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

use pulse_audit::components::component_of;

use super::co_change::{CoChangeAgg, RevisionScope};
use super::finding::{BlobEvidence, ChangeShotgunEvidence, HistoryFinding, HistoryKind};
use super::git::Commit;
use super::thresholds::HistoryThresholds;

pub fn rank(
    commits: &[Commit],
    pairs: &HashMap<(PathBuf, PathBuf), CoChangeAgg>,
    scope: &RevisionScope,
    typed_paths: &HashSet<PathBuf>,
    t: &HistoryThresholds,
) -> Vec<HistoryFinding> {
    if !t.hist.enabled {
        return Vec::new();
    }
    let mut findings = blob_findings(commits, typed_paths, t);
    findings.extend(shotgun_findings(pairs, scope, typed_paths, t));
    findings
}

fn blob_findings(commits: &[Commit], typed_paths: &HashSet<PathBuf>, t: &HistoryThresholds) -> Vec<HistoryFinding> {
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

fn multi_file_participation<'a>(commits: &'a [Commit], t: &HistoryThresholds) -> (HashMap<&'a Path, u32>, u32) {
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

#[derive(Default)]
struct ShotgunAcc {
    partner_count: u32,
    packages: BTreeSet<PathBuf>,
}

fn shotgun_findings(
    pairs: &HashMap<(PathBuf, PathBuf), CoChangeAgg>,
    scope: &RevisionScope,
    typed_paths: &HashSet<PathBuf>,
    t: &HistoryThresholds,
) -> Vec<HistoryFinding> {
    let acc = accumulate_partners(pairs, scope, t);
    let mut findings: Vec<HistoryFinding> = acc
        .into_iter()
        .filter(|(file, _)| typed_paths.contains(file))
        .filter_map(|(file, partners)| shotgun_finding(file, partners, t))
        .collect();
    sort_shotgun_findings(&mut findings);
    findings.truncate(t.hist.max_findings_reported as usize);
    findings
}

fn accumulate_partners(
    pairs: &HashMap<(PathBuf, PathBuf), CoChangeAgg>,
    scope: &RevisionScope,
    t: &HistoryThresholds,
) -> BTreeMap<PathBuf, ShotgunAcc> {
    let mut acc: BTreeMap<PathBuf, ShotgunAcc> = BTreeMap::new();
    for ((a, b), agg) in pairs {
        if agg.support < t.co_change.min_support {
            continue;
        }
        if confidence(agg.support, revisions_of(scope, a)) >= t.hist.shotgun_confidence {
            let entry = acc.entry(a.clone()).or_default();
            entry.partner_count += 1;
            entry.packages.insert(component_of(b));
        }
        if confidence(agg.support, revisions_of(scope, b)) >= t.hist.shotgun_confidence {
            let entry = acc.entry(b.clone()).or_default();
            entry.partner_count += 1;
            entry.packages.insert(component_of(a));
        }
    }
    acc
}

fn revisions_of(scope: &RevisionScope, file: &Path) -> u32 {
    scope.per_file.get(file).copied().unwrap_or(0)
}

fn confidence(support: u32, revisions: u32) -> f64 {
    f64::from(support) / f64::from(revisions.max(1))
}

fn shotgun_finding(file: PathBuf, acc: ShotgunAcc, t: &HistoryThresholds) -> Option<HistoryFinding> {
    if acc.packages.len() as u32 <= t.hist.shotgun_distinct_packages {
        return None;
    }
    Some(HistoryFinding {
        kind: HistoryKind::ChangeShotgun(ChangeShotgunEvidence {
            file,
            partner_count: acc.partner_count,
            package_count: acc.packages.len() as u32,
            packages: acc.packages.into_iter().collect(),
        }),
        action_label: None,
    })
}

fn sort_shotgun_findings(findings: &mut [HistoryFinding]) {
    findings.sort_by(|a, b| {
        let HistoryKind::ChangeShotgun(ea) = &a.kind else { return std::cmp::Ordering::Equal };
        let HistoryKind::ChangeShotgun(eb) = &b.kind else { return std::cmp::Ordering::Equal };
        eb.package_count
            .cmp(&ea.package_count)
            .then_with(|| eb.partner_count.cmp(&ea.partner_count))
            .then_with(|| ea.file.cmp(&eb.file))
    });
}
