use std::path::{Path, PathBuf};

use pulse_history::finding::{
    CatalystEvidence, DecayEvidence, DriftEvidence, FragmentationEvidence, HistoryFinding, HistoryKind, HotspotEvidence,
};

fn catalyst(members: &[&str]) -> HistoryFinding {
    HistoryFinding {
        kind: HistoryKind::CatalystWarning(CatalystEvidence {
            members: members.iter().map(|&m| PathBuf::from(m)).collect(),
        }),
        action_label: None,
    }
}

fn decay(members: &[&str], previous_size: u32, current_size: u32) -> HistoryFinding {
    HistoryFinding {
        kind: HistoryKind::DecayTrend(DecayEvidence {
            members: members.iter().map(|&m| PathBuf::from(m)).collect(),
            previous_size,
            current_size,
        }),
        action_label: None,
    }
}

#[test]
fn catalyst_renders_fresh_cycle_text() {
    let out = format_findings(&[catalyst(&["pkg/a.rs", "pkg/b.rs"])], None);
    assert!(out.contains("fresh cycle"), "got: {out}");
    assert!(out.contains("pkg/a.rs"), "got: {out}");
    assert!(out.contains("→"), "members joined with arrow: {out}");
}

#[test]
fn catalyst_renders_json() {
    let out = format_findings_json(&[catalyst(&["pkg/a.rs", "pkg/b.rs"])], None);
    assert!(serde_json::from_str::<serde_json::Value>(&out).is_ok(), "valid JSON: {out}");
    assert!(out.contains("\"CatalystWarning\""), "got: {out}");
    assert!(out.contains("members"), "got: {out}");
}

#[test]
fn decay_renders_growing_cycle_text() {
    let out = format_findings(&[decay(&["pkg/a.rs", "pkg/b.rs", "pkg/c.rs"], 2, 3)], None);
    assert!(out.contains("growing cycle"), "got: {out}");
    assert!(out.contains("2 → 3"), "size transition shown: {out}");
}

#[test]
fn decay_renders_json() {
    let out = format_findings_json(&[decay(&["pkg/a.rs", "pkg/b.rs", "pkg/c.rs"], 2, 3)], None);
    assert!(serde_json::from_str::<serde_json::Value>(&out).is_ok(), "valid JSON: {out}");
    assert!(out.contains("\"DecayTrend\""), "got: {out}");
    assert!(out.contains("previous_size"), "got: {out}");
    assert!(out.contains("current_size"), "got: {out}");
}
use pulse_history::output::{format_findings, format_findings_json};

fn drift(a: &str, b: &str, support: u32) -> HistoryFinding {
    HistoryFinding {
        kind: HistoryKind::ArchitecturalDrift(DriftEvidence {
            file_a: PathBuf::from(a),
            file_b: PathBuf::from(b),
            support,
            commits: 100,
            confidence: 0.75,
            lift: 2.0,
            jaccard: 0.5,
            last_seen_unix: 1_700_000_000,
            distinct_authors: 3,
        }),
        action_label: None,
    }
}

fn hotspot(file: &str, revisions: u32, cc: u32) -> HistoryFinding {
    HistoryFinding {
        kind: HistoryKind::Hotspot(HotspotEvidence {
            file: PathBuf::from(file),
            revisions,
            sum_cc: cc,
            score: u64::from(revisions) * u64::from(cc),
        }),
        action_label: None,
    }
}

fn ownership(file: &str, total: u32, minor: u32) -> HistoryFinding {
    HistoryFinding {
        kind: HistoryKind::KnowledgeFragmentation(FragmentationEvidence {
            file: PathBuf::from(file),
            total_contributors: total,
            minor_contributor_count: minor,
            minor_contributor_pct: f64::from(minor) / f64::from(total),
            top_minor_authors: vec!["a@x".to_string(), "b@x".to_string()],
        }),
        action_label: None,
    }
}

#[test]
fn empty_findings_shows_zero_total() {
    let out = format_findings(&[], Some(Path::new(".")));
    assert!(out.contains("history: . — 0 findings"));
    assert!(out.contains("0 drift · 0 hotspots · 0 ownership · 0 evolution"));
}

