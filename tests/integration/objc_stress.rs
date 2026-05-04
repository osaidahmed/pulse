
use crate::common::*;

lang_helpers!("m");

// ===========================================================================
// CC counting (16)
// ===========================================================================

#[test]
fn cc_counts_if() {
    let out = debug("@implementation X\n- (void)f:(int)x {\n    if (x > 0) {}\n}\n@end\n");
    assert_eq!(function_metric(&out, "X.f", "cc"), Some(2));
}

#[test]
fn cc_counts_else_if() {
    let out = debug(
        "@implementation X\n- (void)f:(int)x {\n    if (x > 0) {\n    } else if (x < 0) {\n    }\n}\n@end\n",
    );
    assert_eq!(function_metric(&out, "X.f", "cc"), Some(3));
}

#[test]
fn cc_counts_for() {
    let out = debug(
        "@implementation X\n- (void)f {\n    for (int i = 0; i < 10; i++) {}\n}\n@end\n",
    );
    assert_eq!(function_metric(&out, "X.f", "cc"), Some(2));
}

#[test]
fn cc_counts_for_in() {
    let out = debug(
        "@implementation X\n- (void)f:(NSArray *)items {\n    for (id obj in items) {}\n}\n@end\n",
    );
    assert_eq!(function_metric(&out, "X.f", "cc"), Some(2));
}

#[test]
fn cc_counts_while() {
    let out = debug(
        "@implementation X\n- (void)f:(int)x {\n    while (x > 0) { x--; }\n}\n@end\n",
    );
    assert_eq!(function_metric(&out, "X.f", "cc"), Some(2));
}

#[test]
fn cc_counts_do_while() {
    let out = debug(
        "@implementation X\n- (void)f:(int)x {\n    do { x--; } while (x > 0);\n}\n@end\n",
    );
    assert_eq!(function_metric(&out, "X.f", "cc"), Some(2));
}

#[test]
fn cc_counts_switch_cases() {
    let out = debug(concat!(
        "@implementation X\n",
        "- (void)f:(int)x {\n",
        "    switch (x) {\n",
        "        case 1: break;\n",
        "        case 2: break;\n",
        "        case 3: break;\n",
        "        default: break;\n",
        "    }\n",
        "}\n",
        "@end\n"
    ));
    assert_eq!(function_metric(&out, "X.f", "cc"), Some(4));
}

#[test]
fn cc_counts_ternary() {
    let out = debug(
        "@implementation X\n- (int)f:(int)x {\n    return x > 0 ? 1 : 0;\n}\n@end\n",
    );
    assert_eq!(function_metric(&out, "X.f", "cc"), Some(2));
}

#[test]
fn cc_counts_logical_and() {
    let out = debug(
        "@implementation X\n- (void)f:(BOOL)a b:(BOOL)b {\n    if (a && b) {}\n}\n@end\n",
    );
    assert_eq!(function_metric(&out, "X.f", "cc"), Some(3));
}

#[test]
fn cc_counts_logical_or() {
    let out = debug(
        "@implementation X\n- (void)f:(BOOL)a b:(BOOL)b {\n    if (a || b) {}\n}\n@end\n",
    );
    assert_eq!(function_metric(&out, "X.f", "cc"), Some(3));
}

#[test]
fn cc_counts_mixed_boolean() {
    let out = debug(
        "@implementation X\n- (void)f:(BOOL)a b:(BOOL)b c:(BOOL)c {\n    if (a && b || c) {}\n}\n@end\n",
    );
    let cc = function_metric(&out, "X.f", "cc").unwrap_or(0);
    assert!(cc >= 4, "cc={cc}");
}

#[test]
fn cc_multiple_if_accumulates() {
    let out = debug(
        "@implementation X\n- (void)f:(int)x {\n    if (x > 0) {}\n    if (x > 1) {}\n}\n@end\n",
    );
    assert_eq!(function_metric(&out, "X.f", "cc"), Some(3));
}

