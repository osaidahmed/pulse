mod audit_common;

use audit_common::*;
use pulse::audit::discovery::{freqt_mine, RawCluster};
use pulse::audit::walker::SubtreeRecord;
use std::path::PathBuf;

fn record(fp: u64, file: &str, line: u32, snippet: &str) -> SubtreeRecord {
    SubtreeRecord {
        fingerprint: fp,
        file: PathBuf::from(file),
        line,
        depth: 5,
        named_node_count: 8,
        snippet: snippet.to_string(),
    }
}

fn fp_groups(clusters: &[RawCluster]) -> std::collections::HashMap<u64, &RawCluster> {
    clusters.iter().map(|c| (c.fingerprint, c)).collect()
}

#[test]
fn freqt_mine_returns_empty_when_no_records() {
    let clusters = freqt_mine(&[], &t().audit);
    assert!(clusters.is_empty());
}

#[test]
fn freqt_mine_returns_empty_when_all_below_min_support() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 5;
    let records: Vec<SubtreeRecord> = (0..4).map(|i| record(7, &format!("a{i}.py"), 1, "x")).collect();
    let clusters = freqt_mine(&records, &th);
    assert!(clusters.is_empty());
}

#[test]
fn freqt_mine_surfaces_cluster_at_exactly_min_support() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 5;
    let records: Vec<SubtreeRecord> = (0..5).map(|i| record(7, &format!("f{i}.py"), 1, "x == y")).collect();
    let clusters = freqt_mine(&records, &th);
    assert_eq!(clusters.len(), 1);
    assert_eq!(clusters[0].support, 5);
}

#[test]
fn freqt_mine_surfaces_cluster_above_min_support() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 5;
    let records: Vec<SubtreeRecord> = (0..10).map(|i| record(7, &format!("f{i}.py"), 1, "x")).collect();
    let clusters = freqt_mine(&records, &th);
    assert_eq!(clusters[0].support, 10);
}

#[test]
fn freqt_mine_distinguishes_clusters_by_fingerprint() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 3;
    let records: Vec<SubtreeRecord> = vec![
        record(1, "a.py", 1, "x"), record(1, "b.py", 1, "x"), record(1, "c.py", 1, "x"),
        record(2, "d.py", 1, "y"), record(2, "e.py", 1, "y"), record(2, "f.py", 1, "y"),
    ];
    let clusters = freqt_mine(&records, &th);
    assert_eq!(clusters.len(), 2);
    let groups = fp_groups(&clusters);
    assert_eq!(groups.get(&1).unwrap().support, 3);
    assert_eq!(groups.get(&2).unwrap().support, 3);
}

#[test]
fn freqt_mine_aggregates_records_with_same_fingerprint() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 2;
    let records = vec![record(99, "a.py", 1, "x"), record(99, "b.py", 5, "y")];
    let clusters = freqt_mine(&records, &th);
    assert_eq!(clusters.len(), 1);
    assert_eq!(clusters[0].support, 2);
    assert_eq!(clusters[0].file_count, 2);
    assert_eq!(clusters[0].locations.len(), 2);
}

#[test]
fn freqt_mine_counts_unique_files_correctly() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 3;
    let records = vec![
        record(7, "a.py", 1, "x"), record(7, "a.py", 5, "x"), record(7, "a.py", 9, "x"),
    ];
    let clusters = freqt_mine(&records, &th);
    assert_eq!(clusters[0].support, 3);
    assert_eq!(clusters[0].file_count, 1);
}

#[test]
fn freqt_mine_min_support_is_occurrence_count_not_file_count() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 5;
    let records: Vec<SubtreeRecord> = (0..5).map(|i| record(7, "single.py", i, "x")).collect();
    let clusters = freqt_mine(&records, &th);
    assert_eq!(clusters.len(), 1, "5 occurrences in one file → surfaced");
    assert_eq!(clusters[0].file_count, 1);
}

