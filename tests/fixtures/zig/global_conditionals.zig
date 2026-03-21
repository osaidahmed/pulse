const std = @import("std");
const builtin = @import("builtin");

var global_state: i32 = 0;

pub fn init() void {
    global_state = 1;
}

// Top-level conditional logic
fn setup() void {
    if (global_state > 0) {
        if (global_state < 100) {
            global_state += 1;
        }
    }
}
