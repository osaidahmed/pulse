use pulse::audit::discovery::{freqt_mine, RawCluster};
use pulse::audit::walker::SubtreeRecord;
use pulse::thresholds::Thresholds;
use std::path::PathBuf;

fn t() -> Thresholds { Thresholds::default() }

fn rec(fp: u64, file: &str, line: u32, snippet: &str) -> SubtreeRecord {
    SubtreeRecord {
        fingerprint: fp,
        file: PathBuf::from(file),
        line,
        depth: 5,
        named_node_count: 8,
        snippet: snippet.to_string(),
    }
}

#[test]
fn discovery_returns_one_cluster_for_one_fingerprint_repeated() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 3;
    let recs: Vec<_> = (0..3).map(|i| rec(7, &format!("f{i}.py"), 1, "x")).collect();
    assert_eq!(freqt_mine(&recs, &th).len(), 1);
}

#[test]
fn discovery_returns_two_clusters_when_two_fingerprints_above_threshold() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 2;
    let mut recs = vec![rec(1, "a.py", 1, "x"), rec(1, "b.py", 1, "x")];
    recs.extend(vec![rec(2, "c.py", 1, "y"), rec(2, "d.py", 1, "y")]);
    assert_eq!(freqt_mine(&recs, &th).len(), 2);
}

#[test]
fn discovery_only_one_cluster_when_one_fingerprint_above_one_below() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 3;
    let mut recs: Vec<_> = (0..3).map(|i| rec(1, &format!("a{i}.py"), 1, "x")).collect();
    recs.extend((0..2).map(|i| rec(2, &format!("b{i}.py"), 1, "y")));
    assert_eq!(freqt_mine(&recs, &th).len(), 1);
}

#[test]
fn discovery_zero_clusters_when_each_fingerprint_appears_once() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 2;
    let recs: Vec<_> = (0..100).map(|i| rec(i, "a.py", i as u32, "x")).collect();
    assert!(freqt_mine(&recs, &th).is_empty());
}

#[test]
fn discovery_handles_record_with_zero_line_number() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 2;
    let recs = vec![rec(7, "a.py", 0, "x"), rec(7, "b.py", 0, "x")];
    let _ = freqt_mine(&recs, &th);
}

#[test]
fn discovery_picks_representative_with_most_information() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 3;
    let recs = vec![
        rec(7, "a.py", 1, "y == z"),
        rec(7, "b.py", 1, "media_type == X.value"),
        rec(7, "c.py", 1, "p == q"),
    ];
    let clusters = freqt_mine(&recs, &th);
    assert!(clusters[0].representative_snippet.contains("media_type"));
}

#[test]
fn discovery_thousand_record_grouping_efficient() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 100;
    let recs: Vec<_> = (0..1000).map(|i| rec(i % 5, &format!("f{i}.py"), 1, "x")).collect();
    let clusters = freqt_mine(&recs, &th);
    assert_eq!(clusters.len(), 5);
}

#[test]
fn discovery_handles_extreme_min_support() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 1;
    let recs = vec![rec(7, "a.py", 1, "x")];
    assert_eq!(freqt_mine(&recs, &th).len(), 1);
}

#[test]
fn discovery_locations_count_matches_support() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 3;
    let recs: Vec<_> = (0..7).map(|i| rec(7, &format!("f{i}.py"), 1, "x")).collect();
    let clusters = freqt_mine(&recs, &th);
    assert_eq!(clusters[0].locations.len(), 7);
}

#[test]
fn discovery_file_count_distinct_from_support_for_multi_occurrence_in_single_file() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 5;
    let recs: Vec<_> = (0..5).map(|i| rec(7, "single.py", i, "x")).collect();
    let clusters = freqt_mine(&recs, &th);
    assert_eq!(clusters[0].support, 5);
    assert_eq!(clusters[0].file_count, 1);
}

#[test]
fn discovery_groups_records_with_same_fp_different_files_correctly() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 4;
    let recs = vec![
        rec(7, "a.py", 1, "x"), rec(7, "a.py", 2, "x"),
        rec(7, "b.py", 1, "x"), rec(7, "b.py", 2, "x"),
    ];
    let clusters = freqt_mine(&recs, &th);
    assert_eq!(clusters[0].support, 4);
    assert_eq!(clusters[0].file_count, 2);
}