#[test]
fn freqt_mine_preserves_locations() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 2;
    let records = vec![record(7, "a.py", 10, "x"), record(7, "b.py", 20, "x")];
    let clusters = freqt_mine(&records, &th);
    assert_eq!(clusters[0].locations.len(), 2);
}

#[test]
fn freqt_mine_is_deterministic_across_runs() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 2;
    let records = vec![record(7, "a.py", 1, "x"), record(7, "b.py", 1, "x")];
    let a = freqt_mine(&records, &th);
    let b = freqt_mine(&records, &th);
    assert_eq!(a.len(), b.len());
    assert_eq!(a[0].support, b[0].support);
    assert_eq!(a[0].locations, b[0].locations);
}

#[test]
fn freqt_mine_threshold_4_with_cluster_of_4_not_surfaced() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 5;
    let records: Vec<SubtreeRecord> = (0..4).map(|i| record(7, &format!("f{i}.py"), 1, "x")).collect();
    assert!(freqt_mine(&records, &th).is_empty());
}

#[test]
fn freqt_mine_threshold_3_with_cluster_of_3_surfaced() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 3;
    let records: Vec<SubtreeRecord> = (0..3).map(|i| record(7, &format!("f{i}.py"), 1, "x")).collect();
    assert_eq!(freqt_mine(&records, &th).len(), 1);
}

#[test]
fn freqt_mine_handles_thousand_distinct_fingerprints_below_threshold() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 2;
    let records: Vec<SubtreeRecord> = (0..1000).map(|i| record(i, "a.py", 1, "x")).collect();
    assert!(freqt_mine(&records, &th).is_empty());
}

#[test]
fn freqt_mine_handles_one_giant_cluster() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 5;
    let records: Vec<SubtreeRecord> = (0..1000).map(|i| record(42, &format!("f{i}.py"), 1, "x")).collect();
    let clusters = freqt_mine(&records, &th);
    assert_eq!(clusters.len(), 1);
    assert_eq!(clusters[0].support, 1000);
}

#[test]
fn freqt_mine_picks_representative_snippet_excluding_empty() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 3;
    let records = vec![
        record(7, "a.py", 1, ""),
        record(7, "b.py", 1, "media_type == X"),
        record(7, "c.py", 1, ""),
    ];
    let clusters = freqt_mine(&records, &th);
    assert!(!clusters[0].representative_snippet.is_empty());
    assert!(clusters[0].representative_snippet.contains("media_type"));
}

#[test]
fn freqt_mine_picks_longest_non_empty_snippet() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 3;
    let records = vec![
        record(7, "a.py", 1, "x"),
        record(7, "b.py", 1, "media_type == X.value"),
        record(7, "c.py", 1, "y == z"),
    ];
    let clusters = freqt_mine(&records, &th);
    assert_eq!(clusters[0].representative_snippet, "media_type == X.value");
}

#[test]
fn freqt_mine_does_not_collapse_distinct_fingerprints() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 1;
    let records = vec![record(1, "a.py", 1, "x"), record(2, "b.py", 1, "y")];
    let clusters = freqt_mine(&records, &th);
    assert_eq!(clusters.len(), 2);
}

#[test]
fn freqt_mine_does_not_panic_on_unicode_snippet() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 2;
    let records = vec![record(7, "a.py", 1, "λ == δ"), record(7, "b.py", 1, "δ == λ")];
    let clusters = freqt_mine(&records, &th);
    assert_eq!(clusters.len(), 1);
}

#[test]
fn freqt_mine_records_with_zero_named_node_count_handled() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 2;
    let mut r1 = record(7, "a.py", 1, "x");
    let mut r2 = record(7, "b.py", 1, "x");
    r1.named_node_count = 0;
    r2.named_node_count = 0;
    let _ = freqt_mine(&[r1, r2], &th);
}

