use crate::parse::Language;
use crate::thresholds::Thresholds;

use super::{
    CloneClusterConfig, ConfigThresholds, CpgConfig, DuplicationThresholds, NaturalnessConfig,
    PulseConfig,
};

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
            duplication: resolve_duplication(&o.duplication, &base.analysis.duplication),
            clone_cluster: resolve_clone_cluster(&o.clone_cluster, &base.analysis.clone_cluster),
            consecutive_asserts_max: a
                .consecutive_asserts_max
                .unwrap_or(ba.consecutive_asserts_max),
            primitive_ratio_threshold: a
                .primitive_ratio_threshold
                .unwrap_or(ba.primitive_ratio_threshold),
            primitive_min_typed_params: a
                .primitive_min_typed_params
                .unwrap_or(ba.primitive_min_typed_params),
            primitive_min_same_count: a
                .primitive_min_same_count
                .unwrap_or(ba.primitive_min_same_count),
            constructor_dep_injection_min: a
                .constructor_dep_injection_min
                .unwrap_or(ba.constructor_dep_injection_min),
            lcom4_warning: a.lcom4_warning.unwrap_or(ba.lcom4_warning),
            short_var_min_fn_loc: a.short_var_min_fn_loc.unwrap_or(ba.short_var_min_fn_loc),
            short_var_max_count: a.short_var_max_count.unwrap_or(ba.short_var_max_count),
            max_string_match_arms: a.max_string_match_arms.unwrap_or(ba.max_string_match_arms),
        },
        audit: base.audit,
        history: base.history,
        cpg: resolve_cpg(&o.cpg, &base.cpg),
        naturalness: resolve_naturalness(&o.naturalness, &base.naturalness),
    }
}

fn resolve_clone_cluster(
    over: &CloneClusterConfig,
    base: &crate::thresholds::CloneClusterThresholds,
) -> crate::thresholds::CloneClusterThresholds {
    crate::thresholds::CloneClusterThresholds {
        max_sim_threshold: over.max_sim_threshold.unwrap_or(base.max_sim_threshold),
        min_cluster_size: over.min_cluster_size.unwrap_or(base.min_cluster_size),
        loc_window_pct: over.loc_window_pct.unwrap_or(base.loc_window_pct),
    }
}

fn resolve_cpg(
    over: &CpgConfig,
    base: &crate::thresholds::CpgThresholds,
) -> crate::thresholds::CpgThresholds {
    crate::thresholds::CpgThresholds {
        enabled: over.enabled.unwrap_or(base.enabled),
        taint_visit_cap: over.taint_visit_cap.unwrap_or(base.taint_visit_cap),
        taint_max_depth: over.taint_max_depth.unwrap_or(base.taint_max_depth),
        dead_store: over.dead_store.unwrap_or(base.dead_store),
        use_before_def: over.use_before_def.unwrap_or(base.use_before_def),
        unreachable_code: over.unreachable_code.unwrap_or(base.unreachable_code),
        unused_result: over.unused_result.unwrap_or(base.unused_result),
    }
}

fn resolve_naturalness(
    over: &NaturalnessConfig,
    base: &crate::thresholds::NaturalnessThresholds,
) -> crate::thresholds::NaturalnessThresholds {
    crate::thresholds::NaturalnessThresholds {
        enabled: over.enabled.unwrap_or(base.enabled),
        ngram_order: over.ngram_order.unwrap_or(base.ngram_order),
        cache_k: over.cache_k.unwrap_or(base.cache_k),
        jm_gamma: over.jm_gamma.unwrap_or(base.jm_gamma),
        min_fn_tokens: over.min_fn_tokens.unwrap_or(base.min_fn_tokens),
        zscore_cutoff: over.zscore_cutoff.unwrap_or(base.zscore_cutoff),
    }
}

fn resolve_duplication(
    over: &DuplicationThresholds,
    base: &crate::thresholds::DuplicationThresholds,
) -> crate::thresholds::DuplicationThresholds {
    crate::thresholds::DuplicationThresholds {
        min_loc: over.duplication_min_loc.unwrap_or(base.min_loc),
        skeleton_min_loc: over.skeleton_duplication_min_loc.unwrap_or(base.skeleton_min_loc),
        min_group: over.duplication_min_group.unwrap_or(base.min_group),
        min_distinct_kinds: over.duplication_min_distinct_kinds.unwrap_or(base.min_distinct_kinds),
    }
}
