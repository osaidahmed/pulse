use std::path::PathBuf;

use pulse::audit::finding::{
    AuditFinding, AuditKind, ClassIdentityRef, DivergentChangeEvidence, FeatureEnvyEvidence,
    GodClassEvidence, ImportConfidence, ParallelInheritanceEvidence, RefusedBequestEvidence,
    ShotgunSurgeryEvidence,
};
use pulse::audit::output::{format_findings, format_findings_json};

use crate::audit_common::t;

fn shotgun_finding() -> AuditFinding {
    AuditFinding {
        kind: AuditKind::ShotgunSurgery(ShotgunSurgeryEvidence {
            method_file: PathBuf::from("svc.py"),
            method_class: Some("Svc".to_string()),
            method_name: "handle".to_string(),
            method_line: 1,
            changing_classes: 6,
            changing_methods: 12,
            fanout: 7,
            confidence: ImportConfidence::Medium,
            caller_samples: Vec::new(),
        }),
        representative_snippet: String::new(),
        support: 0,
        file_count: 0,
        idf_score: None,
        action_label: None,
        pattern_category: None,
        locations: Vec::new(),
    }
}

fn divergent_finding() -> AuditFinding {
    AuditFinding {
        kind: AuditKind::DivergentChange(DivergentChangeEvidence {
            class_file: PathBuf::from("a.py"),
            class_name: "A".to_string(),
            changing_classes: 8,
            fanout: 9,
            method_count: 7,
            confidence: ImportConfidence::High,
        }),
        representative_snippet: String::new(),
        support: 0,
        file_count: 0,
        idf_score: None,
        action_label: None,
        pattern_category: None,
        locations: Vec::new(),
    }
}

fn feature_envy_finding() -> AuditFinding {
    AuditFinding {
        kind: AuditKind::FeatureEnvy(FeatureEnvyEvidence {
            method_file: PathBuf::from("b.py"),
            method_class: Some("B".to_string()),
            method_name: "compute".to_string(),
            method_line: 5,
            atfd: 8,
            foreign_call_count: 6,
            intra_call_count: 1,
            envied_class: Some("Repo".to_string()),
            confidence: ImportConfidence::Medium,
        }),
        representative_snippet: String::new(),
        support: 0,
        file_count: 0,
        idf_score: None,
        action_label: None,
        pattern_category: None,
        locations: Vec::new(),
    }
}

fn god_class_finding() -> AuditFinding {
    AuditFinding {
        kind: AuditKind::GodClass(GodClassEvidence {
            class_file: PathBuf::from("c.py"),
            class_name: "C".to_string(),
            wmc: 73,
            tcc: 0.1,
            atfd: 11,
            method_count: 25,
            confidence: ImportConfidence::Low,
        }),
        representative_snippet: String::new(),
        support: 0,
        file_count: 0,
        idf_score: None,
        action_label: None,
        pattern_category: None,
        locations: Vec::new(),
    }
}

fn parallel_finding() -> AuditFinding {
    AuditFinding {
        kind: AuditKind::ParallelInheritance(ParallelInheritanceEvidence {
            root_a: ClassIdentityRef {
                file: PathBuf::from("rd.py"),
                name: "Reader".to_string(),
            },
            root_b: ClassIdentityRef {
                file: PathBuf::from("wr.py"),
                name: "Writer".to_string(),
            },
            matched_descendants: vec![
                ("JsonReader".to_string(), "JsonWriter".to_string()),
                ("XmlReader".to_string(), "XmlWriter".to_string()),
            ],
            confidence: ImportConfidence::High,
        }),
        representative_snippet: String::new(),
        support: 0,
        file_count: 0,
        idf_score: None,
        action_label: None,
        pattern_category: None,
        locations: Vec::new(),
    }
}

fn refused_finding() -> AuditFinding {
    AuditFinding {
        kind: AuditKind::RefusedBequest(RefusedBequestEvidence {
            subclass_file: PathBuf::from("ch.py"),
            subclass_name: "Child".to_string(),
            parent_file: PathBuf::from("p.py"),
            parent_name: "Parent".to_string(),
            override_count: 1,
            parent_method_count: 8,
            override_ratio: 0.125,
            confidence: ImportConfidence::Medium,
        }),
        representative_snippet: String::new(),
        support: 0,
        file_count: 0,
        idf_score: None,
        action_label: None,
        pattern_category: None,
        locations: Vec::new(),
    }
}

