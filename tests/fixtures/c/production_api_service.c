struct AppConfig {
    char host[256];
    int port;
    char database_url[512];
    char redis_url[512];
    char jwt_secret[256];
    char log_level[32];
    int max_connections;
    int timeout_secs;
    int retry_count;
    int cache_ttl;
    int rate_limit;
    char upload_dir[256];
    char cors_origin[256];
};

int dispatch_command(const char* cmd) {
    if (strcmp(cmd, "start") == 0) return 1;
    if (strcmp(cmd, "stop") == 0) return 2;
    if (strcmp(cmd, "restart") == 0) return 3;
    if (strcmp(cmd, "status") == 0) return 4;
    if (strcmp(cmd, "deploy") == 0) return 5;
    if (strcmp(cmd, "rollback") == 0) return 6;
    return 0;
}

int process_event(int* data, int n) {
    int a = data[0];
    int b = data[1];
    int c = data[2];
    int d = data[3];
    int e = data[4];
    int f = data[5];
    int g = data[6];
    int h = data[7];
    if (a > 100) {
        return -1;
    }
    if (b > 200) {
        return -2;
    }
    int result = a + b + c + d;
    int extra = e + f + g + h;
    return result + extra;
}
