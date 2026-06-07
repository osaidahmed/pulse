use std::path::PathBuf;

use pulse::history::finding::{
    variant_info, BlobEvidence, ChangeShotgunEvidence, DriftEvidence, FragmentationEvidence,
    HistoryKind, HistoryPillar, HotspotEvidence,
};

fn drift_kind() -> HistoryKind {
    HistoryKind::ArchitecturalDrift(DriftEvidence {
        file_a: PathBuf::from("a.rs"),
        file_b: PathBuf::from("b.rs"),
        support: 5,
        commits: 100,
        confidence: 0.8,
        lift: 2.5,
        jaccard: 0.4,
        last_seen_unix: 1_700_000_000,
        distinct_authors: 2,
    })
}

fn hotspot_kind() -> HistoryKind {
    HistoryKind::Hotspot(HotspotEvidence {
        file: PathBuf::from("h.rs"),
        revisions: 10,
        sum_cc: 50,
        score: 500,
    })
}

fn ownership_kind() -> HistoryKind {
    HistoryKind::KnowledgeFragmentation(FragmentationEvidence {
        file: PathBuf::from("o.rs"),
        total_contributors: 8,
        minor_contributor_count: 5,
        minor_contributor_pct: 0.625,
        top_minor_authors: vec!["a@x".to_string(), "b@x".to_string()],
    })
}

#[test]
fn drift_dispatches_to_drift_pillar() {
    assert_eq!(variant_info(&drift_kind()).pillar, HistoryPillar::Drift);
}

#[test]
fn hotspot_dispatches_to_complexity_pillar() {
    assert_eq!(variant_info(&hotspot_kind()).pillar, HistoryPillar::Complexity);
}

fn blob_kind() -> HistoryKind {
    HistoryKind::FileBlob(BlobEvidence {
        file: PathBuf::from("god.rs"),
        multi_file_commits: 20,
        total_multi_file_commits: 24,
        blob_ratio: 0.83,
    })
}

#[test]
fn ownership_dispatches_to_ownership_pillar() {
    assert_eq!(variant_info(&ownership_kind()).pillar, HistoryPillar::Ownership);
}

fn shotgun_kind() -> HistoryKind {
    HistoryKind::ChangeShotgun(ChangeShotgunEvidence {
        file: PathBuf::from("core/hub.rs"),
        partner_count: 4,
        package_count: 4,
        packages: vec![PathBuf::from("pkg1"), PathBuf::from("pkg2")],
    })
}

#[test]
fn file_blob_dispatches_to_evolution_pillar() {
    assert_eq!(variant_info(&blob_kind()).pillar, HistoryPillar::Evolution);
}

#[test]
fn change_shotgun_dispatches_to_evolution_pillar() {
    assert_eq!(variant_info(&shotgun_kind()).pillar, HistoryPillar::Evolution);
}

#[test]
fn slugs_are_snake_case_lowercase() {
    for k in [drift_kind(), hotspot_kind(), ownership_kind(), blob_kind(), shotgun_kind()] {
        let slug = variant_info(&k).slug;
        assert!(slug.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "slug {slug:?} contains non-snake-case chars");
    }
}

#[test]
fn labels_are_human_readable_with_spaces() {
    assert_eq!(variant_info(&drift_kind()).label, "architectural drift");
    assert_eq!(variant_info(&hotspot_kind()).label, "hotspot");
    assert_eq!(variant_info(&ownership_kind()).label, "knowledge fragmentation");
}

#[test]
fn actions_are_non_empty() {
    for k in [drift_kind(), hotspot_kind(), ownership_kind(), blob_kind(), shotgun_kind()] {
        assert!(!variant_info(&k).action.is_empty());
    }
}
