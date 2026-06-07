use pulse::audit::discovery::RawCluster;
use pulse::audit::finding::AuditKind;
use pulse::audit::scoring::apply_idf;
use pulse::thresholds::Thresholds;
use std::path::PathBuf;

fn t() -> Thresholds {
    Thresholds::default()
}

fn cluster(fp: u64, support: u32, files: u32, snippet: &str) -> RawCluster {
    RawCluster {
        fingerprint: fp,
        support,
        file_count: files,
        representative_snippet: snippet.to_string(),
        locations: (0..support).map(|i| (PathBuf::from(format!("f{}.py", i % files.max(1))), 1)).collect(),
    }
}

#[test]
fn scoring_idf_strict_gt_threshold_50_percent_kept() {
    let c = cluster(7, 5, 5, "x");
    assert_eq!(apply_idf(vec![c], 10, &t().audit).len(), 1);
}

#[test]
fn scoring_idf_six_percent_below_threshold_kept() {
    let c = cluster(7, 1, 1, "x");
    assert_eq!(apply_idf(vec![c], 100, &t().audit).len(), 1);
}

#[test]
fn scoring_idf_with_one_total_file_handled() {
    let c = cluster(7, 1, 1, "x");
    let mut th = t().audit;
    th.pattern_mining.idiom_suppression_threshold = 1.0;
    assert_eq!(apply_idf(vec![c], 1, &th).len(), 1);
}

#[test]
fn scoring_idf_threshold_just_above_zero_filters_almost_all() {
    let mut th = t().audit;
    th.pattern_mining.idiom_suppression_threshold = 0.001;
    let c = cluster(7, 2, 2, "x");
    assert!(apply_idf(vec![c], 100, &th).is_empty());
}

#[test]
fn scoring_idf_score_for_one_in_two_files_log_two() {
    let c = cluster(7, 1, 1, "x");
    let r = apply_idf(vec![c], 2, &t().audit);
    assert_eq!(r.len(), 1);
    let idf = r[0].idf_score.unwrap();
    let expected = (2.0_f64).ln();
    assert!((idf - expected).abs() < 1e-9);
}

#[test]
fn scoring_idf_score_for_one_in_one_thousand_log_thousand() {
    let c = cluster(7, 1, 1, "x");
    let r = apply_idf(vec![c], 1000, &t().audit);
    let idf = r[0].idf_score.unwrap();
    let expected = (1000.0_f64).ln();
    assert!((idf - expected).abs() < 1e-9);
}

#[test]
fn scoring_idf_score_distinct_for_distinct_file_counts() {
    let c1 = cluster(1, 5, 1, "x");
    let c2 = cluster(2, 5, 5, "y");
    let r = apply_idf(vec![c1, c2], 100, &t().audit);
    assert_eq!(r.len(), 2);
    let s1 = r
        .iter()
        .find(|f| matches!(f.kind, AuditKind::UncategorizedPattern { fingerprint } if fingerprint == 1))
        .unwrap();
    let s2 = r
        .iter()
        .find(|f| matches!(f.kind, AuditKind::UncategorizedPattern { fingerprint } if fingerprint == 2))
        .unwrap();
    assert!(s1.idf_score.unwrap() > s2.idf_score.unwrap());
}

#[test]
fn scoring_findings_truncate_to_max_findings_reported() {
    let mut th = t().audit;
    th.pattern_mining.max_findings_reported = 5;
    th.pattern_mining.idiom_suppression_threshold = 1.0;
    let cs: Vec<RawCluster> = (0..20).map(|i| cluster(i, 5, 2, "x")).collect();
    assert_eq!(apply_idf(cs, 100, &th).len(), 5);
}

#[test]
fn scoring_findings_truncate_at_zero_returns_empty() {
    let mut th = t().audit;
    th.pattern_mining.max_findings_reported = 0;
    th.pattern_mining.idiom_suppression_threshold = 1.0;
    let cs: Vec<RawCluster> = (0..10).map(|i| cluster(i, 5, 2, "x")).collect();
    assert!(apply_idf(cs, 100, &th).is_empty());
}

#[test]
fn scoring_findings_truncate_at_one_yields_top_finding() {
    let mut th = t().audit;
    th.pattern_mining.max_findings_reported = 1;
    th.pattern_mining.idiom_suppression_threshold = 1.0;
    let cs = vec![cluster(1, 3, 1, "low"), cluster(2, 7, 1, "high"), cluster(3, 5, 1, "mid")];
    let r = apply_idf(cs, 100, &th);
    assert_eq!(r[0].support, 7);
}

