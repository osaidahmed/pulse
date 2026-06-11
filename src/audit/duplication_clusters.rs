use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;

use crate::parse::Language;
use crate::thresholds::{AuditThresholds, CloneClusterThresholds};

use super::finding::{AuditFinding, AuditKind, AuditLocation, CloneClusterEvidence, ImportConfidence};
use super::{corpus_stats, record_extraction, vendor_filter};

struct Candidate {
    file: PathBuf,
    line: u32,
    loc: u32,
    simhash: u64,
    popcount: u32,
    snippet: String,
}

#[derive(Clone)]
pub struct CloneMember {
    pub file: PathBuf,
    pub line: u32,
    pub loc: u32,
}

struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<u8>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        Self { parent: (0..n).collect(), rank: vec![0; n] }
    }

    fn find(&mut self, x: usize) -> usize {
        let mut root = x;
        while self.parent[root] != root {
            root = self.parent[root];
        }
        let mut cur = x;
        while self.parent[cur] != cur {
            let next = self.parent[cur];
            self.parent[cur] = root;
            cur = next;
        }
        root
    }

    fn union(&mut self, a: usize, b: usize) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra == rb {
            return;
        }
        match self.rank[ra].cmp(&self.rank[rb]) {
            std::cmp::Ordering::Less => self.parent[ra] = rb,
            std::cmp::Ordering::Greater => self.parent[rb] = ra,
            std::cmp::Ordering::Equal => {
                self.parent[rb] = ra;
                self.rank[ra] += 1;
            }
        }
    }
}

pub fn run(typed_files: &[(PathBuf, Language)], thresholds: &AuditThresholds) -> Vec<AuditFinding> {
    run_from(&super::corpus::Corpus::load(typed_files), thresholds)
}

pub fn run_from(corpus: &super::corpus::Corpus, thresholds: &AuditThresholds) -> Vec<AuditFinding> {
    let cands = candidates(corpus, thresholds);
    let thr = &thresholds.clone_cluster;
    cluster(&cands, thr).iter().map(|group| build_finding(&cands, group, thresholds)).collect()
}

pub fn cluster_members(typed_files: &[(PathBuf, Language)], thresholds: &AuditThresholds) -> Vec<Vec<CloneMember>> {
    cluster_members_from(&super::corpus::Corpus::load(typed_files), thresholds)
}

pub fn cluster_members_from(corpus: &super::corpus::Corpus, thresholds: &AuditThresholds) -> Vec<Vec<CloneMember>> {
    let cands = candidates(corpus, thresholds);
    cluster(&cands, &thresholds.clone_cluster)
        .iter()
        .map(|group| {
            group
                .iter()
                .map(|&i| CloneMember { file: cands[i].file.clone(), line: cands[i].line, loc: cands[i].loc })
                .collect()
        })
        .collect()
}

fn candidates(corpus: &super::corpus::Corpus, thresholds: &AuditThresholds) -> Vec<Candidate> {
    let bundle = record_extraction::corpus_bundle_from(corpus, thresholds);
    let stats = corpus_stats::aggregate_corpus(bundle.features);
    let flagged = vendor_filter::flagged_paths(&vendor_filter::classify(&stats, &thresholds.pattern_mining.vendor));
    let min_loc = thresholds.clone_cluster.min_loc;
    let mut cands: Vec<Candidate> = bundle
        .subtrees
        .into_iter()
        .filter(|r| !flagged.contains(&r.file) && r.loc >= min_loc)
        .map(|r| Candidate {
            file: r.file,
            line: r.line,
            loc: r.loc,
            simhash: r.simhash,
            popcount: r.simhash.count_ones(),
            snippet: r.snippet,
        })
        .collect();
    cands.sort_by(|a, b| a.loc.cmp(&b.loc).then(a.file.cmp(&b.file)).then(a.line.cmp(&b.line)));
    cands
}

