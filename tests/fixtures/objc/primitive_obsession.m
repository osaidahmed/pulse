#import <Foundation/Foundation.h>

@implementation Renderer

- (void)draw:(NSInteger)x y:(NSInteger)y width:(NSInteger)w height:(NSInteger)h color:(NSInteger)c {
    NSLog(@"draw at %ld,%ld size %ldx%ld color %ld", (long)x, (long)y, (long)w, (long)h, (long)c);
}

@end
