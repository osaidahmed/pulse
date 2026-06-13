use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use super::stats::{count_quantile, WeightedHist};
use super::Census;
use crate::parse::Language;
use crate::walk::{FileMetrics, FunctionMetrics, ModuleMetrics};

pub const QUANTILE_PROBES: &[f64] = &[0.50, 0.70, 0.75, 0.80, 0.85, 0.90, 0.95, 0.99, 0.995];

type FunctionGetter = fn(&FunctionMetrics) -> u32;
type ModuleGetter = fn(&ModuleMetrics) -> u32;

const FUNCTION_METRICS: &[(&str, FunctionGetter)] = &[
    ("fn_loc", |f| f.loc),
    ("cc", |f| f.cc),
    ("cogc", |f| f.cognitive_complexity),
    ("nesting", |f| f.max_nesting),
    ("bump", |f| f.bump_count),
    ("args", |f| f.arg_count),
    ("compound_conditions", |f| f.compound_condition_count),
    ("embedded_block_loc", |f| f.max_embedded_block_loc),
    ("consecutive_asserts", |f| f.consecutive_asserts),
    ("short_vars", |f| f.short_var_count),
    ("string_match_arms", |f| f.string_match_arms),
];

const MODULE_METRICS: &[(&str, ModuleGetter)] = &[
    ("file_loc", |m| m.total_loc),
    ("file_functions", |m| m.total_functions),
    ("file_total_cc", |m| m.sum_cc),
    ("declarations", |m| m.declaration_count),
    ("global_nesting", |m| m.global_max_nesting),
    ("global_conditionals", |m| m.global_conditional_count),
];

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PriorsTable {
    pub cpg_enabled: bool,
    pub main: BTreeMap<String, LanguagePriors>,
    pub tests: BTreeMap<String, LanguagePriors>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LanguagePriors {
    pub metrics: BTreeMap<String, MetricPrior>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricPrior {
    pub n: u64,
    pub quantiles: Vec<(f64, f64)>,
    #[serde(default)]
    pub cdf: Vec<(u32, u64)>,
}

impl MetricPrior {
    pub fn quantile(&self, p: f64) -> f64 {
        let qs = &self.quantiles;
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

    pub fn cdf_at(&self, value: u32) -> f64 {
        if self.n == 0 {
            return 1.0;
        }
        let mut cumulative = 0u64;
        for &(v, count) in &self.cdf {
            if v > value {
                break;
            }
            cumulative += count;
        }
        cumulative as f64 / self.n as f64
    }

    pub fn firing_rate(&self, threshold: u32) -> f64 {
        1.0 - self.cdf_at(threshold)
    }

    pub fn percentile_of(&self, value: u32) -> f64 {
        self.cdf_at(value) * 100.0
    }
}

#[derive(Default, Serialize, Deserialize)]
pub struct PriorsBuilder {
    main: Stratum,
    tests: Stratum,
}

type Stratum = BTreeMap<String, WeightedHist>;

impl PriorsBuilder {
    pub fn add_census(&mut self, census: &Census) {
        for file in &census.main {
            add_file(&mut self.main, file.lang, &file.functions, &file.module);
        }
        for file in &census.tests {
            add_file(&mut self.tests, file.lang, &file.functions, &file.module);
        }
    }

    pub fn add_file_metrics(&mut self, lang: Language, is_test: bool, metrics: &FileMetrics) {
        let stratum = if is_test { &mut self.tests } else { &mut self.main };
        add_file(stratum, lang, &metrics.functions, &metrics.module);
    }

    pub fn merge(&mut self, other: &PriorsBuilder) {
        merge_stratum(&mut self.main, &other.main);
        merge_stratum(&mut self.tests, &other.tests);
    }

    pub fn merge_json(&mut self, json: &str) -> bool {
        match serde_json::from_str::<PriorsBuilder>(json) {
            Ok(other) => {
                self.merge(&other);
                true
            }
            Err(_) => false,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn build(&self, cpg_enabled: bool) -> PriorsTable {
        PriorsTable { cpg_enabled, main: build_stratum(&self.main), tests: build_stratum(&self.tests) }
    }
}

fn merge_stratum(into: &mut Stratum, other: &Stratum) {
    for (key, hist) in other {
        into.entry(key.clone()).or_default().merge(hist);
    }
}

fn stratum_key(lang: &str, metric: &str) -> String {
    format!("{lang}\u{1f}{metric}")
}

fn split_key(key: &str) -> (&str, &str) {
    key.split_once('\u{1f}').unwrap_or((key, ""))
}

fn add_file(stratum: &mut Stratum, lang: Language, functions: &[FunctionMetrics], module: &ModuleMetrics) {
    let lang = lang.to_config_key();
    for function in functions {
        let weight = f64::from(function.loc.max(1));
        for (metric, getter) in FUNCTION_METRICS {
            stratum.entry(stratum_key(lang, metric)).or_default().observe(getter(function), weight);
        }
    }
    let file_weight = f64::from(module.total_loc.max(1));
    for (metric, getter) in MODULE_METRICS {
        stratum.entry(stratum_key(lang, metric)).or_default().observe(getter(module), file_weight);
    }
    for (_, field_count) in &module.struct_fields {
        stratum.entry(stratum_key(lang, "struct_fields")).or_default().observe(*field_count, file_weight);
    }
}

fn build_stratum(stratum: &Stratum) -> BTreeMap<String, LanguagePriors> {
    let mut out: BTreeMap<String, LanguagePriors> = BTreeMap::new();
    for (key, hist) in stratum {
        let (lang, metric) = split_key(key);
        let prior = MetricPrior {
            n: hist.n,
            quantiles: QUANTILE_PROBES.iter().map(|p| (*p, count_quantile(hist, *p))).collect(),
            cdf: hist.bins.iter().map(|(value, bin)| (*value, bin.count)).collect(),
        };
        out.entry(lang.to_string()).or_default().metrics.insert(metric.to_string(), prior);
    }
    out
}

static PRIORS: OnceLock<PriorsTable> = OnceLock::new();

pub fn corpus_priors() -> &'static PriorsTable {
    PRIORS.get_or_init(|| serde_json::from_str(include_str!("priors.json")).unwrap_or_default())
}
