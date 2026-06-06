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

impl PageRankThresholds {
    pub const DEFAULTS: Self = Self {
        damping: 0.85,
        max_iters: 100,
        epsilon: 1e-6,
    };
}

impl CommunityThresholds {
    pub const DEFAULTS: Self = Self {
        resolution: 1.0,
        max_passes: 20,
        min_split_files: 4,
        split_cohesion: 0.6,
    };
}
