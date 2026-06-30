use std::path::{Path, PathBuf};

use pulse_audit::finding::{
    AuditFinding, AuditKind, AuditLocation, GodComponentEvidence, HubLikeEvidence, ImportConfidence,
    UnstableDepEvidence,
};

use crate::audit_common::*;

fn finding(kind: AuditKind, component: &str) -> AuditFinding {
    AuditFinding {
        kind,
        representative_snippet: String::new(),
        support: 1,
        file_count: 1,
        idf_score: None,
        action_label: None,
        pattern_category: None,
        locality_entropy: None,
        p_value: None,
        locations: vec![AuditLocation { file: PathBuf::from(component), line: 1 }],
    }
}

fn unstable(component: &str, gap: f64, strength: f64, centrality: f64) -> AuditFinding {
    let evidence = UnstableDepEvidence {
        component: PathBuf::from(component),
        instability: 0.9,
        strength,
        gap,
        unstable_deps: 2,
        total_deps: 2,
        centrality,
        confidence: ImportConfidence::Medium,
    };
    finding(AuditKind::UnstableDependency(evidence), component)
}

fn hub(component: &str, afferent: u32, efferent: u32, centrality: f64) -> AuditFinding {
    let evidence = HubLikeEvidence {
        component: PathBuf::from(component),
        afferent,
        efferent,
        imbalance: afferent.abs_diff(efferent),
        centrality,
        confidence: ImportConfidence::Medium,
    };
    finding(AuditKind::HubLikeDependency(evidence), component)
}

fn god(component: &str, loc: u32, density: f64, centrality: f64) -> AuditFinding {
    let evidence = GodComponentEvidence {
        component: PathBuf::from(component),
        loc,
        file_count: 1,
        density,
        centrality,
        confidence: ImportConfidence::Medium,
    };
    finding(AuditKind::GodComponent(evidence), component)
}

fn combined_severity_of(f: &AuditFinding) -> f64 {
    match &f.kind {
        AuditKind::CompoundArchSmell(e) => e.combined_severity,
        _ => panic!("expected a compound finding"),
    }
}

fn compound_component(f: &AuditFinding) -> &Path {
    match &f.kind {
        AuditKind::CompoundArchSmell(e) => e.component.as_path(),
        _ => panic!("expected a compound finding"),
    }
}

#[test]
fn two_compounds_are_ordered_by_combined_severity() {
    let high_unstable = unstable("high", 0.6, 1.0, 0.9);
    let high_hub = hub("high", 4, 4, 0.9);
    let low_unstable = unstable("low", 0.05, 0.5, 0.1);
    let low_hub = hub("low", 2, 2, 0.1);

    let arch = vec![high_unstable, high_hub, low_unstable, low_hub];
    let out = pulse_audit::compound::detect(&arch, &t().audit);

    assert_eq!(out.len(), 2, "two components each carry two distinct arch smells");
    let sev_first = combined_severity_of(&out[0]);
    let sev_second = combined_severity_of(&out[1]);
    assert!(sev_first >= sev_second, "detect sorts descending by combined_severity ({sev_first} >= {sev_second})");
    assert_eq!(compound_component(&out[0]), Path::new("high"), "the more severe component sorts first");
    assert_eq!(compound_component(&out[1]), Path::new("low"));
    assert!(sev_first > 0.0);
}

#[test]
fn three_distinct_smells_raise_combined_severity_and_sort_first() {
    let strong_unstable = unstable("strong", 0.7, 1.0, 0.95);
    let strong_hub = hub("strong", 5, 5, 0.95);
    let strong_god = god("strong", 1200, 1200.0, 0.95);
    let weak_unstable = unstable("weak", 0.02, 0.5, 0.05);
    let weak_hub = hub("weak", 1, 1, 0.05);

    let arch = vec![strong_unstable, strong_hub, strong_god, weak_unstable, weak_hub];
    let out = pulse_audit::compound::detect(&arch, &t().audit);

    assert_eq!(out.len(), 2);
    assert_eq!(compound_component(&out[0]), Path::new("strong"));
    let strong_sev = combined_severity_of(&out[0]);
    let weak_sev = combined_severity_of(&out[1]);
    assert!(strong_sev > weak_sev, "three reinforcing smells outweigh two ({strong_sev} > {weak_sev})");

    match &out[0].kind {
        AuditKind::CompoundArchSmell(e) => {
            assert!(e.constituent_kinds.iter().any(|k| k == "god_component"));
            assert!(e.constituent_kinds.iter().any(|k| k == "unstable_dependency"));
            assert!(e.constituent_kinds.iter().any(|k| k == "hub_like_dependency"));
        }
        _ => panic!("expected a compound finding"),
    }
}