fn cluster(cands: &[Candidate], thr: &CloneClusterThresholds) -> Vec<Vec<usize>> {
    let mut uf = UnionFind::new(cands.len());
    connect_pairs(cands, thr, &mut uf);
    components(cands, &mut uf, thr.min_cluster_size as usize)
}

fn connect_pairs(cands: &[Candidate], thr: &CloneClusterThresholds, uf: &mut UnionFind) {
    for (i, a) in cands.iter().enumerate() {
        let eps = eps_for(a.loc, thr.max_sim_threshold);
        for (j, b) in cands.iter().enumerate().skip(i + 1) {
            if !within_window(a.loc, b.loc, thr.loc_window_pct) {
                break;
            }
            if is_clone_pair(a, b, eps) {
                uf.union(i, j);
            }
        }
    }
}

fn is_clone_pair(a: &Candidate, b: &Candidate, eps: u32) -> bool {
    a.popcount.abs_diff(b.popcount) <= eps && hamming(a.simhash, b.simhash) <= eps && !same_file_overlap(a, b)
}

fn components(cands: &[Candidate], uf: &mut UnionFind, min_size: usize) -> Vec<Vec<usize>> {
    let mut by_root: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in 0..cands.len() {
        let root = uf.find(i);
        by_root.entry(root).or_default().push(i);
    }
    by_root.into_values().filter(|group| distinct_locations(cands, group) >= min_size).collect()
}

fn build_finding(cands: &[Candidate], group: &[usize], thresholds: &AuditThresholds) -> AuditFinding {
    let mut members: Vec<AuditLocation> =
        group.iter().map(|&i| AuditLocation { file: cands[i].file.clone(), line: cands[i].line }).collect();
    members.sort_by(|a, b| a.file.cmp(&b.file).then(a.line.cmp(&b.line)));
    members.dedup();
    let member_count = members.len() as u32;
    let distinct_files = count_distinct_files(&members);
    let max_loc = group.iter().map(|&i| cands[i].loc).max().unwrap_or(0);
    let representative = group.first().map_or_else(String::new, |&i| cands[i].snippet.clone());
    let confidence = if distinct_files >= 2 { ImportConfidence::Medium } else { ImportConfidence::Low };
    members.truncate(thresholds.max_locations_per_finding);
    AuditFinding {
        kind: AuditKind::NearDuplicate(CloneClusterEvidence {
            members,
            member_count,
            max_loc,
            representative,
            confidence,
        }),
        representative_snippet: String::new(),
        support: member_count,
        file_count: distinct_files,
        idf_score: None,
        action_label: None,
        pattern_category: None,
        locality_entropy: None,
        p_value: None,
        locations: Vec::new(),
    }
}

fn eps_for(loc: u32, max_sim: u32) -> u32 {
    let base = match loc {
        0..=7 => 7,
        8..=10 => 8,
        11..=13 => 9,
        14..=16 => 10,
        17..=20 => 11,
        _ => 12,
    };
    base.min(max_sim)
}

fn hamming(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

fn within_window(smaller: u32, larger: u32, pct: u32) -> bool {
    u64::from(larger) * 100 <= u64::from(smaller) * (100 + u64::from(pct))
}

fn same_file_overlap(a: &Candidate, b: &Candidate) -> bool {
    if a.file != b.file {
        return false;
    }
    let a_end = a.line + a.loc;
    let b_end = b.line + b.loc;
    a.line < b_end && b.line < a_end
}

fn distinct_locations(cands: &[Candidate], group: &[usize]) -> usize {
    let mut set: HashSet<(&PathBuf, u32)> = HashSet::new();
    for &i in group {
        set.insert((&cands[i].file, cands[i].line));
    }
    set.len()
}

fn count_distinct_files(members: &[AuditLocation]) -> u32 {
    let set: HashSet<&PathBuf> = members.iter().map(|m| &m.file).collect();
    set.len() as u32
}
