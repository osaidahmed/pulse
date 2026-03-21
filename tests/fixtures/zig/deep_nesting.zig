const std = @import("std");

pub fn deeplyNested(items: []const u8, threshold: u8) u32 {
    var count: u32 = 0;
    for (items) |item| {
        if (item > threshold) {
            if (item < 200) {
                if (item != 128) {
                    if (count < 1000) {
                        count += 1;
                    }
                }
            }
        }
    }
    return count;
}

pub fn moderatelyNested(x: i32, y: i32) i32 {
    if (x > 0) {
        if (y > 0) {
            return x + y;
        }
    }
    return 0;
}
