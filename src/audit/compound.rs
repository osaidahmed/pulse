use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::thresholds::AuditThresholds;

use super::arch_smells::arch_severity;
use super::finding::{
    finding_confidence, kind_slug, AuditFinding, AuditKind, AuditLocation, CompoundEvidence,
    ImportConfidence,
};

pub fn detect(arch_findings: &[AuditFinding], thresholds: &AuditThresholds) -> Vec<AuditFinding> {
    let mut by_component: BTreeMap<PathBuf, Vec<&AuditFinding>> = BTreeMap::new();
    for f in arch_findings {
        if let Some(loc) = f.locations.first() {
            by_component.entry(loc.file.clone()).or_default().push(f);
        }
    }
    let mut out: Vec<AuditFinding> = by_component
        .into_iter()
        .filter_map(|(component, group)| compound_for(component, &group))
        .collect();
    out.sort_by(|a, b| {
        combined_severity(b).partial_cmp(&combined_severity(a)).unwrap_or(std::cmp::Ordering::Equal)
    });
    out.truncate(thresholds.package_metrics.max_arch_findings_reported);
    out
}

fn compound_for(component: PathBuf, group: &[&AuditFinding]) -> Option<AuditFinding> {
    let mut kinds: Vec<&'static str> = group.iter().map(|f| kind_slug(&f.kind)).collect();
    kinds.sort_unstable();
    kinds.dedup();
    if kinds.len() < 2 {
        return None;
    }
    let confidence = group
        .iter()
        .map(|f| finding_confidence(f))
        .min()
        .unwrap_or(ImportConfidence::Medium);
    let evidence = CompoundEvidence {
        component: component.clone(),
        constituent_kinds: kinds.clone(),
        combined_severity: group.iter().map(|f| arch_severity(f)).sum(),
        confidence,
    };
    Some(build_finding(evidence, component, kinds.len() as u32))
}

fn build_finding(evidence: CompoundEvidence, component: PathBuf, support: u32) -> AuditFinding {
    AuditFinding {
        kind: AuditKind::CompoundArchSmell(evidence),
        representative_snippet: String::new(),
        support,
        file_count: 1,
        idf_score: None,
        action_label: None,
        pattern_category: None,
        locality_entropy: None,
        p_value: None,
        locations: vec![AuditLocation { file: component, line: 1 }],
    }
}

fn combined_severity(f: &AuditFinding) -> f64 {
    match &f.kind {
        AuditKind::CompoundArchSmell(e) => e.combined_severity,
        _ => 0.0,
    }
}
