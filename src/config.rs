use std::collections::HashSet;
use std::path::{Path, PathBuf};

use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
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
    pub ignore: IgnoreConfig,
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

#[derive(Debug, Deserialize, Default)]
pub struct IgnoreConfig {
    #[serde(default)]
    pub paths: Vec<String>,
}

pub struct IgnoreMatcher {
    set: GlobSet,
}

impl IgnoreMatcher {
    pub fn from_patterns(patterns: &[String]) -> Self {
        let mut builder = GlobSetBuilder::new();
        for raw in patterns {
            for variant in expand_pattern(raw) {
                if let Ok(glob) = GlobBuilder::new(&variant).literal_separator(true).build() {
                    builder.add(glob);
                }
            }
        }
        let set = builder.build().unwrap_or_else(|_| GlobSet::empty());
        Self { set }
    }

    pub fn matches_file(&self, config_root: &Path, file_path: &Path) -> bool {
        if self.set.is_empty() {
            return false;
        }
        relative_path(config_root, file_path).is_some_and(|rel| self.set.is_match(&rel))
    }
}

fn expand_pattern(raw: &str) -> Vec<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    let stripped = trimmed.trim_end_matches('/');
    if stripped.is_empty() {
        return Vec::new();
    }
    let mut out = vec![stripped.to_string()];
    if !stripped.ends_with("/**") && !stripped.ends_with("**") {
        out.push(format!("{stripped}/**"));
    }
    out
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

pub fn load_config_with_root(start: &Path) -> Option<(PulseConfig, PathBuf)> {
    let path = find_config(start)?;
    let content = std::fs::read_to_string(&path).ok()?;
    let cfg: PulseConfig = toml::from_str(&content).ok()?;
    let root = path.parent()?.to_path_buf();
    Some((cfg, root))
}

pub fn is_ignored_for_file(cfg: &PulseConfig, file_path: &Path) -> bool {
    if cfg.ignore.paths.is_empty() {
        return false;
    }
    let Some(root) = find_config(file_path).and_then(|p| p.parent().map(Path::to_path_buf))
    else {
        return false;
    };
    IgnoreMatcher::from_patterns(&cfg.ignore.paths).matches_file(&root, file_path)
}

fn relative_path(root: &Path, file: &Path) -> Option<PathBuf> {
    let canon_root = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let canon_file = std::fs::canonicalize(file).unwrap_or_else(|_| file.to_path_buf());
    canon_file.strip_prefix(&canon_root).ok().map(Path::to_path_buf)
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
    let bf = &base.function;
    let bm = &base.module;
    let ba = &base.analysis;
    Thresholds {
        function: crate::thresholds::FunctionThresholds {
            cc_warning: f.cc_warning.unwrap_or(bf.cc_warning),
            cc_alert: f.cc_alert.unwrap_or(bf.cc_alert),
            cogc_warning: f.cogc_warning.unwrap_or(bf.cogc_warning),
            cogc_alert: f.cogc_alert.unwrap_or(bf.cogc_alert),
            fn_loc_warning: f.fn_loc_warning.unwrap_or(bf.fn_loc_warning),
            fn_loc_alert: f.fn_loc_alert.unwrap_or(bf.fn_loc_alert),
            nesting_depth: f.nesting_depth.unwrap_or(bf.nesting_depth),
            bump_count: f.bump_count.unwrap_or(bf.bump_count),
            arg_max: f.arg_max.unwrap_or(bf.arg_max),
            constructor_arg_max: f.constructor_arg_max.unwrap_or(bf.constructor_arg_max),
            compound_conditions: f.compound_conditions.unwrap_or(bf.compound_conditions),
            embedded_block_loc: f.embedded_block_loc.unwrap_or(bf.embedded_block_loc),
        },
        module: crate::thresholds::ModuleThresholds {
            file_loc_warning: m.file_loc_warning.unwrap_or(bm.file_loc_warning),
            file_loc_alert: m.file_loc_alert.unwrap_or(bm.file_loc_alert),
            file_function_count: m.file_function_count.unwrap_or(bm.file_function_count),
            file_total_cc: m.file_total_cc.unwrap_or(bm.file_total_cc),
            max_declarations: m.max_declarations.unwrap_or(bm.max_declarations),
            large_fn_loc: m.large_fn_loc.unwrap_or(bm.large_fn_loc),
            large_fn_count: m.large_fn_count.unwrap_or(bm.large_fn_count),
            max_struct_fields: m.max_struct_fields.unwrap_or(bm.max_struct_fields),
        },
        analysis: crate::thresholds::AnalysisThresholds {
            duplication_min_loc: a.duplication_min_loc.unwrap_or(ba.duplication_min_loc),
            skeleton_duplication_min_loc: a.skeleton_duplication_min_loc.unwrap_or(ba.skeleton_duplication_min_loc),
            duplication_min_group: a.duplication_min_group.unwrap_or(ba.duplication_min_group),
            consecutive_asserts_max: a.consecutive_asserts_max.unwrap_or(ba.consecutive_asserts_max),
            primitive_ratio_threshold: a.primitive_ratio_threshold.unwrap_or(ba.primitive_ratio_threshold),
            primitive_min_typed_params: a.primitive_min_typed_params.unwrap_or(ba.primitive_min_typed_params),
            lcom4_warning: a.lcom4_warning.unwrap_or(ba.lcom4_warning),
            short_var_min_fn_loc: a.short_var_min_fn_loc.unwrap_or(ba.short_var_min_fn_loc),
            short_var_max_count: a.short_var_max_count.unwrap_or(ba.short_var_max_count),
            max_string_match_arms: a.max_string_match_arms.unwrap_or(ba.max_string_match_arms),
        },
        audit: base.audit,
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
