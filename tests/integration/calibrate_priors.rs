use std::path::Path;

use pulse::audit::IgnoreFilter;
use pulse::calibrate::priors::{corpus_priors, PriorsBuilder, QUANTILE_PROBES};
use pulse::calibrate::stats::{gpd_fit, weighted_quantile, WeightedHist, GPD_MIN_TAIL_N};
use pulse::config::IgnoreMatcher;

use crate::common::t;

fn hist(samples: &[(u32, f64)]) -> WeightedHist {
    let mut h = WeightedHist::default();
    for (value, weight) in samples {
        h.observe(*value, *weight);
    }
    h
}

#[test]
fn weighted_quantile_respects_weights() {
    let h = hist(&[(1, 1.0), (10, 1.0), (100, 8.0)]);
    assert_eq!(weighted_quantile(&h, 0.05) as u32, 1);
    assert_eq!(weighted_quantile(&h, 0.15) as u32, 10);
    assert_eq!(weighted_quantile(&h, 0.5) as u32, 100);
    assert_eq!(weighted_quantile(&h, 0.99) as u32, 100);
}

#[test]
fn weighted_quantile_on_empty_hist_is_zero() {
    let h = WeightedHist::default();
    assert_eq!(weighted_quantile(&h, 0.5) as u32, 0);
}

fn synthetic_gpd(xi: f64, sigma: f64, n: u32) -> WeightedHist {
    let mut h = WeightedHist::default();
    for k in 0..n {
        let u = (f64::from(k) + 0.5) / f64::from(n);
        let excess = sigma / xi * ((1.0 - u).powf(-xi) - 1.0);
        h.observe(excess.round() as u32, 1.0);
    }
    h
}

#[test]
fn gpd_fit_recovers_synthetic_tail_shape() {
    let h = synthetic_gpd(0.3, 40.0, 2000);
    let fit = gpd_fit(&h, 0.0).expect("fit should converge");
    assert!((fit.xi - 0.3).abs() < 0.1, "xi={}", fit.xi);
    assert!((fit.sigma - 40.0).abs() < 6.0, "sigma={}", fit.sigma);
    assert!(fit.tail_n > 1900, "values rounding onto the threshold are excluded: {}", fit.tail_n);
}

#[test]
fn gpd_fit_requires_minimum_tail_size() {
    let h = synthetic_gpd(0.3, 40.0, GPD_MIN_TAIL_N as u32 - 1);
    assert!(gpd_fit(&h, 0.0).is_none());
}

#[test]
fn gpd_fit_rejects_degenerate_tails() {
    let h = hist(&[(50, 100.0)]);
    assert!(gpd_fit(&h, 0.0).is_none());
}

fn project_census(root: &Path) -> pulse::calibrate::Census {
    let matcher = IgnoreMatcher::from_patterns(&[]);
    let filter = IgnoreFilter::new(&matcher, root);
    pulse::calibrate::collect(root, &t(), &filter)
}

#[test]
fn builder_aggregates_census_into_language_priors() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/app.py"),
        "def alpha(x):\n    if x > 1:\n        return x\n    return 0\n\ndef beta(y):\n    return y\n",
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("tests")).unwrap();
    std::fs::write(dir.path().join("tests/test_app.py"), "def test_alpha():\n    assert True\n").unwrap();
    let mut builder = PriorsBuilder::default();
    builder.add_census(&project_census(dir.path()));
    let table = builder.build(false);
    let python = table.main.get("python").expect("python priors");
    let cc = python.metrics.get("cc").expect("cc prior");
    assert_eq!(cc.n, 2);
    assert_eq!(cc.quantiles.len(), QUANTILE_PROBES.len());
    let file_loc = python.metrics.get("file_loc").expect("file_loc prior");
    assert_eq!(file_loc.n, 1);
    assert!(table.tests.contains_key("python"), "test stratum populated");
    assert!(!table.main.is_empty() && !table.cpg_enabled);
}

#[test]
fn quantiles_are_monotone_in_probability() {
    let h = hist(&[(1, 3.0), (4, 2.0), (9, 1.0), (20, 0.5)]);
    let values: Vec<f64> = QUANTILE_PROBES.iter().map(|p| weighted_quantile(&h, *p)).collect();
    assert!(values.windows(2).all(|w| w[0] <= w[1]), "{values:?}");
}

#[test]
fn embedded_priors_load_without_error() {
    let table = corpus_priors();
    assert!(!table.cpg_enabled);
}

#[test]
fn streaming_accumulate_matches_collect_then_add() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/app.py"),
        "def alpha(x):\n    if x > 1:\n        for y in range(x):\n            return y\n    return 0\n\ndef beta(y):\n    return y\n",
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("tests")).unwrap();
    std::fs::write(dir.path().join("tests/test_app.py"), "def test_alpha():\n    assert True\n    assert 1 == 1\n")
        .unwrap();

    let matcher = IgnoreMatcher::from_patterns(&[]);
    let filter = IgnoreFilter::new(&matcher, dir.path());

    let mut by_collect = PriorsBuilder::default();
    by_collect.add_census(&pulse::calibrate::collect(dir.path(), &t(), &filter));
    let collected = serde_json::to_string(&by_collect.build(false)).unwrap();

    let mut by_stream = PriorsBuilder::default();
    let summary = pulse::calibrate::accumulate(dir.path(), &t(), &filter, &mut by_stream);
    let streamed = serde_json::to_string(&by_stream.build(false)).unwrap();

    assert_eq!(collected, streamed, "streaming accumulation must equal the materialized census path");
    assert_eq!(summary.main_files, 1);
    assert_eq!(summary.test_files, 1);
}
