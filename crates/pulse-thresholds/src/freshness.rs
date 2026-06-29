#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FreshnessThresholds {
    pub min_missed: u32,
    pub abandon_years: i64,
    pub max_findings: usize,
}

impl FreshnessThresholds {
    pub const DEFAULTS: Self = Self { min_missed: 5, abandon_years: 2, max_findings: 30 };
}
