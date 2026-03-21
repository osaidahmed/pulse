use std::collections::HashMap;

use crate::smells::{Finding, Location};
use crate::thresholds::Thresholds;
use crate::walk::FunctionMetrics;
use crate::walk::ModuleMetrics;

pub fn detect_module_smells(
    m: &ModuleMetrics,
    t: &Thresholds,
    has_god_method: bool,
    findings: &mut Vec<Finding>,
) {
    detect_size_smells(m, t, has_god_method, findings);
    detect_global_scope_smells(m, findings);
}

fn detect_size_smells(
    m: &ModuleMetrics,
    t: &Thresholds,
    has_god_method: bool,
    findings: &mut Vec<Finding>,
) {
    if m.total_loc > t.file_loc_warning {
        let sev = if m.total_loc > t.file_loc_alert { "alert" } else { "warning" };
        emit_module("File Too Large", format!("{} LOC [{sev}] (threshold: {})", m.total_loc, t.file_loc_warning), findings);
    }
    emit_module_if(m.total_functions > t.file_function_count,
        "Too Many Functions", || format!("{} functions (threshold: {})", m.total_functions, t.file_function_count), findings);
    emit_module_if(m.sum_cc > t.file_total_cc,
        "Overall Code Complexity", || format!("total cc={} (threshold: {})", m.sum_cc, t.file_total_cc), findings);
    check_god_class(m, t, has_god_method, findings);
    emit_module_if(m.declaration_count > t.max_declarations,
        "Excessive Declarations", || format!("{} declarations in one file (threshold: {})", m.declaration_count, t.max_declarations), findings);
}

fn emit_module(smell: &'static str, detail: String, findings: &mut Vec<Finding>) {
    findings.push(Finding { smell, location: Location::Module, detail });
}

fn emit_module_if(
    condition: bool,
    smell: &'static str,
    detail: impl FnOnce() -> String,
    findings: &mut Vec<Finding>,
) {
    if condition {
        emit_module(smell, detail(), findings);
    }
}

fn check_god_class(
    m: &ModuleMetrics,
    t: &Thresholds,
    has_god_method: bool,
    findings: &mut Vec<Finding>,
) {
    let is_god_class =
        m.total_loc > t.file_loc_warning && m.total_functions > t.file_function_count && has_god_method;
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

fn detect_global_scope_smells(m: &ModuleMetrics, findings: &mut Vec<Finding>) {
    if m.global_conditional_count > 0 {
        findings.push(Finding {
            smell: "Global Conditionals",
            location: Location::Module,
            detail: format!("{} conditionals at module scope", m.global_conditional_count),
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

// ─── Code duplication ──────────────────────────────────────────────────

pub fn detect_code_duplication(
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
        groups.entry(functions[i].structural_hash).or_default().push(i);
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
        groups.entry(functions[i].skeleton_hash).or_default().push(i);
    }

    let already_reported: std::collections::HashSet<usize> = findings
        .iter()
        .filter(|f| f.smell == "Code Duplication")
        .flat_map(|f| extract_line_numbers(&f.detail))
        .collect();

    let filtered: HashMap<u64, Vec<usize>> = groups
        .into_iter()
        .map(|(k, indices)| {
            let novel: Vec<usize> = indices
                .into_iter()
                .filter(|&i| !already_reported.contains(&(functions[i].start_line as usize)))
                .collect();
            let similar = are_size_similar(&novel, functions);
            (k, if similar { novel } else { vec![] })
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

fn extract_line_numbers(detail: &str) -> Vec<usize> {
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
            && !indices.iter().all(|&i| is_test_function(&functions[i].name))
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
            detail: format!("{} functions with {}: {}", members.len(), label, members.join(", ")),
        });
    }
}

fn is_test_function(name: &str) -> bool {
    let base = name.rsplit('.').next().unwrap_or(name);
    base.starts_with("test_")
}

// ─── Overall function size ─────────────────────────────────────────────

pub fn detect_overall_function_size(
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
        detail: format!("{} large functions (>{} LOC, threshold: {}+ functions): {}", large_count, t.large_fn_loc, t.large_fn_count, names.join(", ")),
    });
}

// ─── LCOM4 cohesion ────────────────────────────────────────────────────

pub fn detect_lcom4(functions: &[FunctionMetrics], t: &Thresholds, findings: &mut Vec<Finding>) {
    for (class_name, methods) in &group_methods_by_class(functions) {
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
        non_init[i].field_accesses.iter().any(|f| non_init[j].field_accesses.contains(f))
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
        visit_component(start, &mut visited, &connected);
    }

    components
}

fn visit_component(start: usize, visited: &mut [bool], connected: &impl Fn(usize, usize) -> bool) {
    let mut stack = vec![start];
    while let Some(node) = stack.pop() {
        if visited[node] {
            continue;
        }
        visited[node] = true;
        for (i, v) in visited.iter().enumerate() {
            if !v && connected(node, i) {
                stack.push(i);
            }
        }
    }
}

// ─── Duplicated assertion blocks ───────────────────────────────────────

pub fn detect_duplicated_assertion_blocks(
    functions: &[FunctionMetrics],
    findings: &mut Vec<Finding>,
) {
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
