use pulse_audit::discovery::RawCluster;
use pulse_audit::expression_filter::{is_expression_level, keep_expression_clusters};
use pulse_audit::walker::KindIndex;

#[test]
fn is_expression_level_returns_false_when_fingerprint_absent() {
    let index = KindIndex::new();
    assert!(!is_expression_level(0xDEAD_BEEF_u64, &index));
}

#[test]
fn is_expression_level_false_for_unknown_fp_in_nonempty_index() {
    let mut index = KindIndex::new();
    index.insert(1u64, vec!["string".to_string().into_boxed_str()]);
    assert!(!is_expression_level(999u64, &index));
}

#[test]
fn keep_expression_clusters_drops_clusters_with_unknown_fingerprint() {
    let index = KindIndex::new();
    let cluster = RawCluster {
        fingerprint: 0x1234_5678_u64,
        support: 3,
        file_count: 2,
        representative_snippet: "x = 1".to_string(),
        locations: Vec::new(),
    };
    let kept = keep_expression_clusters(vec![cluster], &index);
    assert!(kept.is_empty());
}
