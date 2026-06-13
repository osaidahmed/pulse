use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use super::stats::{gpd_fit, weighted_quantile, GpdFit, WeightedHist};
use super::Census;
use crate::parse::Language;
use crate::walk::{FileMetrics, FunctionMetrics, ModuleMetrics};

pub const QUANTILE_PROBES: &[f64] = &[0.50, 0.70, 0.75, 0.80, 0.85, 0.90, 0.95, 0.99, 0.995];
const GPD_THRESHOLD_PROBE: f64 = 0.90;

const GPD_METRICS: &[&str] = &["fn_loc", "cc", "cogc", "file_loc", "file_total_cc", "embedded_block_loc"];

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
    pub weight: f64,
    pub quantiles: Vec<(f64, f64)>,
    pub gpd: Option<GpdFit>,
}

#[derive(Default)]
pub struct PriorsBuilder {
    main: Stratum,
    tests: Stratum,
}

type Stratum = BTreeMap<(String, &'static str), WeightedHist>;

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

    pub fn build(&self, cpg_enabled: bool) -> PriorsTable {
        PriorsTable { cpg_enabled, main: build_stratum(&self.main), tests: build_stratum(&self.tests) }
    }
}

fn merge_stratum(into: &mut Stratum, other: &Stratum) {
    for (key, hist) in other {
        into.entry(key.clone()).or_default().merge(hist);
    }
}

fn add_file(stratum: &mut Stratum, lang: Language, functions: &[FunctionMetrics], module: &ModuleMetrics) {
    let key = lang.to_config_key();
    for function in functions {
        let weight = f64::from(function.loc.max(1));
        for (metric, getter) in FUNCTION_METRICS {
            stratum.entry((key.to_string(), metric)).or_default().observe(getter(function), weight);
        }
    }
    let file_weight = f64::from(module.total_loc.max(1));
    for (metric, getter) in MODULE_METRICS {
        stratum.entry((key.to_string(), metric)).or_default().observe(getter(module), file_weight);
    }
    for (_, field_count) in &module.struct_fields {
        stratum.entry((key.to_string(), "struct_fields")).or_default().observe(*field_count, file_weight);
    }
}

fn build_stratum(stratum: &Stratum) -> BTreeMap<String, LanguagePriors> {
    let mut out: BTreeMap<String, LanguagePriors> = BTreeMap::new();
    for ((lang, metric), hist) in stratum {
        let prior = MetricPrior {
            n: hist.n,
            weight: hist.weight,
            quantiles: QUANTILE_PROBES.iter().map(|p| (*p, weighted_quantile(hist, *p))).collect(),
            gpd: tail_fit(metric, hist),
        };
        out.entry(lang.clone()).or_default().metrics.insert((*metric).to_string(), prior);
    }
    out
}

fn tail_fit(metric: &str, hist: &WeightedHist) -> Option<GpdFit> {
    if !GPD_METRICS.contains(&metric) {
        return None;
    }
    gpd_fit(hist, weighted_quantile(hist, GPD_THRESHOLD_PROBE))
}

static PRIORS: OnceLock<PriorsTable> = OnceLock::new();

pub fn corpus_priors() -> &'static PriorsTable {
    PRIORS.get_or_init(|| serde_json::from_str(include_str!("priors.json")).unwrap_or_default())
}
