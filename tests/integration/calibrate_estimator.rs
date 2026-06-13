#![allow(clippy::float_cmp)]

use std::collections::BTreeMap;

use pulse::calibrate::estimator::{estimate, estimate_from_tables, EstimatorConfig};
use pulse::calibrate::priors::{corpus_priors, LanguagePriors, MetricPrior, PriorsTable};
use pulse::calibrate::stats::GpdFit;

const PROBES: &[f64] = &[0.50, 0.70, 0.75, 0.80, 0.85, 0.90, 0.95, 0.99, 0.995];

fn quantiles(values: &[f64]) -> Vec<(f64, f64)> {
    PROBES.iter().copied().zip(values.iter().copied()).collect()
}

fn metric(n: u64, values: &[f64], gpd: Option<GpdFit>) -> MetricPrior {
    MetricPrior { n, weight: n as f64, quantiles: quantiles(values), gpd }
}

fn table(entries: Vec<(&str, &str, MetricPrior)>) -> PriorsTable {
    let mut main: BTreeMap<String, LanguagePriors> = BTreeMap::new();
    for (lang, m, mp) in entries {
        main.entry(lang.to_string()).or_default().metrics.insert(m.to_string(), mp);
    }
    PriorsTable { cpg_enabled: false, main, tests: BTreeMap::new() }
}

fn cfg() -> EstimatorConfig {
    EstimatorConfig::default()
}

const CORPUS_CC: &[f64] = &[2.0, 3.0, 4.0, 5.0, 6.0, 9.0, 14.0, 25.0, 40.0];

#[test]
fn empty_project_degenerates_to_corpus_quantiles() {
    let corpus = table(vec![("python", "cc", metric(900, CORPUS_CC, None))]);
    let project = table(vec![("python", "cc", metric(0, CORPUS_CC, None))]);
    let out = estimate_from_tables(&project, &corpus, &cfg());
    let cc = out.main["python"]["cc"];
    assert_eq!(cc.warning, 9.0, "warning = corpus p90");
    assert_eq!(cc.alert, 25.0, "alert = corpus p99");
    assert_eq!(cc.lambda, 0.0);
}

#[test]
fn large_project_leans_toward_its_own_distribution() {
    let corpus = table(vec![("python", "cc", metric(900, CORPUS_CC, None))]);
    let project =
        table(vec![("python", "cc", metric(100_000, &[3.0, 4.0, 5.0, 6.0, 7.0, 12.0, 18.0, 30.0, 38.0], None))]);
    let out = estimate_from_tables(&project, &corpus, &cfg());
    let cc = out.main["python"]["cc"];
    assert!((cc.warning - 12.0).abs() < 0.1, "warning ~ project p90, got {}", cc.warning);
    assert!(cc.lambda > 0.99);
}

#[test]
fn extreme_project_is_clamped_to_the_corpus_band() {
    let corpus = table(vec![("python", "cc", metric(900, CORPUS_CC, None))]);
    let huge = &[50.0, 100.0, 150.0, 200.0, 300.0, 500.0, 800.0, 1500.0, 2000.0];
    let project = table(vec![("python", "cc", metric(100_000, huge, None))]);
    let out = estimate_from_tables(&project, &corpus, &cfg());
    let cc = out.main["python"]["cc"];
    assert_eq!(cc.warning, 40.0, "clamped up to corpus p995");
    assert_eq!(cc.alert, 41.0, "alert held one above the clamped warning");
    assert!(cc.clamped);
}

#[test]
fn warning_is_always_below_alert() {
    let corpus = table(vec![
        ("python", "cc", metric(900, CORPUS_CC, None)),
        ("python", "args", metric(900, &[1.0, 1.0, 1.0, 2.0, 2.0, 2.0, 3.0, 4.0, 5.0], None)),
    ]);
    let project = table(vec![("python", "cc", metric(40, CORPUS_CC, None))]);
    let out = estimate_from_tables(&project, &corpus, &cfg());
    for metrics in out.main.values() {
        for (name, p) in metrics {
            assert!(p.warning < p.alert, "{name}: warning {} !< alert {}", p.warning, p.alert);
        }
    }
}

#[test]
fn continuous_alert_uses_the_gpd_tail_discrete_does_not() {
    let gpd = GpdFit { threshold: 9.0, xi: 0.2, sigma: 10.0, tail_n: 100 };
    let corpus = table(vec![
        ("python", "cc", metric(900, CORPUS_CC, Some(gpd))),
        ("python", "args", metric(900, &[1.0, 1.0, 1.0, 2.0, 2.0, 2.0, 3.0, 4.0, 5.0], Some(gpd))),
    ]);
    let project = table(vec![("python", "cc", metric(0, CORPUS_CC, None))]);
    let out = estimate_from_tables(&project, &corpus, &cfg());
    let cc = out.main["python"]["cc"];
    assert!(cc.gpd_alert, "cc is continuous: tail-modeled");
    assert!((cc.alert - 38.24).abs() < 0.5, "gpd return level at p99, got {}", cc.alert);
    let args = out.main["python"]["args"];
    assert!(!args.gpd_alert, "args is discrete: never gpd");
    assert_eq!(args.alert, 4.0, "discrete alert = quantile p99");
}

#[test]
fn estimate_runs_against_the_real_corpus() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/app.py"),
        "def alpha(x):\n    if x > 1:\n        return x\n    return 0\n\ndef beta(y):\n    return y + 1\n",
    )
    .unwrap();
    let matcher = pulse::config::IgnoreMatcher::from_patterns(&[]);
    let filter = pulse::audit::IgnoreFilter::new(&matcher, dir.path());
    let census = pulse::calibrate::collect(dir.path(), &crate::common::t(), &filter);
    let out = estimate(&census, corpus_priors(), &cfg());
    let py = out.main.get("python").expect("python thresholds");
    let cc = py.get("cc").expect("cc threshold");
    assert!(cc.warning < cc.alert);
    assert!(cc.warning > 0.0 && cc.alert > 0.0);
    assert!(cc.lambda < 0.1, "a 2-function project leans on the corpus prior");
}
