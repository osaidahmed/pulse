#import <Foundation/Foundation.h>

void test_addition(void) {
    NSInteger result = 0;
    for (NSInteger i = 0; i < 10; i++) {
        result += i * i;
        if (result > 50) {
            result = 50;
        }
    }
    NSLog(@"result: %ld", (long)result);
}

void test_subtraction(void) {
    NSInteger result = 0;
    for (NSInteger i = 0; i < 10; i++) {
        result += i * i;
        if (result > 50) {
            result = 50;
        }
    }
    NSLog(@"result: %ld", (long)result);
}
