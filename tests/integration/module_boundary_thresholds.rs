use crate::common::*;

const LCOM4_MIN_METHODS: usize = 3;

fn py_functions(n: usize) -> String {
    let mut s = String::new();
    for i in 0..n {
        s.push_str(&format!("def f{i}():\n    return {i}\n"));
    }
    s
}

fn py_classes(n: usize) -> String {
    let mut s = String::new();
    for i in 0..n {
        s.push_str(&format!("class C{i}:\n    pass\n"));
    }
    s
}

fn py_lines(n: usize) -> String {
    let mut s = String::new();
    for i in 0..n {
        s.push_str(&format!("x_{i} = {i}\n"));
    }
    s
}

fn py_cc_fn(idx: usize, cc: usize) -> String {
    let mut s = format!("def f{idx}(x):\n");
    for i in 0..(cc - 1) {
        s.push_str(&format!("    if x == {i}:\n        return {i}\n"));
    }
    s.push_str("    return -1\n");
    s
}

fn py_total_cc(total: usize) -> String {
    let n = t().module.file_function_count as usize;
    let base = total / n;
    let rem = total % n;
    let mut s = String::new();
    for i in 0..n {
        s.push_str(&py_cc_fn(i, base + usize::from(i < rem)));
    }
    s
}

fn py_big_fns(count: usize, loc: usize) -> String {
    let mut s = String::new();
    for k in 0..count {
        s.push_str(&format!("def f{k}():\n"));
        for i in 0..(loc - 1) {
            s.push_str(&format!("    x{i} = {i}\n"));
        }
    }
    s
}

fn rs_struct(fields: usize) -> String {
    let mut s = String::from("struct S {\n");
    for i in 0..fields {
        s.push_str(&format!("    f{i}: i32,\n"));
    }
    s.push_str("}\n");
    s
}

fn rs_disconnected(groups: usize, extra_in_first: usize) -> String {
    let fields: Vec<String> = (0..groups).map(|i| format!("f{i}: i32")).collect();
    let mut s = format!("struct Sink {{ {} }}\n", fields.join(", "));
    s.push_str("impl Sink {\n");
    for i in 0..groups {
        s.push_str(&format!("    fn use_{i}(&self) -> i32 {{ self.f{i} }}\n"));
    }
    for j in 0..extra_in_first {
        s.push_str(&format!("    fn extra_{j}(&self) -> i32 {{ self.f0 }}\n"));
    }
    s.push_str("}\n");
    s
}

fn module_metric(debug: &str, key: &str) -> Option<u32> {
    let line = debug.lines().find(|l| l.trim_start().starts_with("Module:"))?;
    let norm = line.replace(',', " ");
    let tok: Vec<&str> = norm.split_whitespace().collect();
    let before_label = |label: &str| -> Option<u32> {
        let idx = tok.iter().position(|&x| x == label)?;
        tok.get(idx.checked_sub(1)?)?.parse().ok()
    };
    let after_eq = |prefix: &str| -> Option<u32> {
        tok.iter().find_map(|x| x.strip_prefix(prefix))?.parse().ok()
    };
    match key {
        "loc" => before_label("LOC"),
        "functions" => before_label("functions"),
        "sum_cc" => after_eq("sum_cc="),
        "declarations" => after_eq("declarations="),
        _ => None,
    }
}

fn mmetric(code: &str, ext: &str, key: &str) -> Option<u32> {
    module_metric(&pulse_debug_code(code, ext), key)
}

fn fn_loc(code: &str, ext: &str, name: &str) -> Option<u32> {
    function_metric(&pulse_debug_code(code, ext), name, "loc")
}

fn fires(code: &str, ext: &str, smell: &str) -> bool {
    has_smell(&pulse_check_code(code, ext), smell)
}

#[test]
fn file_too_large_boundary_is_strict() {
    let warning = t().module.file_loc_warning as usize;

    let at = py_lines(warning);
    assert_eq!(mmetric(&at, "py", "loc"), Some(warning as u32));
    assert!(!fires(&at, "py", "File Too Large"), "loc == warning must not fire (strict >)");

    let above = py_lines(warning + 1);
    assert_eq!(mmetric(&above, "py", "loc"), Some((warning + 1) as u32));
    assert!(fires(&above, "py", "File Too Large"), "loc == warning+1 must fire");
}

#[test]
fn file_too_large_alert_severity_boundary_is_strict() {
    let alert = t().module.file_loc_alert as usize;

    let at = py_lines(alert);
    assert_eq!(mmetric(&at, "py", "loc"), Some(alert as u32));
    let at_out = pulse_check_code(&at, "py");
    assert!(has_smell(&at_out, "File Too Large"), "loc == file_loc_alert is still File Too Large");
    assert!(!has_smell(&at_out, "[alert]"), "loc == file_loc_alert must render warning, not alert (strict >)");

    let above = py_lines(alert + 1);
    assert_eq!(mmetric(&above, "py", "loc"), Some((alert + 1) as u32));
    assert!(has_smell(&pulse_check_code(&above, "py"), "[alert]"), "loc == file_loc_alert+1 must render alert");
}