fn all_six() -> Vec<AuditFinding> {
    vec![
        shotgun_finding(),
        divergent_finding(),
        feature_envy_finding(),
        god_class_finding(),
        parallel_finding(),
        refused_finding(),
    ]
}

#[test]
fn human_renders_all_six_named_smell_kinds_in_one_call() {
    let out = format_findings(&all_six(), None, &t().audit);
    assert!(out.contains("shotgun surgery"));
    assert!(out.contains("divergent change"));
    assert!(out.contains("feature envy"));
    assert!(out.contains("god class"));
    assert!(out.contains("parallel inheritance"));
    assert!(out.contains("refused bequest"));
}

#[test]
fn human_separates_each_finding_with_blank_line() {
    let out = format_findings(&all_six(), None, &t().audit);
    assert_eq!(out.matches("\n\n").count(), 6);
}

#[test]
fn json_renders_all_six_named_smell_kinds_as_array() {
    let out = format_findings_json(&all_six(), None);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let kinds: Vec<&str> = v
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|e| e["kind"].as_str())
        .collect();
    assert_eq!(kinds.len(), 6);
    assert!(kinds.contains(&"ShotgunSurgery"));
    assert!(kinds.contains(&"DivergentChange"));
    assert!(kinds.contains(&"FeatureEnvy"));
    assert!(kinds.contains(&"GodClass"));
    assert!(kinds.contains(&"ParallelInheritance"));
    assert!(kinds.contains(&"RefusedBequest"));
}

#[test]
fn json_preserves_input_order() {
    let out = format_findings_json(&all_six(), None);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let arr = v.as_array().unwrap();
    assert_eq!(arr[0]["kind"], "ShotgunSurgery");
    assert_eq!(arr[1]["kind"], "DivergentChange");
    assert_eq!(arr[2]["kind"], "FeatureEnvy");
    assert_eq!(arr[3]["kind"], "GodClass");
    assert_eq!(arr[4]["kind"], "ParallelInheritance");
    assert_eq!(arr[5]["kind"], "RefusedBequest");
}

#[test]
fn renderers_never_panic_on_arbitrary_orderings() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    for seed in 0..16u64 {
        let mut findings = all_six();
        let mut hasher = DefaultHasher::new();
        seed.hash(&mut hasher);
        let h = hasher.finish() as usize;
        let len = findings.len();
        for i in 0..len {
            let j = (h.wrapping_add(i)) % len;
            findings.swap(i, j);
        }
        let _ = format_findings(&findings, None, &t().audit);
        let _ = format_findings_json(&findings, None);
    }
}

#[test]
fn empty_findings_renders_empty_human() {
    let out = format_findings(&[], None, &t().audit);
    assert!(out.is_empty());
}

#[test]
fn empty_findings_renders_empty_json_array() {
    let out = format_findings_json(&[], None);
    assert_eq!(out, "[]");
}

#[test]
fn naabstraction_label_renders_for_all_named_smells() {
    use pulse::audit::finding::ImportConfidence::NaAbstraction;

    let mut findings = all_six();
    for f in &mut findings {
        match &mut f.kind {
            AuditKind::ShotgunSurgery(e) => e.confidence = NaAbstraction,
            AuditKind::DivergentChange(e) => e.confidence = NaAbstraction,
            AuditKind::FeatureEnvy(e) => e.confidence = NaAbstraction,
            AuditKind::GodClass(e) => e.confidence = NaAbstraction,
            AuditKind::ParallelInheritance(e) => e.confidence = NaAbstraction,
            AuditKind::RefusedBequest(e) => e.confidence = NaAbstraction,
            _ => {}
        }
    }
    let out = format_findings(&findings, None, &t().audit);
    assert_eq!(out.matches("n/a-abstraction").count(), 6);
}
