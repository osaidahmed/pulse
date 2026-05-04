use std::path::PathBuf;

use crate::thresholds::VendorThresholds;

use super::corpus_stats::{CorpusStats, KindHistogram, PerFileFeatures};

const MAD_NORMALIZER: f64 = 0.6745;

#[derive(Debug, Clone)]
pub struct VendorVerdict {
    pub file: PathBuf,
    pub flagged: bool,
    pub failed_features: Vec<&'static str>,
}

pub fn classify(stats: &CorpusStats, thresholds: &VendorThresholds) -> Vec<VendorVerdict> {
    let dist = Distribution::from(stats);
    stats
        .per_file
        .iter()
        .map(|f| evaluate_file(f, &dist, &stats.median_kind_histogram, thresholds))
        .collect()
}

pub fn flagged_paths(verdicts: &[VendorVerdict]) -> std::collections::HashSet<PathBuf> {
    verdicts
        .iter()
        .filter(|v| v.flagged)
        .map(|v| v.file.clone())
        .collect()
}

struct Distribution {
    mean_id_len: FeatureStats,
    var_id_len: FeatureStats,
    nodes_per_byte: FeatureStats,
    max_line_len: FeatureStats,
}

struct FeatureStats {
    median: f64,
    mad: f64,
}

impl Distribution {
    fn from(stats: &CorpusStats) -> Self {
        Self {
            mean_id_len: FeatureStats::compute(stats.per_file.iter().map(|f| f.mean_id_len)),
            var_id_len: FeatureStats::compute(stats.per_file.iter().map(|f| f.var_id_len)),
            nodes_per_byte: FeatureStats::compute(stats.per_file.iter().map(|f| f.ast_nodes_per_byte)),
            max_line_len: FeatureStats::compute(stats.per_file.iter().map(|f| f64::from(f.max_line_len))),
        }
    }
}

impl FeatureStats {
    fn compute(samples: impl Iterator<Item = f64>) -> Self {
        let mut values: Vec<f64> = samples.collect();
        let median = median_value(&mut values);
        let mut deviations: Vec<f64> = values.iter().map(|v| (v - median).abs()).collect();
        let mad = median_value(&mut deviations);
        Self { median, mad }
    }
}

fn median_value(values: &mut [f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    values[values.len() / 2]
}

fn mad_zscore(value: f64, stats: &FeatureStats) -> f64 {
    if stats.mad == 0.0 {
        return 0.0;
    }
    MAD_NORMALIZER * (value - stats.median).abs() / stats.mad
}

pub fn structural_cross_entropy(file_hist: &KindHistogram, project_hist: &KindHistogram) -> f64 {
    if file_hist.total == 0 || project_hist.total == 0 {
        return 0.0;
    }
    let mut kl = 0.0;
    for (kind, count) in &file_hist.counts {
        let p = f64::from(*count) / file_hist.total as f64;
        let q = project_hist.probability(kind, true);
        if p > 0.0 && q > 0.0 {
            kl += p * (p / q).log2();
        }
    }
    kl
}

fn evaluate_file(
    features: &PerFileFeatures,
    dist: &Distribution,
    project_hist: &KindHistogram,
    thresholds: &VendorThresholds,
) -> VendorVerdict {
    let mut verdict = VendorVerdict {
        file: features.file.clone(),
        flagged: false,
        failed_features: Vec::new(),
    };
    if features.size_bytes < thresholds.min_size_bytes {
        return verdict;
    }
    let cross_entropy = structural_cross_entropy(&features.kind_histogram, project_hist);
    for (name, getter) in feature_extractors() {
        let z = mad_zscore(getter(features, dist, cross_entropy), &stats_for(name, dist));
        if z >= thresholds.zscore_cutoff {
            verdict.failed_features.push(*name);
        }
    }
    verdict.flagged = verdict.failed_features.len() as u32 >= thresholds.min_features_failed;
    verdict
}

type Extractor = fn(&PerFileFeatures, &Distribution, f64) -> f64;

fn feature_extractors() -> &'static [(&'static str, Extractor)] {
    &[
        ("mean_id_len", get_mean_id_len),
        ("var_id_len", get_var_id_len),
        ("nodes_per_byte", get_nodes_per_byte),
        ("max_line_len", get_max_line_len),
        ("cross_entropy", get_cross_entropy),
    ]
}

fn get_mean_id_len(f: &PerFileFeatures, _d: &Distribution, _ce: f64) -> f64 { f.mean_id_len }
fn get_var_id_len(f: &PerFileFeatures, _d: &Distribution, _ce: f64) -> f64 { f.var_id_len }
fn get_nodes_per_byte(f: &PerFileFeatures, _d: &Distribution, _ce: f64) -> f64 { f.ast_nodes_per_byte }
fn get_max_line_len(f: &PerFileFeatures, _d: &Distribution, _ce: f64) -> f64 { f64::from(f.max_line_len) }
fn get_cross_entropy(_f: &PerFileFeatures, _d: &Distribution, ce: f64) -> f64 { ce }

fn stats_for(name: &str, dist: &Distribution) -> FeatureStats {
    let s = match name {
        "mean_id_len" => &dist.mean_id_len,
        "var_id_len" => &dist.var_id_len,
        "nodes_per_byte" => &dist.nodes_per_byte,
        "max_line_len" => &dist.max_line_len,
        _ => return FeatureStats { median: 0.0, mad: 1.0 },
    };
    FeatureStats { median: s.median, mad: s.mad }
}
