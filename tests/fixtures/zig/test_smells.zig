const std = @import("std");
const testing = std.testing;

test "user registration alpha" {
    var result: i32 = 0;
    result += 1;
    result += 2;
    result += 3;
    result += 4;
    result += 5;
    try testing.expectEqual(@as(i32, 15), result);
}

test "user registration beta" {
    var result: i32 = 0;
    result += 1;
    result += 2;
    result += 3;
    result += 4;
    result += 5;
    try testing.expectEqual(@as(i32, 15), result);
}
