use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::parse::Language;
use crate::smells::{self, Finding, Smell};
use crate::thresholds::Thresholds;

const CONFIG_FILENAME: &str = ".pulse.toml";

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct PulseConfig {
    #[serde(default)]
    pub thresholds: ConfigThresholds,
    #[serde(default)]
    pub disable: DisableConfig,
    #[serde(default)]
    pub languages: std::collections::HashMap<String, ConfigThresholds>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct ConfigThresholds {
    #[serde(flatten)]
    pub function: FunctionThresholds,
    #[serde(flatten)]
    pub module: ModuleThresholds,
    #[serde(flatten)]
    pub analysis: AnalysisThresholds,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct FunctionThresholds {
    pub cc_warning: Option<u32>,
    pub cc_alert: Option<u32>,
    pub cogc_warning: Option<u32>,
    pub cogc_alert: Option<u32>,
    pub fn_loc_warning: Option<u32>,
    pub fn_loc_alert: Option<u32>,
    pub nesting_depth: Option<u32>,
    pub bump_count: Option<u32>,
    pub arg_max: Option<u32>,
    pub constructor_arg_max: Option<u32>,
    pub compound_conditions: Option<u32>,
    pub embedded_block_loc: Option<u32>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct ModuleThresholds {
    pub file_loc_warning: Option<u32>,
    pub file_loc_alert: Option<u32>,
    pub file_function_count: Option<u32>,
    pub file_total_cc: Option<u32>,
    pub max_declarations: Option<u32>,
    pub large_fn_loc: Option<u32>,
    pub large_fn_count: Option<u32>,
    pub max_struct_fields: Option<u32>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct AnalysisThresholds {
    pub duplication_min_loc: Option<u32>,
    pub skeleton_duplication_min_loc: Option<u32>,
    pub duplication_min_group: Option<u32>,
    pub consecutive_asserts_max: Option<u32>,
    pub primitive_ratio_threshold: Option<f32>,
    pub primitive_min_typed_params: Option<u32>,
    pub lcom4_warning: Option<u32>,
    pub short_var_min_fn_loc: Option<u32>,
    pub short_var_max_count: Option<u32>,
    pub max_string_match_arms: Option<u32>,
}

#[derive(Debug, Deserialize, Default)]
pub struct DisableConfig {
    #[serde(default)]
    pub smells: Vec<String>,
}

pub fn find_config(start: &Path) -> Option<PathBuf> {
    let mut dir = if start.is_file() {
        start.parent()?
    } else {
        start
    };
    loop {
        let candidate = dir.join(CONFIG_FILENAME);
        if candidate.is_file() {
            return Some(candidate);
        }
        dir = dir.parent()?;
    }
}

pub fn load_config(start: &Path) -> Option<PulseConfig> {
    let path = find_config(start)?;
    let content = std::fs::read_to_string(path).ok()?;
    toml::from_str(&content).ok()
}

pub fn resolve_thresholds(config: Option<&PulseConfig>, lang: Language) -> Thresholds {
    let base = Thresholds::default();
    let Some(config) = config else { return base };
    let merged = apply_overrides(&base, &config.thresholds);
    config
        .languages
        .get(lang.to_config_key())
        .map_or(merged.clone(), |lang_overrides| {
            apply_overrides(&merged, lang_overrides)
        })
}

pub fn resolve_base_thresholds(config: Option<&PulseConfig>) -> Thresholds {
    let base = Thresholds::default();
    let Some(config) = config else { return base };
    apply_overrides(&base, &config.thresholds)
}

fn apply_overrides(base: &Thresholds, o: &ConfigThresholds) -> Thresholds {
    let f = &o.function;
    let m = &o.module;
    let a = &o.analysis;
    Thresholds {
        cc_warning: f.cc_warning.unwrap_or(base.cc_warning),
        cc_alert: f.cc_alert.unwrap_or(base.cc_alert),
        cogc_warning: f.cogc_warning.unwrap_or(base.cogc_warning),
        cogc_alert: f.cogc_alert.unwrap_or(base.cogc_alert),
        fn_loc_warning: f.fn_loc_warning.unwrap_or(base.fn_loc_warning),
        fn_loc_alert: f.fn_loc_alert.unwrap_or(base.fn_loc_alert),
        nesting_depth: f.nesting_depth.unwrap_or(base.nesting_depth),
        bump_count: f.bump_count.unwrap_or(base.bump_count),
        arg_max: f.arg_max.unwrap_or(base.arg_max),
        constructor_arg_max: f.constructor_arg_max.unwrap_or(base.constructor_arg_max),
        compound_conditions: f.compound_conditions.unwrap_or(base.compound_conditions),
        embedded_block_loc: f.embedded_block_loc.unwrap_or(base.embedded_block_loc),
        file_loc_warning: m.file_loc_warning.unwrap_or(base.file_loc_warning),
        file_loc_alert: m.file_loc_alert.unwrap_or(base.file_loc_alert),
        file_function_count: m.file_function_count.unwrap_or(base.file_function_count),
        file_total_cc: m.file_total_cc.unwrap_or(base.file_total_cc),
        max_declarations: m.max_declarations.unwrap_or(base.max_declarations),
        large_fn_loc: m.large_fn_loc.unwrap_or(base.large_fn_loc),
        large_fn_count: m.large_fn_count.unwrap_or(base.large_fn_count),
        max_struct_fields: m.max_struct_fields.unwrap_or(base.max_struct_fields),
        duplication_min_loc: a.duplication_min_loc.unwrap_or(base.duplication_min_loc),
        skeleton_duplication_min_loc: a.skeleton_duplication_min_loc.unwrap_or(base.skeleton_duplication_min_loc),
        duplication_min_group: a.duplication_min_group.unwrap_or(base.duplication_min_group),
        consecutive_asserts_max: a.consecutive_asserts_max.unwrap_or(base.consecutive_asserts_max),
        primitive_ratio_threshold: a.primitive_ratio_threshold.unwrap_or(base.primitive_ratio_threshold),
        primitive_min_typed_params: a.primitive_min_typed_params.unwrap_or(base.primitive_min_typed_params),
        lcom4_warning: a.lcom4_warning.unwrap_or(base.lcom4_warning),
        short_var_min_fn_loc: a.short_var_min_fn_loc.unwrap_or(base.short_var_min_fn_loc),
        short_var_max_count: a.short_var_max_count.unwrap_or(base.short_var_max_count),
        max_string_match_arms: a.max_string_match_arms.unwrap_or(base.max_string_match_arms),
    }
}

pub fn resolve_disabled(config: Option<&PulseConfig>) -> HashSet<Smell> {
    let Some(config) = config else {
        return HashSet::new();
    };
    config
        .disable
        .smells
        .iter()
        .filter_map(|s| smells::smell_from_snake_case(s))
        .collect()
}

pub fn filter_disabled(findings: &mut Vec<Finding>, disabled: &HashSet<Smell>) {
    if !disabled.is_empty() {
        findings.retain(|f| !disabled.contains(&f.smell));
    }
}
