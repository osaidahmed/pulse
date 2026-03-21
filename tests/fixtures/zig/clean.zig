const std = @import("std");

pub fn add(a: i32, b: i32) i32 {
    return a + b;
}

pub fn greet(name: []const u8) []const u8 {
    return name;
}

pub fn max(a: i32, b: i32) i32 {
    if (a > b) return a;
    return b;
}
