use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::parse::{self, Language};

use super::finding::{HistoryFinding, HistoryKind, HotspotEvidence};
use super::git::Commit;
use super::thresholds::HistoryThresholds;

pub fn rank(commits: &[Commit], typed_files: &[(PathBuf, Language)], t: &HistoryThresholds) -> Vec<HistoryFinding> {
    let revisions = revisions_per_file(commits);
    let lang_lookup: HashMap<PathBuf, Language> = typed_files.iter().map(|(p, l)| (p.clone(), *l)).collect();
    let mut findings: Vec<HistoryFinding> = revisions
        .into_iter()
        .filter(|(p, count)| *count >= t.hotspot.min_revisions && lang_lookup.contains_key(p))
        .filter_map(|(path, revs)| build_hotspot(&path, revs, &lang_lookup, t))
        .collect();
    sort_hotspot_findings(&mut findings);
    findings.truncate(t.hotspot.max_findings_reported as usize);
    findings
}

fn build_hotspot(
    path: &Path,
    revs: u32,
    lang_lookup: &HashMap<PathBuf, Language>,
    t: &HistoryThresholds,
) -> Option<HistoryFinding> {
    let lang = *lang_lookup.get(path)?;
    let cc = file_cc(path, lang);
    let score = u64::from(revs) * u64::from(cc);
    if score < t.hotspot.min_score {
        return None;
    }
    Some(HistoryFinding {
        kind: HistoryKind::Hotspot(HotspotEvidence { file: path.to_path_buf(), revisions: revs, sum_cc: cc, score }),
        action_label: None,
    })
}

fn sort_hotspot_findings(findings: &mut [HistoryFinding]) {
    findings.sort_by(|a, b| {
        let HistoryKind::Hotspot(ea) = &a.kind else { return std::cmp::Ordering::Equal };
        let HistoryKind::Hotspot(eb) = &b.kind else { return std::cmp::Ordering::Equal };
        eb.score.cmp(&ea.score).then_with(|| ea.file.cmp(&eb.file))
    });
}

pub fn revisions_per_file(commits: &[Commit]) -> HashMap<PathBuf, u32> {
    let mut counts: HashMap<PathBuf, u32> = HashMap::new();
    for c in commits {
        for f in &c.files {
            *counts.entry(f.clone()).or_insert(0) += 1;
        }
    }
    counts
}

fn file_cc(path: &Path, lang: Language) -> u32 {
    let Ok(source) = std::fs::read_to_string(path) else { return 0 };
    let Some(metrics) = parse::parse_and_walk_guarded(&source, lang) else { return 0 };
    metrics.module.sum_cc
}
