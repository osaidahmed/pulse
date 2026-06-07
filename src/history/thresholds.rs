#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HistoryThresholds {
    pub co_change: CoChangeThresholds,
    pub hotspot: HotspotThresholds,
    pub contributors: ContributorThresholds,
    pub hist: HistSmellThresholds,
    pub arch_trend: bool,
    pub max_commit_files: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HistSmellThresholds {
    pub enabled: bool,
    pub blob_commit_pct: f64,
    pub shotgun_confidence: f64,
    pub shotgun_distinct_packages: u32,
    pub max_findings_reported: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoChangeThresholds {
    pub min_support: u32,
    pub min_confidence: f64,
    pub min_lift: f64,
    pub max_findings_reported: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HotspotThresholds {
    pub min_revisions: u32,
    pub min_score: u64,
    pub max_findings_reported: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContributorThresholds {
    pub minor_contributor_pct: f64,
    pub min_minor_authors: u32,
    pub min_total_commits: u32,
    pub max_findings_reported: u32,
}

impl CoChangeThresholds {
    pub const DEFAULTS: Self = Self { min_support: 3, min_confidence: 0.5, min_lift: 1.0, max_findings_reported: 20 };
}

impl HotspotThresholds {
    pub const DEFAULTS: Self = Self { min_revisions: 3, min_score: 30, max_findings_reported: 10 };
}

impl ContributorThresholds {
    pub const DEFAULTS: Self =
        Self { minor_contributor_pct: 5.0, min_minor_authors: 3, min_total_commits: 5, max_findings_reported: 20 };
}

impl HistSmellThresholds {
    pub const DEFAULTS: Self = Self {
        enabled: false,
        blob_commit_pct: 0.08,
        shotgun_confidence: 0.70,
        shotgun_distinct_packages: 3,
        max_findings_reported: 20,
    };
}

impl HistoryThresholds {
    pub const DEFAULTS: Self = Self {
        co_change: CoChangeThresholds::DEFAULTS,
        hotspot: HotspotThresholds::DEFAULTS,
        contributors: ContributorThresholds::DEFAULTS,
        hist: HistSmellThresholds::DEFAULTS,
        arch_trend: false,
        max_commit_files: 40,
    };
}

impl Default for HistoryThresholds {
    fn default() -> Self {
        Self::DEFAULTS
    }
}
