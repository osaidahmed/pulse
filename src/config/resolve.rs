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

macro_rules! field_resolvers {
    ( $( fn $name:ident($Cfg:path => $Th:path) { $($field:ident <- $src:ident),+ $(,)? } )+ ) => {
        $(
            fn $name(over: &$Cfg, base: &$Th) -> $Th {
                $Th { $($field: over.$src.unwrap_or(base.$field)),+ }
            }
        )+
    };
}

field_resolvers! {
    fn resolve_clone_cluster(CloneClusterConfig => crate::thresholds::CloneClusterThresholds) {
        max_sim_threshold <- max_sim_threshold,
        min_cluster_size <- min_cluster_size,
        loc_window_pct <- loc_window_pct,
    }
    fn resolve_cpg(CpgConfig => crate::thresholds::CpgThresholds) {
        enabled <- enabled,
        dead_store <- dead_store,
        use_before_def <- use_before_def,
        unreachable_code <- unreachable_code,
        unused_result <- unused_result,
    }
    fn resolve_naturalness(NaturalnessConfig => crate::thresholds::NaturalnessThresholds) {
        enabled <- enabled,
        ngram_order <- ngram_order,
        cache_k <- cache_k,
        jm_gamma <- jm_gamma,
        min_fn_tokens <- min_fn_tokens,
        zscore_cutoff <- zscore_cutoff,
    }
    fn resolve_duplication(DuplicationThresholds => crate::thresholds::DuplicationThresholds) {
        min_loc <- duplication_min_loc,
        skeleton_min_loc <- skeleton_duplication_min_loc,
        min_group <- duplication_min_group,
        min_distinct_kinds <- duplication_min_distinct_kinds,
    }
}
