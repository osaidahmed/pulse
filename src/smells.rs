use crate::module_smells;
use crate::thresholds::Thresholds;
use crate::walk::{FileMetrics, FunctionMetrics};

#[derive(Debug)]
pub struct Finding {
    pub smell: &'static str,
    pub location: Location,
    pub detail: String,
}

#[derive(Debug)]
pub enum Location {
    Function {
        name: String,
        start_line: u32,
        end_line: u32,
    },
    Module,
}

pub fn detect((functions, module): &FileMetrics, _source: &str, t: &Thresholds) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut has_god_method = false;

    for f in functions {
        detect_function_smells(f, t, &mut findings, &mut has_god_method);
    }

    module_smells::detect_module_smells(module, t, has_god_method, &mut findings);
    module_smells::detect_code_duplication(functions, t, &mut findings);
    module_smells::detect_overall_function_size(functions, t, &mut findings);
    detect_primitive_obsession(functions, t, &mut findings);
    detect_large_assertion_blocks(functions, t, &mut findings);
    module_smells::detect_duplicated_assertion_blocks(functions, &mut findings);
    module_smells::detect_lcom4(functions, t, &mut findings);
    detect_empty_error_handlers(functions, &mut findings);

    findings
}

pub fn func_loc(f: &FunctionMetrics) -> Location {
    Location::Function {
        name: f.name.clone(),
        start_line: f.start_line,
        end_line: f.end_line,
    }
}

fn detect_function_smells(
    f: &FunctionMetrics,
    t: &Thresholds,
    findings: &mut Vec<Finding>,
    has_god_method: &mut bool,
) {
    detect_complexity_smells(f, t, findings, has_god_method);
    detect_structural_smells(f, t, findings);
    detect_argument_smells(f, t, findings);
    detect_embedded_smells(f, t, findings);
}

fn detect_complexity_smells(
    f: &FunctionMetrics,
    t: &Thresholds,
    findings: &mut Vec<Finding>,
    has_god_method: &mut bool,
) {
    let cc_complex = f.cc >= t.cc_warning;
    let cogc_complex = f.cognitive_complexity >= t.cogc_warning;
    let is_complex = cc_complex || cogc_complex;
    let is_large = f.loc >= t.fn_loc_warning;

    if is_complex && is_large {
        *has_god_method = true;
        let detail = complexity_detail(f, t, cc_complex, cogc_complex);
        findings.push(Finding {
            smell: "God Method",
            location: func_loc(f),
            detail: format!("{}, {} lines (both thresholds exceeded)", detail, f.loc),
        });
        return;
    }

    check_complex_method(f, t, cc_complex, cogc_complex, findings);
    check_large_method(f, t, is_large, findings);
}

fn complexity_detail(
    f: &FunctionMetrics,
    t: &Thresholds,
    cc_complex: bool,
    cogc_complex: bool,
) -> String {
    match (cc_complex, cogc_complex) {
        (true, true) => {
            let sev = if f.cc > t.cc_alert || f.cognitive_complexity > t.cogc_alert {
                "alert"
            } else {
                "warning"
            };
            format!("cc={}, cogc={} [{}]", f.cc, f.cognitive_complexity, sev)
        }
        (false, true) => {
            let sev = if f.cognitive_complexity > t.cogc_alert {
                "alert"
            } else {
                "warning"
            };
            format!("cogc={} [{}]", f.cognitive_complexity, sev)
        }
        _ => {
            let sev = if f.cc > t.cc_alert { "alert" } else { "warning" };
            format!("cc={} [{}]", f.cc, sev)
        }
    }
}

fn check_complex_method(
    f: &FunctionMetrics,
    t: &Thresholds,
    cc_complex: bool,
    cogc_complex: bool,
    findings: &mut Vec<Finding>,
) {
    if !cc_complex && !cogc_complex {
        return;
    }
    findings.push(Finding {
        smell: "Complex Method",
        location: func_loc(f),
        detail: complexity_detail(f, t, cc_complex, cogc_complex),
    });
}

