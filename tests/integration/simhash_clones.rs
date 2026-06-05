use pulse::parse::Language;
use pulse::walk::simhash::simhash_of;

fn hamming(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

#[test]
fn simhash_is_deterministic() {
    let src = "def f(x):\n    return x + 1\n";
    assert_eq!(simhash_of(src, Language::Python), simhash_of(src, Language::Python));
}

#[test]
fn simhash_is_rename_invariant() {
    let a = simhash_of("def f(x):\n    return x + 1\n", Language::Python).unwrap();
    let b = simhash_of("def g(y):\n    return y + 1\n", Language::Python).unwrap();
    assert_eq!(hamming(a, b), 0, "renamed identifiers share node kinds: {a:x} vs {b:x}");
}

#[test]
fn simhash_small_edit_small_distance() {
    let a = simhash_of("def f(x):\n    return x + 1\n", Language::Python).unwrap();
    let b = simhash_of("def f(x):\n    return x + 1 + 2\n", Language::Python).unwrap();
    let d = hamming(a, b);
    assert!(d > 0 && d <= 12, "small edit should be a small distance, got {d}");
}

#[test]
fn simhash_different_shapes_diverge() {
    let a = simhash_of("def f(x):\n    return x\n", Language::Python).unwrap();
    let b = simhash_of(
        "def f(x):\n    for i in x:\n        if i > 0:\n            print(i)\n    return 0\n",
        Language::Python,
    )
    .unwrap();
    assert!(hamming(a, b) >= 5, "different control-flow shapes should diverge: {a:x} vs {b:x}");
}

#[test]
fn simhash_rust_rename_invariant() {
    let a = simhash_of("fn f(x: i32) -> i32 {\n    x + 1\n}\n", Language::Rust).unwrap();
    let b = simhash_of("fn g(y: i32) -> i32 {\n    y + 1\n}\n", Language::Rust).unwrap();
    assert_eq!(hamming(a, b), 0, "{a:x} vs {b:x}");
}
