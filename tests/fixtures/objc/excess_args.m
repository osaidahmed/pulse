#import <Foundation/Foundation.h>

@implementation UserService

- (BOOL)createUser:(NSString *)name
             email:(NSString *)email
               age:(NSInteger)age
              role:(NSString *)role
        department:(NSString *)department
            salary:(double)salary {
    NSLog(@"Creating user %@", name);
    return YES;
}

- (instancetype)initWithName:(NSString *)name
                       email:(NSString *)email
                         age:(NSInteger)age
                        role:(NSString *)role
                  department:(NSString *)department
                      salary:(double)salary
                      extra7:(id)extra7
                      extra8:(id)extra8
                      extra9:(id)extra9 {
    self = [super init];
    if (self) {
        _name = name;
    }
    return self;
}

- (NSString *)getName {
    return _name;
}

@end