#[test]
fn cc_nested_if_accumulates() {
    let out = debug(
        "@implementation X\n- (void)f:(int)x {\n    if (x > 0) {\n        if (x > 1) {}\n    }\n}\n@end\n",
    );
    assert_eq!(function_metric(&out, "X.f", "cc"), Some(3));
}

#[test]
fn cc_catch_increments() {
    let out = debug(
        "@implementation X\n- (void)f {\n    @try {} @catch (NSException *e) {}\n}\n@end\n",
    );
    assert_eq!(function_metric(&out, "X.f", "cc"), Some(2));
}

#[test]
fn cc_for_in_with_if() {
    let out = debug(
        "@implementation X\n- (void)f:(NSArray *)items {\n    for (id obj in items) {\n        if (obj != nil) {}\n    }\n}\n@end\n",
    );
    assert_eq!(function_metric(&out, "X.f", "cc"), Some(3));
}

#[test]
fn cc_combined_constructs() {
    let out = debug(concat!(
        "@implementation X\n",
        "- (void)f:(int)x {\n",
        "    if (x > 0) {\n",
        "        for (int i = 0; i < x; i++) {\n",
        "            while (i > 0) { break; }\n",
        "        }\n",
        "    } else if (x < 0) {\n",
        "        switch (x) {\n",
        "            case -1: break;\n",
        "            default: break;\n",
        "        }\n",
        "    }\n",
        "}\n",
        "@end\n"
    ));
    let cc = function_metric(&out, "X.f", "cc").unwrap_or(0);
    assert!(cc >= 6, "cc={cc}");
}

// ===========================================================================
// Nesting precision (8)
// ===========================================================================

#[test]
fn nesting_flat() {
    let out = debug("@implementation X\n- (void)f {\n    NSLog(@\"hi\");\n}\n@end\n");
    assert_eq!(function_metric(&out, "X.f", "nesting"), Some(0));
}

#[test]
fn nesting_single_if() {
    let out = debug(
        "@implementation X\n- (void)f:(int)x {\n    if (x > 0) {}\n}\n@end\n",
    );
    assert_eq!(function_metric(&out, "X.f", "nesting"), Some(1));
}

#[test]
fn nesting_double() {
    let out = debug(
        "@implementation X\n- (void)f:(int)x {\n    if (x > 0) {\n        if (x > 1) {}\n    }\n}\n@end\n",
    );
    assert_eq!(function_metric(&out, "X.f", "nesting"), Some(2));
}

#[test]
fn nesting_triple() {
    let out = debug(
        "@implementation X\n- (void)f:(int)x {\n    if (x > 0) {\n        if (x > 1) {\n            if (x > 2) {}\n        }\n    }\n}\n@end\n",
    );
    assert_eq!(function_metric(&out, "X.f", "nesting"), Some(3));
}

#[test]
fn nesting_four_plus() {
    let out = debug(concat!(
        "@implementation X\n",
        "- (void)f:(int)x {\n",
        "    if (x > 0) {\n",
        "        if (x > 1) {\n",
        "            if (x > 2) {\n",
        "                if (x > 3) {}\n",
        "            }\n",
        "        }\n",
        "    }\n",
        "}\n",
        "@end\n"
    ));
    let nesting = function_metric(&out, "X.f", "nesting").unwrap_or(0);
    assert!(nesting >= 4, "nesting={nesting}");
}

#[test]
fn nesting_for_if_combo() {
    let out = debug(
        "@implementation X\n- (void)f {\n    for (int i = 0; i < 10; i++) {\n        if (i > 5) {}\n    }\n}\n@end\n",
    );
    assert_eq!(function_metric(&out, "X.f", "nesting"), Some(2));
}

#[test]
fn nesting_while_if() {
    let out = debug(
        "@implementation X\n- (void)f:(int)x {\n    while (x > 0) {\n        if (x > 5) {}\n        x--;\n    }\n}\n@end\n",
    );
    assert_eq!(function_metric(&out, "X.f", "nesting"), Some(2));
}