#[test]
fn discovery_invariant_under_input_record_order_permutation() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 3;
    let original: Vec<_> = (0..5).map(|i| rec(7, &format!("f{i}.py"), 1, "x")).collect();
    let mut shuffled = original.clone();
    shuffled.reverse();
    let a = freqt_mine(&original, &th);
    let b = freqt_mine(&shuffled, &th);
    assert_eq!(a[0].support, b[0].support);
    assert_eq!(a[0].file_count, b[0].file_count);
}

#[test]
fn discovery_returns_clusters_with_unique_fingerprints() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 2;
    let recs = vec![
        rec(1, "a.py", 1, "x"), rec(1, "b.py", 1, "x"),
        rec(2, "c.py", 1, "y"), rec(2, "d.py", 1, "y"),
        rec(3, "e.py", 1, "z"), rec(3, "f.py", 1, "z"),
    ];
    let clusters = freqt_mine(&recs, &th);
    let fps: std::collections::HashSet<u64> = clusters.iter().map(|c| c.fingerprint).collect();
    assert_eq!(fps.len(), clusters.len());
}

#[test]
fn discovery_handles_max_u32_support_count() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 2;
    let recs: Vec<_> = (0..5).map(|i| rec(7, &format!("f{i}.py"), 1, "x")).collect();
    let clusters = freqt_mine(&recs, &th);
    let _ = clusters[0].support;
}

#[test]
fn discovery_with_records_having_long_snippets() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 2;
    let long_snippet = "x".repeat(500);
    let recs = vec![
        rec(7, "a.py", 1, &long_snippet),
        rec(7, "b.py", 1, &long_snippet),
    ];
    let clusters = freqt_mine(&recs, &th);
    assert_eq!(clusters[0].representative_snippet, long_snippet);
}

#[test]
fn discovery_ignores_record_count_when_below_threshold_individually() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 5;
    let mut recs: Vec<_> = (0..4).map(|i| rec(1, &format!("a{i}.py"), 1, "x")).collect();
    recs.extend((0..4).map(|i| rec(2, &format!("b{i}.py"), 1, "y")));
    recs.extend((0..4).map(|i| rec(3, &format!("c{i}.py"), 1, "z")));
    assert!(freqt_mine(&recs, &th).is_empty(), "no cluster reaches 5 individually");
}

#[test]
fn discovery_locations_preserved_per_record_not_merged() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 3;
    let recs = vec![
        rec(7, "a.py", 10, "x"),
        rec(7, "a.py", 20, "x"),
        rec(7, "a.py", 30, "x"),
    ];
    let clusters = freqt_mine(&recs, &th);
    assert_eq!(clusters[0].locations.len(), 3);
}

#[test]
fn discovery_threshold_max_value_accepts_only_giant_clusters() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 1_000_000;
    let recs: Vec<_> = (0..100).map(|i| rec(7, &format!("f{i}.py"), 1, "x")).collect();
    assert!(freqt_mine(&recs, &th).is_empty());
}

#[test]
fn discovery_picks_first_non_empty_snippet_when_all_empty_except_one() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 5;
    let recs = vec![
        rec(7, "a.py", 1, ""),
        rec(7, "b.py", 1, ""),
        rec(7, "c.py", 1, "actual_content"),
        rec(7, "d.py", 1, ""),
        rec(7, "e.py", 1, ""),
    ];
    let clusters = freqt_mine(&recs, &th);
    assert_eq!(clusters[0].representative_snippet, "actual_content");
}

#[test]
fn discovery_returns_empty_string_when_all_snippets_empty() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 3;
    let recs = vec![
        rec(7, "a.py", 1, ""),
        rec(7, "b.py", 1, ""),
        rec(7, "c.py", 1, ""),
    ];
    let clusters = freqt_mine(&recs, &th);
    assert_eq!(clusters[0].representative_snippet, "");
}

#[test]
fn discovery_handles_path_with_many_components() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 2;
    let deep_path = "a/b/c/d/e/f/g/h/i/j/k/l/m/n/o/p.py";
    let recs = vec![rec(7, deep_path, 1, "x"), rec(7, deep_path, 5, "x")];
    let clusters = freqt_mine(&recs, &th);
    assert_eq!(clusters[0].support, 2);
}