#[test]
fn too_many_functions_boundary_is_strict() {
    let max = t().module.file_function_count as usize;

    let at = py_functions(max);
    assert_eq!(mmetric(&at, "py", "functions"), Some(max as u32));
    assert!(!fires(&at, "py", "Too Many Functions"), "count == threshold must not fire (strict >)");

    let above = py_functions(max + 1);
    assert_eq!(mmetric(&above, "py", "functions"), Some((max + 1) as u32));
    assert!(fires(&above, "py", "Too Many Functions"), "count == threshold+1 must fire");
}

#[test]
fn overall_code_complexity_boundary_is_strict() {
    let max = t().module.file_total_cc as usize;

    let at = py_total_cc(max);
    assert_eq!(mmetric(&at, "py", "sum_cc"), Some(max as u32));
    assert!(!fires(&at, "py", "Overall Code Complexity"), "sum_cc == threshold must not fire (strict >)");

    let above = py_total_cc(max + 1);
    assert_eq!(mmetric(&above, "py", "sum_cc"), Some((max + 1) as u32));
    assert!(fires(&above, "py", "Overall Code Complexity"), "sum_cc == threshold+1 must fire");
}

#[test]
fn excessive_declarations_boundary_is_strict() {
    let max = t().module.max_declarations as usize;

    let at = py_classes(max);
    assert_eq!(mmetric(&at, "py", "declarations"), Some(max as u32));
    assert!(!fires(&at, "py", "Excessive Declarations"), "count == threshold must not fire (strict >)");

    let above = py_classes(max + 1);
    assert_eq!(mmetric(&above, "py", "declarations"), Some((max + 1) as u32));
    assert!(fires(&above, "py", "Excessive Declarations"), "count == threshold+1 must fire");
}

#[test]
fn large_struct_boundary_is_strict() {
    let max = t().module.max_struct_fields as usize;

    let at = rs_struct(max);
    assert_eq!(mmetric(&at, "rs", "declarations"), Some(1));
    assert!(
        pulse_check_code(&at, "rs").trim().is_empty(),
        "fields == threshold must produce no finding (strict >)"
    );

    let above = rs_struct(max + 1);
    assert!(fires(&above, "rs", "Large Struct"), "fields == threshold+1 must fire");
}

#[test]
fn overall_function_size_count_boundary_is_inclusive() {
    let min_count = t().module.large_fn_count as usize;
    let big = t().module.large_fn_loc as usize + 5;

    let at = py_big_fns(min_count, big);
    assert_eq!(fn_loc(&at, "py", "f0"), Some(big as u32));
    assert!(fires(&at, "py", "Overall Function Size"), "count == threshold must fire (>=)");

    let below = py_big_fns(min_count - 1, big);
    assert!(!fires(&below, "py", "Overall Function Size"), "count == threshold-1 must not fire");
}

#[test]
fn overall_function_size_per_function_loc_boundary_is_inclusive() {
    let loc_min = t().module.large_fn_loc as usize;
    let count = t().module.large_fn_count as usize;

    let at = py_big_fns(count, loc_min);
    assert_eq!(fn_loc(&at, "py", "f0"), Some(loc_min as u32));
    let at_out = pulse_check_code(&at, "py");
    assert!(has_smell(&at_out, "Overall Function Size"), "fn loc == large_fn_loc must count (>=)");
    assert!(
        has_smell(&at_out, &format!("({loc_min}L)")),
        "detail must name a function at exactly large_fn_loc"
    );

    let below = py_big_fns(count, loc_min - 1);
    assert_eq!(fn_loc(&below, "py", "f0"), Some((loc_min - 1) as u32));
    assert!(!fires(&below, "py", "Overall Function Size"), "fn loc == large_fn_loc-1 must not count");
}

#[test]
fn low_cohesion_boundary_is_inclusive() {
    let warning = t().analysis.lcom4_warning as usize;

    let at = rs_disconnected(warning, LCOM4_MIN_METHODS.saturating_sub(warning));
    assert!(fires(&at, "rs", "Low Cohesion"), "LCOM4 == warning must fire (>=)");

    let extra = LCOM4_MIN_METHODS.saturating_sub(warning - 1);
    let below = rs_disconnected(warning - 1, extra);
    assert!(!fires(&below, "rs", "Low Cohesion"), "LCOM4 == warning-1 must not fire");
}