#[test]
fn nesting_switch_counts() {
    let out = debug(concat!(
        "@implementation X\n",
        "- (void)f:(int)x {\n",
        "    switch (x) {\n",
        "        case 1:\n",
        "            if (x > 0) {}\n",
        "            break;\n",
        "        default: break;\n",
        "    }\n",
        "}\n",
        "@end\n"
    ));
    let nesting = function_metric(&out, "X.f", "nesting").unwrap_or(0);
    assert!(nesting >= 2, "nesting={nesting}");
}

// ===========================================================================
// Argument counting (8)
// ===========================================================================

#[test]
fn args_zero() {
    let out = debug("@implementation X\n- (void)f {\n    NSLog(@\"hi\");\n}\n@end\n");
    assert_eq!(function_metric(&out, "X.f", "args"), Some(0));
}

#[test]
fn args_one() {
    let out = debug("@implementation X\n- (void)f:(int)x {\n    NSLog(@\"%d\", x);\n}\n@end\n");
    assert_eq!(function_metric(&out, "X.f", "args"), Some(1));
}

#[test]
fn args_two() {
    let out = debug(
        "@implementation X\n- (void)f:(int)x y:(int)y {\n    NSLog(@\"%d %d\", x, y);\n}\n@end\n",
    );
    assert_eq!(function_metric(&out, "X.f", "args"), Some(2));
}

#[test]
fn args_six_triggers() {
    let code = concat!(
        "@implementation X\n",
        "- (void)f:(int)a b:(int)b c:(int)c d:(int)d e:(int)e g:(int)g {\n",
        "    NSLog(@\"hi\");\n",
        "}\n",
        "@end\n"
    );
    let out = check(code);
    assert!(has_smell(&out, "Excess Arguments"), "got: {out}");
}

#[test]
fn args_c_function() {
    let out = debug("void f(int a, int b, int c) {\n    return;\n}\n");
    assert_eq!(function_metric(&out, "f", "args"), Some(3));
}

#[test]
fn args_selector_syntax() {
    let out = debug(
        "@implementation X\n- (void)doThis:(int)a andThat:(int)b withExtra:(int)c {\n    NSLog(@\"hi\");\n}\n@end\n",
    );
    assert_eq!(function_metric(&out, "X.doThis", "args"), Some(3));
}

#[test]
fn args_init_constructor() {
    let code = concat!(
        "@implementation X\n",
        "- (instancetype)initWithA:(int)a b:(int)b c:(int)c d:(int)d e:(int)e f:(int)f {\n",
        "    self = [super init];\n",
        "    return self;\n",
        "}\n",
        "@end\n"
    );
    let out = check(code);
    assert!(has_smell(&out, "Constructor Over-Injection"), "got: {out}");
}

#[test]
fn args_class_method() {
    let out = debug(
        "@implementation X\n+ (void)doStuff:(int)a b:(int)b {\n    NSLog(@\"hi\");\n}\n@end\n",
    );
    assert_eq!(function_metric(&out, "X.doStuff", "args"), Some(2));
}

// ===========================================================================
// Duplication (6)
// ===========================================================================

#[test]
fn exact_duplication_detected() {
    let code = concat!(
        "@implementation X\n",
        "- (NSInteger)a:(NSInteger)n {\n",
        "    NSInteger r = 0;\n",
        "    for (NSInteger i = 0; i < n; i++) {\n",
        "        r += i * i;\n",
        "        if (r > 100) { r = 100; }\n",
        "    }\n",
        "    return r;\n",
        "}\n",
        "- (NSInteger)b:(NSInteger)n {\n",
        "    NSInteger r = 0;\n",
        "    for (NSInteger i = 0; i < n; i++) {\n",
        "        r += i * i;\n",
        "        if (r > 100) { r = 100; }\n",
        "    }\n",
        "    return r;\n",
        "}\n",
        "@end\n"
    );
    let out = check(code);
    assert!(has_smell(&out, "Code Duplication"), "got: {out}");
}

