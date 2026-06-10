use std::path::PathBuf;

use pulse::audit::finding::{
    AuditKind, CompoundEvidence, HubLikeEvidence, ImportConfidence, SplitComponentEvidence, UnstableDepEvidence,
};
use pulse::audit::output_arch::{arch_json, write_arch};

#[test]
fn unstable_dependency_human_and_json() {
    let e = UnstableDepEvidence {
        component: PathBuf::from("src/net"),
        instability: 0.875,
        strength: 0.5,
        gap: 0.25,
        unstable_deps: 3,
        total_deps: 5,
        centrality: 0.1234,
        confidence: ImportConfidence::High,
    };
    let kind = AuditKind::UnstableDependency(e);
    let mut out = String::new();
    assert!(write_arch(&mut out, &kind, Some(&PathBuf::from("src")), "invert the dependency"));
    assert!(out.contains("audit: unstable dependency — net"), "{out}");
    assert!(out.contains("instability:   0.875"), "{out}");
    assert!(out.contains("3 of 5 deps less stable"), "{out}");
    assert!(out.contains("gap:           0.250"), "{out}");
    assert!(out.contains("centrality:    0.1234"), "{out}");
    assert!(out.contains("confidence:    high"), "{out}");
    assert!(out.contains("action:        invert the dependency"), "{out}");

    let j = arch_json(&kind, None).expect("json");
    assert_eq!(j["kind"], "UnstableDependency");
    assert_eq!(j["component"], "src/net");
    assert_eq!(j["unstable_deps"], 3);
    assert_eq!(j["total_deps"], 5);
    assert_eq!(j["confidence"], "high");
}

#[test]
fn hub_like_dependency_human_and_json() {
    let e = HubLikeEvidence {
        component: PathBuf::from("src/core"),
        afferent: 12,
        efferent: 9,
        imbalance: 3,
        centrality: 0.4321,
        confidence: ImportConfidence::Medium,
    };
    let kind = AuditKind::HubLikeDependency(e);
    let mut out = String::new();
    assert!(write_arch(&mut out, &kind, Some(&PathBuf::from("src")), "split this hub"));
    assert!(out.contains("audit: hub-like dependency — core"), "{out}");
    assert!(out.contains("Ca:            12"), "{out}");
    assert!(out.contains("Ce:            9"), "{out}");
    assert!(out.contains("imbalance:     3"), "{out}");
    assert!(out.contains("centrality:    0.4321"), "{out}");
    assert!(out.contains("confidence:    medium"), "{out}");

    let j = arch_json(&kind, None).expect("json");
    assert_eq!(j["kind"], "HubLikeDependency");
    assert_eq!(j["afferent"], 12);
    assert_eq!(j["efferent"], 9);
    assert_eq!(j["imbalance"], 3);
    assert_eq!(j["confidence"], "medium");
}

#[test]
fn compound_arch_smell_human_and_json() {
    let e = CompoundEvidence {
        component: PathBuf::from("src/legacy"),
        constituent_kinds: vec!["god_component", "unstable_dependency"],
        combined_severity: 0.7654,
        confidence: ImportConfidence::Low,
    };
    let kind = AuditKind::CompoundArchSmell(e);
    let mut out = String::new();
    assert!(write_arch(&mut out, &kind, None, ""));
    assert!(out.contains("audit: compound architecture smell — src/legacy"), "{out}");
    assert!(out.contains("smells:        god_component + unstable_dependency"), "{out}");
    assert!(out.contains("severity:      0.7654"), "{out}");
    assert!(out.contains("confidence:    low"), "{out}");
    assert!(!out.contains("action:"), "no action line when action is empty: {out}");

    let j = arch_json(&kind, None).expect("json");
    assert_eq!(j["kind"], "CompoundArchSmell");
    assert_eq!(j["constituent_kinds"][0], "god_component");
    assert_eq!(j["constituent_kinds"][1], "unstable_dependency");
    assert_eq!(j["confidence"], "low");
}

#[test]
fn split_component_json() {
    let e = SplitComponentEvidence {
        component: PathBuf::from("src/big"),
        file_count: 40,
        community_count: 3,
        cohesion: 0.42,
        confidence: ImportConfidence::BestEffort,
    };
    let kind = AuditKind::SplitComponent(e);
    let j = arch_json(&kind, None).expect("json");
    assert_eq!(j["kind"], "SplitComponent");
    assert_eq!(j["component"], "src/big");
    assert_eq!(j["file_count"], 40);
    assert_eq!(j["community_count"], 3);
    assert_eq!(j["confidence"], "best-effort");
}

#[test]
fn non_arch_kind_is_not_rendered() {
    let kind = AuditKind::ZeroEdgeProject { module_count: 2 };
    let mut out = String::new();
    assert!(!write_arch(&mut out, &kind, None, "x"), "non-arch kind must return false");
    assert!(out.is_empty(), "{out}");
    assert!(arch_json(&kind, None).is_none(), "non-arch kind must yield no json");
}
