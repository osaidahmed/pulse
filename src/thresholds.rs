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
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            cc_warning: 9,
            cc_alert: 15,
            cogc_warning: 15,
            cogc_alert: 25,
            fn_loc_warning: 50,
            fn_loc_alert: 100,
            nesting_depth: 4,
            bump_count: 2,
            arg_max: 5,
            constructor_arg_max: 5,
            compound_conditions: 2,
            embedded_block_loc: 15,
            file_loc_warning: 400,
            file_loc_alert: 700,
            file_function_count: 20,
            file_total_cc: 100,
            duplication_min_loc: 6,
            duplication_min_group: 2,
            max_declarations: 20,
            large_fn_loc: 40,
            large_fn_count: 3,
            consecutive_asserts_max: 10,
            primitive_ratio_threshold: 0.7,
            primitive_min_typed_params: 4,
            lcom4_warning: 3,
        }
    }
}
