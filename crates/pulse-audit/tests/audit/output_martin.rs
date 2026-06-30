use std::path::PathBuf;

use pulse_audit::finding::{
    AuditFinding, AuditKind, AuditLocation, CycleMembership, ImportConfidence, MartinMetrics, MartinTier,
};
use pulse_audit::output::{format_findings, format_findings_json};
use pulse_thresholds::Thresholds;

fn t() -> Thresholds {
    Thresholds::default()
}

fn p(s: &str) -> PathBuf {
    PathBuf::from(s)
}

fn martin_finding(module: &str, ca: u32, ce: u32, i: f64, a: f64, d: f64, tier: MartinTier) -> AuditFinding {
    AuditFinding {
        kind: AuditKind::DistanceFromMainSequence(MartinMetrics {
            module: p(module),
            afferent: ca,
            efferent: ce,
            instability: i,
            abstractness: a,
            distance: d,
            tier,
            confidence: ImportConfidence::High,
        }),
        representative_snippet: format!("instability: {i:.3} | abstractness: {a:.3} | distance: {d:.3}"),
        support: ca + ce,
        file_count: 1,
        idf_score: None,
        action_label: None,
        pattern_category: None,
        locality_entropy: None,
        p_value: None,
        locations: vec![AuditLocation { file: p(module), line: 1 }],
    }
}

fn cycle_finding(members: &[&str], confidence: ImportConfidence) -> AuditFinding {
    let mut sorted: Vec<&str> = members.to_vec();
    sorted.sort_unstable();
    let member_paths: Vec<PathBuf> = sorted.iter().map(|s| p(s)).collect();
    let edges: Vec<(PathBuf, PathBuf)> = (0..members.len())
        .map(|i| {
            let next = (i + 1) % members.len();
            (member_paths[i].clone(), member_paths[next].clone())
        })
        .collect();
    AuditFinding {
        kind: AuditKind::ImportCycle(CycleMembership {
            members: member_paths.clone(),
            edges: edges.clone(),
            confidence,
            shape: pulse_audit::finding::CycleShape::Circle,
            centrality: 0.0,
            feedback_edge: None,
        }),
        representative_snippet: format!("cycle of {} modules", members.len()),
        support: edges.len() as u32,
        file_count: members.len() as u32,
        idf_score: None,
        action_label: None,
        pattern_category: None,
        locality_entropy: None,
        p_value: None,
        locations: member_paths.iter().map(|p| AuditLocation { file: p.clone(), line: 1 }).collect(),
    }
}

fn zero_edge_finding(count: u32) -> AuditFinding {
    AuditFinding {
        kind: AuditKind::ZeroEdgeProject { module_count: count },
        representative_snippet: String::new(),
        support: 0,
        file_count: count,
        idf_score: None,
        action_label: None,
        pattern_category: None,
        locality_entropy: None,
        p_value: None,
        locations: Vec::new(),
    }
}

#[test]
fn martin_human_starts_with_audit_prefix() {
    let f = martin_finding("foo.rs", 3, 5, 0.625, 0.0, 0.375, MartinTier::Healthy);
    let out = format_findings(&[f], None, &t().audit);
    assert!(out.starts_with("audit: distance from main sequence"));
}

#[test]
fn martin_human_includes_module_path() {
    let f = martin_finding("src/foo.rs", 0, 0, 0.0, 0.0, 1.0, MartinTier::Alert);
    let out = format_findings(&[f], None, &t().audit);
    assert!(out.contains("src/foo.rs"));
}

#[test]
fn martin_human_includes_ca_ce_i_a_d_lines() {
    let f = martin_finding("foo.rs", 3, 5, 0.625, 0.0, 0.375, MartinTier::Healthy);
    let out = format_findings(&[f], None, &t().audit);
    assert!(out.contains("Ca:"));
    assert!(out.contains("Ce:"));
    assert!(out.contains("instability:"));
    assert!(out.contains("abstractness:"));
    assert!(out.contains("distance:"));
}

#[test]
fn martin_human_renders_distance_to_three_decimals() {
    let f = martin_finding("foo.rs", 3, 5, 0.625, 0.123, 0.502, MartinTier::Healthy);
    let out = format_findings(&[f], None, &t().audit);
    assert!(out.contains("0.502"));
    assert!(out.contains("0.625"));
    assert!(out.contains("0.123"));
}

#[test]
fn martin_human_renders_warning_tier() {
    let f = martin_finding("foo.rs", 0, 5, 1.0, 0.0, 1.0, MartinTier::Warning);
    let out = format_findings(&[f], None, &t().audit);
    assert!(out.contains("tier:"));
    assert!(out.contains("warning"));
}

#[test]
fn martin_human_renders_alert_tier() {
    let f = martin_finding("foo.rs", 0, 5, 1.0, 0.0, 1.0, MartinTier::Alert);
    let out = format_findings(&[f], None, &t().audit);
    assert!(out.contains("alert"));
}

#[test]
fn martin_human_renders_healthy_tier() {
    let f = martin_finding("foo.rs", 5, 5, 0.5, 0.5, 0.0, MartinTier::Healthy);
    let out = format_findings(&[f], None, &t().audit);
    assert!(out.contains("healthy"));
}

