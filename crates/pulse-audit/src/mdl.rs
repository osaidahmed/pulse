use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy)]
pub struct CorpusScale {
    pub vocab: usize,
    pub total_occurrences: u64,
}

pub fn mdl_gain(usage: u32, pattern_size: u32, corpus: CorpusScale) -> f64 {
    if usage == 0 || corpus.total_occurrences == 0 {
        return 0.0;
    }
    let bits_per_node = (corpus.vocab.max(2) as f64).log2();
    let u = f64::from(usage);
    let s = f64::from(pattern_size.max(1));
    let n = corpus.total_occurrences as f64;
    let raw_compression = u * (s - 1.0) * bits_per_node;
    let model_cost = s * bits_per_node;
    let ref_entropy = u * (n.log2() - u.log2());
    raw_compression - model_cost - ref_entropy
}

pub fn locality_entropy(per_file_counts: &[u32]) -> f64 {
    let total: u64 = per_file_counts.iter().map(|&c| u64::from(c)).sum();
    if total == 0 {
        return 0.0;
    }
    let n = total as f64;
    -per_file_counts
        .iter()
        .filter(|&&c| c > 0)
        .map(|&c| {
            let p = f64::from(c) / n;
            p * p.log2()
        })
        .sum::<f64>()
}

pub fn file_counts(locations: &[(PathBuf, u32)]) -> Vec<u32> {
    let mut by_file: HashMap<&PathBuf, u32> = HashMap::new();
    for (file, _line) in locations {
        *by_file.entry(file).or_insert(0) += 1;
    }
    by_file.into_values().collect()
}
