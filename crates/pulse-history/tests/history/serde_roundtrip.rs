use pulse_history::finding::{
    BlobEvidence, BuildCoChangeEvidence, CatalystEvidence, ChangeShotgunEvidence, DecayEvidence, DefectProneEvidence,
    DriftEvidence, FragmentationEvidence, HistoryFinding, HistoryKind, HotspotEvidence,
};

fn reserialize(f: &HistoryFinding) -> String {
    let json = serde_json::to_string(f).unwrap();
    let back: HistoryFinding = serde_json::from_str(&json).unwrap();
    serde_json::to_string(&back).unwrap()
}

fn finding(kind: HistoryKind) -> HistoryFinding {
    HistoryFinding { kind, action_label: Some("split it") }
}

// ── positive paths ──

#[test]
fn defect_prone_finding_round_trips() {
    let f = finding(HistoryKind::DefectProneFile(DefectProneEvidence {
        file: "src/analyze.rs".into(),
        function: "run".into(),
        fix_count: 7,
        introducer_count: 3,
    }));
    assert_eq!(reserialize(&f), serde_json::to_string(&f).unwrap());
}

#[test]
fn hotspot_finding_round_trips() {
    let f = finding(HistoryKind::Hotspot(HotspotEvidence {
        file: "src/hook.rs".into(),
        revisions: 42,
        sum_cc: 900,
        score: 37_800,
    }));
    assert_eq!(reserialize(&f), serde_json::to_string(&f).unwrap());
}

#[test]
fn drift_finding_with_signed_and_float_fields_round_trips() {
    let f = finding(HistoryKind::ArchitecturalDrift(DriftEvidence {
        file_a: "a.rs".into(),
        file_b: "b.rs".into(),
        support: 5,
        commits: 12,
        confidence: 0.83,
        lift: 2.1,
        jaccard: 0.4,
        last_seen_unix: 1_700_000_000,
        distinct_authors: 2,
    }));
    assert_eq!(reserialize(&f), serde_json::to_string(&f).unwrap());
}

#[test]
fn evidence_round_trips_standalone() {
    let e = DefectProneEvidence { file: "x.rs".into(), function: "f".into(), fix_count: 1, introducer_count: 1 };
    let json = serde_json::to_string(&e).unwrap();
    let back: DefectProneEvidence = serde_json::from_str(&json).unwrap();
    assert_eq!(serde_json::to_string(&back).unwrap(), json);
}

#[test]
fn action_label_is_skipped_and_never_read_back() {
    let f = finding(HistoryKind::Hotspot(HotspotEvidence { file: "x.rs".into(), revisions: 1, sum_cc: 1, score: 1 }));
    assert!(!serde_json::to_string(&f).unwrap().contains("action_label"));
    let back: HistoryFinding = serde_json::from_str(&serde_json::to_string(&f).unwrap()).unwrap();
    assert_eq!(back.action_label, None);
}

#[test]
fn every_remaining_history_kind_variant_round_trips() {
    let kinds = vec![
        HistoryKind::KnowledgeFragmentation(FragmentationEvidence {
            file: "x.rs".into(),
            total_contributors: 5,
            minor_contributor_count: 3,
            minor_contributor_pct: 0.6,
            top_minor_authors: vec!["a".into(), "b".into()],
        }),
        HistoryKind::FileBlob(BlobEvidence {
            file: "x.rs".into(),
            multi_file_commits: 10,
            total_multi_file_commits: 40,
            blob_ratio: 0.25,
        }),
        HistoryKind::ChangeShotgun(ChangeShotgunEvidence {
            file: "x.rs".into(),
            partner_count: 8,
            package_count: 3,
            packages: vec!["a".into(), "b".into()],
        }),
        HistoryKind::CatalystWarning(CatalystEvidence { members: vec!["a.rs".into(), "b.rs".into()] }),
        HistoryKind::DecayTrend(DecayEvidence { members: vec!["a.rs".into()], previous_size: 3, current_size: 7 }),
        HistoryKind::BuildCoChange(BuildCoChangeEvidence {
            build_file: "Cargo.toml".into(),
            source_file: "src/lib.rs".into(),
            support: 6,
            source_revisions: 20,
            ratio: 0.3,
            last_seen_unix: 1_700_000_000,
            distinct_authors: 2,
        }),
    ];
    for kind in kinds {
        let f = finding(kind);
        assert_eq!(reserialize(&f), serde_json::to_string(&f).unwrap());
    }
}

// ── negative paths ──

#[test]
fn malformed_json_errors() {
    assert!(serde_json::from_str::<HistoryFinding>("nonsense").is_err());
    assert!(serde_json::from_str::<HotspotEvidence>("3.14").is_err());
}

#[test]
fn unknown_history_kind_variant_errors() {
    assert!(serde_json::from_str::<HistoryKind>(r#"{"MysteryKind":{}}"#).is_err());
}

#[test]
fn missing_required_field_errors() {
    // HotspotEvidence without `score`
    assert!(serde_json::from_str::<HotspotEvidence>(r#"{"file":"x.rs","revisions":1,"sum_cc":1}"#).is_err());
}

#[test]
fn unknown_extra_field_is_tolerated() {
    let e: HotspotEvidence =
        serde_json::from_str(r#"{"file":"x.rs","revisions":1,"sum_cc":1,"score":1,"future":42}"#).unwrap();
    assert_eq!(e.revisions, 1);
}
