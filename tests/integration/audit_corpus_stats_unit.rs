use std::path::PathBuf;

use pulse::audit::corpus_stats::{
    aggregate_corpus, line_length_stats, KindHistogram, PerFileFeatures, WelfordIdentifierStats,
};

#[test]
fn welford_zero_observations_returns_zeros() {
    let w = WelfordIdentifierStats::default();
    assert_eq!(w.finalize(), (0.0, 0.0));
}

#[test]
fn welford_single_sample_zero_variance() {
    let mut w = WelfordIdentifierStats::default();
    w.observe(7);
    let (mean, var) = w.finalize();
    assert!((mean - 7.0).abs() < 1e-9, "mean: {mean}");
    assert!(var.abs() < 1e-9, "variance must be 0 for one sample, got {var}");
}

#[test]
fn welford_three_samples_4_6_8() {
    let mut w = WelfordIdentifierStats::default();
    for &v in &[4u32, 6, 8] {
        w.observe(v);
    }
    let (mean, var) = w.finalize();
    assert!((mean - 6.0).abs() < 1e-9, "expected mean=6, got {mean}");
    assert!((var - 4.0).abs() < 1e-9, "expected sample variance=4, got {var}");
}

#[test]
fn welford_matches_naive_variance_for_random_sequence() {
    let xs: Vec<u32> = vec![3, 7, 2, 11, 5, 9, 4, 8, 6, 10];
    let mut w = WelfordIdentifierStats::default();
    for &v in &xs {
        w.observe(v);
    }
    let (mean, var) = w.finalize();
    let naive_mean: f64 = xs.iter().map(|v| f64::from(*v)).sum::<f64>() / xs.len() as f64;
    let naive_var: f64 = xs.iter().map(|v| (f64::from(*v) - naive_mean).powi(2)).sum::<f64>() / (xs.len() as f64 - 1.0);
    assert!((mean - naive_mean).abs() < 1e-9, "mean disagrees");
    assert!((var - naive_var).abs() < 1e-9, "variance disagrees");
}

#[test]
fn welford_variance_is_nonnegative() {
    let mut w = WelfordIdentifierStats::default();
    for v in &[1u32, 2, 3, 4, 5, 6, 7, 8, 9, 10] {
        w.observe(*v);
    }
    let (_, var) = w.finalize();
    assert!(var >= 0.0, "variance must be non-negative, got {var}");
}

#[test]
fn kind_histogram_observe_increments_count_and_total() {
    let mut h = KindHistogram::default();
    for _ in 0..3 {
        h.observe("identifier");
    }
    h.observe("call");
    assert_eq!(h.total, 4);
    assert_eq!(h.counts.get("identifier").copied(), Some(3));
    assert_eq!(h.counts.get("call").copied(), Some(1));
}

#[test]
fn kind_histogram_probability_unsmoothed_exact() {
    let mut h = KindHistogram::default();
    for _ in 0..3 {
        h.observe("identifier");
    }
    h.observe("call");
    assert!((h.probability("identifier", false) - 0.75).abs() < 1e-9);
    assert!((h.probability("call", false) - 0.25).abs() < 1e-9);
    assert!((h.probability("missing", false) - 0.0).abs() < 1e-9);
}

#[test]
fn kind_histogram_probability_smoothed_exact() {
    let mut h = KindHistogram::default();
    for _ in 0..3 {
        h.observe("identifier");
    }
    h.observe("call");
    let vocab = 2u64;
    let total = 4u64;
    let denom = (total + vocab) as f64;
    assert!((h.probability("identifier", true) - (3.0 + 1.0) / denom).abs() < 1e-9);
    assert!((h.probability("call", true) - (1.0 + 1.0) / denom).abs() < 1e-9);
    assert!((h.probability("missing", true) - (0.0 + 1.0) / denom).abs() < 1e-9);
}

#[test]
fn kind_histogram_probability_unsmoothed_zero_total() {
    let h = KindHistogram::default();
    assert!((h.probability("anything", false) - 0.0).abs() < 1e-9);
}

#[test]
fn kind_histogram_probabilities_sum_to_one_unsmoothed() {
    let mut h = KindHistogram::default();
    for kind in &["a", "a", "b", "c", "c", "c"] {
        h.observe(kind);
    }
    let sum = h.probability("a", false) + h.probability("b", false) + h.probability("c", false);
    assert!((sum - 1.0).abs() < 1e-9, "expected sum=1.0, got {sum}");
}

#[test]
fn kind_histogram_merged_sums_counts_and_total() {
    let mut a = KindHistogram::default();
    a.observe("x");
    a.observe("x");
    a.observe("y");
    let mut b = KindHistogram::default();
    b.observe("x");
    b.observe("z");
    let merged = a.merged(&b);
    assert_eq!(merged.total, 5);
    assert_eq!(merged.counts.get("x").copied(), Some(3));
    assert_eq!(merged.counts.get("y").copied(), Some(1));
    assert_eq!(merged.counts.get("z").copied(), Some(1));
}

#[test]
fn line_length_stats_known_input() {
    let source = "short\nmedium length line\nx\nthe longest line of all here\nmedium two";
    let (max, median) = line_length_stats(source);
    let mut lens: Vec<u32> = source.lines().map(|l| l.chars().count() as u32).collect();
    lens.sort_unstable();
    assert_eq!(max, *lens.last().unwrap());
    assert_eq!(median, lens[lens.len() / 2]);
}

#[test]
fn line_length_stats_empty_input() {
    assert_eq!(line_length_stats(""), (0, 0));
}

#[test]
fn aggregate_corpus_preserves_per_file_and_sums_project_histogram() {
    let mut a = KindHistogram::default();
    a.observe("identifier");
    a.observe("identifier");
    a.observe("call");
    let mut b = KindHistogram::default();
    b.observe("identifier");
    b.observe("string");

    let per_file = vec![
        PerFileFeatures {
            file: PathBuf::from("a.py"),
            mean_id_len: 5.0,
            var_id_len: 1.0,
            ast_nodes_per_byte: 0.1,
            max_line_len: 80,
            median_line_len: 40,
            kind_histogram: a,
            size_bytes: 1000,
        },
        PerFileFeatures {
            file: PathBuf::from("b.py"),
            mean_id_len: 6.0,
            var_id_len: 2.0,
            ast_nodes_per_byte: 0.12,
            max_line_len: 70,
            median_line_len: 35,
            kind_histogram: b,
            size_bytes: 800,
        },
    ];
    let stats = aggregate_corpus(per_file);
    assert_eq!(stats.per_file.len(), 2);
    assert_eq!(stats.project_kind_histogram.total, 5);
    assert_eq!(stats.project_kind_histogram.counts.get("identifier").copied(), Some(3));
    assert_eq!(stats.project_kind_histogram.counts.get("call").copied(), Some(1));
    assert_eq!(stats.project_kind_histogram.counts.get("string").copied(), Some(1));
}
