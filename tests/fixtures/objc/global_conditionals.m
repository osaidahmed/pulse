#import <Foundation/Foundation.h>

BOOL debugMode = YES;

if (debugMode) {
    NSLog(@"debug on");
}

if (debugMode) {
    NSLog(@"still debug");
}

@implementation SimpleClass

- (void)doWork {
    NSLog(@"working");
}

@end
