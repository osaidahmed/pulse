use crate::duplication;
use crate::module_smells;
use crate::thresholds::Thresholds;
use crate::walk::{func_loc, FileMetrics, FunctionMetrics};

pub use crate::core::{smell_from_snake_case, Finding, Location, Smell, ALL_SMELLS};

pub fn detect(metrics: &FileMetrics, _source: &str, t: &Thresholds) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut has_god_method = false;

    for f in &metrics.functions {
        detect_function_smells(f, t, &mut findings, &mut has_god_method);
    }

    module_smells::detect_module_smells(&metrics.module, t, has_god_method, &mut findings);
    duplication::detect_code_duplication(&metrics.functions, t, &mut findings);
    module_smells::detect_overall_function_size(&metrics.functions, t, &mut findings);
    detect_primitive_obsession(&metrics.functions, t, &mut findings);
    detect_batch_thresholds(&metrics.functions, t, &mut findings);
    module_smells::detect_duplicated_assertion_blocks(&metrics.functions, t, &mut findings);
    module_smells::detect_lcom4(&metrics.functions, t, &mut findings);
    detect_empty_error_handlers(&metrics.functions, &mut findings);

    if t.cpg.enabled {
        crate::cpg::detect_all(&metrics.functions, t, &mut findings);
    }

    findings
}

fn detect_function_smells(f: &FunctionMetrics, t: &Thresholds, findings: &mut Vec<Finding>, has_god_method: &mut bool) {
    detect_complexity_smells(f, t, findings, has_god_method);
    detect_structural_smells(f, t, findings);
    detect_argument_smells(f, t, findings);
    detect_embedded_smells(f, t, findings);
    detect_short_variable_names(f, t, findings);
}

fn detect_complexity_smells(
    f: &FunctionMetrics,
    t: &Thresholds,
    findings: &mut Vec<Finding>,
    has_god_method: &mut bool,
) {
    let cc_complex = f.cc >= t.function.cc_warning;
    let cogc_complex = f.cognitive_complexity >= t.function.cogc_warning;
    let is_complex = cc_complex || cogc_complex;
    let is_large = f.loc >= t.function.fn_loc_warning;

    if is_complex && is_large {
        *has_god_method = true;
        let detail = complexity_detail(f, t, cc_complex, cogc_complex);
        findings.push(Finding {
            smell: Smell::GodMethod,
            location: func_loc(f),
            detail: format!("{}, {} lines (both thresholds exceeded)", detail, f.loc),
        });
        return;
    }

    check_complex_method(f, t, cc_complex, cogc_complex, findings);
    check_large_method(f, t, is_large, findings);
}

fn complexity_detail(f: &FunctionMetrics, t: &Thresholds, cc_complex: bool, cogc_complex: bool) -> String {
    match (cc_complex, cogc_complex) {
        (true, true) => {
            let sev = if f.cc > t.function.cc_alert || f.cognitive_complexity > t.function.cogc_alert {
                "alert"
            } else {
                "warning"
            };
            format!(
                "cc={}, cogc={} [{}] (cc threshold: {}, cogc threshold: {})",
                f.cc, f.cognitive_complexity, sev, t.function.cc_warning, t.function.cogc_warning
            )
        }
        (false, true) => {
            let sev = if f.cognitive_complexity > t.function.cogc_alert { "alert" } else { "warning" };
            format!("cogc={} [{}] (threshold: {})", f.cognitive_complexity, sev, t.function.cogc_warning)
        }
        _ => {
            let sev = if f.cc > t.function.cc_alert { "alert" } else { "warning" };
            format!("cc={} [{}] (threshold: {})", f.cc, sev, t.function.cc_warning)
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
        smell: Smell::ComplexMethod,
        location: func_loc(f),
        detail: complexity_detail(f, t, cc_complex, cogc_complex),
    });
}

fn check_large_method(f: &FunctionMetrics, t: &Thresholds, is_large: bool, findings: &mut Vec<Finding>) {
    if !is_large {
        return;
    }
    let severity = if f.loc > t.function.fn_loc_alert { "alert" } else { "warning" };
    findings.push(Finding {
        smell: Smell::LargeMethod,
        location: func_loc(f),
        detail: format!("{} lines [{}] (threshold: {})", f.loc, severity, t.function.fn_loc_warning),
    });
}

fn detect_structural_smells(f: &FunctionMetrics, t: &Thresholds, findings: &mut Vec<Finding>) {
    if f.bump_count >= t.function.bump_count {
        findings.push(Finding {
            smell: Smell::NestedConditionalChunks,
            location: func_loc(f),
            detail: format!("{} nested conditional chunks (threshold: {})", f.bump_count, t.function.bump_count),
        });
    }

    if f.max_nesting >= t.function.nesting_depth {
        findings.push(Finding {
            smell: Smell::DeepNestedComplexity,
            location: func_loc(f),
            detail: format!("depth={} (threshold: {})", f.max_nesting, t.function.nesting_depth),
        });
    }

    if f.compound_condition_count > t.function.compound_conditions {
        findings.push(Finding {
            smell: Smell::ComplexConditional,
            location: func_loc(f),
            detail: format!(
                "{} complex conditions (threshold: {})",
                f.compound_condition_count, t.function.compound_conditions
            ),
        });
    }
}

