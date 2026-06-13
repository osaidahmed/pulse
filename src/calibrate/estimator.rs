use std::collections::BTreeMap;

use super::priors::{LanguagePriors, MetricPrior, PriorsBuilder, PriorsTable, GPD_METRICS, GPD_THRESHOLD_PROBE};
use super::stats::GpdFit;
use super::Census;

const GUARDRAIL_LO_PROBE: f64 = 0.75;
const GUARDRAIL_HI_PROBE: f64 = 0.995;
const MIN_THRESHOLD_GAP: f64 = 1.0;

#[derive(Debug, Clone, Copy)]
pub struct EstimatorConfig {
    pub alpha_warning: f64,
    pub alpha_alert: f64,
    pub prior_strength: f64,
}

impl Default for EstimatorConfig {
    fn default() -> Self {
        Self { alpha_warning: 0.90, alpha_alert: 0.99, prior_strength: 50.0 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThresholdPair {
    pub warning: f64,
    pub alert: f64,
    pub lambda: f64,
    pub project_n: u64,
    pub gpd_alert: bool,
    pub clamped: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Calibrated {
    pub main: Stratum,
    pub tests: Stratum,
}

pub type Stratum = BTreeMap<String, BTreeMap<String, ThresholdPair>>;

pub fn estimate(project: &Census, corpus: &PriorsTable, cfg: &EstimatorConfig) -> Calibrated {
    let mut builder = PriorsBuilder::default();
    builder.add_census(project);
    estimate_from_tables(&builder.build(project.cpg_enabled), corpus, cfg)
}

pub fn estimate_from_tables(project: &PriorsTable, corpus: &PriorsTable, cfg: &EstimatorConfig) -> Calibrated {
    Calibrated {
        main: estimate_stratum(&project.main, &corpus.main, cfg),
        tests: estimate_stratum(&project.tests, &corpus.tests, cfg),
    }
}

fn estimate_stratum(
    project: &BTreeMap<String, LanguagePriors>,
    corpus: &BTreeMap<String, LanguagePriors>,
    cfg: &EstimatorConfig,
) -> Stratum {
    let mut out = Stratum::new();
    for (lang, project_lang) in project {
        let Some(corpus_lang) = corpus.get(lang) else { continue };
        let metrics = corpus_lang
            .metrics
            .iter()
            .map(|(metric, cm)| (metric.clone(), estimate_metric(metric, project_lang.metrics.get(metric), cm, cfg)))
            .collect();
        out.insert(lang.clone(), metrics);
    }
    out
}

fn estimate_metric(
    metric: &str,
    project: Option<&MetricPrior>,
    corpus: &MetricPrior,
    cfg: &EstimatorConfig,
) -> ThresholdPair {
    let n = project.map_or(0, |p| p.n);
    let lambda = lambda_for(n, cfg.prior_strength);
    let blend = |p: f64, use_gpd: bool| {
        let cv = point_estimate(metric, corpus, p, use_gpd);
        let pv = project.map_or(cv, |pr| point_estimate(metric, pr, p, use_gpd));
        lambda * pv + (1.0 - lambda) * cv
    };
    let lo = quantile_at(corpus, GUARDRAIL_LO_PROBE);
    let hi = quantile_at(corpus, GUARDRAIL_HI_PROBE);
    let warn_raw = blend(cfg.alpha_warning, false);
    let alert_raw = blend(cfg.alpha_alert, true);
    let warning = warn_raw.clamp(lo, hi);
    let alert = alert_raw.clamp(lo, hi).max(warning + MIN_THRESHOLD_GAP);
    ThresholdPair {
        warning,
        alert,
        lambda,
        project_n: n,
        gpd_alert: GPD_METRICS.contains(&metric) && (corpus.gpd.is_some() || project.is_some_and(|p| p.gpd.is_some())),
        clamped: outside_band(warn_raw, lo, hi) || outside_band(alert_raw, lo, hi),
    }
}

fn lambda_for(n: u64, prior_strength: f64) -> f64 {
    let n = n as f64;
    n / (n + prior_strength)
}

fn outside_band(value: f64, lo: f64, hi: f64) -> bool {
    value < lo || value > hi
}

fn point_estimate(metric: &str, prior: &MetricPrior, p: f64, use_gpd: bool) -> f64 {
    if use_gpd && GPD_METRICS.contains(&metric) {
        if let Some(fit) = &prior.gpd {
            return gpd_return_level(fit, p);
        }
    }
    quantile_at(prior, p)
}

fn gpd_return_level(fit: &GpdFit, target_p: f64) -> f64 {
    let zeta_u = 1.0 - GPD_THRESHOLD_PROBE;
    let ratio = (1.0 - target_p) / zeta_u;
    if fit.xi.abs() < 1e-6 {
        fit.threshold + fit.sigma * (1.0 / ratio).ln()
    } else {
        fit.threshold + (fit.sigma / fit.xi) * (ratio.powf(-fit.xi) - 1.0)
    }
}

fn quantile_at(prior: &MetricPrior, p: f64) -> f64 {
    let qs = &prior.quantiles;
    let Some(&(first_p, first_v)) = qs.first() else { return 0.0 };
    if p <= first_p {
        return first_v;
    }
    for window in qs.windows(2) {
        let (p0, v0) = window[0];
        let (p1, v1) = window[1];
        if p <= p1 {
            let span = p1 - p0;
            let t = if span.abs() < 1e-12 { 0.0 } else { (p - p0) / span };
            return v0 + t * (v1 - v0);
        }
    }
    qs.last().map_or(0.0, |&(_, v)| v)
}