#[test]
fn fuzzy_duplication_detected() {
    let code = concat!(
        "@implementation X\n",
        "- (NSInteger)a:(NSInteger)n {\n",
        "    NSInteger r = 0;\n",
        "    for (NSInteger i = 0; i < n; i++) {\n",
        "        r += i * 2;\n",
        "        if (r > 200) { r = 200; }\n",
        "    }\n",
        "    return r;\n",
        "}\n",
        "- (NSInteger)b:(NSInteger)n {\n",
        "    NSInteger r = 0;\n",
        "    for (NSInteger i = 0; i < n; i++) {\n",
        "        r += i * 3;\n",
        "        if (r > 300) { r = 300; }\n",
        "    }\n",
        "    return r;\n",
        "}\n",
        "@end\n"
    );
    let out = check(code);
    assert!(has_smell(&out, "Code Duplication"), "got: {out}");
}

#[test]
fn different_bodies_not_flagged() {
    let code = concat!(
        "@implementation X\n",
        "- (void)a {\n    NSLog(@\"a\");\n}\n",
        "- (void)b {\n    for (int i = 0; i < 10; i++) {}\n}\n",
        "@end\n"
    );
    let out = check(code);
    assert!(!has_smell(&out, "Code Duplication"), "got: {out}");
}

#[test]
fn test_function_duplication_suppressed() {
    let code = concat!(
        "void test_a(void) {\n",
        "    NSInteger r = 0;\n",
        "    for (NSInteger i = 0; i < 10; i++) {\n",
        "        r += i * i;\n",
        "        if (r > 50) { r = 50; }\n",
        "    }\n",
        "    NSLog(@\"%ld\", (long)r);\n",
        "}\n",
        "void test_b(void) {\n",
        "    NSInteger r = 0;\n",
        "    for (NSInteger i = 0; i < 10; i++) {\n",
        "        r += i * i;\n",
        "        if (r > 50) { r = 50; }\n",
        "    }\n",
        "    NSLog(@\"%ld\", (long)r);\n",
        "}\n"
    );
    let out = check(code);
    assert!(!has_smell(&out, "Code Duplication"), "got: {out}");
}

#[test]
fn c_function_duplication() {
    let code = concat!(
        "int compute_a(int n) {\n",
        "    int r = 0;\n",
        "    for (int i = 0; i < n; i++) {\n",
        "        r += i * i;\n",
        "        if (r > 100) { r = 100; }\n",
        "    }\n",
        "    return r;\n",
        "}\n",
        "int compute_b(int n) {\n",
        "    int r = 0;\n",
        "    for (int i = 0; i < n; i++) {\n",
        "        r += i * i;\n",
        "        if (r > 100) { r = 100; }\n",
        "    }\n",
        "    return r;\n",
        "}\n"
    );
    let out = check(code);
    assert!(has_smell(&out, "Code Duplication"), "got: {out}");
}

#[test]
fn three_way_duplication() {
    let code = concat!(
        "@implementation X\n",
        "- (NSInteger)a:(NSInteger)n {\n",
        "    NSInteger r = 0;\n",
        "    for (NSInteger i = 0; i < n; i++) {\n",
        "        r += i;\n",
        "        if (r > 50) { r = 50; }\n",
        "    }\n",
        "    return r;\n",
        "}\n",
        "- (NSInteger)b:(NSInteger)n {\n",
        "    NSInteger r = 0;\n",
        "    for (NSInteger i = 0; i < n; i++) {\n",
        "        r += i;\n",
        "        if (r > 50) { r = 50; }\n",
        "    }\n",
        "    return r;\n",
        "}\n",
        "- (NSInteger)c:(NSInteger)n {\n",
        "    NSInteger r = 0;\n",
        "    for (NSInteger i = 0; i < n; i++) {\n",
        "        r += i;\n",
        "        if (r > 50) { r = 50; }\n",
        "    }\n",
        "    return r;\n",
        "}\n",
        "@end\n"
    );
    let out = check(code);
    assert!(has_smell(&out, "Code Duplication"), "got: {out}");
}

// ===========================================================================
// Declaration thresholds (4)
// ===========================================================================