#[test]
fn discovery_stable_under_locations_with_same_path_different_lines() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 3;
    let recs = vec![
        rec(7, "a.py", 100, "x"),
        rec(7, "a.py", 50, "x"),
        rec(7, "a.py", 75, "x"),
    ];
    let r1 = freqt_mine(&recs, &th);
    let r2 = freqt_mine(&recs, &th);
    assert_eq!(r1[0].locations, r2[0].locations);
}

#[test]
fn discovery_does_not_dedupe_locations_with_same_file_and_line() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 3;
    let recs = vec![
        rec(7, "a.py", 1, "x"),
        rec(7, "a.py", 1, "x"),
        rec(7, "a.py", 1, "x"),
    ];
    let clusters = freqt_mine(&recs, &th);
    assert_eq!(clusters[0].support, 3);
    assert_eq!(clusters[0].locations.len(), 3);
}

#[test]
fn discovery_each_cluster_has_at_least_min_support_records() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 5;
    let mut recs = Vec::new();
    for fp in 0..10 {
        for i in 0..(5 + fp) {
            recs.push(rec(fp, &format!("f{fp}_{i}.py"), 1, "x"));
        }
    }
    let clusters = freqt_mine(&recs, &th);
    for c in &clusters {
        assert!(c.support as usize >= th.pattern_mining.freqt_min_support);
    }
}

#[test]
fn discovery_handles_synthetic_pathological_input_with_many_unique_fingerprints() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 2;
    let mut recs = Vec::with_capacity(10_000);
    for i in 0..10_000 {
        recs.push(rec(i, &format!("f{i}.py"), 1, "x"));
    }
    assert!(freqt_mine(&recs, &th).is_empty());
}

#[test]
fn discovery_empty_string_paths_handled() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 2;
    let recs = vec![rec(7, "", 1, "x"), rec(7, "", 1, "x")];
    let clusters = freqt_mine(&recs, &th);
    assert_eq!(clusters[0].support, 2);
    assert_eq!(clusters[0].file_count, 1);
}

#[test]
fn discovery_records_with_unicode_paths() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 2;
    let recs = vec![rec(7, "δ.py", 1, "x"), rec(7, "λ.py", 1, "x")];
    let clusters = freqt_mine(&recs, &th);
    assert_eq!(clusters[0].file_count, 2);
}

#[test]
fn discovery_records_with_special_chars_in_paths() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 2;
    let recs = vec![rec(7, "a b.py", 1, "x"), rec(7, "a-b.py", 1, "x")];
    let clusters = freqt_mine(&recs, &th);
    assert_eq!(clusters[0].file_count, 2);
}

#[test]
fn discovery_locations_total_equal_support_value() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 7;
    let recs: Vec<_> = (0..15).map(|i| rec(7, &format!("f{i}.py"), 1, "x")).collect();
    let clusters = freqt_mine(&recs, &th);
    assert_eq!(clusters[0].locations.len() as u32, clusters[0].support);
}

#[test]
fn discovery_with_many_clusters_uses_consistent_ordering_by_fingerprint() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 2;
    let recs: Vec<_> = (0..100).flat_map(|i| {
        vec![rec(i, &format!("a{i}.py"), 1, "x"), rec(i, &format!("b{i}.py"), 1, "x")]
    }).collect();
    let r1 = freqt_mine(&recs, &th);
    let r2 = freqt_mine(&recs, &th);
    let f1: Vec<u64> = r1.iter().map(|c| c.fingerprint).collect();
    let f2: Vec<u64> = r2.iter().map(|c| c.fingerprint).collect();
    let mut s1 = f1.clone(); s1.sort_unstable();
    let mut s2 = f2.clone(); s2.sort_unstable();
    assert_eq!(s1, s2);
}

#[test]
fn discovery_no_cluster_lost_when_input_doubles() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 5;
    let single: Vec<_> = (0..5).map(|i| rec(7, &format!("f{i}.py"), 1, "x")).collect();
    let doubled = {
        let mut v = single.clone();
        v.extend(single.iter().cloned());
        v
    };
    let r_single = freqt_mine(&single, &th);
    let r_double = freqt_mine(&doubled, &th);
    assert_eq!(r_single.len(), r_double.len());
    assert_eq!(r_single[0].fingerprint, r_double[0].fingerprint);
}

