#import <Foundation/Foundation.h>

@implementation GodObject

- (void)handleName {
    self.name = @"test";
    NSLog(@"%@", self.name);
}

- (void)handleEmail {
    self.email = @"test@example.com";
    NSLog(@"%@", self.email);
}

- (void)handleAge {
    self.age = 25;
    NSLog(@"%ld", (long)self.age);
}

- (void)handleRole {
    self.role = @"admin";
    NSLog(@"%@", self.role);
}

- (void)handleDept {
    self.department = @"eng";
    NSLog(@"%@", self.department);
}

- (void)handleSalary {
    self.salary = 100000;
    NSLog(@"%f", self.salary);
}

@end
