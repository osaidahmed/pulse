#![allow(clippy::float_cmp)]

use std::collections::{BTreeMap, BTreeSet};

use pulse::calibrate::estimator::{estimate, estimate_languages, EstimatorConfig};
use pulse::calibrate::priors::{corpus_priors, LanguagePriors, MetricPrior};

use crate::common::t;

fn mp(quantiles: &[(f64, f64)]) -> MetricPrior {
    MetricPrior { n: 900, quantiles: quantiles.to_vec(), cdf: Vec::new() }
}

fn corpus(lang: &str, metrics: &[(&str, MetricPrior)]) -> BTreeMap<String, LanguagePriors> {
    let mut lp = LanguagePriors::default();
    for (k, v) in metrics {
        lp.metrics.insert((*k).to_string(), v.clone());
    }
    let mut out = BTreeMap::new();
    out.insert(lang.to_string(), lp);
    out
}

fn langset(lang: &str) -> BTreeSet<String> {
    let mut s = BTreeSet::new();
    s.insert(lang.to_string());
    s
}

fn cfg() -> EstimatorConfig {
    EstimatorConfig::default()
}

#[test]
fn editorial_default_is_the_floor_when_corpus_sits_below() {
    let c = corpus("python", &[("cc", mp(&[(0.5, 2.0), (0.75, 5.0), (0.95, 14.0)]))]);
    let out = estimate_languages(&langset("python"), &c, &cfg());
    let cc = out["python"]["cc"];
    assert_eq!(cc.warning, 9.0, "editorial cc_warning=9 is the floor; corpus p75=5 does not tighten it");
    assert!(!cc.warning_loosened());
}

#[test]
fn corpus_loosens_per_language_only_where_it_exceeds_the_editorial_bar() {
    let c = corpus("c", &[("cc", mp(&[(0.5, 8.0), (0.75, 15.0), (0.95, 40.0)]))]);
    let out = estimate_languages(&langset("c"), &c, &cfg());
    let cc = out["c"]["cc"];
    assert_eq!(cc.warning, 15.0, "C complexity legitimately runs higher, so the warning loosens to the corpus p75");
    assert!(cc.warning_loosened());
    assert_eq!(cc.alert, 40.0, "alert loosens to corpus p95");
}

#[test]
fn alert_never_drops_below_the_editorial_alert_floor() {
    let c = corpus("python", &[("file_loc", mp(&[(0.75, 200.0), (0.95, 400.0)]))]);
    let out = estimate_languages(&langset("python"), &c, &cfg());
    let fl = out["python"]["file_loc"];
    assert_eq!(fl.warning, 500.0, "editorial floor 500");
    assert_eq!(fl.alert, 700.0, "editorial alert floor 700 — '1500 is still a smell' encoded as a tighten-only floor");
}

#[test]
fn size_metric_is_not_self_inflated() {
    let c = corpus("c", &[("file_loc", mp(&[(0.75, 800.0), (0.95, 1500.0)]))]);
    let out = estimate_languages(&langset("c"), &c, &cfg());
    let fl = out["c"]["file_loc"];
    assert_eq!(fl.warning, 800.0, "C file warning at its real per-file p75, not a self-weighted 2508");
    assert_eq!(fl.alert, 1500.0, "and alert at its real per-file p95 — the literal user goal");
}

#[test]
fn estimate_against_the_real_corpus_respects_floors() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/app.py"),
        "def alpha(x):\n    if x > 1:\n        return x\n    return 0\n\ndef beta(y):\n    return y + 1\n",
    )
    .unwrap();
    let matcher = pulse::config::IgnoreMatcher::from_patterns(&[]);
    let filter = pulse::audit::IgnoreFilter::new(&matcher, dir.path());
    let census = pulse::calibrate::collect(dir.path(), &t(), &filter);
    let out = estimate(&census, corpus_priors(), &cfg());
    let py = out.main.get("python").expect("python thresholds");
    let cc = py["cc"];
    assert!(cc.warning >= 9.0 && cc.alert >= 18.0, "never tighter than the editorial floor");
    let fl = py["file_loc"];
    assert!(fl.warning >= 500.0, "editorial floor held");
    assert!(fl.warning < 2000.0, "no longer the self-weighted 2508: {}", fl.warning);
}