fn detect_argument_smells(f: &FunctionMetrics, t: &Thresholds, findings: &mut Vec<Finding>) {
    if !f.is_constructor {
        if f.arg_count > t.function.arg_max {
            findings.push(Finding {
                smell: Smell::ExcessArguments,
                location: func_loc(f),
                detail: format!("{} args (threshold: {})", f.arg_count, t.function.arg_max),
            });
        }
        return;
    }
    let deps = f.typed_param_count.saturating_sub(f.primitive_type_count);
    let too_many = f.arg_count > t.function.constructor_arg_max;
    let over_injected = deps >= t.analysis.constructor_dep_injection_min;
    if !too_many && !over_injected {
        return;
    }
    let detail = if over_injected {
        format!("{deps} non-primitive dependencies injected (threshold: {})", t.analysis.constructor_dep_injection_min)
    } else {
        format!("{} args (threshold: {})", f.arg_count, t.function.constructor_arg_max)
    };
    findings.push(Finding { smell: Smell::ConstructorOverInjection, location: func_loc(f), detail });
}

fn detect_embedded_smells(f: &FunctionMetrics, t: &Thresholds, findings: &mut Vec<Finding>) {
    if f.max_embedded_block_loc > t.function.embedded_block_loc {
        findings.push(Finding {
            smell: Smell::LargeEmbeddedBlock,
            location: func_loc(f),
            detail: format!(
                "{} lines of embedded content (threshold: {})",
                f.max_embedded_block_loc, t.function.embedded_block_loc
            ),
        });
    }
}

fn detect_primitive_obsession(functions: &[FunctionMetrics], t: &Thresholds, findings: &mut Vec<Finding>) {
    for f in functions {
        if !has_high_primitive_ratio(f, t) {
            continue;
        }
        let ratio = f.primitive_type_count as f32 / f.typed_param_count as f32;
        findings.push(Finding {
            smell: Smell::PrimitiveObsession,
            location: func_loc(f),
            detail: format!(
                "{}/{} typed params are primitives ({:.0}%); {} share one primitive type (data clump)",
                f.primitive_type_count,
                f.typed_param_count,
                ratio * 100.0,
                f.max_same_primitive_count
            ),
        });
    }
}

fn has_high_primitive_ratio(f: &FunctionMetrics, t: &Thresholds) -> bool {
    if f.typed_param_count < t.analysis.primitive_min_typed_params || f.primitive_type_count == 0 {
        return false;
    }
    if f.max_same_primitive_count < t.analysis.primitive_min_same_count {
        return false;
    }
    let ratio = f.primitive_type_count as f32 / f.typed_param_count as f32;
    ratio >= t.analysis.primitive_ratio_threshold
}

fn detect_batch_thresholds(functions: &[FunctionMetrics], t: &Thresholds, findings: &mut Vec<Finding>) {
    let assert_threshold = t.analysis.consecutive_asserts_max;
    let string_threshold = t.analysis.max_string_match_arms;
    for f in functions {
        if f.consecutive_asserts > assert_threshold {
            findings.push(Finding {
                smell: Smell::LargeAssertionBlock,
                location: func_loc(f),
                detail: format!("{} consecutive assertions (threshold: {assert_threshold})", f.consecutive_asserts),
            });
        }
        if f.string_match_arms > string_threshold {
            findings.push(Finding {
                smell: Smell::StringlyTypedSwitch,
                location: func_loc(f),
                detail: format!(
                    "match/switch on string with {} arms (threshold: {string_threshold})",
                    f.string_match_arms
                ),
            });
        }
    }
}

fn detect_empty_error_handlers(functions: &[FunctionMetrics], findings: &mut Vec<Finding>) {
    findings.extend(functions.iter().filter(|f| f.empty_catch_count > 0).map(|f| Finding {
        smell: Smell::EmptyErrorHandler,
        location: func_loc(f),
        detail: format!("{} empty catch block{}", f.empty_catch_count, if f.empty_catch_count == 1 { "" } else { "s" }),
    }));
}

fn detect_short_variable_names(f: &FunctionMetrics, t: &Thresholds, findings: &mut Vec<Finding>) {
    let dominated = f.loc < t.analysis.short_var_min_fn_loc || f.short_var_count <= t.analysis.short_var_max_count;
    if dominated {
        return;
    }
    findings.push(Finding {
        smell: Smell::ShortVariableNames,
        location: func_loc(f),
        detail: format!(
            "{} single-char variables in {} LOC function (threshold: {})",
            f.short_var_count, f.loc, t.analysis.short_var_max_count
        ),
    });
}
