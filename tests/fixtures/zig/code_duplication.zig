const std = @import("std");

pub fn processAlpha(data: []const u8) u32 {
    var result: u32 = 0;
    for (data) |item| {
        if (item > 100) {
            result += 2;
        } else {
            result += 1;
        }
    }
    return result;
}

pub fn processBeta(data: []const u8) u32 {
    var result: u32 = 0;
    for (data) |item| {
        if (item > 100) {
            result += 2;
        } else {
            result += 1;
        }
    }
    return result;
}

pub fn processGamma(items: []const u8) u32 {
    var count: u32 = 0;
    for (items) |val| {
        if (val > 100) {
            count += 2;
        } else {
            count += 1;
        }
    }
    return count;
}
