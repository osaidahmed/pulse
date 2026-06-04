#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CloneClusterThresholds {
    pub max_sim_threshold: u32,
    pub min_cluster_size: u32,
    pub loc_window_pct: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
pub struct CpgThresholds {
    pub enabled: bool,
    pub taint_visit_cap: u32,
    pub taint_max_depth: u32,
    pub dead_store: bool,
    pub use_before_def: bool,
    pub unreachable_code: bool,
    pub unused_result: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NaturalnessThresholds {
    pub enabled: bool,
    pub ngram_order: u32,
    pub cache_k: u32,
    pub jm_gamma: f64,
    pub min_fn_tokens: u32,
    pub zscore_cutoff: f64,
}

impl CloneClusterThresholds {
    pub const DEFAULTS: Self = Self {
        max_sim_threshold: 12,
        min_cluster_size: 2,
        loc_window_pct: 30,
    };
}

impl CpgThresholds {
    pub const DEFAULTS: Self = Self {
        enabled: false,
        taint_visit_cap: 2,
        taint_max_depth: 16,
        dead_store: true,
        use_before_def: true,
        unreachable_code: true,
        unused_result: false,
    };
}

impl NaturalnessThresholds {
    pub const DEFAULTS: Self = Self {
        enabled: false,
        ngram_order: 6,
        cache_k: 5000,
        jm_gamma: 1.0,
        min_fn_tokens: 20,
        zscore_cutoff: 3.0,
    };
}