#[test]
fn single_class_counted() {
    let out = debug("@implementation X\n- (void)f {}\n@end\n");
    assert!(out.contains("declarations=1"), "got: {out}");
}

#[test]
fn multiple_classes_counted() {
    let out = debug(
        "@implementation A\n- (void)f {}\n@end\n@implementation B\n- (void)g {}\n@end\n",
    );
    assert!(out.contains("declarations=2"), "got: {out}");
}

#[test]
fn protocol_counted() {
    let out = debug(
        "@protocol P <NSObject>\n- (void)didFinish;\n@end\n@implementation X\n- (void)f {}\n@end\n",
    );
    assert!(out.contains("declarations=2"), "got: {out}");
}

#[test]
fn below_declaration_threshold_not_flagged() {
    let code = "@implementation X\n- (void)f {}\n@end\n";
    let out = check(code);
    assert!(!has_smell(&out, "Excessive Declarations"), "got: {out}");
}

// ===========================================================================
// God class / method (4)
// ===========================================================================

#[test]
fn god_method_triggered() {
    let mut code = String::from("@implementation X\n- (void)god:(int)x {\n");
    for i in 0..cc_branches() {
        code.push_str(&format!("    if (x > {i}) {{\n        NSLog(@\"{i}\");\n    }}\n"));
    }
    for _ in 0..fn_padding() {
        code.push_str("    NSLog(@\"padding\");\n");
    }
    code.push_str("}\n@end\n");
    let out = check(&code);
    assert!(has_smell(&out, "God Method"), "got: {out}");
}

#[test]
fn god_method_subsumes() {
    let mut code = String::from("@implementation X\n- (void)god:(int)x {\n");
    for i in 0..cc_branches() {
        code.push_str(&format!("    if (x > {i}) {{\n        NSLog(@\"{i}\");\n    }}\n"));
    }
    for _ in 0..fn_padding() {
        code.push_str("    NSLog(@\"padding\");\n");
    }
    code.push_str("}\n@end\n");
    let out = check(&code);
    assert!(has_smell(&out, "God Method"));
    let god_lines: Vec<&str> = out.lines().filter(|l| l.contains("X.god")).collect();
    assert!(
        !god_lines.iter().any(|l| l.contains("Complex Method")),
        "got: {god_lines:?}"
    );
}

#[test]
fn god_class_via_many_methods() {
    let mut code = String::from("@implementation X\n");
    for i in 0..functions_above() {
        code.push_str(&format!("- (void)f{i}:(int)x {{\n    if (x > 0) {{}}\n}}\n"));
    }
    code.push_str("@end\n");
    let out = check(&code);
    assert!(has_smell(&out, "Too Many Functions"), "got: {out}");
}

#[test]
fn god_class_via_high_cc() {
    let mut code = String::from("@implementation X\n");
    for i in 0..8 {
        code.push_str(&format!(
            "- (void)f{i}:(int)x {{\n    if (x>0) {{}}\n    if (x>1) {{}}\n    if (x>2) {{}}\n    if (x>3) {{}}\n    if (x>4) {{}}\n    if (x>5) {{}}\n    if (x>6) {{}}\n    if (x>7) {{}}\n    if (x>8) {{}}\n    if (x>9) {{}}\n    if (x>10) {{}}\n    if (x>11) {{}}\n    if (x>12) {{}}\n}}\n"
        ));
    }
    code.push_str("@end\n");
    let out = check(&code);
    assert!(has_smell(&out, "Overall Code Complexity"), "got: {out}");
}

// ===========================================================================
// Performance (2)
// ===========================================================================

#[test]
fn large_file_under_500ms() {
    let mut code = String::from("@implementation X\n");
    code.push_str("- (void)big:(int)x {\n");
    for i in 0..1000 {
        code.push_str(&format!("    NSLog(@\"{i}\");\n"));
    }
    code.push_str("}\n@end\n");
    let start = std::time::Instant::now();
    let _ = check(&code);
    assert!(start.elapsed().as_millis() < 500);
}