#[test]
fn freqt_mine_locations_sorted_for_determinism() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 3;
    let records = vec![
        record(7, "z.py", 5, "x"),
        record(7, "a.py", 10, "x"),
        record(7, "m.py", 1, "x"),
    ];
    let clusters = freqt_mine(&records, &th);
    let locs = &clusters[0].locations;
    assert!(locs[0].0 <= locs[1].0);
    assert!(locs[1].0 <= locs[2].0);
}

#[test]
fn freqt_mine_with_default_thresholds_uses_five() {
    let records: Vec<SubtreeRecord> = (0..5).map(|i| record(7, &format!("f{i}.py"), 1, "x")).collect();
    let clusters = freqt_mine(&records, &t().audit);
    assert_eq!(clusters.len(), 1);
}

#[test]
fn freqt_mine_with_default_thresholds_rejects_four() {
    let records: Vec<SubtreeRecord> = (0..4).map(|i| record(7, &format!("f{i}.py"), 1, "x")).collect();
    assert!(freqt_mine(&records, &t().audit).is_empty());
}

#[test]
fn freqt_mine_against_shotgun_media_type_fixture_finds_cluster() {
    use pulse::audit::extract_subtrees_for_dir;
    use pulse::parse::Language;
    let dir = scenario_path("shotgun_media_type", "python");
    let records = extract_subtrees_for_dir(&dir, Language::Python, &t().audit);
    let clusters = freqt_mine(&records, &t().audit);
    assert!(!clusters.is_empty(), "should find at least one cluster");
    let media_clusters = clusters.iter().filter(|c| c.representative_snippet.contains("media_type")).count();
    assert!(media_clusters >= 1, "expected at least one media_type cluster, got {} total", clusters.len());
}

#[test]
fn freqt_mine_against_shotgun_cache_pattern_finds_cluster() {
    use pulse::audit::extract_subtrees_for_dir;
    use pulse::parse::Language;
    let dir = scenario_path("shotgun_cache_pattern", "python");
    let records = extract_subtrees_for_dir(&dir, Language::Python, &t().audit);
    let clusters = freqt_mine(&records, &t().audit);
    assert!(!clusters.is_empty());
}

#[test]
fn freqt_mine_against_shotgun_multi_class_init_finds_cluster() {
    use pulse::audit::extract_subtrees_for_dir;
    use pulse::parse::Language;
    let dir = scenario_path("shotgun_multi_class_init", "python");
    let records = extract_subtrees_for_dir(&dir, Language::Python, &t().audit);
    let clusters = freqt_mine(&records, &t().audit);
    assert!(!clusters.is_empty());
    assert!(clusters.iter().any(|c| c.file_count >= 5));
}

#[test]
fn freqt_mine_clean_small_yields_no_clusters() {
    use pulse::audit::extract_subtrees_for_dir;
    use pulse::parse::Language;
    let dir = scenario_path("clean_small", "python");
    let records = extract_subtrees_for_dir(&dir, Language::Python, &t().audit);
    let clusters = freqt_mine(&records, &t().audit);
    assert!(clusters.is_empty(), "got: {clusters:?}");
}

#[test]
fn freqt_mine_clean_disjoint_yields_no_clusters() {
    use pulse::audit::extract_subtrees_for_dir;
    use pulse::parse::Language;
    let dir = scenario_path("clean_disjoint", "python");
    let records = extract_subtrees_for_dir(&dir, Language::Python, &t().audit);
    let clusters = freqt_mine(&records, &t().audit);
    assert!(clusters.is_empty(), "disjoint files should not cluster, got: {clusters:?}");
}

#[test]
fn freqt_mine_returns_empty_vector_not_panic_on_empty() {
    let result = freqt_mine(&[], &t().audit);
    assert!(result.is_empty());
}

#[test]
fn freqt_mine_cluster_file_count_at_most_unique_records() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 3;
    let records: Vec<SubtreeRecord> = (0..10).map(|i| record(7, &format!("f{}.py", i % 3), 1, "x")).collect();
    let clusters = freqt_mine(&records, &th);
    assert_eq!(clusters[0].file_count, 3);
    assert_eq!(clusters[0].support, 10);
}
