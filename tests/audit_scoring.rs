mod audit_common;

use audit_common::*;
use pulse::audit::discovery::RawCluster;
use pulse::audit::finding::AuditKind;
use pulse::audit::scoring::apply_idf;
use std::path::PathBuf;

fn cluster(fp: u64, support: u32, file_count: u32, snippet: &str) -> RawCluster {
    RawCluster {
        fingerprint: fp,
        support,
        file_count,
        representative_snippet: snippet.to_string(),
        locations: (0..support)
            .map(|i| (PathBuf::from(format!("f{}.py", i % file_count)), 1))
            .collect(),
    }
}

fn fingerprint_of(f: &pulse::audit::finding::AuditFinding) -> u64 {
    let AuditKind::UncategorizedPattern { fingerprint } = f.kind else {
        return 0;
    };
    fingerprint
}

#[test]
fn apply_idf_passes_through_when_no_clusters() {
    let result = apply_idf(vec![], 10, &t().audit);
    assert!(result.is_empty());
}

#[test]
fn apply_idf_suppresses_cluster_above_threshold() {
    let mut th = t().audit;
    th.pattern_mining.idiom_suppression_threshold = 0.5;
    let cluster_60pct = cluster(7, 6, 6, "x");
    let result = apply_idf(vec![cluster_60pct], 10, &th);
    assert!(result.is_empty(), "60% file presence should be suppressed");
}

#[test]
fn apply_idf_keeps_cluster_below_threshold() {
    let mut th = t().audit;
    th.pattern_mining.idiom_suppression_threshold = 0.5;
    let cluster_30pct = cluster(7, 3, 3, "x");
    let result = apply_idf(vec![cluster_30pct], 10, &th);
    assert_eq!(result.len(), 1);
}

#[test]
fn apply_idf_at_exactly_threshold_kept() {
    let mut th = t().audit;
    th.pattern_mining.idiom_suppression_threshold = 0.5;
    let cluster_at_50 = cluster(7, 5, 5, "x");
    let result = apply_idf(vec![cluster_at_50], 10, &th);
    assert_eq!(result.len(), 1, "at threshold (50%) should be kept; suppression is strict >");
}

#[test]
fn apply_idf_just_above_threshold_suppressed() {
    let mut th = t().audit;
    th.pattern_mining.idiom_suppression_threshold = 0.5;
    let cluster_at_60 = cluster(7, 6, 6, "x");
    let result = apply_idf(vec![cluster_at_60], 10, &th);
    assert!(result.is_empty());
}

#[test]
fn apply_idf_with_zero_total_files_returns_empty() {
    let result = apply_idf(vec![cluster(7, 5, 5, "x")], 0, &t().audit);
    assert!(result.is_empty());
}

#[test]
fn apply_idf_computes_log_correctly() {
    let result = apply_idf(vec![cluster(7, 1, 1, "x")], 10, &t().audit);
    assert_eq!(result.len(), 1);
    let idf = result[0].idf_score.unwrap();
    let expected = (10.0_f64 / 1.0).ln();
    assert!((idf - expected).abs() < 1e-9, "idf={idf} expected={expected}");
}

#[test]
fn apply_idf_idf_score_populated() {
    let result = apply_idf(vec![cluster(7, 3, 3, "x")], 10, &t().audit);
    assert!(result[0].idf_score.is_some());
}

#[test]
fn apply_idf_preserves_locations() {
    let c = cluster(7, 3, 3, "x");
    let original_count = c.locations.len();
    let result = apply_idf(vec![c], 10, &t().audit);
    assert_eq!(result[0].locations.len(), original_count);
}

#[test]
fn apply_idf_preserves_snippet() {
    let result = apply_idf(vec![cluster(7, 3, 3, "media_type == X")], 10, &t().audit);
    assert!(result[0].representative_snippet.contains("media_type"));
}

#[test]
fn apply_idf_with_threshold_zero_keeps_only_unique() {
    let mut th = t().audit;
    th.pattern_mining.idiom_suppression_threshold = 0.0;
    let result = apply_idf(vec![cluster(7, 3, 3, "x")], 10, &th);
    assert!(result.is_empty(), "threshold 0 means anything in >0% is suppressed");
}

#[test]
fn apply_idf_with_threshold_one_keeps_all() {
    let mut th = t().audit;
    th.pattern_mining.idiom_suppression_threshold = 1.0;
    let result = apply_idf(vec![cluster(7, 10, 10, "x")], 10, &th);
    assert_eq!(result.len(), 1, "threshold 1.0 means even 100% is kept (suppression is strict >)");
}

