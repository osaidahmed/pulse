#import <Foundation/Foundation.h>

@implementation OrderProcessor

- (BOOL)processOrder:(NSDictionary *)order
              status:(NSInteger)status
              config:(NSDictionary *)config {
    if ([order objectForKey:@"id"] == nil) {
        return NO;
    }
    if (status == 1) {
        if ([[order objectForKey:@"priority"] integerValue] > 5) {
            for (NSString *item in [order objectForKey:@"items"]) {
                if ([item length] > 0) {
                    if ([config objectForKey:@"validate"] != nil) {
                        NSLog(@"Validating %@", item);
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
    } else if (status == 2) {
        switch ([[order objectForKey:@"type"] integerValue]) {
            case 1:
                if ([config objectForKey:@"fast"] != nil) {
                    NSLog(@"fast path");
                }
                break;
            case 2:
                break;
            case 3:
                break;
            default:
                break;
        }
    } else if (status == 3) {
        if ([[order objectForKey:@"amount"] doubleValue] > 100.0) {
            NSLog(@"large order");
        }
    }
    return YES;
}

- (BOOL)retry:(NSDictionary *)order {
    return YES;
}

@end