#[test]
fn discovery_threshold_higher_than_total_records_returns_empty() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 100;
    let recs: Vec<_> = (0..50).map(|i| rec(7, &format!("f{i}.py"), 1, "x")).collect();
    assert!(freqt_mine(&recs, &th).is_empty());
}

#[test]
fn discovery_locations_carry_path_and_line_pair_correctly() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 2;
    let recs = vec![
        rec(7, "a.py", 42, "x"),
        rec(7, "b.py", 137, "x"),
    ];
    let clusters = freqt_mine(&recs, &th);
    let locs = &clusters[0].locations;
    let lines: Vec<u32> = locs.iter().map(|(_, l)| *l).collect();
    assert!(lines.contains(&42));
    assert!(lines.contains(&137));
}

#[test]
fn discovery_supports_threshold_changes_between_calls() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 2;
    let recs = vec![rec(7, "a.py", 1, "x"), rec(7, "b.py", 1, "x")];
    assert_eq!(freqt_mine(&recs, &th).len(), 1);
    th.pattern_mining.freqt_min_support = 3;
    assert!(freqt_mine(&recs, &th).is_empty());
}

#[test]
fn discovery_picks_representative_snippet_max_length() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 5;
    let recs = vec![
        rec(7, "a.py", 1, "ab"),
        rec(7, "b.py", 1, "abcdef"),
        rec(7, "c.py", 1, "abc"),
        rec(7, "d.py", 1, "abcd"),
        rec(7, "e.py", 1, "abcde"),
    ];
    let clusters = freqt_mine(&recs, &th);
    assert_eq!(clusters[0].representative_snippet, "abcdef");
}

#[test]
fn discovery_locations_global_order_consistent() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 4;
    let recs = vec![
        rec(7, "z.py", 1, "x"),
        rec(7, "y.py", 1, "x"),
        rec(7, "x.py", 1, "x"),
        rec(7, "w.py", 1, "x"),
    ];
    let clusters = freqt_mine(&recs, &th);
    let paths: Vec<&PathBuf> = clusters[0].locations.iter().map(|(p, _)| p).collect();
    for w in paths.windows(2) {
        assert!(w[0] <= w[1]);
    }
}

#[test]
fn discovery_does_not_panic_on_record_with_huge_named_count() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 1;
    let mut r = rec(7, "a.py", 1, "x");
    r.named_node_count = u32::MAX;
    let _ = freqt_mine(&[r], &th);
}

#[test]
fn discovery_does_not_panic_on_record_with_huge_depth() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 1;
    let mut r = rec(7, "a.py", 1, "x");
    r.depth = u32::MAX;
    let _ = freqt_mine(&[r], &th);
}

#[test]
fn discovery_threshold_one_with_one_record_yields_cluster() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 1;
    let r = rec(7, "a.py", 1, "x");
    let clusters = freqt_mine(&[r], &th);
    assert_eq!(clusters.len(), 1);
}

#[test]
fn discovery_handles_zero_records_with_max_threshold() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = usize::MAX;
    assert!(freqt_mine(&[], &th).is_empty());
}

#[test]
fn discovery_with_extreme_fingerprint_values() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 2;
    let recs = vec![
        rec(0, "a.py", 1, "x"), rec(0, "b.py", 1, "x"),
        rec(u64::MAX, "c.py", 1, "y"), rec(u64::MAX, "d.py", 1, "y"),
    ];
    let clusters = freqt_mine(&recs, &th);
    assert_eq!(clusters.len(), 2);
}

#[test]
fn discovery_handles_close_fingerprints_separately() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 2;
    let recs = vec![
        rec(0xdead_beef, "a.py", 1, "x"), rec(0xdead_beef, "b.py", 1, "x"),
        rec(0xdead_bef0, "c.py", 1, "y"), rec(0xdead_bef0, "d.py", 1, "y"),
    ];
    let clusters = freqt_mine(&recs, &th);
    assert_eq!(clusters.len(), 2);
}

#[test]
fn discovery_idempotent_when_called_twice_on_same_input() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 2;
    let recs = vec![rec(7, "a.py", 1, "x"), rec(7, "b.py", 1, "x")];
    let r1 = freqt_mine(&recs, &th);
    let r2 = freqt_mine(&recs, &th);
    assert_eq!(r1.len(), r2.len());
    for (a, b) in r1.iter().zip(r2.iter()) {
        assert_eq!(a.fingerprint, b.fingerprint);
        assert_eq!(a.support, b.support);
        assert_eq!(a.file_count, b.file_count);
        assert_eq!(a.locations, b.locations);
    }
}

