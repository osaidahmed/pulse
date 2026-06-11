use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::smells::{smell_from_snake_case, Finding, Location, Smell};
use crate::walk::{FileMetrics, FunctionMetrics};

#[derive(Debug, Serialize, Deserialize)]
pub struct FunctionBaselineEntry {
    pub name: String,
    pub hash: u64,
    pub smell: String,
    pub mags: [u32; 3],
    pub count: u32,
}

#[derive(Default)]
pub struct FunctionBaseline {
    by_name: HashMap<(String, Smell), ([u32; 3], u32)>,
    by_hash: HashMap<(u64, Smell), (String, [u32; 3], u32)>,
}

pub fn magnitudes(smell: Smell, f: &FunctionMetrics) -> [u32; 3] {
    let deps = f.typed_param_count.saturating_sub(f.primitive_type_count);
    let table: [(Smell, [u32; 3]); 14] = [
        (Smell::GodMethod, [f.cc, f.cognitive_complexity, f.loc]),
        (Smell::ComplexMethod, [f.cc, f.cognitive_complexity, 0]),
        (Smell::LargeMethod, [0, 0, f.loc]),
        (Smell::NestedConditionalChunks, [f.bump_count, 0, 0]),
        (Smell::DeepNestedComplexity, [f.max_nesting, 0, 0]),
        (Smell::ComplexConditional, [f.compound_condition_count, 0, 0]),
        (Smell::ExcessArguments, [f.arg_count, 0, 0]),
        (Smell::ConstructorOverInjection, [f.arg_count, deps, 0]),
        (Smell::LargeEmbeddedBlock, [f.max_embedded_block_loc, 0, 0]),
        (Smell::PrimitiveObsession, [f.primitive_type_count, f.max_same_primitive_count, 0]),
        (Smell::LargeAssertionBlock, [f.consecutive_asserts, 0, 0]),
        (Smell::EmptyErrorHandler, [f.empty_catch_count, 0, 0]),
        (Smell::ShortVariableNames, [f.short_var_count, 0, 0]),
        (Smell::StringlyTypedSwitch, [f.string_match_arms, 0, 0]),
    ];
    table.iter().find(|(s, _)| *s == smell).map_or([0, 0, 0], |(_, m)| *m)
}

fn smell_family(smell: Smell) -> &'static [Smell] {
    match smell {
        Smell::GodMethod | Smell::ComplexMethod | Smell::LargeMethod => {
            &[Smell::GodMethod, Smell::ComplexMethod, Smell::LargeMethod]
        }
        _ => &[],
    }
}

pub fn entries_from(findings: &[Finding], metrics: &FileMetrics) -> Vec<FunctionBaselineEntry> {
    let mut grouped: HashMap<(String, Smell), ([u32; 3], u32, u64)> = HashMap::new();
    for f in findings {
        let Location::Function { name, start_line, .. } = &f.location else { continue };
        let Some(fm) = function_at(metrics, name, *start_line) else { continue };
        let mags = magnitudes(f.smell, fm);
        let slot = grouped.entry((name.clone(), f.smell)).or_insert(([0; 3], 0, fm.structural_hash));
        for (held, new) in slot.0.iter_mut().zip(mags) {
            *held = (*held).max(new);
        }
        slot.1 += 1;
    }
    grouped
        .into_iter()
        .map(|((name, smell), (mags, count, hash))| FunctionBaselineEntry {
            name,
            hash,
            smell: smell.snake_name().to_string(),
            mags,
            count,
        })
        .collect()
}

pub fn baseline_from_entries(entries: Vec<FunctionBaselineEntry>) -> FunctionBaseline {
    let mut baseline = FunctionBaseline::default();
    for e in entries {
        let Some(smell) = smell_from_snake_case(&e.smell) else { continue };
        baseline.by_hash.insert((e.hash, smell), (e.name.clone(), e.mags, e.count));
        baseline.by_name.insert((e.name, smell), (e.mags, e.count));
    }
    baseline
}

pub fn filter_worsened(findings: Vec<Finding>, baseline: &FunctionBaseline, metrics: &FileMetrics) -> Vec<Finding> {
    let mut counts: HashMap<(String, Smell), u32> = HashMap::new();
    for f in &findings {
        if let Location::Function { name, .. } = &f.location {
            *counts.entry((name.clone(), f.smell)).or_default() += 1;
        }
    }
    let current_names: std::collections::HashSet<&str> = metrics.functions.iter().map(|fm| fm.name.as_str()).collect();
    findings
        .into_iter()
        .filter(|f| {
            let Location::Function { name, start_line, .. } = &f.location else { return true };
            let Some(fm) = function_at(metrics, name, *start_line) else { return true };
            let count = counts.get(&(name.clone(), f.smell)).copied().unwrap_or(1);
            baseline.is_worsened(f.smell, name, fm, count, &current_names)
        })
        .collect()
}

fn function_at<'a>(metrics: &'a FileMetrics, name: &str, start_line: u32) -> Option<&'a FunctionMetrics> {
    metrics.functions.iter().find(|fm| fm.name == name && fm.start_line == start_line)
}

impl FunctionBaseline {
    fn lookup(
        &self,
        smell: Smell,
        name: &str,
        hash: u64,
        current_names: &std::collections::HashSet<&str>,
    ) -> Option<([u32; 3], u32)> {
        if let Some(found) = self.by_name.get(&(name.to_string(), smell)) {
            return Some(*found);
        }
        let (orig_name, mags, count) = self.by_hash.get(&(hash, smell))?;
        let vanished = !current_names.contains(orig_name.as_str());
        vanished.then_some((*mags, *count))
    }

    fn baseline_for(
        &self,
        smell: Smell,
        name: &str,
        hash: u64,
        current_names: &std::collections::HashSet<&str>,
    ) -> Option<([u32; 3], u32)> {
        if let Some(found) = self.lookup(smell, name, hash, current_names) {
            return Some(found);
        }
        smell_family(smell).iter().find_map(|&relative| self.lookup(relative, name, hash, current_names))
    }

    pub fn is_worsened(
        &self,
        smell: Smell,
        name: &str,
        fm: &FunctionMetrics,
        count: u32,
        current_names: &std::collections::HashSet<&str>,
    ) -> bool {
        let Some((base_mags, base_count)) = self.baseline_for(smell, name, fm.structural_hash, current_names) else {
            return true;
        };
        let mags = magnitudes(smell, fm);
        mags.iter().zip(base_mags).any(|(cur, base)| *cur > base) || count > base_count
    }
}
