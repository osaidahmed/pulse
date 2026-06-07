use crate::common::*;

fn cc_code(branches: usize) -> String {
    let mut s = String::from("def f(x):\n");
    for i in 0..branches {
        s.push_str(&format!("    if x == {i}:\n        return {i}\n"));
    }
    s.push_str("    return -1\n");
    s
}

fn nested_code(depth: usize) -> String {
    let mut s = String::from("def f(a):\n");
    let mut indent = String::from("    ");
    for i in 0..depth {
        s.push_str(&format!("{indent}if a == {i}:\n"));
        indent.push_str("    ");
    }
    s.push_str(&format!("{indent}return 0\n"));
    s
}

fn args_code(n: usize) -> String {
    let params: Vec<String> = (0..n).map(|i| format!("a{i}")).collect();
    format!("def f({}):\n    return 0\n", params.join(", "))
}

fn compound_code(conditions: usize) -> String {
    let mut s = String::from("def f(a, b, c):\n");
    for i in 0..conditions {
        s.push_str(&format!("    if a and b and c:\n        return {i}\n"));
    }
    s.push_str("    return -1\n");
    s
}

fn loc_code(body_lines: usize) -> String {
    let mut s = String::from("def f():\n");
    for i in 0..body_lines {
        s.push_str(&format!("    x{i} = {i}\n"));
    }
    s
}

fn metric(code: &str, name: &str) -> Option<u32> {
    function_metric(&pulse_debug_code(code, "py"), "f", name)
}

fn fires(code: &str, smell: &str) -> bool {
    has_smell(&pulse_check_code(code, "py"), smell)
}

#[test]
fn cc_complex_method_boundary_is_inclusive() {
    let warning = t().function.cc_warning as usize;

    let at = cc_code(warning - 1);
    assert_eq!(metric(&at, "cc"), Some(warning as u32));
    assert!(fires(&at, "Complex Method"), "cc == warning must fire (>=)");

    let below = cc_code(warning - 2);
    assert_eq!(metric(&below, "cc"), Some((warning - 1) as u32));
    assert!(!fires(&below, "Complex Method"), "cc == warning-1 must not fire");
}

#[test]
fn nesting_deep_nested_boundary_is_inclusive() {
    let depth = t().function.nesting_depth as usize;

    let at = nested_code(depth);
    assert_eq!(metric(&at, "nesting"), Some(depth as u32));
    assert!(fires(&at, "Deep Nested"), "nesting == threshold must fire (>=)");

    let below = nested_code(depth - 1);
    assert_eq!(metric(&below, "nesting"), Some((depth - 1) as u32));
    assert!(!fires(&below, "Deep Nested"), "nesting == threshold-1 must not fire");
}

#[test]
fn fn_loc_large_method_boundary_is_inclusive() {
    let warning = t().function.fn_loc_warning as usize;

    let at = loc_code(warning - 1);
    assert_eq!(metric(&at, "loc"), Some(warning as u32));
    assert!(fires(&at, "Large Method"), "loc == warning must fire (>=)");

    let below = loc_code(warning - 2);
    assert_eq!(metric(&below, "loc"), Some((warning - 1) as u32));
    assert!(!fires(&below, "Large Method"), "loc == warning-1 must not fire");
}

#[test]
fn args_excess_arguments_boundary_is_strict() {
    let max = t().function.arg_max as usize;

    let at = args_code(max);
    assert_eq!(metric(&at, "args"), Some(max as u32));
    assert!(!fires(&at, "Excess Arguments"), "arg_count == max must not fire (strict >)");

    let above = args_code(max + 1);
    assert_eq!(metric(&above, "args"), Some((max + 1) as u32));
    assert!(fires(&above, "Excess Arguments"), "arg_count == max+1 must fire");
}

#[test]
fn compound_complex_conditional_boundary_is_strict() {
    let threshold = t().function.compound_conditions as usize;

    let at = compound_code(threshold);
    assert_eq!(metric(&at, "conditions"), Some(threshold as u32));
    assert!(!fires(&at, "Complex Conditional"), "count == threshold must not fire (strict >)");

    let above = compound_code(threshold + 1);
    assert_eq!(metric(&above, "conditions"), Some((threshold + 1) as u32));
    assert!(fires(&above, "Complex Conditional"), "count == threshold+1 must fire");
}
