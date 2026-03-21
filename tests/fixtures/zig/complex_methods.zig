const std = @import("std");

pub fn processOrder(
    status: u8,
    priority: u8,
    amount: i32,
    discount: i32,
    tax: i32,
) i32 {
    var result: i32 = 0;
    if (status == 1) {
        if (priority > 5) {
            result = amount * 2;
        } else if (priority > 3) {
            result = amount;
        } else {
            result = amount / 2;
        }
    } else if (status == 2) {
        if (discount > 0) {
            result = amount - discount;
        }
    } else if (status == 3) {
        result = switch (priority) {
            1 => amount + tax,
            2 => amount - tax,
            3 => amount,
            else => 0,
        };
    }
    if (result < 0) {
        result = 0;
    }
    return result;
}

pub fn simpleHelper(x: i32) i32 {
    return x + 1;
}
