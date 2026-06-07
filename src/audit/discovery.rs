use std::collections::{BTreeSet, HashMap, HashSet};
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
    groups.into_iter().filter_map(|(fp, idx)| materialize(fp, &idx, records, thresholds)).collect()
}

pub fn closed_mine(
    records: &[SubtreeRecord],
    expression_fps: &HashSet<u64>,
    thresholds: &AuditThresholds,
) -> Vec<RawCluster> {
    let mut index = ClosureIndex::default();
    for r in records {
        index.observe(r);
    }
    let groups = group_by_fingerprint(records);
    groups
        .into_iter()
        .filter(|(fp, _)| index.is_closed(*fp, expression_fps))
        .filter_map(|(fp, idx)| materialize(fp, &idx, records, thresholds))
        .collect()
}

#[derive(Default)]
struct ClosureIndex {
    support_by_fp: HashMap<u64, u32>,
    parent_counts: HashMap<u64, HashMap<u64, u32>>,
}

impl ClosureIndex {
    fn observe(&mut self, record: &SubtreeRecord) {
        *self.support_by_fp.entry(record.fingerprint).or_insert(0) += 1;
        if let Some(parent) = record.parent_fingerprint {
            *self.parent_counts.entry(record.fingerprint).or_default().entry(parent).or_insert(0) += 1;
        }
    }

    fn is_closed(&self, fp: u64, expression_fps: &HashSet<u64>) -> bool {
        let support = self.support_by_fp.get(&fp).copied().unwrap_or(0);
        match self.parent_counts.get(&fp) {
            None => true,
            Some(counts) => !counts.iter().any(|(parent, &count)| {
                count == support
                    && self.support_by_fp.get(parent).copied().unwrap_or(0) == support
                    && expression_fps.contains(parent)
            }),
        }
    }
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
    let mut files: BTreeSet<PathBuf> = BTreeSet::new();
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
    let mut locs: Vec<(PathBuf, u32)> = indices.iter().map(|&i| (records[i].file.clone(), records[i].line)).collect();
    locs.sort();
    locs
}