#[test]
fn empty_pillars_each_show_none() {
    let out = format_findings(&[], None);
    assert_eq!(out.matches("(none)").count(), 4);
}

#[test]
fn drift_finding_renders_pair_with_arrow() {
    let out = format_findings(&[drift("src/a.rs", "src/b.rs", 7)], None);
    assert!(out.contains("src/a.rs ↔ src/b.rs"));
    assert!(out.contains("7 co-changes"));
    assert!(out.contains("conf 0.75"));
    assert!(out.contains("lift 2.0"));
    assert!(out.contains("3 authors"));
}

#[test]
fn hotspot_finding_renders_score_calculation() {
    let out = format_findings(&[hotspot("src/h.rs", 10, 8)], None);
    assert!(out.contains("src/h.rs"));
    assert!(out.contains("10 revisions × cc=8 = 80"));
}

#[test]
fn ownership_finding_renders_total_minor_pct() {
    let out = format_findings(&[ownership("src/o.rs", 10, 6)], None);
    assert!(out.contains("src/o.rs"));
    assert!(out.contains("total=10, minor=6 (60%)"));
    assert!(out.contains("a@x, b@x"));
}

#[test]
fn mixed_findings_produce_all_three_sections() {
    let findings = vec![drift("a.rs", "b.rs", 5), hotspot("h.rs", 4, 10), ownership("o.rs", 8, 5)];
    let out = format_findings(&findings, None);
    assert!(out.contains("architectural drift"));
    assert!(out.contains("hotspots"));
    assert!(out.contains("knowledge fragmentation"));
}

#[test]
fn json_envelope_has_summary_and_findings_keys() {
    let json_str = format_findings_json(&[], None);
    let v: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert!(v.get("summary").is_some());
    assert!(v.get("findings").is_some());
}

#[test]
fn json_summary_findings_total_matches_array_len() {
    let findings = vec![drift("a.rs", "b.rs", 3), hotspot("h.rs", 5, 5)];
    let json_str = format_findings_json(&findings, None);
    let v: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert_eq!(v["summary"]["findings_total"], 2);
    assert_eq!(v["findings"].as_array().unwrap().len(), 2);
}

#[test]
fn json_by_pillar_breakdown_correct() {
    let findings =
        vec![drift("a.rs", "b.rs", 3), drift("c.rs", "d.rs", 4), hotspot("h.rs", 5, 5), ownership("o.rs", 8, 5)];
    let json_str = format_findings_json(&findings, None);
    let v: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert_eq!(v["summary"]["by_pillar"]["drift"], 2);
    assert_eq!(v["summary"]["by_pillar"]["complexity"], 1);
    assert_eq!(v["summary"]["by_pillar"]["ownership"], 1);
}

#[test]
fn json_drift_includes_kind_and_evidence_fields() {
    let json_str = format_findings_json(&[drift("a.rs", "b.rs", 7)], None);
    let v: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    let f = &v["findings"][0];
    assert_eq!(f["kind"], "ArchitecturalDrift");
    assert_eq!(f["file_a"], "a.rs");
    assert_eq!(f["file_b"], "b.rs");
    assert_eq!(f["support"], 7);
}

#[test]
fn json_hotspot_includes_score() {
    let json_str = format_findings_json(&[hotspot("h.rs", 4, 6)], None);
    let v: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    let f = &v["findings"][0];
    assert_eq!(f["kind"], "Hotspot");
    assert_eq!(f["score"], 24);
}

#[test]
fn json_ownership_includes_top_authors_array() {
    let json_str = format_findings_json(&[ownership("o.rs", 10, 5)], None);
    let v: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    let f = &v["findings"][0];
    assert_eq!(f["kind"], "KnowledgeFragmentation");
    assert!(f["top_minor_authors"].is_array());
}

#[test]
fn json_action_present_when_set() {
    let mut f = drift("a.rs", "b.rs", 3);
    f.action_label = Some("do the thing");
    let json_str = format_findings_json(&[f], None);
    let v: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert_eq!(v["findings"][0]["action"], "do the thing");
}

#[test]
fn json_action_absent_when_unset() {
    let json_str = format_findings_json(&[drift("a.rs", "b.rs", 3)], None);
    let v: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert!(v["findings"][0].get("action").is_none());
}
