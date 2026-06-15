use serde::Deserialize;

#[derive(Debug, Deserialize, Default, Clone, Copy)]
#[serde(deny_unknown_fields)]
pub struct TaintConfig {
    pub visit_cap: Option<u32>,
    pub max_depth: Option<u32>,
    pub max_findings: Option<usize>,
}

#[derive(Debug, Deserialize, Default, Clone, Copy)]
#[serde(deny_unknown_fields)]
pub struct CloneClusterConfig {
    pub max_sim_threshold: Option<u32>,
    pub min_cluster_size: Option<u32>,
    pub loc_window_pct: Option<u32>,
    pub min_loc: Option<u32>,
}

#[derive(Debug, Deserialize, Default, Clone, Copy)]
#[serde(deny_unknown_fields)]
pub struct NaturalnessConfig {
    pub enabled: Option<bool>,
    pub ngram_order: Option<u32>,
    pub cache_k: Option<u32>,
    pub jm_gamma: Option<f64>,
    pub min_fn_tokens: Option<u32>,
    pub zscore_cutoff: Option<f64>,
}

#[derive(Debug, Deserialize, Default, Clone, Copy)]
#[serde(deny_unknown_fields)]
pub struct NamedSmellsConfig {
    #[serde(default)]
    pub god_class: GodClassConfig,
    #[serde(default)]
    pub refused_bequest: RefusedBequestConfig,
}

#[derive(Debug, Deserialize, Default, Clone, Copy)]
#[serde(deny_unknown_fields)]
pub struct RefusedBequestConfig {
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize, Default, Clone, Copy)]
#[serde(deny_unknown_fields)]
pub struct GodClassConfig {
    pub wmc: Option<u32>,
    pub tcc: Option<f64>,
    pub atfd: Option<u32>,
}

#[derive(Debug, Deserialize, Default, Clone, Copy)]
#[serde(deny_unknown_fields)]
pub struct PackageMetricsConfig {
    pub martin_distance_warning: Option<f64>,
    pub martin_distance_alert: Option<f64>,
    pub include_tests_in_graph: Option<bool>,
    pub martin_cycle_min_size: Option<u32>,
    pub max_cycle_findings_reported: Option<usize>,
    pub max_martin_findings_reported: Option<usize>,
    pub unstable_dep_strength: Option<f64>,
    pub hublike_imbalance_ratio: Option<f64>,
    pub god_component_loc_percentile: Option<f64>,
    pub martin_min_coupling: Option<u32>,
    pub max_arch_findings_reported: Option<usize>,
}
