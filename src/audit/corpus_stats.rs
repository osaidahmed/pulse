use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct KindHistogram {
    pub counts: HashMap<Box<str>, u32>,
    pub total: u64,
}

impl KindHistogram {
    pub fn observe(&mut self, kind: &str) {
        *self.counts.entry(kind.into()).or_insert(0) += 1;
        self.total += 1;
    }

    #[must_use]
    pub fn merged(self, other: &Self) -> Self {
        let mut out = self;
        for (kind, count) in &other.counts {
            *out.counts.entry(kind.clone()).or_insert(0) += count;
        }
        out.total += other.total;
        out
    }

    pub fn probability(&self, kind: &str, smoothing: bool) -> f64 {
        let raw = self.counts.get(kind).copied().unwrap_or(0);
        if smoothing {
            let vocab = self.counts.len() as u64;
            let denom = (self.total + vocab).max(1) as f64;
            (f64::from(raw) + 1.0) / denom
        } else if self.total == 0 {
            0.0
        } else {
            f64::from(raw) / self.total as f64
        }
    }
}

#[derive(Debug, Clone)]
pub struct PerFileFeatures {
    pub file: PathBuf,
    pub mean_id_len: f64,
    pub var_id_len: f64,
    pub ast_nodes_per_byte: f64,
    pub max_line_len: u32,
    pub median_line_len: u32,
    pub kind_histogram: KindHistogram,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Default)]
pub struct CorpusStats {
    pub project_kind_histogram: KindHistogram,
    pub median_kind_histogram: KindHistogram,
    pub per_file: Vec<PerFileFeatures>,
}

pub fn aggregate_corpus(per_file: Vec<PerFileFeatures>) -> CorpusStats {
    let project = build_project_histogram(&per_file);
    let median = build_median_histogram(&per_file);
    CorpusStats {
        project_kind_histogram: project,
        median_kind_histogram: median,
        per_file,
    }
}

fn build_project_histogram(per_file: &[PerFileFeatures]) -> KindHistogram {
    per_file
        .iter()
        .fold(KindHistogram::default(), |acc, f| acc.merged(&f.kind_histogram))
}

fn build_median_histogram(per_file: &[PerFileFeatures]) -> KindHistogram {
    let kinds = collect_unique_kinds(per_file);
    let mut counts: HashMap<Box<str>, u32> = HashMap::new();
    let mut total: u64 = 0;
    for kind in &kinds {
        let median = median_count_for_kind(per_file, kind);
        if median > 0 {
            counts.insert(kind.clone(), median);
            total += u64::from(median);
        }
    }
    KindHistogram { counts, total }
}

fn collect_unique_kinds(per_file: &[PerFileFeatures]) -> Vec<Box<str>> {
    let mut seen: HashSet<Box<str>> = HashSet::new();
    for f in per_file {
        for kind in f.kind_histogram.counts.keys() {
            seen.insert(kind.clone());
        }
    }
    seen.into_iter().collect()
}

fn median_count_for_kind(per_file: &[PerFileFeatures], kind: &str) -> u32 {
    let mut samples: Vec<u32> = per_file
        .iter()
        .map(|f| f.kind_histogram.counts.get(kind).copied().unwrap_or(0))
        .collect();
    samples.sort_unstable();
    let mid = samples.len() / 2;
    samples.get(mid).copied().unwrap_or(0)
}

pub fn line_length_stats(source: &str) -> (u32, u32) {
    let mut lengths: Vec<u32> = source
        .lines()
        .map(|line| line.chars().count() as u32)
        .collect();
    if lengths.is_empty() {
        return (0, 0);
    }
    lengths.sort_unstable();
    let max = *lengths.last().unwrap_or(&0);
    let median = lengths[lengths.len() / 2];
    (max, median)
}

#[derive(Debug, Default, Clone)]
pub struct WelfordIdentifierStats {
    count: u64,
    mean: f64,
    m2: f64,
}

impl WelfordIdentifierStats {
    pub fn observe(&mut self, length: u32) {
        self.count += 1;
        let delta = f64::from(length) - self.mean;
        self.mean += delta / self.count as f64;
        let delta2 = f64::from(length) - self.mean;
        self.m2 += delta * delta2;
    }

    pub fn finalize(&self) -> (f64, f64) {
        if self.count == 0 {
            return (0.0, 0.0);
        }
        let variance = if self.count > 1 {
            self.m2 / (self.count - 1) as f64
        } else {
            0.0
        };
        (self.mean, variance)
    }
}
