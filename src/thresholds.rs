use crate::history::thresholds::HistoryThresholds;

mod arch;
mod features;
pub use arch::{CommunityThresholds, PageRankThresholds};
pub use features::{CloneClusterThresholds, CpgThresholds, NaturalnessThresholds};

#[derive(Debug, Clone, PartialEq)]
pub struct Thresholds {
    pub function: FunctionThresholds,
    pub module: ModuleThresholds,
    pub analysis: AnalysisThresholds,
    pub audit: AuditThresholds,
    pub history: HistoryThresholds,
    pub cpg: CpgThresholds,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FunctionThresholds {
    pub cc_warning: u32,
    pub cc_alert: u32,
    pub cogc_warning: u32,
    pub cogc_alert: u32,
    pub fn_loc_warning: u32,
    pub fn_loc_alert: u32,
    pub nesting_depth: u32,
    pub bump_count: u32,
    pub arg_max: u32,
    pub constructor_arg_max: u32,
    pub compound_conditions: u32,
    pub embedded_block_loc: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModuleThresholds {
    pub file_loc_warning: u32,
    pub file_loc_alert: u32,
    pub file_function_count: u32,
    pub file_total_cc: u32,
    pub max_declarations: u32,
    pub large_fn_loc: u32,
    pub large_fn_count: u32,
    pub max_struct_fields: u32,
    pub global_conditionals_max: u32,
    pub global_nesting_depth: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnalysisThresholds {
    pub duplication: DuplicationThresholds,
    pub consecutive_asserts_max: u32,
    pub primitive_ratio_threshold: f32,
    pub primitive_min_typed_params: u32,
    pub primitive_min_same_count: u32,
    pub constructor_dep_injection_min: u32,
    pub lcom4_warning: u32,
    pub short_var_min_fn_loc: u32,
    pub short_var_max_count: u32,
    pub max_string_match_arms: u32,
    pub dup_assert_min: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DuplicationThresholds {
    pub min_loc: u32,
    pub skeleton_min_loc: u32,
    pub min_group: u32,
    pub min_distinct_kinds: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AuditThresholds {
    pub pattern_mining: PatternMiningThresholds,
    pub package_metrics: PackageMetricsThresholds,
    pub named_smells: NamedSmellThresholds,
    pub taint: TaintThresholds,
    pub clone_cluster: CloneClusterThresholds,
    pub naturalness: NaturalnessThresholds,
    pub max_locations_per_finding: usize,
    pub cross_validate_history: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TaintThresholds {
    pub visit_cap: u32,
    pub max_depth: u32,
    pub max_findings: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PatternMiningThresholds {
    pub freqt_min_support: usize,
    pub subtree_min_depth: usize,
    pub subtree_min_nodes: usize,
    pub idiom_suppression_threshold: f64,
    pub max_findings_reported: usize,
    pub vendor: VendorThresholds,
    pub complexity: ComplexityFloorThresholds,
    pub mdl: MdlThresholds,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VendorThresholds {
    pub zscore_cutoff: f64,
    pub min_features_failed: u32,
    pub min_size_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComplexityFloorThresholds {
    pub distinct_kind_floor: u32,
    pub branching_factor_floor: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdlThresholds {
    pub compression_floor_bits: f64,
    pub max_iterations: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PackageMetricsThresholds {
    pub martin_distance_warning: f64,
    pub martin_distance_alert: f64,
    pub include_tests_in_graph: bool,
    pub martin_cycle_min_size: u32,
    pub max_cycle_findings_reported: usize,
    pub max_martin_findings_reported: usize,
    pub unstable_dep_strength: f64,
    pub hublike_imbalance_ratio: f64,
    pub god_component_loc_percentile: f64,
    pub pagerank: PageRankThresholds,
    pub community: CommunityThresholds,
    pub max_arch_findings_reported: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NamedSmellThresholds {
    pub shotgun_surgery: ShotgunSurgeryThresholds,
    pub divergent_change: DivergentChangeThresholds,
    pub feature_envy: FeatureEnvyThresholds,
    pub god_class: GodClassThresholds,
    pub parallel_inheritance: ParallelInheritanceThresholds,
    pub refused_bequest: RefusedBequestThresholds,
    pub max_findings_reported: usize,
    pub max_caller_samples_per_finding: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShotgunSurgeryThresholds {
    pub changing_classes: u32,
    pub changing_methods: u32,
    pub fanout: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DivergentChangeThresholds {
    pub changing_classes: u32,
    pub fanout: u32,
    pub method_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FeatureEnvyThresholds {
    pub atfd: u32,
    pub foreign_ratio: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GodClassThresholds {
    pub wmc: u32,
    pub tcc: f64,
    pub atfd: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParallelInheritanceThresholds {
    pub min_overlap: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefusedBequestThresholds {
    pub max_override_ratio: f64,
    pub min_parent_methods: u32,
}

impl FunctionThresholds {
    pub const DEFAULTS: Self = Self {
        cc_warning: 9,
        cc_alert: 18,
        cogc_warning: 15,
        cogc_alert: 25,
        fn_loc_warning: 65,
        fn_loc_alert: 100,
        nesting_depth: 4,
        bump_count: 2,
        arg_max: 5,
        constructor_arg_max: 8,
        compound_conditions: 2,
        embedded_block_loc: 15,
    };
}

impl ModuleThresholds {
    pub const DEFAULTS: Self = Self {
        file_loc_warning: 500,
        file_loc_alert: 700,
        file_function_count: 20,
        file_total_cc: 100,
        max_declarations: 20,
        large_fn_loc: 40,
        large_fn_count: 3,
        max_struct_fields: 12,
        global_conditionals_max: 0,
        global_nesting_depth: 3,
    };
}

impl AnalysisThresholds {
    pub const DEFAULTS: Self = Self {
        duplication: DuplicationThresholds::DEFAULTS,
        consecutive_asserts_max: 10,
        primitive_ratio_threshold: 0.7,
        primitive_min_typed_params: 4,
        primitive_min_same_count: 2,
        constructor_dep_injection_min: 4,
        lcom4_warning: 3,
        short_var_min_fn_loc: 15,
        short_var_max_count: 3,
        max_string_match_arms: 5,
        dup_assert_min: 5,
    };
}

impl DuplicationThresholds {
    pub const DEFAULTS: Self = Self { min_loc: 6, skeleton_min_loc: 20, min_group: 2, min_distinct_kinds: 3 };
}

impl PatternMiningThresholds {
    pub const DEFAULTS: Self = Self {
        freqt_min_support: 5,
        subtree_min_depth: 3,
        subtree_min_nodes: 5,
        idiom_suppression_threshold: 0.5,
        max_findings_reported: 25,
        vendor: VendorThresholds::DEFAULTS,
        complexity: ComplexityFloorThresholds::DEFAULTS,
        mdl: MdlThresholds::DEFAULTS,
    };
}

impl VendorThresholds {
    pub const DEFAULTS: Self = Self { zscore_cutoff: 3.0, min_features_failed: 2, min_size_bytes: 200 };
}

impl ComplexityFloorThresholds {
    pub const DEFAULTS: Self = Self { distinct_kind_floor: 2, branching_factor_floor: 0.5 };
}

impl MdlThresholds {
    pub const DEFAULTS: Self = Self { compression_floor_bits: 0.0, max_iterations: 1000 };
}

impl PackageMetricsThresholds {
    pub const DEFAULTS: Self = Self {
        martin_distance_warning: 0.7,
        martin_distance_alert: 0.85,
        include_tests_in_graph: false,
        martin_cycle_min_size: 2,
        max_cycle_findings_reported: 50,
        max_martin_findings_reported: 100,
        unstable_dep_strength: 0.30,
        hublike_imbalance_ratio: 0.25,
        god_component_loc_percentile: 0.90,
        pagerank: PageRankThresholds::DEFAULTS,
        community: CommunityThresholds::DEFAULTS,
        max_arch_findings_reported: 50,
    };
}

impl ShotgunSurgeryThresholds {
    pub const DEFAULTS: Self = Self { changing_classes: 5, changing_methods: 10, fanout: 5 };
}

impl DivergentChangeThresholds {
    pub const DEFAULTS: Self = Self { changing_classes: 6, fanout: 6, method_count: 5 };
}

impl FeatureEnvyThresholds {
    pub const DEFAULTS: Self = Self { atfd: 5, foreign_ratio: 0.6 };
}

impl GodClassThresholds {
    pub const DEFAULTS: Self = Self { wmc: 47, tcc: 0.33, atfd: 5 };
}

impl ParallelInheritanceThresholds {
    pub const DEFAULTS: Self = Self { min_overlap: 3 };
}

impl RefusedBequestThresholds {
    pub const DEFAULTS: Self = Self { max_override_ratio: 0.3, min_parent_methods: 3 };
}

impl NamedSmellThresholds {
    pub const DEFAULTS: Self = Self {
        shotgun_surgery: ShotgunSurgeryThresholds::DEFAULTS,
        divergent_change: DivergentChangeThresholds::DEFAULTS,
        feature_envy: FeatureEnvyThresholds::DEFAULTS,
        god_class: GodClassThresholds::DEFAULTS,
        parallel_inheritance: ParallelInheritanceThresholds::DEFAULTS,
        refused_bequest: RefusedBequestThresholds::DEFAULTS,
        max_findings_reported: 50,
        max_caller_samples_per_finding: 20,
    };
}

impl TaintThresholds {
    pub const DEFAULTS: Self = Self { visit_cap: 2, max_depth: 16, max_findings: 50 };
}

impl AuditThresholds {
    pub const DEFAULTS: Self = Self {
        pattern_mining: PatternMiningThresholds::DEFAULTS,
        package_metrics: PackageMetricsThresholds::DEFAULTS,
        named_smells: NamedSmellThresholds::DEFAULTS,
        taint: TaintThresholds::DEFAULTS,
        clone_cluster: CloneClusterThresholds::DEFAULTS,
        naturalness: NaturalnessThresholds::DEFAULTS,
        max_locations_per_finding: 10,
        cross_validate_history: false,
    };
}

impl Default for AuditThresholds {
    fn default() -> Self {
        Self::DEFAULTS
    }
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            function: FunctionThresholds::DEFAULTS,
            module: ModuleThresholds::DEFAULTS,
            analysis: AnalysisThresholds::DEFAULTS,
            audit: AuditThresholds::DEFAULTS,
            history: HistoryThresholds::DEFAULTS,
            cpg: CpgThresholds::DEFAULTS,
        }
    }
}
