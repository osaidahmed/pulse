use std::collections::HashSet;
use std::path::{Path, PathBuf};

use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use serde::Deserialize;

use crate::audit::finding::{kind_slug, AuditFinding, AuditKind, PatternCategory};
use crate::smells::{self, Finding, Smell};

pub use crate::config_history::{
    combined_history_ignore_patterns, resolve_history_thresholds, HistoryCliOverrides, HistoryConfig,
};

mod audit_overrides;
mod resolve;
pub use audit_overrides::{CloneClusterConfig, NaturalnessConfig, PackageMetricsConfig, TaintConfig};
pub use resolve::{resolve_base_thresholds, resolve_thresholds};

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
    pub audit: AuditConfig,
    #[serde(default)]
    pub history: HistoryConfig,
    #[serde(default)]
    pub languages: std::collections::HashMap<String, ConfigThresholds>,
}

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
#[allow(clippy::struct_field_names)]
pub struct AuditConfig {
    #[serde(default)]
    pub hide_categories: Vec<String>,
    #[serde(default)]
    pub hide_smells: Vec<String>,
    #[serde(default)]
    pub hide_patterns: Vec<String>,
    #[serde(default)]
    pub cross_validate_history: Option<bool>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct ConfigThresholds {
    #[serde(flatten)]
    pub function: FunctionThresholds,
    #[serde(flatten)]
    pub module: ModuleThresholds,
    #[serde(flatten)]
    pub analysis: AnalysisThresholds,
    #[serde(flatten)]
    pub duplication: DuplicationThresholds,
    #[serde(default)]
    pub cpg: CpgConfig,
    #[serde(default)]
    pub package_metrics: PackageMetricsConfig,
    #[serde(default)]
    pub taint: TaintConfig,
    #[serde(default)]
    pub clone_cluster: CloneClusterConfig,
    #[serde(default)]
    pub naturalness: NaturalnessConfig,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct DuplicationThresholds {
    pub duplication_min_loc: Option<u32>,
    pub skeleton_duplication_min_loc: Option<u32>,
    pub duplication_min_group: Option<u32>,
    pub duplication_min_distinct_kinds: Option<u32>,
}

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct CpgConfig {
    pub enabled: Option<bool>,
    pub dead_store: Option<bool>,
    pub use_before_def: Option<bool>,
    pub unreachable_code: Option<bool>,
    pub unused_result: Option<bool>,
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
    pub global_conditionals_max: Option<u32>,
    pub global_nesting_depth: Option<u32>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct AnalysisThresholds {
    pub consecutive_asserts_max: Option<u32>,
    pub primitive_ratio_threshold: Option<f32>,
    pub primitive_min_typed_params: Option<u32>,
    pub primitive_min_same_count: Option<u32>,
    pub constructor_dep_injection_min: Option<u32>,
    pub lcom4_warning: Option<u32>,
    pub short_var_min_fn_loc: Option<u32>,
    pub short_var_max_count: Option<u32>,
    pub max_string_match_arms: Option<u32>,
    pub dup_assert_min: Option<u32>,
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

pub struct AuditSuppression {
    categories: HashSet<String>,
    smells: HashSet<String>,
    patterns: GlobSet,
}

impl AuditSuppression {
    pub fn new() -> Self {
        Self { categories: HashSet::new(), smells: HashSet::new(), patterns: GlobSet::empty() }
    }

    pub fn from_config(cfg: Option<&AuditConfig>) -> Self {
        let Some(cfg) = cfg else { return Self::new() };
        let mut builder = GlobSetBuilder::new();
        for raw in &cfg.hide_patterns {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Ok(glob) = GlobBuilder::new(trimmed).literal_separator(false).build() {
                builder.add(glob);
            }
        }
        let patterns = builder.build().unwrap_or_else(|_| GlobSet::empty());
        Self {
            categories: cfg.hide_categories.iter().map(|s| s.trim().to_string()).collect(),
            smells: cfg.hide_smells.iter().map(|s| s.trim().to_string()).collect(),
            patterns,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.categories.is_empty() && self.smells.is_empty() && self.patterns.is_empty()
    }

    pub fn is_hidden(&self, f: &AuditFinding) -> bool {
        match f.kind {
            AuditKind::UncategorizedPattern { .. } => pattern_hidden(self, f),
            _ => self.smells.contains(kind_slug(&f.kind)),
        }
    }
}

fn pattern_hidden(s: &AuditSuppression, f: &AuditFinding) -> bool {
    let category_hit = f.pattern_category.is_some_and(|cat| s.categories.contains(PatternCategory::slug(cat)));
    category_hit || glob_matches_text(&s.patterns, &f.representative_snippet)
}

fn glob_matches_text(set: &GlobSet, candidate: &str) -> bool {
    !set.is_empty() && set.is_match(candidate)
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
    let mut dir = if start.is_file() { start.parent()? } else { start };
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
    let Some(root) = find_config(file_path).and_then(|p| p.parent().map(Path::to_path_buf)) else {
        return false;
    };
    is_ignored_with_root(cfg, &root, file_path)
}

pub fn is_ignored_with_root(cfg: &PulseConfig, root: &Path, file_path: &Path) -> bool {
    IgnoreMatcher::from_patterns(&cfg.ignore.paths).matches_file(root, file_path)
}

fn relative_path(root: &Path, file: &Path) -> Option<PathBuf> {
    let canon_root = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let canon_file = std::fs::canonicalize(file).unwrap_or_else(|_| file.to_path_buf());
    canon_file.strip_prefix(&canon_root).ok().map(Path::to_path_buf)
}

pub fn resolve_disabled(config: Option<&PulseConfig>) -> HashSet<Smell> {
    let Some(config) = config else {
        return HashSet::new();
    };
    config.disable.smells.iter().filter_map(|s| smells::smell_from_snake_case(s)).collect()
}

pub fn filter_disabled(findings: &mut Vec<Finding>, disabled: &HashSet<Smell>) {
    if !disabled.is_empty() {
        findings.retain(|f| !disabled.contains(&f.smell));
    }
}
