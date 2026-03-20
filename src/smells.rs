use crate::thresholds::Thresholds;
use crate::walk::{FileMetrics, FunctionMetrics, ModuleMetrics};

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

pub fn detect(
    (functions, module): &FileMetrics,
    _source: &str,
    t: &Thresholds,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut has_god_method = false;

    for f in functions {
        detect_function_smells(f, t, &mut findings, &mut has_god_method);
    }

    detect_module_smells(module, t, has_god_method, &mut findings);

    findings
}

fn detect_function_smells(
    f: &FunctionMetrics,
    t: &Thresholds,
    findings: &mut Vec<Finding>,
    has_god_method: &mut bool,
) {
    let loc = |f: &FunctionMetrics| Location::Function {
        name: f.name.clone(),
        start_line: f.start_line,
        end_line: f.end_line,
    };

    let is_complex = f.cc >= t.cc_warning;
    let is_large = f.loc >= t.fn_loc_warning;

    // God Method: both complex AND large
    if is_complex && is_large {
        *has_god_method = true;
        findings.push(Finding {
            smell: "God Method",
            location: loc(f),
            detail: format!("cc={}, {} lines (both thresholds exceeded)", f.cc, f.loc),
        });
    } else {
        // Report individually only if not already a god method
        if is_complex {
            let severity = if f.cc > t.cc_alert { "alert" } else { "warning" };
            findings.push(Finding {
                smell: "Complex Method",
                location: loc(f),
                detail: format!("cc={} [{}]", f.cc, severity),
            });
        }
        if is_large {
            let severity = if f.loc > t.fn_loc_alert { "alert" } else { "warning" };
            findings.push(Finding {
                smell: "Large Method",
                location: loc(f),
                detail: format!("{} lines [{}]", f.loc, severity),
            });
        }
    }

    if f.bump_count >= t.bump_count {
        findings.push(Finding {
            smell: "Nested Conditional Chunks",
            location: loc(f),
            detail: format!("{} nested conditional chunks", f.bump_count),
        });
    }

    if f.max_nesting > t.nesting_depth {
        findings.push(Finding {
            smell: "Deep Nested Complexity",
            location: loc(f),
            detail: format!("depth={}", f.max_nesting),
        });
    }

    if f.compound_condition_count > t.compound_conditions {
        findings.push(Finding {
            smell: "Complex Conditional",
            location: loc(f),
            detail: format!("{} complex conditions", f.compound_condition_count),
        });
    }

    let arg_threshold = if f.is_constructor {
        t.constructor_arg_max
    } else {
        t.arg_max
    };
    if f.arg_count > arg_threshold {
        let smell = if f.is_constructor {
            "Constructor Over-Injection"
        } else {
            "Excess Arguments"
        };
        findings.push(Finding {
            smell,
            location: loc(f),
            detail: format!("{} args", f.arg_count),
        });
    }

    if f.max_embedded_block_loc > t.embedded_block_loc {
        findings.push(Finding {
            smell: "Large Embedded Block",
            location: loc(f),
            detail: format!("{} lines of embedded content", f.max_embedded_block_loc),
        });
    }
}

fn detect_module_smells(
    m: &ModuleMetrics,
    t: &Thresholds,
    has_god_method: bool,
    findings: &mut Vec<Finding>,
) {
    if m.total_loc > t.file_loc_warning {
        let severity = if m.total_loc > t.file_loc_alert {
            "alert"
        } else {
            "warning"
        };
        findings.push(Finding {
            smell: "File Too Large",
            location: Location::Module,
            detail: format!("{} LOC [{}]", m.total_loc, severity),
        });
    }

    if m.total_functions > t.file_function_count {
        findings.push(Finding {
            smell: "Too Many Functions",
            location: Location::Module,
            detail: format!("{} functions", m.total_functions),
        });
    }

    if m.sum_cc > t.file_total_cc {
        findings.push(Finding {
            smell: "Overall Code Complexity",
            location: Location::Module,
            detail: format!("total cc={}", m.sum_cc),
        });
    }

    // God Class: large file + many functions + has a god method
    if m.total_loc > t.file_loc_warning
        && m.total_functions > t.file_function_count
        && has_god_method
    {
        findings.push(Finding {
            smell: "God Class",
            location: Location::Module,
            detail: format!(
                "{} LOC, {} functions, contains god method(s)",
                m.total_loc, m.total_functions
            ),
        });
    }

    if m.global_conditional_count > 0 {
        findings.push(Finding {
            smell: "Global Conditionals",
            location: Location::Module,
            detail: format!("{} conditionals at module scope", m.global_conditional_count),
        });
    }

    if m.global_max_nesting > t.nesting_depth {
        findings.push(Finding {
            smell: "Deep Global Nesting",
            location: Location::Module,
            detail: format!("depth={} at module scope", m.global_max_nesting),
        });
    }
}
