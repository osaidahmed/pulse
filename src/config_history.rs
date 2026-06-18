use serde::Deserialize;

use crate::config::PulseConfig;
use crate::history::jit_thresholds::JitThresholds;
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
    #[serde(default)]
    pub jit: JitPassConfig,
}

#[derive(Debug, Deserialize, Default, Clone, Copy)]
#[serde(deny_unknown_fields)]
pub struct JitPassConfig {
    pub use_lt: Option<bool>,
    pub use_age: Option<bool>,
    pub use_entropy: Option<bool>,
    pub entropy_bits: Option<f64>,
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
    pub no_szz: bool,
}

pub fn resolve_history_thresholds(config: Option<&PulseConfig>, overrides: HistoryCliOverrides) -> HistoryThresholds {
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
    if overrides.no_szz {
        t.szz.enabled = false;
    }
    t
}

fn or_set<T: Copy>(field: &mut T, opt: Option<T>) {
    *field = opt.unwrap_or(*field);
}

fn apply_config_overrides(t: &mut HistoryThresholds, c: &PulseConfig) {
    or_set(&mut t.co_change.max_findings_reported, c.history.co_change.max_findings);
    or_set(&mut t.co_change.min_confidence, c.history.co_change.min_confidence);
    or_set(&mut t.co_change.min_lift, c.history.co_change.min_lift);
    or_set(&mut t.hotspot.max_findings_reported, c.history.hotspot.max_findings);
    or_set(&mut t.contributors.max_findings_reported, c.history.contributors.max_findings);
    apply_jit_overrides(&mut t.jit, &c.history.jit);
}

fn apply_jit_overrides(t: &mut JitThresholds, c: &JitPassConfig) {
    or_set(&mut t.use_lt, c.use_lt);
    or_set(&mut t.use_age, c.use_age);
    or_set(&mut t.use_entropy, c.use_entropy);
    or_set(&mut t.entropy_bits, c.entropy_bits);
}

pub fn combined_history_ignore_patterns(config: Option<&PulseConfig>) -> Vec<String> {
    let Some(c) = config else { return Vec::new() };
    let mut patterns = c.ignore.paths.clone();
    patterns.extend(c.history.ignore_paths.iter().cloned());
    patterns
}
