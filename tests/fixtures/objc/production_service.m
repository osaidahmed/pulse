#import <Foundation/Foundation.h>

@protocol OrderDelegate <NSObject>
- (void)orderDidComplete:(NSString *)orderId;
@end

@implementation OrderService

- (instancetype)initWithDatabase:(id)db
                           cache:(id)cache
                          logger:(id)logger
                         metrics:(id)metrics
                        notifier:(id)notifier
                          config:(NSDictionary *)config {
    self = [super init];
    if (self) {
        _db = db;
        _cache = cache;
        _logger = logger;
        _metrics = metrics;
        _notifier = notifier;
        _config = config;
    }
    return self;
}

- (BOOL)processOrder:(NSDictionary *)order
              userId:(NSString *)userId
             options:(NSDictionary *)options {
    if (order == nil) {
        return NO;
    }
    if (userId == nil || [userId length] == 0) {
        return NO;
    }
    NSString *type = [order objectForKey:@"type"];
    if ([type isEqualToString:@"express"]) {
        if ([[order objectForKey:@"priority"] integerValue] > 5) {
            for (NSString *item in [order objectForKey:@"items"]) {
                if ([item length] > 0) {
                    if ([options objectForKey:@"validate"] != nil) {
                        for (NSString *rule in [options objectForKey:@"rules"]) {
                            if ([self applyRule:rule toItem:item]) {
                                NSLog(@"Rule %@ passed for %@", rule, item);
                            }
                        }
                    }
                }
            }
        } else if ([[order objectForKey:@"priority"] integerValue] > 3) {
            while ([[order objectForKey:@"retries"] integerValue] > 0) {
                if ([self retry:order]) {
                    break;
                }
            }
        }
    } else if ([type isEqualToString:@"standard"]) {
        switch ([[order objectForKey:@"category"] integerValue]) {
            case 1:
                if ([[order objectForKey:@"amount"] doubleValue] > 100.0) {
                    if ([options objectForKey:@"discount"] != nil) {
                        NSLog(@"applying discount");
                    }
                }
                break;
            case 2:
                if ([options objectForKey:@"bulk"] != nil) {
                    NSLog(@"bulk processing");
                }
                break;
            case 3:
                break;
            default:
                break;
        }
    } else if ([type isEqualToString:@"subscription"]) {
        if ([[order objectForKey:@"months"] integerValue] > 12) {
            NSLog(@"long subscription");
        }
    }
    return YES;
}

- (BOOL)applyRule:(NSString *)rule toItem:(NSString *)item {
    return YES;
}

- (BOOL)retry:(NSDictionary *)order {
    return YES;
}

- (NSDictionary *)fetchReport:(NSString *)reportId
                         from:(NSDate *)startDate
                           to:(NSDate *)endDate
                       format:(NSString *)format
                       filter:(NSString *)filter
                        limit:(NSInteger)limit {
    NSLog(@"Fetching report %@", reportId);
    return @{};
}

- (void)handleA {
    self.fieldA = @"a";
    NSLog(@"%@", self.fieldA);
}

- (void)handleB {
    self.fieldB = @"b";
    NSLog(@"%@", self.fieldB);
}

- (void)handleC {
    self.fieldC = @"c";
    NSLog(@"%@", self.fieldC);
}

- (void)handleD {
    self.fieldD = @"d";
    NSLog(@"%@", self.fieldD);
}

- (void)handleE {
    self.fieldE = @"e";
    NSLog(@"%@", self.fieldE);
}

- (void)handleF {
    self.fieldF = @"f";
    NSLog(@"%@", self.fieldF);
}

@end
