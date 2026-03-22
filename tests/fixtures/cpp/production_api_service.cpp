struct AppConfig {
    std::string host;
    int port;
    std::string database_url;
    std::string redis_url;
    std::string jwt_secret;
    std::string log_level;
    int max_connections;
    int timeout_secs;
    int retry_count;
    int cache_ttl;
    int rate_limit;
    std::string upload_dir;
    std::string cors_origin;
};

class RequestHandler {
public:
    std::string dispatch(const std::string& action) {
        if (action == "create") return "creating";
        if (action == "delete") return "deleting";
        if (action == "update") return "updating";
        if (action == "reset") return "resetting";
        if (action == "notify") return "notifying";
        if (action == "export") return "exporting";
        return "unknown";
    }

    int processEvent(int* data) {
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
};
