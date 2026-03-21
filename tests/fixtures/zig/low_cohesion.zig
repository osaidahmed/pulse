const Self = @This();

const Service = struct {
    db_host: []const u8,
    db_port: u32,
    cache_host: []const u8,
    cache_port: u32,
    log_level: u8,
    log_path: []const u8,
    metric_host: []const u8,
    metric_port: u32,

    pub fn getDbHost(self: Service) []const u8 {
        return self.db_host;
    }
    pub fn getDbPort(self: Service) u32 {
        return self.db_port;
    }
    pub fn getCacheHost(self: Service) []const u8 {
        return self.cache_host;
    }
    pub fn getCachePort(self: Service) u32 {
        return self.cache_port;
    }
    pub fn getLogLevel(self: Service) u8 {
        return self.log_level;
    }
    pub fn getLogPath(self: Service) []const u8 {
        return self.log_path;
    }
    pub fn getMetricHost(self: Service) []const u8 {
        return self.metric_host;
    }
    pub fn getMetricPort(self: Service) u32 {
        return self.metric_port;
    }
};
