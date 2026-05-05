use std::collections::HashMap;
use std::fmt::Write;
use std::path::Path;

use crate::thresholds::AuditThresholds;

use super::finding::{
    finding_confidence, kind_label, kind_pillar, AuditFinding, AuditPillar, ImportConfidence,
    PatternCategory,
};
use super::output_grouped::RenderPatternFn;

pub struct PillarCtx<'a> {
    pub root: Option<&'a Path>,
    pub thresholds: &'a AuditThresholds,
    pub render_pattern: RenderPatternFn,
    pub render_other: fn(&mut String, &AuditFinding, Option<&Path>, &AuditThresholds),
}

pub fn render_pillars(out: &mut String, visible: &[&AuditFinding], ctx: &PillarCtx) {
    for (pillar, name) in PILLAR_ORDER {
        let group: Vec<&AuditFinding> = visible
            .iter()
            .filter(|f| kind_pillar(&f.kind) == *pillar)
            .copied()
            .collect();
        if !group.is_empty() {
            write_pillar(out, name, &group, *pillar, ctx);
        }
    }
}

const PILLAR_ORDER: &[(AuditPillar, &str)] = &[
    (AuditPillar::Architecture, "ARCHITECTURE"),
    (AuditPillar::ClassSmells, "CLASS-LEVEL SMELLS"),
    (AuditPillar::Patterns, "CROSS-FILE PATTERNS"),
];

fn write_pillar(
    out: &mut String,
    name: &str,
    group: &[&AuditFinding],
    pillar: AuditPillar,
    ctx: &PillarCtx,
) {
    write_section_header(out, name, group);
    let pattern_pillar = matches!(pillar, AuditPillar::Patterns);
    let mut sorted: Vec<&AuditFinding> = group.to_vec();
    if pattern_pillar {
        sorted.sort_by_key(|f| f.pattern_category.unwrap_or(PatternCategory::Other).order());
    }
    let subgroups = group_findings(&sorted, |f| {
        if pattern_pillar {
            f.pattern_category.unwrap_or(PatternCategory::Other).header()
        } else {
            kind_label(&f.kind)
        }
    });
    for (label, members) in subgroups {
        let _ = writeln!(out, "### {label} ({})\n", members.len());
        for f in members {
            if pattern_pillar {
                (ctx.render_pattern)(out, f, ctx.root, ctx.thresholds);
            } else {
                (ctx.render_other)(out, f, ctx.root, ctx.thresholds);
            }
        }
    }
}

fn write_section_header(out: &mut String, name: &str, group: &[&AuditFinding]) {
    let mut high = 0u32;
    let mut medium = 0u32;
    let mut low = 0u32;
    for f in group {
        match finding_confidence(f) {
            ImportConfidence::High => high += 1,
            ImportConfidence::Medium => medium += 1,
            _ => low += 1,
        }
    }
    let _ = writeln!(
        out,
        "## {name} ({} findings, {high} high · {medium} medium · {low} low)\n",
        group.len()
    );
}

fn group_findings<'a, F>(
    group: &[&'a AuditFinding],
    key_fn: F,
) -> Vec<(&'static str, Vec<&'a AuditFinding>)>
where
    F: Fn(&AuditFinding) -> &'static str,
{
    let mut order: Vec<&'static str> = Vec::new();
    let mut buckets: HashMap<&'static str, Vec<&'a AuditFinding>> = HashMap::new();
    for f in group {
        let key = key_fn(f);
        if !buckets.contains_key(key) {
            order.push(key);
        }
        buckets.entry(key).or_default().push(*f);
    }
    order.into_iter().map(|k| { let v = buckets.remove(k).unwrap_or_default(); (k, v) }).collect()
}
