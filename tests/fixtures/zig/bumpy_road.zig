const std = @import("std");

pub fn bumpyProcess(items: []const u8) i32 {
    var result: i32 = 0;
    if (items.len > 0) {
        if (items[0] > 0) {
            if (items[0] > 10) {
                result += 1;
            }
        }
    }
    result += 1;
    if (items.len > 5) {
        if (items[5] > 0) {
            if (items[5] > 10) {
                result += 2;
            }
        }
    }
    result += 2;
    if (items.len > 10) {
        if (items[10] > 0) {
            if (items[10] > 10) {
                result += 3;
            }
        }
    }
    return result;
}
