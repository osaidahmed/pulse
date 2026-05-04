use std::collections::HashMap;
use std::path::PathBuf;

use crate::thresholds::AuditThresholds;

use super::walker::SubtreeRecord;

#[derive(Debug, Clone)]
pub struct RawCluster {
    pub fingerprint: u64,
    pub support: u32,
    pub file_count: u32,
    pub representative_snippet: String,
    pub locations: Vec<(PathBuf, u32)>,
}

pub fn freqt_mine(records: &[SubtreeRecord], thresholds: &AuditThresholds) -> Vec<RawCluster> {
    let groups = group_by_fingerprint(records);
    groups
        .into_iter()
        .filter_map(|(fp, idx)| materialize(fp, &idx, records, thresholds))
        .collect()
}

fn group_by_fingerprint(records: &[SubtreeRecord]) -> HashMap<u64, Vec<usize>> {
    let mut groups: HashMap<u64, Vec<usize>> = HashMap::new();
    for (i, r) in records.iter().enumerate() {
        groups.entry(r.fingerprint).or_default().push(i);
    }
    groups
}

fn materialize(
    fp: u64,
    indices: &[usize],
    records: &[SubtreeRecord],
    thresholds: &AuditThresholds,
) -> Option<RawCluster> {
    let support = indices.len() as u32;
    if (support as usize) < thresholds.pattern_mining.freqt_min_support {
        return None;
    }
    let mut files: std::collections::BTreeSet<PathBuf> = std::collections::BTreeSet::new();
    for &i in indices {
        files.insert(records[i].file.clone());
    }
    let representative = pick_representative(indices, records);
    let locations = collect_locations(indices, records);
    Some(RawCluster {
        fingerprint: fp,
        support,
        file_count: files.len() as u32,
        representative_snippet: representative,
        locations,
    })
}

fn pick_representative(indices: &[usize], records: &[SubtreeRecord]) -> String {
    indices
        .iter()
        .map(|&i| &records[i].snippet)
        .filter(|s| !s.is_empty())
        .max_by_key(|s| s.len())
        .cloned()
        .unwrap_or_default()
}

fn collect_locations(indices: &[usize], records: &[SubtreeRecord]) -> Vec<(PathBuf, u32)> {
    let mut locs: Vec<(PathBuf, u32)> = indices
        .iter()
        .map(|&i| (records[i].file.clone(), records[i].line))
        .collect();
    locs.sort();
    locs
}
