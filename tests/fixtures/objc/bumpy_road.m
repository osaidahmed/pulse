#import <Foundation/Foundation.h>

@implementation Validator

- (BOOL)validate:(NSDictionary *)data config:(NSDictionary *)config {
    if ([data objectForKey:@"name"] != nil) {
        if ([[data objectForKey:@"name"] length] > 0) {
            if ([config objectForKey:@"strictName"] != nil) {
                NSLog(@"strict name check");
            }
        }
    }

    NSLog(@"gap");

    if ([data objectForKey:@"email"] != nil) {
        if ([[data objectForKey:@"email"] length] > 0) {
            if ([config objectForKey:@"strictEmail"] != nil) {
                NSLog(@"strict email check");
            }
        }
    }

    NSLog(@"gap");

    if ([data objectForKey:@"age"] != nil) {
        if ([[data objectForKey:@"age"] integerValue] > 0) {
            if ([config objectForKey:@"strictAge"] != nil) {
                NSLog(@"strict age check");
            }
        }
    }

    return YES;
}

@end
