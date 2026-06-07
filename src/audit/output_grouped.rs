use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::Path;

use crate::thresholds::AuditThresholds;

use super::finding::{AuditFinding, AuditKind, PatternCategory};

pub type RenderPatternFn = fn(&mut String, &AuditFinding, Option<&Path>, &AuditThresholds);

pub fn split<'a>(findings: &[&'a AuditFinding]) -> (Vec<&'a AuditFinding>, Vec<&'a AuditFinding>) {
    let mut patterns = Vec::new();
    let mut others = Vec::new();
    for f in findings {
        if matches!(f.kind, AuditKind::UncategorizedPattern { .. }) {
            patterns.push(*f);
        } else {
            others.push(*f);
        }
    }
    (patterns, others)
}

pub fn render(
    out: &mut String,
    patterns: &[&AuditFinding],
    root: Option<&Path>,
    t: &AuditThresholds,
    render_pattern: RenderPatternFn,
) {
    if patterns.is_empty() {
        return;
    }
    let groups = group_by_category(patterns);
    for (_, group) in groups {
        write_group(out, &group, root, t, render_pattern);
    }
}

fn group_by_category<'a>(patterns: &[&'a AuditFinding]) -> BTreeMap<u8, Vec<&'a AuditFinding>> {
    let mut groups: BTreeMap<u8, Vec<&AuditFinding>> = BTreeMap::new();
    for p in patterns {
        let cat = p.pattern_category.unwrap_or(PatternCategory::Other);
        groups.entry(cat.order()).or_default().push(*p);
    }
    groups
}

fn write_group(
    out: &mut String,
    group: &[&AuditFinding],
    root: Option<&Path>,
    t: &AuditThresholds,
    render_pattern: RenderPatternFn,
) {
    let header = group[0].pattern_category.unwrap_or(PatternCategory::Other).header();
    let _ = writeln!(out, "## {} ({} patterns)\n", header, group.len());
    for p in group {
        render_pattern(out, p, root, t);
    }
}