#[test]
fn martin_human_renders_confidence_label() {
    let f = martin_finding("foo.rs", 0, 5, 1.0, 0.0, 1.0, MartinTier::Alert);
    let out = format_findings(&[f], None, &t().audit);
    assert!(out.contains("confidence:"));
    assert!(out.contains("high"));
}

#[test]
fn cycle_human_lists_members_and_edges() {
    let f = cycle_finding(&["a.rs", "b.rs"], ImportConfidence::High);
    let out = format_findings(&[f], None, &t().audit);
    assert!(out.contains("members:"));
    assert!(out.contains("edges:"));
    assert!(out.contains("a.rs"));
    assert!(out.contains("b.rs"));
}

#[test]
fn cycle_human_renders_size() {
    let f = cycle_finding(&["a.rs", "b.rs", "c.rs"], ImportConfidence::Medium);
    let out = format_findings(&[f], None, &t().audit);
    assert!(out.contains("size 3"));
}

#[test]
fn cycle_human_renders_min_confidence() {
    let f = cycle_finding(&["a.rs", "b.rs"], ImportConfidence::Low);
    let out = format_findings(&[f], None, &t().audit);
    assert!(out.contains("low"));
}

#[test]
fn zero_edge_human_renders_module_count_and_hint() {
    let f = zero_edge_finding(7);
    let out = format_findings(&[f], None, &t().audit);
    assert!(out.contains("7 modules"));
    assert!(out.contains("hint:"));
}

#[test]
fn martin_json_kind_is_distance_from_main_sequence() {
    let f = martin_finding("foo.rs", 3, 5, 0.625, 0.0, 0.375, MartinTier::Healthy);
    let s = format_findings_json(&[f], None);
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(v[0]["kind"], "DistanceFromMainSequence");
}

#[test]
fn martin_json_includes_metric_fields() {
    let f = martin_finding("foo.rs", 3, 5, 0.625, 0.0, 0.375, MartinTier::Healthy);
    let s = format_findings_json(&[f], None);
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert!(v[0]["afferent"].is_number());
    assert!(v[0]["efferent"].is_number());
    assert!(v[0]["instability"].is_number());
    assert!(v[0]["abstractness"].is_number());
    assert!(v[0]["distance"].is_number());
    assert_eq!(v[0]["tier"], "healthy");
    assert_eq!(v[0]["confidence"], "high");
}

#[test]
fn cycle_json_has_kind_import_cycle_and_size() {
    let f = cycle_finding(&["a.rs", "b.rs"], ImportConfidence::High);
    let s = format_findings_json(&[f], None);
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(v[0]["kind"], "ImportCycle");
    assert_eq!(v[0]["size"], 2);
}

#[test]
fn cycle_json_members_in_alphabetical_order() {
    let f = cycle_finding(&["z.rs", "a.rs", "m.rs"], ImportConfidence::High);
    let s = format_findings_json(&[f], None);
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    let members: Vec<String> =
        v[0]["members"].as_array().unwrap().iter().map(|x| x.as_str().unwrap().to_string()).collect();
    let mut sorted = members.clone();
    sorted.sort();
    assert_eq!(members, sorted);
}

#[test]
fn cycle_json_edges_are_path_pairs() {
    let f = cycle_finding(&["a.rs", "b.rs"], ImportConfidence::High);
    let s = format_findings_json(&[f], None);
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    let edges = v[0]["edges"].as_array().unwrap();
    assert!(!edges.is_empty());
    assert!(edges[0].is_array());
    assert_eq!(edges[0].as_array().unwrap().len(), 2);
}

#[test]
fn zero_edge_json_kind_and_count() {
    let f = zero_edge_finding(4);
    let s = format_findings_json(&[f], None);
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(v[0]["kind"], "ZeroEdgeProject");
    assert_eq!(v[0]["module_count"], 4);
}

#[test]
fn paths_displayed_relative_to_root() {
    let root = p("/abs/project");
    let f = martin_finding("/abs/project/src/foo.rs", 3, 5, 0.6, 0.0, 0.4, MartinTier::Healthy);
    let out = format_findings(&[f], Some(&root), &t().audit);
    assert!(out.contains("src/foo.rs"));
    assert!(!out.contains("/abs/project/"));
}

#[test]
fn multiple_findings_render_in_input_order() {
    let a = martin_finding("a.rs", 0, 5, 1.0, 0.0, 1.0, MartinTier::Alert);
    let b = cycle_finding(&["x.rs", "y.rs"], ImportConfidence::Medium);
    let out = format_findings(&[a, b], None, &t().audit);
    let martin_pos = out.find("distance from main sequence").unwrap();
    let cycle_pos = out.find("import cycle").unwrap();
    assert!(martin_pos < cycle_pos);
}

#[test]
fn empty_findings_produces_empty_string() {
    assert_eq!(format_findings(&[], None, &t().audit), "");
}

#[test]
fn empty_findings_produces_empty_json_array() {
    assert_eq!(format_findings_json(&[], None), "[]");
}
