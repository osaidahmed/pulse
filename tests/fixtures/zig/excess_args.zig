const std = @import("std");

pub fn createUser(
    name: []const u8,
    email: []const u8,
    age: u32,
    role: u8,
    active: bool,
    score: i32,
    level: u8,
    department: []const u8,
) void {
    _ = name;
    _ = email;
    _ = age;
    _ = role;
    _ = active;
    _ = score;
    _ = level;
    _ = department;
}

pub fn simpleFunc(a: i32, b: i32) i32 {
    return a + b;
}

const UserService = struct {
    name: []const u8,
    email: []const u8,
    db: u32,
    cache: u32,
    logger: u32,
    config: u32,

    pub fn init(
        name: []const u8,
        email: []const u8,
        db: u32,
        cache: u32,
        logger: u32,
        config: u32,
    ) UserService {
        return .{
            .name = name,
            .email = email,
            .db = db,
            .cache = cache,
            .logger = logger,
            .config = config,
        };
    }
};