#[test]
fn many_methods_under_500ms() {
    let mut code = String::from("@implementation X\n");
    for i in 0..50 {
        code.push_str(&format!(
            "- (void)m{i}:(int)x {{\n    if (x > 0) {{ NSLog(@\"a\"); }}\n}}\n"
        ));
    }
    code.push_str("@end\n");
    let start = std::time::Instant::now();
    let _ = check(&code);
    assert!(start.elapsed().as_millis() < 500);
}

// ===========================================================================
// ObjC-specific edge cases (15)
// ===========================================================================

#[test]
fn try_catch_cc() {
    let out = debug(concat!(
        "@implementation X\n",
        "- (void)f {\n",
        "    @try {\n",
        "        NSLog(@\"try\");\n",
        "    } @catch (NSException *e) {\n",
        "        NSLog(@\"catch\");\n",
        "    }\n",
        "}\n",
        "@end\n"
    ));
    assert_eq!(function_metric(&out, "X.f", "cc"), Some(2));
}

#[test]
fn try_finally_no_cc() {
    let out = debug(concat!(
        "@implementation X\n",
        "- (void)f {\n",
        "    @try {\n",
        "        NSLog(@\"try\");\n",
        "    } @catch (NSException *e) {\n",
        "        NSLog(@\"catch\");\n",
        "    } @finally {\n",
        "        NSLog(@\"finally\");\n",
        "    }\n",
        "}\n",
        "@end\n"
    ));
    // Only @catch adds CC, not @finally
    assert_eq!(function_metric(&out, "X.f", "cc"), Some(2));
}

#[test]
fn empty_catch_detected() {
    let code = concat!(
        "@implementation X\n",
        "- (void)f {\n",
        "    @try {\n",
        "        NSLog(@\"try\");\n",
        "    } @catch (NSException *e) {\n",
        "    }\n",
        "}\n",
        "@end\n"
    );
    let out = check(code);
    assert!(has_smell(&out, "Empty Error Handler"), "got: {out}");
}

#[test]
fn non_empty_catch_not_flagged() {
    let code = concat!(
        "@implementation X\n",
        "- (void)f {\n",
        "    @try {\n",
        "        NSLog(@\"try\");\n",
        "    } @catch (NSException *e) {\n",
        "        NSLog(@\"error: %@\", e);\n",
        "    }\n",
        "}\n",
        "@end\n"
    );
    let out = check(code);
    assert!(!has_smell(&out, "Empty Error Handler"), "got: {out}");
}

#[test]
fn class_method_plus() {
    let out = debug(
        "@implementation X\n+ (instancetype)shared {\n    static X *s = nil;\n    return s;\n}\n@end\n",
    );
    assert!(out.contains("X.shared"), "got: {out}");
}

#[test]
fn instance_method_minus() {
    let out = debug(
        "@implementation X\n- (void)doWork {\n    NSLog(@\"work\");\n}\n@end\n",
    );
    assert!(out.contains("X.doWork"), "got: {out}");
}

#[test]
fn self_field_access() {
    let out = debug(concat!(
        "@implementation X\n",
        "- (void)f {\n",
        "    self.name = @\"test\";\n",
        "    NSLog(@\"%@\", self.name);\n",
        "}\n",
        "@end\n"
    ));
    assert!(out.contains("fields=[\"name\"]"), "got: {out}");
}

#[test]
fn self_field_multiple() {
    let out = debug(concat!(
        "@implementation X\n",
        "- (void)f {\n",
        "    self.name = @\"test\";\n",
        "    self.age = 25;\n",
        "}\n",
        "@end\n"
    ));
    assert!(out.contains("age"), "got: {out}");
    assert!(out.contains("name"), "got: {out}");
}

#[test]
fn message_expression_in_body() {
    let out = debug(concat!(
        "@implementation X\n",
        "- (void)f {\n",
        "    [self doStuff];\n",
        "    [[NSObject alloc] init];\n",
        "}\n",
        "@end\n"
    ));
    assert!(out.contains("X.f"), "got: {out}");
    assert_eq!(function_metric(&out, "X.f", "cc"), Some(1));
}

