#import <Foundation/Foundation.h>

@implementation MathHelper

- (NSInteger)computeA:(NSInteger)n {
    NSInteger result = 0;
    for (NSInteger i = 0; i < n; i++) {
        result += i * i;
        if (result > 1000) {
            result = 1000;
        }
    }
    return result;
}

- (NSInteger)computeB:(NSInteger)n {
    NSInteger result = 0;
    for (NSInteger i = 0; i < n; i++) {
        result += i * i;
        if (result > 1000) {
            result = 1000;
        }
    }
    return result;
}

@end
