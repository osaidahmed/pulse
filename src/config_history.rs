use serde::Deserialize;

use crate::config::PulseConfig;
use crate::history::thresholds::HistoryThresholds;

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct HistoryConfig {
    #[serde(default)]
    pub ignore_paths: Vec<String>,
    #[serde(default)]
    pub co_change: HistoryPassConfig,
    #[serde(default)]
    pub hotspot: HistoryPassConfig,
    #[serde(default)]
    pub contributors: HistoryPassConfig,
}

#[derive(Debug, Deserialize, Default, Clone, Copy)]
#[serde(deny_unknown_fields)]
pub struct HistoryPassConfig {
    pub max_findings: Option<u32>,
}

#[derive(Debug, Default, Clone, Copy)]
#[allow(clippy::struct_field_names)]
pub struct HistoryCliOverrides {
    pub co_change_top: Option<u32>,
    pub hotspot_top: Option<u32>,
    pub contributors_top: Option<u32>,
}

pub fn resolve_history_thresholds(
    config: Option<&PulseConfig>,
    overrides: HistoryCliOverrides,
) -> HistoryThresholds {
    let mut t = HistoryThresholds::DEFAULTS;
    if let Some(c) = config {
        if let Some(v) = c.history.co_change.max_findings {
            t.co_change.max_findings_reported = v;
        }
        if let Some(v) = c.history.hotspot.max_findings {
            t.hotspot.max_findings_reported = v;
        }
        if let Some(v) = c.history.contributors.max_findings {
            t.contributors.max_findings_reported = v;
        }
    }
    if let Some(v) = overrides.co_change_top {
        t.co_change.max_findings_reported = v;
    }
    if let Some(v) = overrides.hotspot_top {
        t.hotspot.max_findings_reported = v;
    }
    if let Some(v) = overrides.contributors_top {
        t.contributors.max_findings_reported = v;
    }
    t
}

pub fn combined_history_ignore_patterns(config: Option<&PulseConfig>) -> Vec<String> {
    let Some(c) = config else { return Vec::new() };
    let mut patterns = c.ignore.paths.clone();
    patterns.extend(c.history.ignore_paths.iter().cloned());
    patterns
}