#[test]
fn nsstring_embedded_block() {
    let mut code = String::from("@implementation X\n- (NSString *)f {\n    NSString *s = @\"line1\\n\"\n");
    for i in 2..20 {
        code.push_str(&format!("        \"line{i}\\n\"\n"));
    }
    code.push_str("        \"end\";\n    return s;\n}\n@end\n");
    let out = check(&code);
    assert!(has_smell(&out, "Large Embedded Block"), "got: {out}");
}

#[test]
fn short_nsstring_not_flagged() {
    let code = "@implementation X\n- (void)f {\n    NSString *s = @\"hello\";\n    NSLog(@\"%@\", s);\n}\n@end\n";
    let out = check(code);
    assert!(!has_smell(&out, "Embedded Block"), "got: {out}");
}

#[test]
fn at_syntax_not_confused() {
    let code = concat!(
        "@implementation X\n",
        "- (void)f {\n",
        "    @try {\n",
        "        NSLog(@\"string\");\n",
        "    } @catch (NSException *e) {\n",
        "        NSLog(@\"error\");\n",
        "    }\n",
        "}\n",
        "@end\n"
    );
    let out = debug(code);
    assert!(out.contains("X.f"), "got: {out}");
    assert_eq!(function_metric(&out, "X.f", "cc"), Some(2));
}

#[test]
fn for_in_enumeration() {
    let out = debug(concat!(
        "@implementation X\n",
        "- (void)f:(NSArray *)items {\n",
        "    for (NSString *item in items) {\n",
        "        NSLog(@\"%@\", item);\n",
        "    }\n",
        "}\n",
        "@end\n"
    ));
    assert_eq!(function_metric(&out, "X.f", "cc"), Some(2));
    assert_eq!(function_metric(&out, "X.f", "nesting"), Some(1));
}

#[test]
fn c_function_in_objc_file() {
    let code = concat!(
        "@implementation X\n",
        "- (void)method {\n    NSLog(@\"method\");\n}\n",
        "@end\n",
        "void standalone(int x) {\n    if (x > 0) {}\n}\n"
    );
    let out = debug(code);
    assert!(out.contains("X.method"), "got: {out}");
    assert!(out.contains("standalone"), "got: {out}");
}

#[test]
fn multiple_classes_in_file() {
    let code = concat!(
        "@implementation A\n- (void)fa { NSLog(@\"a\"); }\n@end\n",
        "@implementation B\n- (void)fb { NSLog(@\"b\"); }\n@end\n"
    );
    let out = debug(code);
    assert!(out.contains("A.fa"), "got: {out}");
    assert!(out.contains("B.fb"), "got: {out}");
}

// ===========================================================================
// Constructor detection (4)
// ===========================================================================

#[test]
fn init_is_constructor() {
    let code = concat!(
        "@implementation X\n",
        "- (instancetype)init {\n",
        "    self = [super init];\n",
        "    return self;\n",
        "}\n",
        "@end\n"
    );
    let _out = check(code);
    // init with 0 args doesn't trigger, but verify it's detected
    let d = debug(code);
    assert!(d.contains("X.init"), "got: {d}");
}

#[test]
fn init_with_is_constructor() {
    let code = concat!(
        "@implementation X\n",
        "- (instancetype)initWithName:(NSString *)name age:(NSInteger)age\n",
        "    role:(NSString *)role dept:(NSString *)dept\n",
        "    level:(NSInteger)level active:(BOOL)active {\n",
        "    self = [super init];\n",
        "    return self;\n",
        "}\n",
        "@end\n"
    );
    let out = check(code);
    assert!(has_smell(&out, "Constructor Over-Injection"), "got: {out}");
}

#[test]
fn regular_method_not_constructor() {
    let code = concat!(
        "@implementation X\n",
        "- (void)setup:(NSString *)a b:(NSString *)b c:(NSString *)c\n",
        "    d:(NSString *)d e:(NSString *)e f:(NSString *)f {\n",
        "    NSLog(@\"setup\");\n",
        "}\n",
        "@end\n"
    );
    let out = check(code);
    assert!(has_smell(&out, "Excess Arguments"), "got: {out}");
    assert!(!has_smell(&out, "Constructor Over-Injection"), "got: {out}");
}

