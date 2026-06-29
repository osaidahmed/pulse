#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageRankThresholds {
    pub damping: f64,
    pub max_iters: u32,
    pub epsilon: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CommunityThresholds {
    pub resolution: f64,
    pub max_passes: u32,
    pub min_split_files: u32,
    pub split_cohesion: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FragmentationThresholds {
    pub min_coupled_files: u32,
    pub min_density: f64,
    pub max_avg_loc: u32,
}

impl PageRankThresholds {
    pub const DEFAULTS: Self = Self { damping: 0.85, max_iters: 100, epsilon: 1e-6 };
}

impl FragmentationThresholds {
    pub const DEFAULTS: Self = Self { min_coupled_files: 8, min_density: 1.5, max_avg_loc: 80 };
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConceptualCohesionThresholds {
    pub enabled: bool,
    pub min_methods: u32,
    pub min_vocab: u32,
    pub min_cohesion: f64,
}

impl ConceptualCohesionThresholds {
    pub const DEFAULTS: Self = Self { enabled: true, min_methods: 5, min_vocab: 12, min_cohesion: 0.10 };
}

impl CommunityThresholds {
    pub const DEFAULTS: Self = Self { resolution: 1.0, max_passes: 20, min_split_files: 4, split_cohesion: 0.6 };
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MultivariateAnomalyThresholds {
    pub enabled: bool,
    pub min_classes: u32,
    pub distance_quantile: f64,
    pub max_findings: u32,
}

impl MultivariateAnomalyThresholds {
    pub const DEFAULTS: Self = Self { enabled: true, min_classes: 8, distance_quantile: 11.143, max_findings: 10 };
}
