#[derive(Debug, Clone, PartialEq)]
pub struct Thresholds {
    // Function-level
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

    // Module-level
    pub file_loc_warning: u32,
    pub file_loc_alert: u32,
    pub file_function_count: u32,
    pub file_total_cc: u32,

    // Duplication
    pub duplication_min_loc: u32,
    pub skeleton_duplication_min_loc: u32,
    pub duplication_min_group: u32,

    // Declarations
    pub max_declarations: u32,

    // Overall function size
    pub large_fn_loc: u32,
    pub large_fn_count: u32,

    // Test-specific
    pub consecutive_asserts_max: u32,

    // Primitive obsession
    pub primitive_ratio_threshold: f32,
    pub primitive_min_typed_params: u32,

    // LCOM4
    pub lcom4_warning: u32,

    // Large struct
    pub max_struct_fields: u32,

    // Short variable names
    pub short_var_min_fn_loc: u32,
    pub short_var_max_count: u32,

    // Stringly-typed switch
    pub max_string_match_arms: u32,

    pub audit: AuditThresholds,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AuditThresholds {
    pub freqt_min_support: usize,
    pub subtree_min_depth: usize,
    pub subtree_min_nodes: usize,
    pub idiom_suppression_threshold: f64,
    pub max_findings_reported: usize,
    pub max_locations_per_finding: usize,
    pub martin_distance_warning: f64,
    pub martin_distance_alert: f64,
    pub include_tests_in_graph: bool,
    pub martin_cycle_min_size: u32,
    pub max_cycle_findings_reported: usize,
    pub max_martin_findings_reported: usize,
}

impl AuditThresholds {
    pub const DEFAULTS: Self = Self {
        freqt_min_support: 5,
        subtree_min_depth: 3,
        subtree_min_nodes: 5,
        idiom_suppression_threshold: 0.5,
        max_findings_reported: 50,
        max_locations_per_finding: 20,
        martin_distance_warning: 0.7,
        martin_distance_alert: 0.85,
        include_tests_in_graph: false,
        martin_cycle_min_size: 2,
        max_cycle_findings_reported: 50,
        max_martin_findings_reported: 100,
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
            cc_warning: 9,
            cc_alert: 18,
            cogc_warning: 15,
            cogc_alert: 25,
            fn_loc_warning: 65,
            fn_loc_alert: 100,
            nesting_depth: 4,
            bump_count: 2,
            arg_max: 5,
            constructor_arg_max: 5,
            compound_conditions: 2,
            embedded_block_loc: 15,
            file_loc_warning: 500,
            file_loc_alert: 700,
            file_function_count: 20,
            file_total_cc: 100,
            duplication_min_loc: 6,
            skeleton_duplication_min_loc: 20,
            duplication_min_group: 2,
            max_declarations: 20,
            large_fn_loc: 40,
            large_fn_count: 3,
            consecutive_asserts_max: 10,
            primitive_ratio_threshold: 0.7,
            primitive_min_typed_params: 4,
            lcom4_warning: 3,
            max_struct_fields: 12,
            short_var_min_fn_loc: 15,
            short_var_max_count: 3,
            max_string_match_arms: 5,
            audit: AuditThresholds::DEFAULTS,
        }
    }
}
