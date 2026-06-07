use serde::Deserialize;

use crate::config::PulseConfig;
use crate::history::thresholds::HistoryThresholds;

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct HistoryConfig {
    #[serde(default)]
    pub ignore_paths: Vec<String>,
    #[serde(default)]
    pub co_change: CoChangePassConfig,
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

#[derive(Debug, Deserialize, Default, Clone, Copy)]
#[serde(deny_unknown_fields)]
pub struct CoChangePassConfig {
    pub max_findings: Option<u32>,
    pub min_confidence: Option<f64>,
    pub min_lift: Option<f64>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct HistoryCliOverrides {
    pub co_change_top: Option<u32>,
    pub hotspot_top: Option<u32>,
    pub contributors_top: Option<u32>,
    pub hist: bool,
    pub arch_trend: bool,
}

pub fn resolve_history_thresholds(
    config: Option<&PulseConfig>,
    overrides: HistoryCliOverrides,
) -> HistoryThresholds {
    let mut t = HistoryThresholds::DEFAULTS;
    if let Some(c) = config {
        apply_config_overrides(&mut t, c);
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
    if overrides.hist {
        t.hist.enabled = true;
    }
    if overrides.arch_trend {
        t.arch_trend = true;
    }
    t
}

fn apply_config_overrides(t: &mut HistoryThresholds, c: &PulseConfig) {
    if let Some(v) = c.history.co_change.max_findings {
        t.co_change.max_findings_reported = v;
    }
    if let Some(v) = c.history.co_change.min_confidence {
        t.co_change.min_confidence = v;
    }
    if let Some(v) = c.history.co_change.min_lift {
        t.co_change.min_lift = v;
    }
    if let Some(v) = c.history.hotspot.max_findings {
        t.hotspot.max_findings_reported = v;
    }
    if let Some(v) = c.history.contributors.max_findings {
        t.contributors.max_findings_reported = v;
    }
}

pub fn combined_history_ignore_patterns(config: Option<&PulseConfig>) -> Vec<String> {
    let Some(c) = config else { return Vec::new() };
    let mut patterns = c.ignore.paths.clone();
    patterns.extend(c.history.ignore_paths.iter().cloned());
    patterns
}
