class AppConfig {
    String host;
    int port;
    String databaseUrl;
    String redisUrl;
    String jwtSecret;
    String logLevel;
    int maxConnections;
    int timeoutSecs;
    int retryCount;
    int cacheTtl;
    int rateLimit;
    String uploadDir;
    String corsOrigin;
}

class RequestHandler {
    String dispatch(String action) {
        switch (action) {
        case "create":
            return "creating";
        case "delete":
            return "deleting";
        case "update":
            return "updating";
        case "reset":
            return "resetting";
        case "notify":
            return "notifying";
        case "export":
            return "exporting";
        default:
            return "unknown";
        }
    }

    int processEvent(int[] data) {
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
}