#[test]
fn scoring_orders_correctly_when_all_distinct_supports() {
    let cs = vec![cluster(1, 3, 1, "a"), cluster(2, 9, 1, "b"), cluster(3, 6, 1, "c"), cluster(4, 1, 1, "d")];
    let r = apply_idf(cs, 100, &t().audit);
    assert_eq!(r[0].support, 9);
    assert_eq!(r[1].support, 6);
    assert_eq!(r[2].support, 3);
    assert_eq!(r[3].support, 1);
}

#[test]
fn scoring_secondary_tiebreak_by_file_count() {
    let cs = vec![cluster(1, 5, 2, "a"), cluster(2, 5, 4, "b"), cluster(3, 5, 1, "c")];
    let r = apply_idf(cs, 100, &t().audit);
    assert_eq!(r[0].file_count, 4);
    assert_eq!(r[1].file_count, 2);
    assert_eq!(r[2].file_count, 1);
}

#[test]
fn scoring_tertiary_tiebreak_by_fingerprint() {
    let cs = vec![cluster(99, 5, 3, "a"), cluster(11, 5, 3, "b"), cluster(50, 5, 3, "c")];
    let r = apply_idf(cs, 100, &t().audit);
    let fps: Vec<u64> = r
        .iter()
        .filter_map(|f| match f.kind {
            AuditKind::UncategorizedPattern { fingerprint } => Some(fingerprint),
            _ => None,
        })
        .collect();
    assert_eq!(fps, vec![11, 50, 99]);
}

#[test]
fn scoring_handles_threshold_one_keeps_all_clusters() {
    let mut th = t().audit;
    th.pattern_mining.idiom_suppression_threshold = 1.0;
    let cs: Vec<RawCluster> = (0..10).map(|i| cluster(i, 5, 5, "x")).collect();
    let r = apply_idf(cs, 5, &th);
    assert_eq!(r.len(), 10);
}

#[test]
fn scoring_handles_threshold_above_one_keeps_all() {
    let mut th = t().audit;
    th.pattern_mining.idiom_suppression_threshold = 2.0;
    let c = cluster(7, 5, 5, "x");
    assert_eq!(apply_idf(vec![c], 5, &th).len(), 1);
}

#[test]
fn scoring_idf_log_returns_zero_when_file_count_equals_total() {
    let mut th = t().audit;
    th.pattern_mining.idiom_suppression_threshold = 1.0;
    let c = cluster(7, 50, 50, "x");
    let r = apply_idf(vec![c], 50, &th);
    assert!(r[0].idf_score.unwrap().abs() < 1e-9);
}

#[test]
fn scoring_action_label_remains_none_after_scoring() {
    let c = cluster(7, 5, 1, "x");
    let r = apply_idf(vec![c], 100, &t().audit);
    assert!(r[0].action_label.is_none());
}

#[test]
fn scoring_locations_count_unchanged_through_scoring() {
    let mut th = t().audit;
    th.pattern_mining.idiom_suppression_threshold = 1.0;
    let c = cluster(7, 7, 7, "x");
    let r = apply_idf(vec![c], 100, &th);
    assert_eq!(r[0].locations.len(), 7);
}

#[test]
fn scoring_kind_remains_uncategorized_after_scoring() {
    let c = cluster(7, 5, 1, "x");
    let r = apply_idf(vec![c], 100, &t().audit);
    assert!(matches!(r[0].kind, AuditKind::UncategorizedPattern { .. }));
}

#[test]
fn scoring_does_not_mutate_input_cluster_locations() {
    let c = cluster(7, 5, 1, "x");
    let original_loc_count = c.locations.len();
    let _ = apply_idf(vec![c.clone()], 100, &t().audit);
    let _ = c;
    assert_eq!(original_loc_count, 5);
}

#[test]
fn scoring_handles_thousands_of_clusters_under_threshold() {
    let mut th = t().audit;
    th.pattern_mining.idiom_suppression_threshold = 1.0;
    let cs: Vec<RawCluster> = (0..1000).map(|i| cluster(i, 5, 1, "x")).collect();
    let r = apply_idf(cs, 1000, &th);
    assert_eq!(r.len(), th.pattern_mining.max_findings_reported);
}

#[test]
fn scoring_finds_highest_support_first_among_thousands() {
    let mut th = t().audit;
    th.pattern_mining.idiom_suppression_threshold = 1.0;
    let mut cs: Vec<RawCluster> = (0..100).map(|i| cluster(i, 5, 1, "x")).collect();
    cs.push(cluster(999, 999, 1, "winner"));
    let r = apply_idf(cs, 100, &th);
    assert_eq!(r[0].representative_snippet, "winner");
}

