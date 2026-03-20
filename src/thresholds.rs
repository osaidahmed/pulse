pub struct Thresholds {
    // Function-level
    pub cc_warning: u32,
    pub cc_alert: u32,
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
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            cc_warning: 9,
            cc_alert: 15,
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
        }
    }
}
