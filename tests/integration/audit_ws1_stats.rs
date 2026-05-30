use std::path::PathBuf;

use pulse::audit::confidence::{named_smell_confidence, EvidenceQuality};
use pulse::audit::finding::ImportConfidence;
use pulse::audit::mdl::{file_counts, locality_entropy, mdl_gain, CorpusScale};
use pulse::audit::swap_significance::{p_values, IncidenceMatrix, SwapConfig};
use pulse::parse::Language;

const CORPUS: CorpusScale = CorpusScale { vocab: 64, total_occurrences: 200 };

#[test]
fn mdl_gain_monotone_in_pattern_size() {
    assert!(mdl_gain(10, 6, CORPUS) > mdl_gain(10, 3, CORPUS));
}

#[test]
fn mdl_gain_zero_usage_is_zero() {
    assert!((mdl_gain(0, 5, CORPUS)).abs() < 1e-12);
}

#[test]
fn mdl_gain_finite_for_degenerate_vocab() {
    let corpus = CorpusScale { vocab: 0, total_occurrences: 10 };
    assert!(mdl_gain(5, 3, corpus).is_finite());
}

#[test]
fn mdl_gain_finite_for_zero_total() {
    let corpus = CorpusScale { vocab: 64, total_occurrences: 0 };
    assert!((mdl_gain(5, 3, corpus)).abs() < 1e-12);
}

#[test]
fn locality_entropy_even_distribution_is_log2_n() {
    assert!((locality_entropy(&[1, 1, 1, 1]) - 2.0).abs() < 1e-9);
}

#[test]
fn locality_entropy_single_file_is_zero() {
    assert!((locality_entropy(&[7])).abs() < 1e-12);
}

#[test]
fn locality_entropy_empty_is_zero() {
    assert!((locality_entropy(&[])).abs() < 1e-12);
}

#[test]
fn file_counts_groups_occurrences_by_file() {
    let locs = vec![
        (PathBuf::from("a.rs"), 1),
        (PathBuf::from("a.rs"), 5),
        (PathBuf::from("b.rs"), 2),
    ];
    let mut counts = file_counts(&locs);
    counts.sort_unstable();
    assert_eq!(counts, vec![1, 2]);
}

fn swap_cfg(samples: usize, max_cols: usize) -> SwapConfig {
    SwapConfig { samples, step_multiplier: 5, seed: 0x1234_5678, max_cols }
}

fn sample_matrix() -> IncidenceMatrix {
    let edges = vec![(0, 0), (1, 0), (1, 1), (2, 1), (2, 2), (0, 2)];
    IncidenceMatrix::new(3, 3, &edges)
}

#[test]
fn p_values_are_deterministic_for_fixed_seed() {
    let matrix = sample_matrix();
    assert_eq!(p_values(&matrix, swap_cfg(100, 64)), p_values(&matrix, swap_cfg(100, 64)));
}

#[test]
fn p_values_stay_in_unit_half_range() {
    for p in p_values(&sample_matrix(), swap_cfg(100, 64)) {
        assert!((0.0..=0.5).contains(&p), "p={p}");
    }
}

#[test]
fn p_values_skip_above_max_cols() {
    let matrix = IncidenceMatrix::new(2, 2, &[(0, 0), (1, 1)]);
    assert_eq!(p_values(&matrix, swap_cfg(100, 1)), vec![0.5, 0.5]);
}

#[test]
fn p_values_neutral_with_too_few_edges() {
    let matrix = IncidenceMatrix::new(1, 1, &[(0, 0)]);
    assert_eq!(p_values(&matrix, swap_cfg(100, 64)), vec![0.5]);
}

#[test]
fn confidence_resolved_in_file_keeps_base() {
    assert_eq!(
        named_smell_confidence(Language::Rust, EvidenceQuality::ResolvedInFile),
        ImportConfidence::High
    );
}

#[test]
fn confidence_downgrades_by_evidence_quality() {
    let unique = named_smell_confidence(Language::Rust, EvidenceQuality::NameKeyedUnique);
    let ambiguous = named_smell_confidence(Language::Rust, EvidenceQuality::NameKeyedAmbiguous);
    let heuristic = named_smell_confidence(Language::Rust, EvidenceQuality::Heuristic);
    assert!(ambiguous < unique);
    assert!(heuristic < ambiguous);
}

#[test]
fn confidence_floors_at_best_effort() {
    assert_eq!(
        named_smell_confidence(Language::C, EvidenceQuality::Heuristic),
        ImportConfidence::BestEffort
    );
}
