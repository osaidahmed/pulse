use std::collections::{BTreeMap, HashMap};

use crate::duplication::is_test_function;
use pulse_core::{Finding, Location, Smell};
use pulse_syntax::walk::FunctionMetrics;
use pulse_syntax::walk::ModuleMetrics;
use pulse_thresholds::Thresholds;

pub fn detect_module_smells(m: &ModuleMetrics, t: &Thresholds, has_god_method: bool, findings: &mut Vec<Finding>) {
    detect_size_smells(m, t, has_god_method, findings);
    detect_global_scope_smells(m, t, findings);
    detect_large_structs(m, t, findings);
}

fn detect_large_structs(m: &ModuleMetrics, t: &Thresholds, findings: &mut Vec<Finding>) {
    findings.extend(m.struct_fields.iter().filter_map(|(name, count)| {
        (*count > t.module.max_struct_fields).then_some(Finding {
            smell: Smell::LargeStruct,
            location: Location::Module,
            detail: format!("{name}: {count} fields (threshold: {})", t.module.max_struct_fields),
        })
    }));
}

fn detect_size_smells(m: &ModuleMetrics, t: &Thresholds, has_god_method: bool, findings: &mut Vec<Finding>) {
    if m.total_loc >= t.module.file_loc_warning {
        let sev = if m.total_loc > t.module.file_loc_alert { "alert" } else { "warning" };
        emit_module(
            Smell::FileTooLarge,
            format!("{} LOC [{sev}] (threshold: {})", m.total_loc, t.module.file_loc_warning),
            findings,
        );
    }
    emit_module_if(
        m.total_functions > t.module.file_function_count,
        Smell::TooManyFunctions,
        || format!("{} functions (threshold: {})", m.total_functions, t.module.file_function_count),
        findings,
    );
    emit_module_if(
        m.sum_cc > t.module.file_total_cc,
        Smell::OverallCodeComplexity,
        || format!("total cc={} (threshold: {})", m.sum_cc, t.module.file_total_cc),
        findings,
    );
    check_god_class(m, t, has_god_method, findings);
    emit_module_if(
        m.declaration_count > t.module.max_declarations,
        Smell::ExcessiveDeclarations,
        || format!("{} declarations in one file (threshold: {})", m.declaration_count, t.module.max_declarations),
        findings,
    );
}

fn emit_module(smell: Smell, detail: String, findings: &mut Vec<Finding>) {
    findings.push(Finding { smell, location: Location::Module, detail });
}

fn emit_module_if(condition: bool, smell: Smell, detail: impl FnOnce() -> String, findings: &mut Vec<Finding>) {
    if condition {
        emit_module(smell, detail(), findings);
    }
}

fn check_god_class(m: &ModuleMetrics, t: &Thresholds, has_god_method: bool, findings: &mut Vec<Finding>) {
    let is_god_class =
        m.total_loc >= t.module.file_loc_warning && m.total_functions > t.module.file_function_count && has_god_method;
    if is_god_class {
        findings.push(Finding {
            smell: Smell::GodClass,
            location: Location::Module,
            detail: format!("{} LOC, {} functions, contains god method(s)", m.total_loc, m.total_functions),
        });
    }
}

fn detect_global_scope_smells(m: &ModuleMetrics, t: &Thresholds, findings: &mut Vec<Finding>) {
    if m.global_conditional_count > t.module.global_conditionals_max {
        findings.push(Finding {
            smell: Smell::GlobalConditionals,
            location: Location::Module,
            detail: format!("{} conditionals at module scope", m.global_conditional_count),
        });
    }

    if m.global_max_nesting >= t.module.global_nesting_depth {
        findings.push(Finding {
            smell: Smell::DeepGlobalNesting,
            location: Location::Module,
            detail: format!("depth={} at module scope", m.global_max_nesting),
        });
    }
}

// ─── Overall function size ─────────────────────────────────────────────

pub fn detect_overall_function_size(functions: &[FunctionMetrics], t: &Thresholds, findings: &mut Vec<Finding>) {
    let large_count = functions.iter().filter(|f| f.loc >= t.module.large_fn_loc).count() as u32;
    if large_count < t.module.large_fn_count {
        return;
    }
    let names: Vec<String> = functions
        .iter()
        .filter(|f| f.loc >= t.module.large_fn_loc)
        .map(|f| format!("{} ({}L)", f.name, f.loc))
        .collect();
    findings.push(Finding {
        smell: Smell::OverallFunctionSize,
        location: Location::Module,
        detail: format!(
            "{} large functions (>{} LOC, threshold: {}+ functions): {}",
            large_count,
            t.module.large_fn_loc,
            t.module.large_fn_count,
            names.join(", ")
        ),
    });
}

// ─── LCOM4 cohesion ────────────────────────────────────────────────────

pub fn detect_lcom4(functions: &[FunctionMetrics], t: &Thresholds, findings: &mut Vec<Finding>) {
    for (class_name, methods) in &group_methods_by_class(functions) {
        let components = compute_lcom4(methods);
        if components >= t.analysis.lcom4_warning {
            findings.push(Finding {
                smell: Smell::LowCohesion,
                location: Location::Module,
                detail: format!("{class_name}: LCOM4={components} ({components} disconnected method groups)"),
            });
        }
    }
}

fn group_methods_by_class(functions: &[FunctionMetrics]) -> BTreeMap<String, Vec<&FunctionMetrics>> {
    let mut groups: BTreeMap<String, Vec<&FunctionMetrics>> = BTreeMap::new();
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
    let unqualified: Vec<&str> = non_init.iter().map(|m| unqualified_name(&m.name)).collect();
    let calls_method = |caller: usize, callee: usize| -> bool {
        non_init[caller].field_accesses.iter().any(|f| f == unqualified[callee])
    };
    let connected =
        |i: usize, j: usize| -> bool { i != j && (shared_field(i, j) || calls_method(i, j) || calls_method(j, i)) };

    count_connected_components(n, connected)
}

fn unqualified_name(name: &str) -> &str {
    let after_dot = name.rsplit('.').next().unwrap_or(name);
    after_dot.rsplit("::").next().unwrap_or(after_dot)
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

pub fn detect_duplicated_assertion_blocks(functions: &[FunctionMetrics], t: &Thresholds, findings: &mut Vec<Finding>) {
    let test_fns: Vec<(usize, &FunctionMetrics)> = functions
        .iter()
        .enumerate()
        .filter(|(_, f)| is_test_function(&f.name) && f.consecutive_asserts >= t.analysis.dup_assert_min)
        .collect();

    if test_fns.len() < 2 {
        return;
    }

    let mut hash_groups: HashMap<u64, Vec<usize>> = HashMap::new();
    for &(i, f) in &test_fns {
        hash_groups.entry(f.assert_hash).or_default().push(i);
    }

    let mut groups: Vec<&Vec<usize>> = hash_groups.values().collect();
    groups.sort_by_key(|indices| indices.first().copied());
    for indices in groups {
        if indices.len() < 2 {
            continue;
        }
        let names: Vec<String> = indices.iter().map(|&i| functions[i].name.clone()).collect();
        findings.push(Finding {
            smell: Smell::DuplicatedAssertionBlocks,
            location: Location::Module,
            detail: format!("{} test functions with identical assertion structure: {}", names.len(), names.join(", ")),
        });
    }
}