#[test]
fn apply_idf_orders_by_support_descending() {
    let cs = vec![cluster(1, 3, 3, "a"), cluster(2, 7, 3, "b"), cluster(3, 5, 3, "c")];
    let result = apply_idf(cs, 10, &t().audit);
    assert_eq!(result[0].support, 7);
    assert_eq!(result[1].support, 5);
    assert_eq!(result[2].support, 3);
}

#[test]
fn apply_idf_breaks_ties_by_file_count_descending() {
    let cs = vec![cluster(1, 5, 2, "a"), cluster(2, 5, 5, "b"), cluster(3, 5, 3, "c")];
    let result = apply_idf(cs, 10, &t().audit);
    assert_eq!(result[0].file_count, 5);
    assert_eq!(result[1].file_count, 3);
    assert_eq!(result[2].file_count, 2);
}

#[test]
fn apply_idf_breaks_secondary_ties_by_fingerprint_ascending() {
    let cs = vec![cluster(99, 5, 3, "a"), cluster(11, 5, 3, "b"), cluster(50, 5, 3, "c")];
    let result = apply_idf(cs, 10, &t().audit);
    assert_eq!(fingerprint_of(&result[0]), 11);
    assert_eq!(fingerprint_of(&result[1]), 50);
    assert_eq!(fingerprint_of(&result[2]), 99);
}

#[test]
fn apply_idf_is_deterministic() {
    let make = || vec![cluster(7, 5, 3, "x"), cluster(8, 4, 4, "y")];
    let a = apply_idf(make(), 10, &t().audit);
    let b = apply_idf(make(), 10, &t().audit);
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(b.iter()) {
        assert_eq!(x.support, y.support);
        assert_eq!(fingerprint_of(x), fingerprint_of(y));
    }
}

#[test]
fn apply_idf_truncates_to_max_findings_reported() {
    let mut th = t().audit;
    th.pattern_mining.max_findings_reported = 3;
    th.pattern_mining.idiom_suppression_threshold = 1.0;
    let cs: Vec<RawCluster> = (0..10).map(|i| cluster(i, 5, 1, "x")).collect();
    let result = apply_idf(cs, 100, &th);
    assert_eq!(result.len(), 3);
}

#[test]
fn apply_idf_handles_thousand_clusters_no_panic() {
    let cs: Vec<RawCluster> = (0..1000).map(|i| cluster(i, 5, 3, "x")).collect();
    let _ = apply_idf(cs, 100, &t().audit);
}

#[test]
fn apply_idf_handles_cluster_at_total_files_zero_idf() {
    let result = apply_idf(vec![cluster(7, 10, 10, "x")], 10, &{
        let mut th = t().audit;
        th.pattern_mining.idiom_suppression_threshold = 1.0;
        th
    });
    assert_eq!(result.len(), 1);
    let idf = result[0].idf_score.unwrap();
    assert!(idf.abs() < 1e-9);
}

#[test]
fn apply_idf_does_not_modify_action_label() {
    let result = apply_idf(vec![cluster(7, 3, 3, "x")], 10, &t().audit);
    assert!(result[0].action_label.is_none());
}

#[test]
fn apply_idf_higher_idf_for_rarer_pattern() {
    let cs = vec![cluster(1, 2, 1, "rare"), cluster(2, 2, 5, "common")];
    let result = apply_idf(cs, 10, &t().audit);
    let rare = result.iter().find(|f| fingerprint_of(f) == 1).unwrap();
    let common = result.iter().find(|f| fingerprint_of(f) == 2).unwrap();
    assert!(rare.idf_score.unwrap() > common.idf_score.unwrap());
}

#[test]
fn apply_idf_kind_is_uncategorized_pattern_at_layer_3() {
    let result = apply_idf(vec![cluster(7, 3, 3, "x")], 10, &t().audit);
    assert!(matches!(result[0].kind, AuditKind::UncategorizedPattern { .. }));
}

#[test]
fn apply_idf_total_files_independent_of_record_count() {
    let result_low_total = apply_idf(vec![cluster(7, 6, 6, "x")], 10, &t().audit);
    let result_high_total = apply_idf(vec![cluster(7, 6, 6, "x")], 100, &t().audit);
    assert!(result_low_total.is_empty(), "6/10 = 60% > 0.5 threshold");
    assert!(!result_high_total.is_empty(), "6/100 = 6% below threshold");
}

#[test]
fn apply_idf_default_threshold_is_strict_greater_than_half() {
    let result = apply_idf(vec![cluster(7, 5, 5, "x")], 10, &t().audit);
    assert_eq!(result.len(), 1, "exactly 50% should be kept by default threshold");
}
