use std::collections::{BTreeMap, BTreeSet};

use super::priors::{LanguagePriors, MetricPrior, PriorsTable};
use super::{Census, FileCensus};

#[derive(Debug, Clone, Copy)]
pub struct EstimatorConfig {
    pub warn_percentile: f64,
    pub alert_percentile: f64,
}

impl Default for EstimatorConfig {
    fn default() -> Self {
        Self { warn_percentile: 0.75, alert_percentile: 0.95 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThresholdPair {
    pub warning: f64,
    pub alert: f64,
    pub editorial_warning: u32,
    pub editorial_alert: u32,
    pub corpus_warning: f64,
    pub corpus_alert: f64,
}

impl ThresholdPair {
    pub fn warning_loosened(&self) -> bool {
        self.warning > f64::from(self.editorial_warning)
    }

    pub fn alert_loosened(&self) -> bool {
        self.alert > f64::from(self.editorial_alert)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Calibrated {
    pub main: Stratum,
    pub tests: Stratum,
}

pub type Stratum = BTreeMap<String, BTreeMap<String, ThresholdPair>>;

pub fn estimate(census: &Census, corpus: &PriorsTable, cfg: &EstimatorConfig) -> Calibrated {
    Calibrated {
        main: estimate_stratum(&project_langs(&census.main), &corpus.main, cfg),
        tests: estimate_stratum(&project_langs(&census.tests), &corpus.tests, cfg),
    }
}

pub fn estimate_languages(
    langs: &BTreeSet<String>,
    corpus: &BTreeMap<String, LanguagePriors>,
    cfg: &EstimatorConfig,
) -> Stratum {
    estimate_stratum(langs, corpus, cfg)
}

fn project_langs(files: &[FileCensus]) -> BTreeSet<String> {
    files.iter().map(|f| f.lang.to_config_key().to_string()).collect()
}

fn estimate_stratum(
    langs: &BTreeSet<String>,
    corpus: &BTreeMap<String, LanguagePriors>,
    cfg: &EstimatorConfig,
) -> Stratum {
    let editorial = editorial_defaults();
    let mut out = Stratum::new();
    for lang in langs {
        let Some(corpus_lang) = corpus.get(lang) else { continue };
        let metrics: BTreeMap<String, ThresholdPair> = editorial
            .iter()
            .filter_map(|(metric, warn, alert)| {
                corpus_lang
                    .metrics
                    .get(*metric)
                    .map(|cm| ((*metric).to_string(), threshold_for(*warn, *alert, cm, cfg)))
            })
            .collect();
        if !metrics.is_empty() {
            out.insert(lang.clone(), metrics);
        }
    }
    out
}

fn threshold_for(
    editorial_warning: u32,
    editorial_alert: u32,
    corpus: &MetricPrior,
    cfg: &EstimatorConfig,
) -> ThresholdPair {
    let corpus_warning = corpus.quantile(cfg.warn_percentile);
    let corpus_alert = corpus.quantile(cfg.alert_percentile);
    ThresholdPair {
        warning: f64::from(editorial_warning).max(corpus_warning),
        alert: f64::from(editorial_alert).max(corpus_alert),
        editorial_warning,
        editorial_alert,
        corpus_warning,
        corpus_alert,
    }
}

fn editorial_defaults() -> Vec<(&'static str, u32, u32)> {
    let t = pulse_thresholds::Thresholds::default();
    let (f, m, a) = (&t.function, &t.module, &t.analysis);
    vec![
        ("cc", f.cc_warning, f.cc_alert),
        ("cogc", f.cogc_warning, f.cogc_alert),
        ("fn_loc", f.fn_loc_warning, f.fn_loc_alert),
        ("file_loc", m.file_loc_warning, m.file_loc_alert),
        ("nesting", f.nesting_depth, f.nesting_depth),
        ("global_nesting", m.global_nesting_depth, m.global_nesting_depth),
        ("bump", f.bump_count, f.bump_count),
        ("args", f.arg_max, f.arg_max),
        ("compound_conditions", f.compound_conditions, f.compound_conditions),
        ("embedded_block_loc", f.embedded_block_loc, f.embedded_block_loc),
        ("consecutive_asserts", a.consecutive_asserts_max, a.consecutive_asserts_max),
        ("short_vars", a.short_var_max_count, a.short_var_max_count),
        ("string_match_arms", a.max_string_match_arms, a.max_string_match_arms),
        ("struct_fields", m.max_struct_fields, m.max_struct_fields),
        ("file_functions", m.file_function_count, m.file_function_count),
        ("file_total_cc", m.file_total_cc, m.file_total_cc),
        ("declarations", m.max_declarations, m.max_declarations),
        ("global_conditionals", m.global_conditionals_max, m.global_conditionals_max),
    ]
}