#[test]
fn scoring_zero_total_files_returns_empty_regardless_of_clusters() {
    let cs: Vec<RawCluster> = (0..50).map(|i| cluster(i, 5, 1, "x")).collect();
    assert!(apply_idf(cs, 0, &t().audit).is_empty());
}

#[test]
fn scoring_idf_same_for_same_ratio_different_scales() {
    let c1 = cluster(1, 1, 1, "x");
    let c2 = cluster(2, 1, 1, "y");
    let r1 = apply_idf(vec![c1], 10, &t().audit);
    let r2 = apply_idf(vec![c2], 100, &t().audit);
    assert!(r2[0].idf_score.unwrap() > r1[0].idf_score.unwrap());
}

#[test]
fn scoring_max_findings_one_with_zero_clusters_returns_empty() {
    let mut th = t().audit;
    th.pattern_mining.max_findings_reported = 1;
    assert!(apply_idf(vec![], 100, &th).is_empty());
}

#[test]
fn scoring_handles_empty_locations_in_cluster() {
    let c = RawCluster {
        fingerprint: 7,
        support: 5,
        file_count: 1,
        representative_snippet: "x".to_string(),
        locations: vec![],
    };
    let r = apply_idf(vec![c], 100, &t().audit);
    assert_eq!(r.len(), 1);
    assert!(r[0].locations.is_empty());
}

#[test]
fn scoring_handles_cluster_with_locations_count_mismatching_support() {
    let c = RawCluster {
        fingerprint: 7,
        support: 100,
        file_count: 1,
        representative_snippet: "x".to_string(),
        locations: vec![(PathBuf::from("a.py"), 1)],
    };
    let r = apply_idf(vec![c], 100, &t().audit);
    assert_eq!(r[0].locations.len(), 1);
    assert_eq!(r[0].support, 100);
}

#[test]
fn scoring_thresholds_independence_across_calls() {
    let mut th = t().audit;
    let c1 = cluster(7, 5, 5, "x");
    let r1 = apply_idf(vec![c1], 10, &th);
    th.pattern_mining.idiom_suppression_threshold = 0.3;
    let c2 = cluster(7, 5, 5, "x");
    let r2 = apply_idf(vec![c2], 10, &th);
    assert_eq!(r1.len(), 1);
    assert!(r2.is_empty());
}

#[test]
fn scoring_finding_struct_consistent_after_idf() {
    let c = cluster(7, 5, 1, "media_type == X");
    let r = apply_idf(vec![c], 100, &t().audit);
    assert_eq!(r.len(), 1);
    let f = &r[0];
    assert_eq!(f.support, 5);
    assert_eq!(f.file_count, 1);
    assert_eq!(f.representative_snippet, "media_type == X");
}

#[test]
fn scoring_total_files_smaller_than_file_count_handled() {
    let mut th = t().audit;
    th.pattern_mining.idiom_suppression_threshold = 3.0;
    let c = cluster(7, 5, 100, "x");
    let r = apply_idf(vec![c], 50, &th);
    assert_eq!(r.len(), 1, "ratio 100/50=2.0 below threshold 3.0");
}

#[test]
fn scoring_keeps_cluster_with_idiom_threshold_just_above_actual() {
    let mut th = t().audit;
    th.pattern_mining.idiom_suppression_threshold = 0.6;
    let c = cluster(7, 5, 5, "x");
    let r = apply_idf(vec![c], 10, &th);
    assert_eq!(r.len(), 1);
}

#[test]
fn scoring_suppresses_when_threshold_below_actual_ratio() {
    let mut th = t().audit;
    th.pattern_mining.idiom_suppression_threshold = 0.4;
    let c = cluster(7, 5, 5, "x");
    let r = apply_idf(vec![c], 10, &th);
    assert!(r.is_empty());
}

#[test]
fn scoring_finds_uniform_idf_for_identical_inputs() {
    let cs = vec![cluster(1, 5, 5, "x"), cluster(2, 5, 5, "y")];
    let r = apply_idf(cs, 100, &t().audit);
    let s1 = r[0].idf_score.unwrap();
    let s2 = r[1].idf_score.unwrap();
    assert!((s1 - s2).abs() < 1e-9);
}

#[test]
fn scoring_idf_grows_with_total_files_for_same_file_count() {
    let r1 = apply_idf(vec![cluster(7, 1, 1, "x")], 10, &t().audit);
    let r2 = apply_idf(vec![cluster(7, 1, 1, "x")], 100, &t().audit);
    let r3 = apply_idf(vec![cluster(7, 1, 1, "x")], 1000, &t().audit);
    assert!(r1[0].idf_score.unwrap() < r2[0].idf_score.unwrap());
    assert!(r2[0].idf_score.unwrap() < r3[0].idf_score.unwrap());
}
