use std::collections::HashMap;

use crate::smells::{Finding, Location, Smell};
use crate::thresholds::{FunctionThresholds, Thresholds};
use crate::walk::{FileMetrics, FunctionMetrics};

pub fn normalize_exceedance(value: f64, min: f64, max: f64) -> f64 {
    if max <= min {
        return if value > min { 10.0 } else { 0.0 };
    }
    (((value - min) / (max - min)) * 10.0).clamp(0.0, 10.0)
}

fn mean(parts: &[f64]) -> f64 {
    if parts.is_empty() {
        return 0.0;
    }
    parts.iter().sum::<f64>() / parts.len() as f64
}

fn mean_of_satisfied(parts: &[(bool, f64)]) -> f64 {
    let kept: Vec<f64> = parts.iter().filter(|(on, _)| *on).map(|(_, v)| *v).collect();
    mean(&kept)
}

pub fn for_finding(smell: Smell, f: &FunctionMetrics, t: &Thresholds) -> f64 {
    complexity_intensity(smell, f, &t.function)
        .or_else(|| structural_intensity(smell, f, t))
        .unwrap_or(0.0)
}

fn complexity_intensity(smell: Smell, f: &FunctionMetrics, fun: &FunctionThresholds) -> Option<f64> {
    let cc = (
        f.cc >= fun.cc_warning,
        normalize_exceedance(f64::from(f.cc), f64::from(fun.cc_warning), f64::from(fun.cc_alert)),
    );
    let cogc = (
        f.cognitive_complexity >= fun.cogc_warning,
        normalize_exceedance(
            f64::from(f.cognitive_complexity),
            f64::from(fun.cogc_warning),
            f64::from(fun.cogc_alert),
        ),
    );
    let loc = (
        f.loc >= fun.fn_loc_warning,
        normalize_exceedance(f64::from(f.loc), f64::from(fun.fn_loc_warning), f64::from(fun.fn_loc_alert)),
    );
    match smell {
        Smell::GodMethod => Some(mean_of_satisfied(&[cc, cogc, loc])),
        Smell::ComplexMethod => Some(mean_of_satisfied(&[cc, cogc])),
        Smell::LargeMethod => Some(mean_of_satisfied(&[loc])),
        _ => None,
    }
}

fn structural_intensity(smell: Smell, f: &FunctionMetrics, t: &Thresholds) -> Option<f64> {
    let fun = &t.function;
    let (value, min) = match smell {
        Smell::DeepNestedComplexity => (f.max_nesting, fun.nesting_depth),
        Smell::ComplexConditional => (f.compound_condition_count, fun.compound_conditions),
        Smell::ExcessArguments => (f.arg_count, fun.arg_max),
        Smell::ConstructorOverInjection => (f.arg_count, fun.constructor_arg_max),
        Smell::LargeEmbeddedBlock => (f.max_embedded_block_loc, fun.embedded_block_loc),
        Smell::NestedConditionalChunks => (f.bump_count, fun.bump_count),
        _ => return None,
    };
    let min_f = f64::from(min);
    Some(normalize_exceedance(f64::from(value), min_f, min_f * 2.0))
}

pub fn rank_findings(findings: &[Finding], metrics: &FileMetrics, t: &Thresholds) -> Vec<Finding> {
    let by_loc: HashMap<(&str, u32), &FunctionMetrics> = metrics
        .functions
        .iter()
        .map(|fm| ((fm.name.as_str(), fm.start_line), fm))
        .collect();
    let mut keyed: Vec<(f64, Finding)> = findings
        .iter()
        .map(|f| (finding_score(f, &by_loc, t), f.clone()))
        .collect();
    keyed.sort_by(|a, b| b.0.total_cmp(&a.0));
    keyed.into_iter().map(|(_, f)| f).collect()
}

fn finding_score(f: &Finding, by_loc: &HashMap<(&str, u32), &FunctionMetrics>, t: &Thresholds) -> f64 {
    match &f.location {
        Location::Function { name, start_line, .. } => by_loc
            .get(&(name.as_str(), *start_line))
            .map_or(0.0, |fm| for_finding(f.smell, fm, t)),
        Location::Module => 0.0,
    }
}
