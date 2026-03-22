#import <Foundation/Foundation.h>

@implementation DeepProcessor

- (void)process:(NSArray *)items config:(NSDictionary *)config {
    if ([items count] > 0) {
        for (NSDictionary *item in items) {
            if ([item objectForKey:@"active"] != nil) {
                if ([[item objectForKey:@"priority"] integerValue] > 3) {
                    if ([config objectForKey:@"strict"] != nil) {
                        NSLog(@"processing %@", item);
                    }
                }
            }
        }
    }
}

@end
