use std::collections::{HashMap, HashSet};

use crate::smells::{Finding, Location, Smell};
use crate::thresholds::Thresholds;
use crate::walk::FunctionMetrics;

pub fn detect_code_duplication(functions: &[FunctionMetrics], t: &Thresholds, findings: &mut Vec<Finding>) {
    let eligible: Vec<usize> =
        functions.iter().enumerate().filter(|(_, f)| f.loc >= t.analysis.duplication.min_loc).map(|(i, _)| i).collect();

    let reported = detect_exact_clones(&eligible, functions, t, findings);
    detect_similar_clones(functions, t, &reported, findings);
}

fn detect_exact_clones(
    eligible: &[usize],
    functions: &[FunctionMetrics],
    t: &Thresholds,
    findings: &mut Vec<Finding>,
) -> HashSet<usize> {
    let mut groups: HashMap<u64, Vec<usize>> = HashMap::new();
    for &i in eligible {
        groups.entry(functions[i].structural_hash).or_default().push(i);
    }
    let filtered: HashMap<u64, Vec<usize>> = groups
        .into_iter()
        .map(|(k, indices)| {
            let keep = are_size_similar(&indices, functions) && has_distinct_kind_variety(&indices, functions, t);
            (k, if keep { indices } else { vec![] })
        })
        .collect();
    emit_duplication_findings(&filtered, functions, t, findings, "identical structure")
}

fn detect_similar_clones(
    functions: &[FunctionMetrics],
    t: &Thresholds,
    already_reported: &HashSet<usize>,
    findings: &mut Vec<Finding>,
) {
    // Skeleton matches use a higher LOC floor than exact matches
    let skeleton_eligible: Vec<usize> = functions
        .iter()
        .enumerate()
        .filter(|(_, f)| f.loc >= t.analysis.duplication.skeleton_min_loc)
        .map(|(i, _)| i)
        .collect();
    let mut groups: HashMap<u64, Vec<usize>> = HashMap::new();
    for &i in &skeleton_eligible {
        groups.entry(functions[i].skeleton_hash).or_default().push(i);
    }

    let filtered: HashMap<u64, Vec<usize>> = groups
        .into_iter()
        .map(|(k, indices)| {
            let novel: Vec<usize> = indices.into_iter().filter(|i| !already_reported.contains(i)).collect();
            let similar = are_size_similar(&novel, functions);
            (k, if similar { novel } else { vec![] })
        })
        .collect();

    emit_duplication_findings(&filtered, functions, t, findings, "similar structure");
}

fn has_distinct_kind_variety(indices: &[usize], functions: &[FunctionMetrics], t: &Thresholds) -> bool {
    indices
        .iter()
        .map(|&i| functions[i].distinct_node_kinds)
        .min()
        .is_some_and(|min| min >= t.analysis.duplication.min_distinct_kinds)
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

fn emit_duplication_findings(
    groups: &HashMap<u64, Vec<usize>>,
    functions: &[FunctionMetrics],
    t: &Thresholds,
    findings: &mut Vec<Finding>,
    label: &str,
) -> HashSet<usize> {
    let duplicated = groups.values().filter(|indices| {
        indices.len() >= t.analysis.duplication.min_group as usize
            && !indices.iter().all(|&i| is_test_function(&functions[i].name))
    });

    let mut reported = HashSet::new();
    for indices in duplicated {
        reported.extend(indices.iter().copied());
        let members: Vec<String> = indices
            .iter()
            .map(|&i| {
                let f = &functions[i];
                format!("{} (L{}-{})", f.name, f.start_line, f.end_line)
            })
            .collect();
        findings.push(Finding {
            smell: Smell::CodeDuplication,
            location: Location::Module,
            detail: format!("{} functions with {}: {}", members.len(), label, members.join(", ")),
        });
    }
    reported
}

pub(crate) fn is_test_function(name: &str) -> bool {
    let base = name.rsplit('.').next().unwrap_or(name);
    base.starts_with("test_")
}