fn check_large_method(
    f: &FunctionMetrics,
    t: &Thresholds,
    is_large: bool,
    findings: &mut Vec<Finding>,
) {
    if !is_large {
        return;
    }
    let severity = if f.loc > t.fn_loc_alert { "alert" } else { "warning" };
    findings.push(Finding {
        smell: "Large Method",
        location: func_loc(f),
        detail: format!("{} lines [{}]", f.loc, severity),
    });
}

fn detect_structural_smells(f: &FunctionMetrics, t: &Thresholds, findings: &mut Vec<Finding>) {
    if f.bump_count >= t.bump_count {
        findings.push(Finding {
            smell: "Nested Conditional Chunks",
            location: func_loc(f),
            detail: format!("{} nested conditional chunks", f.bump_count),
        });
    }

    if f.max_nesting >= t.nesting_depth {
        findings.push(Finding {
            smell: "Deep Nested Complexity",
            location: func_loc(f),
            detail: format!("depth={}", f.max_nesting),
        });
    }

    if f.compound_condition_count > t.compound_conditions {
        findings.push(Finding {
            smell: "Complex Conditional",
            location: func_loc(f),
            detail: format!("{} complex conditions", f.compound_condition_count),
        });
    }
}

fn detect_argument_smells(f: &FunctionMetrics, t: &Thresholds, findings: &mut Vec<Finding>) {
    let threshold = if f.is_constructor {
        t.constructor_arg_max
    } else {
        t.arg_max
    };
    if f.arg_count <= threshold {
        return;
    }
    let smell = if f.is_constructor {
        "Constructor Over-Injection"
    } else {
        "Excess Arguments"
    };
    findings.push(Finding {
        smell,
        location: func_loc(f),
        detail: format!("{} args", f.arg_count),
    });
}

fn detect_embedded_smells(f: &FunctionMetrics, t: &Thresholds, findings: &mut Vec<Finding>) {
    if f.max_embedded_block_loc > t.embedded_block_loc {
        findings.push(Finding {
            smell: "Large Embedded Block",
            location: func_loc(f),
            detail: format!("{} lines of embedded content", f.max_embedded_block_loc),
        });
    }
}

fn detect_primitive_obsession(
    functions: &[FunctionMetrics],
    t: &Thresholds,
    findings: &mut Vec<Finding>,
) {
    for f in functions {
        if !has_high_primitive_ratio(f, t) {
            continue;
        }
        let ratio = f.primitive_type_count as f32 / f.typed_param_count as f32;
        findings.push(Finding {
            smell: "Primitive Obsession",
            location: func_loc(f),
            detail: format!(
                "{}/{} typed params are primitives ({:.0}%)",
                f.primitive_type_count,
                f.typed_param_count,
                ratio * 100.0
            ),
        });
    }
}

fn has_high_primitive_ratio(f: &FunctionMetrics, t: &Thresholds) -> bool {
    if f.typed_param_count < t.primitive_min_typed_params || f.primitive_type_count == 0 {
        return false;
    }
    let ratio = f.primitive_type_count as f32 / f.typed_param_count as f32;
    ratio >= t.primitive_ratio_threshold
}

fn detect_large_assertion_blocks(
    functions: &[FunctionMetrics],
    t: &Thresholds,
    findings: &mut Vec<Finding>,
) {
    let threshold = t.consecutive_asserts_max;
    findings.extend(functions.iter().filter_map(|f| {
        (f.consecutive_asserts > threshold).then_some(Finding {
            smell: "Large Assertion Block",
            location: func_loc(f),
            detail: format!("{} consecutive assertions", f.consecutive_asserts),
        })
    }));
}

fn detect_empty_error_handlers(functions: &[FunctionMetrics], findings: &mut Vec<Finding>) {
    findings.extend(functions.iter().filter(|f| f.empty_catch_count > 0).map(|f| {
        Finding {
            smell: "Empty Error Handler",
            location: func_loc(f),
            detail: format!(
                "{} empty catch block{}",
                f.empty_catch_count,
                if f.empty_catch_count == 1 { "" } else { "s" }
            ),
        }
    }));
}
