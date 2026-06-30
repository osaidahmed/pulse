#![allow(clippy::float_cmp)]

use std::path::Path;

use pulse_calibrate::priors::{corpus_priors, MetricPrior, PriorsBuilder, QUANTILE_PROBES};
use pulse_calibrate::stats::{count_quantile, weighted_quantile, WeightedHist};
use pulse_config::IgnoreMatcher;
use pulse_corpus::IgnoreFilter;

use crate::common::t;

fn project_census(root: &Path) -> pulse_calibrate::Census {
    let matcher = IgnoreMatcher::from_patterns(&[]);
    let filter = IgnoreFilter::new(&matcher, root);
    pulse_calibrate::collect(root, &t(), &filter)
}

#[test]
fn count_quantile_measures_entities_not_loc_mass() {
    let mut h = WeightedHist::default();
    for _ in 0..90 {
        h.observe(100, 100.0);
    }
    for _ in 0..10 {
        h.observe(2000, 2000.0);
    }
    assert!(count_quantile(&h, 0.90) <= 200.0, "90% of FILES are the small ones: {}", count_quantile(&h, 0.90));
    assert!(weighted_quantile(&h, 0.90) >= 1000.0, "but 90% of CODE LINES live in the big ones — the old bug");
}

#[test]
fn metric_prior_quantile_interpolates_and_clamps() {
    let mp = MetricPrior { n: 100, quantiles: vec![(0.5, 4.0), (0.75, 8.0), (0.95, 16.0)], cdf: Vec::new() };
    assert_eq!(mp.quantile(0.75), 8.0);
    assert_eq!(mp.quantile(0.4), 4.0, "below the first probe clamps to it");
    assert_eq!(mp.quantile(0.99), 16.0, "above the last probe clamps to it");
    let mid = mp.quantile(0.85);
    assert!(mid > 8.0 && mid < 16.0, "interpolated: {mid}");
}

#[test]
fn cdf_at_counts_mass_at_or_below_value() {
    let mp = MetricPrior { n: 10, quantiles: vec![], cdf: vec![(1, 5), (3, 3), (8, 2)] };
    assert_eq!(mp.cdf_at(0), 0.0);
    assert_eq!(mp.cdf_at(1), 0.5);
    assert_eq!(mp.cdf_at(2), 0.5, "no bin at 2 so the mass is unchanged from 1");
    assert_eq!(mp.cdf_at(3), 0.8);
    assert_eq!(mp.cdf_at(100), 1.0);
    assert_eq!(mp.firing_rate(1), 0.5);
    assert_eq!(mp.percentile_of(3), 80.0);
}

#[test]
fn builder_produces_count_quantiles_per_language() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/app.py"),
        "def alpha(x):\n    if x > 1:\n        return x\n    return 0\n\ndef beta(y):\n    return y\n",
    )
    .unwrap();
    let mut builder = PriorsBuilder::default();
    builder.add_census(&project_census(dir.path()));
    let table = builder.build(false);
    let cc = table.main.get("python").expect("python").metrics.get("cc").expect("cc");
    assert_eq!(cc.n, 2);
    assert_eq!(cc.quantiles.len(), QUANTILE_PROBES.len());
}

#[test]
fn streaming_accumulate_matches_collect_then_add() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/app.py"), "def alpha(x):\n    if x > 1:\n        return x\n    return 0\n")
        .unwrap();
    let matcher = IgnoreMatcher::from_patterns(&[]);
    let filter = IgnoreFilter::new(&matcher, dir.path());

    let mut by_collect = PriorsBuilder::default();
    by_collect.add_census(&pulse_calibrate::collect(dir.path(), &t(), &filter));
    let collected = serde_json::to_string(&by_collect.build(false)).unwrap();

    let mut by_stream = PriorsBuilder::default();
    pulse_calibrate::accumulate(dir.path(), &t(), &filter, &mut by_stream);
    let streamed = serde_json::to_string(&by_stream.build(false)).unwrap();

    assert_eq!(collected, streamed);
}

#[test]
fn embedded_priors_load_and_cover_all_languages() {
    let table = corpus_priors();
    assert!(!table.cpg_enabled);
    assert!(table.main.len() >= 20, "corpus spans the supported languages: {}", table.main.len());
    let java = table.main.get("java").expect("java priors");
    assert!(java.metrics.contains_key("cc") && java.metrics.contains_key("file_loc"));
}