#[test]
fn init_constructor_over_injection() {
    let code = concat!(
        "@implementation X\n",
        "- (instancetype)initWithA:(int)a b:(int)b c:(int)c d:(int)d e:(int)e f:(int)f {\n",
        "    self = [super init];\n",
        "    return self;\n",
        "}\n",
        "@end\n"
    );
    let out = check(code);
    assert!(has_smell(&out, "Constructor Over-Injection"), "got: {out}");
}

// ===========================================================================
// Primitive obsession (4)
// ===========================================================================

#[test]
fn all_primitive_params_flagged() {
    let code = concat!(
        "@implementation X\n",
        "- (void)f:(NSInteger)a b:(NSInteger)b c:(NSInteger)c d:(NSInteger)d e:(NSInteger)e {\n",
        "    NSLog(@\"hi\");\n",
        "}\n",
        "@end\n"
    );
    let out = check(code);
    assert!(has_smell(&out, "Primitive Obsession"), "got: {out}");
}

#[test]
fn mixed_types_not_flagged() {
    let code = concat!(
        "@implementation X\n",
        "- (void)f:(NSString *)name age:(NSInteger)age email:(NSString *)email role:(NSString *)role {\n",
        "    NSLog(@\"hi\");\n",
        "}\n",
        "@end\n"
    );
    let out = check(code);
    assert!(!has_smell(&out, "Primitive Obsession"), "got: {out}");
}

#[test]
fn objc_specific_primitives() {
    let code = concat!(
        "@implementation X\n",
        "- (void)f:(BOOL)a b:(CGFloat)c d:(NSInteger)d e:(NSUInteger)e {\n",
        "    NSLog(@\"hi\");\n",
        "}\n",
        "@end\n"
    );
    let d = debug(code);
    let _prim = function_metric(&d, "X.f", "primitives");
    // All 4 should be detected as primitive
    assert!(d.contains("primitives=4/4"), "got: {d}");
}

#[test]
fn below_primitive_threshold_not_flagged() {
    let code = concat!(
        "@implementation X\n",
        "- (void)f:(NSInteger)a b:(NSInteger)b c:(NSInteger)c {\n",
        "    NSLog(@\"hi\");\n",
        "}\n",
        "@end\n"
    );
    let out = check(code);
    assert!(!has_smell(&out, "Primitive Obsession"), "got: {out}");
}

// ===========================================================================
// Cognitive complexity (4)
// ===========================================================================

#[test]
fn cogc_flat_is_zero() {
    let out = debug("@implementation X\n- (void)f {\n    NSLog(@\"hi\");\n}\n@end\n");
    assert_eq!(function_metric(&out, "X.f", "cogc"), Some(0));
}

#[test]
fn cogc_single_if() {
    let out = debug(
        "@implementation X\n- (void)f:(int)x {\n    if (x > 0) {}\n}\n@end\n",
    );
    assert_eq!(function_metric(&out, "X.f", "cogc"), Some(1));
}

#[test]
fn cogc_nested_if_increments_more() {
    let out = debug(
        "@implementation X\n- (void)f:(int)x {\n    if (x > 0) {\n        if (x > 1) {}\n    }\n}\n@end\n",
    );
    let cogc = function_metric(&out, "X.f", "cogc").unwrap_or(0);
    // Outer if: 1, inner if: 1 + 1 (nesting) = 2, total = 3
    assert!(cogc >= 3, "cogc={cogc}");
}

#[test]
fn cogc_else_adds_flat() {
    let out = debug(
        "@implementation X\n- (void)f:(int)x {\n    if (x > 0) {\n    } else {\n    }\n}\n@end\n",
    );
    let cogc = function_metric(&out, "X.f", "cogc").unwrap_or(0);
    // if: 1, else: 1 (flat), total = 2
    assert_eq!(cogc, 2);
}
