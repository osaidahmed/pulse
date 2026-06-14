use crate::parse::Language;
use crate::thresholds::{
    AuditThresholds, CloneClusterThresholds, NaturalnessThresholds, PackageMetricsThresholds, TaintThresholds,
    Thresholds,
};

use super::{
    CloneClusterConfig, ConfigThresholds, CpgConfig, DuplicationThresholds, NaturalnessConfig, PackageMetricsConfig,
    PulseConfig, TaintConfig,
};

pub fn resolve_thresholds(config: Option<&PulseConfig>, lang: Language) -> Thresholds {
    let base = Thresholds::default();
    let Some(config) = config else { return base };
    let merged = apply_overrides(&base, &config.thresholds);
    config
        .languages
        .get(lang.to_config_key())
        .map_or(merged.clone(), |lang_overrides| apply_overrides(&merged, lang_overrides))
}

pub fn resolve_base_thresholds(config: Option<&PulseConfig>) -> Thresholds {
    let base = Thresholds::default();
    let Some(config) = config else { return base };
    let mut t = apply_overrides(&base, &config.thresholds);
    t.audit.cross_validate_history = config.audit.cross_validate_history.unwrap_or(t.audit.cross_validate_history);
    t
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
            global_conditionals_max: m.global_conditionals_max.unwrap_or(bm.global_conditionals_max),
            global_nesting_depth: m.global_nesting_depth.unwrap_or(bm.global_nesting_depth),
        },
        analysis: crate::thresholds::AnalysisThresholds {
            duplication: resolve_duplication(&o.duplication, &base.analysis.duplication),
            consecutive_asserts_max: a.consecutive_asserts_max.unwrap_or(ba.consecutive_asserts_max),
            primitive_ratio_threshold: a.primitive_ratio_threshold.unwrap_or(ba.primitive_ratio_threshold),
            primitive_min_typed_params: a.primitive_min_typed_params.unwrap_or(ba.primitive_min_typed_params),
            primitive_min_same_count: a.primitive_min_same_count.unwrap_or(ba.primitive_min_same_count),
            constructor_dep_injection_min: a.constructor_dep_injection_min.unwrap_or(ba.constructor_dep_injection_min),
            framework_dep_injection_min: a.framework_dep_injection_min.unwrap_or(ba.framework_dep_injection_min),
            lcom4_warning: a.lcom4_warning.unwrap_or(ba.lcom4_warning),
            short_var_min_fn_loc: a.short_var_min_fn_loc.unwrap_or(ba.short_var_min_fn_loc),
            short_var_max_count: a.short_var_max_count.unwrap_or(ba.short_var_max_count),
            max_string_match_arms: a.max_string_match_arms.unwrap_or(ba.max_string_match_arms),
            dup_assert_min: a.dup_assert_min.unwrap_or(ba.dup_assert_min),
        },
        audit: resolve_audit(&base.audit, o),
        history: base.history,
        cpg: resolve_cpg(&o.cpg, &base.cpg),
    }
}

fn resolve_audit(base: &AuditThresholds, o: &ConfigThresholds) -> AuditThresholds {
    AuditThresholds {
        package_metrics: resolve_package_metrics(&o.package_metrics, &base.package_metrics),
        taint: resolve_taint(&o.taint, &base.taint),
        clone_cluster: resolve_clone_cluster(&o.clone_cluster, &base.clone_cluster),
        naturalness: resolve_naturalness(&o.naturalness, &base.naturalness),
        ..*base
    }
}

fn resolve_package_metrics(o: &PackageMetricsConfig, base: &PackageMetricsThresholds) -> PackageMetricsThresholds {
    PackageMetricsThresholds {
        martin_distance_warning: o.martin_distance_warning.unwrap_or(base.martin_distance_warning),
        martin_distance_alert: o.martin_distance_alert.unwrap_or(base.martin_distance_alert),
        include_tests_in_graph: o.include_tests_in_graph.unwrap_or(base.include_tests_in_graph),
        martin_cycle_min_size: o.martin_cycle_min_size.unwrap_or(base.martin_cycle_min_size),
        max_cycle_findings_reported: o.max_cycle_findings_reported.unwrap_or(base.max_cycle_findings_reported),
        max_martin_findings_reported: o.max_martin_findings_reported.unwrap_or(base.max_martin_findings_reported),
        unstable_dep_strength: o.unstable_dep_strength.unwrap_or(base.unstable_dep_strength),
        hublike_imbalance_ratio: o.hublike_imbalance_ratio.unwrap_or(base.hublike_imbalance_ratio),
        god_component_loc_percentile: o.god_component_loc_percentile.unwrap_or(base.god_component_loc_percentile),
        pagerank: base.pagerank,
        community: base.community,
        max_arch_findings_reported: o.max_arch_findings_reported.unwrap_or(base.max_arch_findings_reported),
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
    fn resolve_cpg(CpgConfig => crate::thresholds::CpgThresholds) {
        enabled <- enabled,
        dead_store <- dead_store,
        use_before_def <- use_before_def,
        unreachable_code <- unreachable_code,
    }
    fn resolve_duplication(DuplicationThresholds => crate::thresholds::DuplicationThresholds) {
        min_loc <- duplication_min_loc,
        skeleton_min_loc <- skeleton_duplication_min_loc,
        min_group <- duplication_min_group,
        min_distinct_kinds <- duplication_min_distinct_kinds,
    }
    fn resolve_taint(TaintConfig => TaintThresholds) {
        visit_cap <- visit_cap,
        max_depth <- max_depth,
        max_findings <- max_findings,
    }
    fn resolve_clone_cluster(CloneClusterConfig => CloneClusterThresholds) {
        max_sim_threshold <- max_sim_threshold,
        min_cluster_size <- min_cluster_size,
        loc_window_pct <- loc_window_pct,
        min_loc <- min_loc,
    }
    fn resolve_naturalness(NaturalnessConfig => NaturalnessThresholds) {
        enabled <- enabled,
        ngram_order <- ngram_order,
        cache_k <- cache_k,
        jm_gamma <- jm_gamma,
        min_fn_tokens <- min_fn_tokens,
        zscore_cutoff <- zscore_cutoff,
    }
}
