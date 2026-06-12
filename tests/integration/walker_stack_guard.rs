use std::path::PathBuf;

use pulse::audit::record_extraction;
use pulse::parse::Language;

use crate::common::t;

const DEGENERATE_DEPTH: usize = 30_000;
const MODERATE_DEPTH: usize = 1_500;
const TINY_STACK_BYTES: usize = 512 * 1024;

fn nested_fixture(dir: &std::path::Path, depth: usize) -> Vec<(PathBuf, Language)> {
    let source = format!("x = {}1{}\n", "(".repeat(depth), ")".repeat(depth));
    let path = dir.join("deep.py");
    std::fs::write(&path, source).unwrap();
    vec![(path, Language::Python)]
}

fn features_on_small_stack(typed: Vec<(PathBuf, Language)>) -> usize {
    let thresholds = t().audit;
    std::thread::Builder::new()
        .stack_size(TINY_STACK_BYTES)
        .spawn(move || record_extraction::corpus_bundle(&typed, &thresholds).features.len())
        .unwrap()
        .join()
        .expect("extraction must not crash the calling thread")
}

#[test]
fn degenerate_depth_does_not_crash_small_caller_stacks() {
    let dir = tempfile::tempdir().unwrap();
    let features = features_on_small_stack(nested_fixture(dir.path(), DEGENERATE_DEPTH));
    assert!(features <= 1, "degenerate trees either measure or fail open, never abort");
}

#[test]
fn moderate_depth_is_still_measured_not_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let features = features_on_small_stack(nested_fixture(dir.path(), MODERATE_DEPTH));
    assert_eq!(features, 1, "fail-open must not swallow merely-deep files");
}
