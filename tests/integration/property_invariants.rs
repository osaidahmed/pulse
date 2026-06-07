use pulse::parse::{parse_and_walk, Language};
use pulse::walk::FunctionMetrics;

fn first_fn(src: &str) -> FunctionMetrics {
    parse_and_walk(src, Language::Python).expect("parse").functions.into_iter().next().expect("a function")
}

#[test]
fn analysis_is_deterministic_across_runs() {
    let src = "def f(x):\n    if x > 0:\n        return 1\n    for i in range(x):\n        if i:\n            return i\n    return -1\n";
    let a = first_fn(src);
    let b = first_fn(src);
    assert_eq!(a.cc, b.cc);
    assert_eq!(a.cognitive_complexity, b.cognitive_complexity);
    assert_eq!(a.structural_hash, b.structural_hash);
    assert_eq!(a.skeleton_hash, b.skeleton_hash);
}

#[test]
fn structural_fingerprint_collapses_identifiers_and_literals() {
    let a = first_fn("def f(alpha):\n    if alpha > 0:\n        return 1\n    return 2\n");
    let b = first_fn("def g(beta):\n    if beta > 999:\n        return 7\n    return 8\n");
    assert_eq!(a.structural_hash, b.structural_hash, "structural hash must collapse identifier and literal values");
}

#[test]
fn structural_fingerprint_varies_with_control_flow() {
    let one = first_fn("def f(x):\n    if x > 0:\n        return 1\n    return 2\n");
    let two = first_fn("def f(x):\n    if x > 0:\n        return 1\n    if x < 0:\n        return 3\n    return 2\n");
    assert_ne!(one.structural_hash, two.structural_hash, "an added branch must change the structural hash");
}

#[test]
fn complexity_is_invariant_under_reformatting() {
    let compact = "def f(x):\n    if x>0:\n        return 1\n    return 2\n";
    let spread = "def f(x):\n\n    if x > 0:\n\n        return 1\n\n    return 2\n";
    let a = first_fn(compact);
    let b = first_fn(spread);
    assert_eq!(a.cc, b.cc, "cc must not depend on whitespace");
    assert_eq!(a.cognitive_complexity, b.cognitive_complexity, "cogc must not depend on whitespace");
    assert_eq!(a.structural_hash, b.structural_hash, "structural hash must not depend on whitespace");
}

#[test]
fn cc_increases_when_a_branch_is_added() {
    let fewer = first_fn("def f(x):\n    if x > 0:\n        return 1\n    return 2\n");
    let more =
        first_fn("def f(x):\n    if x > 0:\n        return 1\n    elif x < 0:\n        return 3\n    return 2\n");
    assert!(more.cc > fewer.cc, "adding a branch must increase cc ({} vs {})", more.cc, fewer.cc);
}
