use serde::Deserialize;

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
    pub max_arch_findings_reported: Option<usize>,
}
