use std::path::PathBuf;

use pulse::audit::corpus_stats::{aggregate_corpus, KindHistogram, PerFileFeatures};
use pulse::audit::vendor_filter::{classify, flagged_paths, structural_cross_entropy};
use pulse::thresholds::VendorThresholds;

fn build_kind_histogram(entries: &[(&str, u32)]) -> KindHistogram {
    let mut h = KindHistogram::default();
    for (kind, count) in entries {
        for _ in 0..*count {
            h.observe(kind);
        }
    }
    h
}

fn normal_file(idx: usize) -> PerFileFeatures {
    let jitter = idx as f64 * 0.1;
    PerFileFeatures {
        file: PathBuf::from(format!("normal_{idx}.py")),
        mean_id_len: 8.0 + jitter,
        var_id_len: 4.0 + jitter,
        ast_nodes_per_byte: 0.05 + (jitter / 100.0),
        max_line_len: 80 + idx as u32,
        median_line_len: 40 + idx as u32,
        kind_histogram: build_kind_histogram(&[
            ("function_definition", 5 + idx as u32),
            ("identifier", 50 + idx as u32 * 2),
            ("call", 25 + idx as u32),
            ("string", 10 + idx as u32),
        ]),
        size_bytes: 4_000 + idx as u64 * 100,
    }
}

fn outlier_file() -> PerFileFeatures {
    PerFileFeatures {
        file: PathBuf::from("vendor.min.js"),
        mean_id_len: 1.5,
        var_id_len: 0.1,
        ast_nodes_per_byte: 0.4,
        max_line_len: 4_000,
        median_line_len: 4_000,
        kind_histogram: build_kind_histogram(&[("identifier", 5_000)]),
        size_bytes: 80_000,
    }
}

#[test]
fn vendor_outlier_is_flagged() {
    let mut per_file: Vec<PerFileFeatures> = (0..10).map(normal_file).collect();
    let outlier = outlier_file();
    let outlier_path = outlier.file.clone();
    per_file.push(outlier);
    let stats = aggregate_corpus(per_file);
    let verdicts = classify(&stats, &VendorThresholds::DEFAULTS);
    let flagged = flagged_paths(&verdicts);
    assert!(
        flagged.contains(&outlier_path),
        "outlier should be flagged, got: {flagged:?}"
    );
}

#[test]
fn small_file_skipped_by_size_gate() {
    let mut per_file: Vec<PerFileFeatures> = (0..10).map(normal_file).collect();
    let mut tiny_outlier = outlier_file();
    tiny_outlier.size_bytes = VendorThresholds::DEFAULTS.min_size_bytes - 1;
    let tiny_path = tiny_outlier.file.clone();
    per_file.push(tiny_outlier);
    let stats = aggregate_corpus(per_file);
    let verdicts = classify(&stats, &VendorThresholds::DEFAULTS);
    let flagged = flagged_paths(&verdicts);
    assert!(
        !flagged.contains(&tiny_path),
        "tiny file below size gate must not be flagged, got: {flagged:?}"
    );
}

#[test]
fn homogeneous_corpus_flags_nothing() {
    let per_file: Vec<PerFileFeatures> = (0..10).map(normal_file).collect();
    let stats = aggregate_corpus(per_file);
    let verdicts = classify(&stats, &VendorThresholds::DEFAULTS);
    let flagged = flagged_paths(&verdicts);
    assert!(
        flagged.is_empty(),
        "homogeneous corpus must not flag anything, got: {flagged:?}"
    );
}

#[test]
fn structural_cross_entropy_zero_when_either_empty() {
    let empty = KindHistogram::default();
    let nonempty = build_kind_histogram(&[("identifier", 10), ("call", 5)]);
    assert_eq!(structural_cross_entropy(&empty, &nonempty), 0.0);
    assert_eq!(structural_cross_entropy(&nonempty, &empty), 0.0);
    assert_eq!(structural_cross_entropy(&empty, &empty), 0.0);
}

#[test]
fn structural_cross_entropy_zero_when_distributions_match() {
    let a = build_kind_histogram(&[("identifier", 10), ("call", 5)]);
    let b = build_kind_histogram(&[("identifier", 10), ("call", 5)]);
    let ce = structural_cross_entropy(&a, &b);
    assert!(
        ce.abs() < 0.05,
        "matched distributions should yield ~0 cross-entropy, got {ce}"
    );
}

#[test]
fn structural_cross_entropy_positive_for_divergent_distributions() {
    let a = build_kind_histogram(&[("identifier", 100), ("call", 1)]);
    let b = build_kind_histogram(&[("identifier", 1), ("call", 100)]);
    let ce_ab = structural_cross_entropy(&a, &b);
    let ce_ba = structural_cross_entropy(&b, &a);
    assert!(
        ce_ab > 0.5,
        "divergent A->B cross-entropy must be clearly positive, got {ce_ab}"
    );
    assert!(
        ce_ba > 0.5,
        "divergent B->A cross-entropy must be clearly positive, got {ce_ba}"
    );
}

#[test]
fn min_features_failed_threshold_strict_ge() {
    let per_file: Vec<PerFileFeatures> = (0..10).map(normal_file).collect();
    let stats = aggregate_corpus(per_file);

    let mut strict = VendorThresholds::DEFAULTS;
    strict.min_features_failed = 1_000_000;
    let verdicts = classify(&stats, &strict);
    assert!(
        flagged_paths(&verdicts).is_empty(),
        "with absurdly high min_features_failed nothing flags"
    );

    let mut zero = VendorThresholds::DEFAULTS;
    zero.min_features_failed = 0;
    let verdicts = classify(&stats, &zero);
    let flagged = flagged_paths(&verdicts);
    assert_eq!(
        flagged.len(),
        stats.per_file.len(),
        "with min_features_failed=0 every file is flagged (verdict.flagged = failed_count >= 0)"
    );
}

#[test]
fn classify_returns_one_verdict_per_file() {
    let per_file: Vec<PerFileFeatures> = (0..7).map(normal_file).collect();
    let stats = aggregate_corpus(per_file);
    let verdicts = classify(&stats, &VendorThresholds::DEFAULTS);
    assert_eq!(verdicts.len(), 7);
}
