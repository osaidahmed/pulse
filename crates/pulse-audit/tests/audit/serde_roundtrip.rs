use pulse_audit::finding::{
    AuditFinding, AuditKind, AuditLocation, CompoundEvidence, CycleMembership, CycleShape, GodClassEvidence,
    ImportConfidence, MartinMetrics, MartinTier,
};
use std::path::PathBuf;

fn finding(kind: AuditKind) -> AuditFinding {
    AuditFinding {
        kind,
        representative_snippet: "snip".into(),
        support: 3,
        file_count: 2,
        idf_score: Some(1.5),
        action_label: Some("refactor it"),
        locations: vec![AuditLocation { file: PathBuf::from("a/b.rs"), line: 40 }],
        pattern_category: None,
        locality_entropy: Some(0.25),
        p_value: None,
    }
}

fn reserialize(f: &AuditFinding) -> String {
    let json = serde_json::to_string(f).unwrap();
    let back: AuditFinding = serde_json::from_str(&json).unwrap();
    serde_json::to_string(&back).unwrap()
}

// ── positive paths: every embedded evidence shape survives a full round-trip ──

#[test]
fn god_class_finding_round_trips() {
    let f = finding(AuditKind::GodClass(GodClassEvidence {
        class_file: PathBuf::from("svc.rs"),
        class_name: "Service".into(),
        wmc: 40,
        tcc: 0.12,
        atfd: 9,
        method_count: 22,
        confidence: ImportConfidence::High,
    }));
    assert_eq!(reserialize(&f), serde_json::to_string(&f).unwrap());
}

#[test]
fn compound_arch_smell_preserves_owned_constituent_kinds() {
    let f = finding(AuditKind::CompoundArchSmell(CompoundEvidence {
        component: PathBuf::from("src"),
        constituent_kinds: vec!["god_component".into(), "unstable_dependency".into()],
        combined_severity: 1.0,
        confidence: ImportConfidence::Medium,
    }));
    let back: AuditFinding = serde_json::from_str(&serde_json::to_string(&f).unwrap()).unwrap();
    match back.kind {
        AuditKind::CompoundArchSmell(e) => {
            assert_eq!(e.constituent_kinds, vec!["god_component".to_string(), "unstable_dependency".to_string()]);
        }
        other => panic!("wrong variant: {other:?}"),
    }
}

#[test]
fn martin_metrics_with_nested_enum_round_trips() {
    let f = finding(AuditKind::DistanceFromMainSequence(MartinMetrics {
        module: PathBuf::from("walk/shared.rs"),
        afferent: 4,
        efferent: 0,
        instability: 0.0,
        abstractness: 0.0,
        distance: 1.0,
        tier: MartinTier::Alert,
        confidence: ImportConfidence::High,
    }));
    assert_eq!(reserialize(&f), serde_json::to_string(&f).unwrap());
}

#[test]
fn cycle_membership_with_tuples_and_option_round_trips() {
    let f = finding(AuditKind::ImportCycle(CycleMembership {
        members: vec![PathBuf::from("a.rs"), PathBuf::from("b.rs")],
        edges: vec![(PathBuf::from("a.rs"), PathBuf::from("b.rs"))],
        confidence: ImportConfidence::Low,
        shape: CycleShape::Circle,
        centrality: 0.5,
        feedback_edge: Some((PathBuf::from("b.rs"), PathBuf::from("a.rs"))),
    }));
    assert_eq!(reserialize(&f), serde_json::to_string(&f).unwrap());
}

#[test]
fn evidence_struct_and_enums_round_trip_by_value() {
    let e = GodClassEvidence {
        class_file: PathBuf::from("x.rs"),
        class_name: "X".into(),
        wmc: 1,
        tcc: 0.0,
        atfd: 0,
        method_count: 1,
        confidence: ImportConfidence::BestEffort,
    };
    let back: GodClassEvidence = serde_json::from_str(&serde_json::to_string(&e).unwrap()).unwrap();
    assert_eq!(back, e);

    for c in [
        ImportConfidence::NaAbstraction,
        ImportConfidence::BestEffort,
        ImportConfidence::Low,
        ImportConfidence::Medium,
        ImportConfidence::High,
    ] {
        let r: ImportConfidence = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
        assert_eq!(r, c);
    }
}

#[test]
fn action_label_is_derived_state_and_never_deserialized() {
    // it is skipped on serialize, and ignored (reset to None) on deserialize
    let f = finding(AuditKind::ZeroEdgeProject { module_count: 3 });
    assert!(!serde_json::to_string(&f).unwrap().contains("action_label"));
    let injected = serde_json::to_string(&f).unwrap().replace("\"support\"", "\"action_label\":\"HACK\",\"support\"");
    let back: AuditFinding = serde_json::from_str(&injected).unwrap();
    assert_eq!(back.action_label, None);
}

// ── negative paths ──

#[test]
fn malformed_json_errors() {
    assert!(serde_json::from_str::<AuditFinding>("{").is_err());
    assert!(serde_json::from_str::<GodClassEvidence>("[]").is_err());
}

#[test]
fn unknown_kind_variant_errors() {
    assert!(serde_json::from_str::<AuditKind>(r#"{"NotAKind":{}}"#).is_err());
}

#[test]
fn evidence_missing_required_field_errors() {
    // GodClassEvidence without `wmc`
    assert!(serde_json::from_str::<GodClassEvidence>(
        r#"{"class_file":"x.rs","class_name":"X","tcc":0.0,"atfd":0,"method_count":1,"confidence":"High"}"#
    )
    .is_err());
}

#[test]
fn unknown_confidence_variant_errors() {
    assert!(serde_json::from_str::<ImportConfidence>(r#""Ultra""#).is_err());
}

#[test]
fn unknown_extra_field_is_tolerated() {
    let e: GodClassEvidence = serde_json::from_str(
        r#"{"class_file":"x.rs","class_name":"X","wmc":1,"tcc":0.0,"atfd":0,"method_count":1,"confidence":"High","future":true}"#,
    )
    .unwrap();
    assert_eq!(e.class_name, "X");
}
