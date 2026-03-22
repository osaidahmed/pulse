/// Realistic Rust API service with intentional smells for testing.

struct AppConfig {
    host: String,
    port: u16,
    database_url: String,
    redis_url: String,
    jwt_secret: String,
    cors_origins: Vec<String>,
    log_level: String,
    max_connections: u32,
    timeout_secs: u64,
    retry_count: u32,
    cache_ttl: u64,
    rate_limit: u32,
    upload_dir: String,
}

fn dispatch_command(cmd: &str) -> &str {
    match cmd {
        "start" => "starting",
        "stop" => "stopping",
        "restart" => "restarting",
        "status" => "checking",
        "deploy" => "deploying",
        "rollback" => "rolling back",
        "migrate" => "migrating",
        "seed" => "seeding",
        _ => "unknown",
    }
}

fn process_event(data: &[u8]) -> i32 {
    let a = data.get(0).copied().unwrap_or(0);
    let b = data.get(1).copied().unwrap_or(0);
    let c = data.get(2).copied().unwrap_or(0);
    let d = data.get(3).copied().unwrap_or(0);
    let e = data.get(4).copied().unwrap_or(0);
    let f = data.get(5).copied().unwrap_or(0);
    let g = data.get(6).copied().unwrap_or(0);
    let h = data.get(7).copied().unwrap_or(0);
    if a > 100 {
        return -1;
    }
    if b > 200 {
        return -2;
    }
    let result = a as i32 + b as i32 + c as i32 + d as i32;
    let extra = e as i32 + f as i32 + g as i32 + h as i32;
    result + extra
}

fn clean_helper(x: i32, y: i32) -> i32 {
    x + y
}
