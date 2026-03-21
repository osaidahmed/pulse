use std::collections::HashMap;

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

pub fn detect((functions, module): &FileMetrics, _source: &str, t: &Thresholds) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut has_god_method = false;

    for f in functions {
        detect_function_smells(f, t, &mut findings, &mut has_god_method);
    }

    detect_module_smells(module, t, has_god_method, &mut findings);
    detect_code_duplication(functions, t, &mut findings);
    detect_overall_function_size(functions, t, &mut findings);
    detect_primitive_obsession(functions, t, &mut findings);
    detect_assertion_smells(functions, t, &mut findings);
    detect_lcom4(functions, t, &mut findings);
    detect_empty_error_handlers(functions, &mut findings);

    findings
}

fn func_loc(f: &FunctionMetrics) -> Location {
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
            let sev = if f.cc > t.cc_alert {
                "alert"
            } else {
                "warning"
            };
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
    let detail = complexity_detail(f, t, cc_complex, cogc_complex);
    findings.push(Finding {
        smell: "Complex Method",
        location: func_loc(f),
        detail,
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
    let severity = if f.loc > t.fn_loc_alert {
        "alert"
    } else {
        "warning"
    };
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

fn detect_module_smells(
    m: &ModuleMetrics,
    t: &Thresholds,
    has_god_method: bool,
    findings: &mut Vec<Finding>,
) {
    detect_size_smells(m, t, has_god_method, findings);
    detect_global_scope_smells(m, t, findings);
}

fn detect_size_smells(
    m: &ModuleMetrics,
    t: &Thresholds,
    has_god_method: bool,
    findings: &mut Vec<Finding>,
) {
    check_file_loc(m, t, findings);
    check_function_count(m, t, findings);
    check_overall_cc(m, t, findings);
    check_god_class(m, t, has_god_method, findings);
    check_declarations(m, t, findings);
}

fn check_file_loc(m: &ModuleMetrics, t: &Thresholds, findings: &mut Vec<Finding>) {
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
}

fn check_function_count(m: &ModuleMetrics, t: &Thresholds, findings: &mut Vec<Finding>) {
    if m.total_functions > t.file_function_count {
        findings.push(Finding {
            smell: "Too Many Functions",
            location: Location::Module,
            detail: format!("{} functions", m.total_functions),
        });
    }
}

fn check_overall_cc(m: &ModuleMetrics, t: &Thresholds, findings: &mut Vec<Finding>) {
    if m.sum_cc > t.file_total_cc {
        findings.push(Finding {
            smell: "Overall Code Complexity",
            location: Location::Module,
            detail: format!("total cc={}", m.sum_cc),
        });
    }
}

fn check_god_class(
    m: &ModuleMetrics,
    t: &Thresholds,
    has_god_method: bool,
    findings: &mut Vec<Finding>,
) {
    let is_god_class = m.total_loc > t.file_loc_warning
        && m.total_functions > t.file_function_count
        && has_god_method;
    if is_god_class {
        findings.push(Finding {
            smell: "God Class",
            location: Location::Module,
            detail: format!(
                "{} LOC, {} functions, contains god method(s)",
                m.total_loc, m.total_functions
            ),
        });
    }
}

fn check_declarations(m: &ModuleMetrics, t: &Thresholds, findings: &mut Vec<Finding>) {
    if m.declaration_count > t.max_declarations {
        findings.push(Finding {
            smell: "Excessive Declarations",
            location: Location::Module,
            detail: format!("{} declarations in one file", m.declaration_count),
        });
    }
}

fn detect_global_scope_smells(m: &ModuleMetrics, _t: &Thresholds, findings: &mut Vec<Finding>) {
    if m.global_conditional_count > 0 {
        findings.push(Finding {
            smell: "Global Conditionals",
            location: Location::Module,
            detail: format!(
                "{} conditionals at module scope",
                m.global_conditional_count
            ),
        });
    }

    if m.global_max_nesting >= 3 {
        findings.push(Finding {
            smell: "Deep Global Nesting",
            location: Location::Module,
            detail: format!("depth={} at module scope", m.global_max_nesting),
        });
    }
}

fn detect_code_duplication(
    functions: &[FunctionMetrics],
    t: &Thresholds,
    findings: &mut Vec<Finding>,
) {
    let eligible: Vec<usize> = functions
        .iter()
        .enumerate()
        .filter(|(_, f)| f.loc >= t.duplication_min_loc)
        .map(|(i, _)| i)
        .collect();

    detect_exact_clones(&eligible, functions, t, findings);
    detect_similar_clones(&eligible, functions, t, findings);
}

fn detect_exact_clones(
    eligible: &[usize],
    functions: &[FunctionMetrics],
    t: &Thresholds,
    findings: &mut Vec<Finding>,
) {
    let mut groups: HashMap<u64, Vec<usize>> = HashMap::new();
    for &i in eligible {
        groups
            .entry(functions[i].structural_hash)
            .or_default()
            .push(i);
    }
    emit_duplication_findings(&groups, functions, t, findings, "identical structure");
}

fn detect_similar_clones(
    eligible: &[usize],
    functions: &[FunctionMetrics],
    t: &Thresholds,
    findings: &mut Vec<Finding>,
) {
    let mut groups: HashMap<u64, Vec<usize>> = HashMap::new();
    for &i in eligible {
        groups
            .entry(functions[i].skeleton_hash)
            .or_default()
            .push(i);
    }

    // Only report skeleton groups that weren't already caught by exact matching
    let already_reported: std::collections::HashSet<usize> = findings
        .iter()
        .filter(|f| f.smell == "Code Duplication")
        .flat_map(|f| extract_line_numbers_from_detail(&f.detail))
        .collect();

    // Filter to size-similar functions and exclude already-reported ones
    let filtered: HashMap<u64, Vec<usize>> = groups
        .into_iter()
        .map(|(k, indices)| {
            let novel: Vec<usize> = indices
                .into_iter()
                .filter(|&i| !already_reported.contains(&(functions[i].start_line as usize)))
                .collect();
            // Keep only groups where all functions are similar in size (within 50%)
            let size_similar = are_size_similar(&novel, functions);
            (k, if size_similar { novel } else { vec![] })
        })
        .collect();

    emit_duplication_findings(&filtered, functions, t, findings, "similar structure");
}

fn are_size_similar(indices: &[usize], functions: &[FunctionMetrics]) -> bool {
    if indices.len() < 2 {
        return false;
    }
    let locs: Vec<u32> = indices.iter().map(|&i| functions[i].loc).collect();
    let min = *locs.iter().min().unwrap_or(&0);
    let max = *locs.iter().max().unwrap_or(&0);
    if min == 0 {
        return false;
    }
    (max as f32 / min as f32) <= 1.3
}

fn extract_line_numbers_from_detail(detail: &str) -> Vec<usize> {
    detail
        .split("(L")
        .skip(1)
        .filter_map(|s| s.split('-').next()?.parse().ok())
        .collect()
}

fn emit_duplication_findings(
    groups: &HashMap<u64, Vec<usize>>,
    functions: &[FunctionMetrics],
    t: &Thresholds,
    findings: &mut Vec<Finding>,
    label: &str,
) {
    let duplicated = groups.values().filter(|indices| {
        indices.len() >= t.duplication_min_group as usize
            && !indices
                .iter()
                .all(|&i| is_test_function(&functions[i].name))
    });

    for indices in duplicated {
        let members: Vec<String> = indices
            .iter()
            .map(|&i| {
                let f = &functions[i];
                format!("{} (L{}-{})", f.name, f.start_line, f.end_line)
            })
            .collect();
        findings.push(Finding {
            smell: "Code Duplication",
            location: Location::Module,
            detail: format!(
                "{} functions with {}: {}",
                members.len(),
                label,
                members.join(", ")
            ),
        });
    }
}

fn is_test_function(name: &str) -> bool {
    let base = name.rsplit('.').next().unwrap_or(name);
    base.starts_with("test_")
}

fn detect_overall_function_size(
    functions: &[FunctionMetrics],
    t: &Thresholds,
    findings: &mut Vec<Finding>,
) {
    let large_count = functions.iter().filter(|f| f.loc >= t.large_fn_loc).count() as u32;
    if large_count < t.large_fn_count {
        return;
    }
    let names: Vec<String> = functions
        .iter()
        .filter(|f| f.loc >= t.large_fn_loc)
        .map(|f| format!("{} ({}L)", f.name, f.loc))
        .collect();
    findings.push(Finding {
        smell: "Overall Function Size",
        location: Location::Module,
        detail: format!("{} large functions: {}", large_count, names.join(", ")),
    });
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

fn detect_assertion_smells(
    functions: &[FunctionMetrics],
    t: &Thresholds,
    findings: &mut Vec<Finding>,
) {
    detect_large_assertion_blocks(functions, t, findings);
    detect_duplicated_assertion_blocks(functions, findings);
}

fn detect_large_assertion_blocks(
    functions: &[FunctionMetrics],
    t: &Thresholds,
    findings: &mut Vec<Finding>,
) {
    for f in functions {
        if f.consecutive_asserts > t.consecutive_asserts_max {
            findings.push(Finding {
                smell: "Large Assertion Block",
                location: func_loc(f),
                detail: format!("{} consecutive assertions", f.consecutive_asserts),
            });
        }
    }
}

fn detect_duplicated_assertion_blocks(functions: &[FunctionMetrics], findings: &mut Vec<Finding>) {
    let test_fns: Vec<(usize, &FunctionMetrics)> = functions
        .iter()
        .enumerate()
        .filter(|(_, f)| is_test_function(&f.name) && f.consecutive_asserts > 5)
        .collect();

    if test_fns.len() < 2 {
        return;
    }

    let mut hash_groups: HashMap<u64, Vec<usize>> = HashMap::new();
    for &(i, f) in &test_fns {
        hash_groups.entry(f.assert_hash).or_default().push(i);
    }

    for indices in hash_groups.values() {
        if indices.len() < 2 {
            continue;
        }
        let names: Vec<String> = indices.iter().map(|&i| functions[i].name.clone()).collect();
        findings.push(Finding {
            smell: "Duplicated Assertion Blocks",
            location: Location::Module,
            detail: format!(
                "{} test functions with identical assertion structure: {}",
                names.len(),
                names.join(", ")
            ),
        });
    }
}

fn detect_lcom4(functions: &[FunctionMetrics], t: &Thresholds, findings: &mut Vec<Finding>) {
    let class_methods = group_methods_by_class(functions);

    for (class_name, methods) in &class_methods {
        let components = compute_lcom4(methods);
        if components >= t.lcom4_warning {
            findings.push(Finding {
                smell: "Low Cohesion",
                location: Location::Module,
                detail: format!(
                    "{class_name}: LCOM4={components} ({components} disconnected method groups)"
                ),
            });
        }
    }
}

fn group_methods_by_class(functions: &[FunctionMetrics]) -> HashMap<String, Vec<&FunctionMetrics>> {
    let mut groups: HashMap<String, Vec<&FunctionMetrics>> = HashMap::new();
    for f in functions {
        if let Some(ref class_name) = f.class_name {
            groups.entry(class_name.clone()).or_default().push(f);
        }
    }
    groups
}

fn compute_lcom4(methods: &[&FunctionMetrics]) -> u32 {
    if methods.len() < 3 {
        return 1;
    }

    let non_init: Vec<&&FunctionMetrics> = methods.iter().filter(|m| !m.is_constructor).collect();
    if non_init.len() < 2 {
        return 1;
    }

    let n = non_init.len();
    let shared_field = |i: usize, j: usize| -> bool {
        non_init[i]
            .field_accesses
            .iter()
            .any(|f| non_init[j].field_accesses.contains(f))
    };

    count_connected_components(n, shared_field)
}

fn count_connected_components(n: usize, connected: impl Fn(usize, usize) -> bool) -> u32 {
    let mut visited = vec![false; n];
    let mut components: u32 = 0;

    for start in 0..n {
        if visited[start] {
            continue;
        }
        components += 1;
        let mut stack = vec![start];
        while let Some(node) = stack.pop() {
            if visited[node] {
                continue;
            }
            visited[node] = true;
            for (neighbor, visited_flag) in visited.iter().enumerate() {
                if !visited_flag && connected(node, neighbor) {
                    stack.push(neighbor);
                }
            }
        }
    }

    components
}

fn detect_empty_error_handlers(functions: &[FunctionMetrics], findings: &mut Vec<Finding>) {
    for f in functions {
        if f.empty_catch_count > 0 {
            findings.push(Finding {
                smell: "Empty Error Handler",
                location: func_loc(f),
                detail: format!(
                    "{} empty catch block{}",
                    f.empty_catch_count,
                    if f.empty_catch_count == 1 { "" } else { "s" }
                ),
            });
        }
    }
}