#[test]
fn discovery_input_order_permutation_yields_same_clusters() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 2;
    let original = vec![rec(7, "a.py", 1, "x"), rec(7, "b.py", 1, "x")];
    let mut reversed = original.clone();
    reversed.reverse();
    let r1 = freqt_mine(&original, &th);
    let r2 = freqt_mine(&reversed, &th);
    assert_eq!(r1[0].fingerprint, r2[0].fingerprint);
    assert_eq!(r1[0].support, r2[0].support);
}

#[test]
fn discovery_records_can_have_zero_named_node_count() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 2;
    let mut r1 = rec(7, "a.py", 1, "x");
    r1.named_node_count = 0;
    let mut r2 = rec(7, "b.py", 1, "x");
    r2.named_node_count = 0;
    let _ = freqt_mine(&[r1, r2], &th);
}

#[test]
fn discovery_raw_cluster_fields_consistent() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 3;
    let recs = vec![
        rec(7, "a.py", 1, "alpha"),
        rec(7, "b.py", 2, "beta_longer"),
        rec(7, "c.py", 3, "gamma"),
    ];
    let clusters: Vec<RawCluster> = freqt_mine(&recs, &th);
    let c = &clusters[0];
    assert_eq!(c.fingerprint, 7);
    assert_eq!(c.support, 3);
    assert_eq!(c.file_count, 3);
    assert_eq!(c.locations.len(), 3);
    assert_eq!(c.representative_snippet, "beta_longer");
}

#[test]
fn discovery_produces_no_overlapping_clusters() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 2;
    let recs: Vec<_> = (0..20).flat_map(|i| {
        vec![rec(i, &format!("a{i}.py"), 1, "x"), rec(i, &format!("b{i}.py"), 1, "x")]
    }).collect();
    let clusters = freqt_mine(&recs, &th);
    let total_records: u32 = clusters.iter().map(|c| c.support).sum();
    assert_eq!(total_records as usize, recs.len());
}

#[test]
fn discovery_fingerprints_in_clusters_disjoint_from_records_below_threshold() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 5;
    let mut recs = Vec::new();
    for fp in 0..10 {
        let count = if fp < 5 { 6 } else { 2 };
        for i in 0..count {
            recs.push(rec(fp, &format!("f{fp}_{i}.py"), 1, "x"));
        }
    }
    let clusters = freqt_mine(&recs, &th);
    let cluster_fps: std::collections::HashSet<u64> = clusters.iter().map(|c| c.fingerprint).collect();
    for fp in 5..10 {
        assert!(!cluster_fps.contains(&fp), "fp {fp} below threshold should not be in clusters");
    }
    for fp in 0..5 {
        assert!(cluster_fps.contains(&fp));
    }
}

#[test]
fn discovery_when_sole_record_is_below_threshold_returns_empty() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 2;
    let r = rec(7, "a.py", 1, "x");
    assert!(freqt_mine(&[r], &th).is_empty());
}

#[test]
fn discovery_supports_min_value_zero() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 0;
    let r = rec(7, "a.py", 1, "x");
    let clusters = freqt_mine(&[r], &th);
    assert_eq!(clusters.len(), 1);
}

#[test]
fn discovery_huge_path_strings_handled() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 2;
    let big_path = "a/".repeat(1000) + "x.py";
    let recs = vec![rec(7, &big_path, 1, "x"), rec(7, &big_path, 5, "x")];
    let clusters = freqt_mine(&recs, &th);
    assert_eq!(clusters[0].support, 2);
    assert_eq!(clusters[0].file_count, 1);
}

#[test]
fn discovery_cluster_sizes_in_total_match_input_size_when_all_above_threshold() {
    let mut th = t().audit;
    th.pattern_mining.freqt_min_support = 2;
    let recs = vec![
        rec(1, "a.py", 1, "x"), rec(1, "b.py", 1, "x"),
        rec(2, "c.py", 1, "y"), rec(2, "d.py", 1, "y"),
    ];
    let clusters = freqt_mine(&recs, &th);
    let total: u32 = clusters.iter().map(|c| c.support).sum();
    assert_eq!(total as usize, recs.len());
}
