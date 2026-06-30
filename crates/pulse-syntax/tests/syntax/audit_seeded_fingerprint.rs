use pulse_syntax::parse::{self, Language};
use pulse_syntax::walk::fingerprint::compute_subtree_fingerprint_seeded;

fn root_node_hash(src: &str, lang: Language, seed: u64) -> u64 {
    let tree = parse::parse_only(src, lang).unwrap();
    compute_subtree_fingerprint_seeded(tree.root_node(), seed)
}

#[test]
fn seeded_fingerprint_seeds_zero_through_ten_distinct() {
    let src = "def f(x):\n    if x == 1:\n        return x\n    return 0\n";
    let mut hashes = std::collections::HashSet::new();
    for s in 0..11 {
        hashes.insert(root_node_hash(src, Language::Python, s));
    }
    assert_eq!(hashes.len(), 11);
}

#[test]
fn seeded_fingerprint_lang_index_collision_rate_zero() {
    let src = "def f(x):\n    if x == 1:\n        return x\n    return 0\n";
    let mut seen = std::collections::HashSet::new();
    for lang_idx in 0_u64..22 {
        seen.insert(root_node_hash(src, Language::Python, lang_idx));
    }
    assert_eq!(seen.len(), 22);
}

#[test]
fn seeded_fingerprint_seed_one_distinct_from_seed_two() {
    let src = "x = 1\n";
    assert_ne!(root_node_hash(src, Language::Python, 1), root_node_hash(src, Language::Python, 2));
}

#[test]
fn seeded_fingerprint_seed_extreme_values_handled() {
    let src = "x = 1\n";
    let _ = root_node_hash(src, Language::Python, 0);
    let _ = root_node_hash(src, Language::Python, u64::MAX);
    let _ = root_node_hash(src, Language::Python, u64::MAX / 2);
}

#[test]
fn seeded_fingerprint_consistent_across_repeated_calls() {
    let src = "x = 1\n";
    let a = root_node_hash(src, Language::Python, 7);
    let b = root_node_hash(src, Language::Python, 7);
    assert_eq!(a, b);
}

#[test]
fn seeded_fingerprint_prepended_seed_changes_entire_hash() {
    let src = "x = 1\n";
    let a = root_node_hash(src, Language::Python, 0);
    let b = root_node_hash(src, Language::Python, 1);
    let c = root_node_hash(src, Language::Python, 2);
    assert_ne!(a, b);
    assert_ne!(b, c);
    assert_ne!(a, c);
}

#[test]
fn seeded_fingerprint_python_vs_javascript_indices_yield_distinct_hashes() {
    let src = "def f(x):\n    if x == 1:\n        return x\n    return 0\n";
    let py = root_node_hash(src, Language::Python, Language::Python as u64);
    let js = root_node_hash(src, Language::Python, Language::JavaScript as u64);
    assert_ne!(py, js);
}
