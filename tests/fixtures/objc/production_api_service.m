@interface AppConfig : NSObject
@property NSString *host;
@property int port;
@property NSString *databaseUrl;
@property NSString *redisUrl;
@property NSString *jwtSecret;
@property NSString *logLevel;
@property int maxConnections;
@property int timeoutSecs;
@property int retryCount;
@property int cacheTtl;
@property int rateLimit;
@property NSString *uploadDir;
@property NSString *corsOrigin;
@end

@implementation AppConfig
@end

int dispatchCommand(NSString *cmd) {
    if ([cmd isEqualToString:@"start"]) return 1;
    if ([cmd isEqualToString:@"stop"]) return 2;
    if ([cmd isEqualToString:@"restart"]) return 3;
    if ([cmd isEqualToString:@"status"]) return 4;
    if ([cmd isEqualToString:@"deploy"]) return 5;
    if ([cmd isEqualToString:@"rollback"]) return 6;
    return 0;
}

int processEvent(int *data) {
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
