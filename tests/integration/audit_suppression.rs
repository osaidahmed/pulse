use pulse::audit::finding::{
    AuditFinding, AuditKind, AuditLocation, ImportConfidence, PatternCategory, ShotgunSurgeryEvidence,
};
use pulse::audit::output::{format_findings_filtered, format_findings_json_filtered};
use pulse::config::{AuditConfig, AuditSuppression};
use std::path::PathBuf;

use crate::audit_common::t;

fn pattern_finding(snippet: &str, category: PatternCategory) -> AuditFinding {
    AuditFinding {
        kind: AuditKind::UncategorizedPattern { fingerprint: 1 },
        representative_snippet: snippet.to_string(),
        support: 25,
        file_count: 5,
        idf_score: Some(2.0),
        action_label: Some("wrap repeated literals in a typed object"),
        pattern_category: Some(category),
        locality_entropy: None,
        p_value: None,
        locations: vec![AuditLocation { file: PathBuf::from("a.py"), line: 1 }],
    }
}

fn shotgun_finding(name: &str) -> AuditFinding {
    AuditFinding {
        kind: AuditKind::ShotgunSurgery(ShotgunSurgeryEvidence {
            method_file: PathBuf::from("svc.py"),
            method_class: Some("Svc".to_string()),
            method_name: name.to_string(),
            method_line: 1,
            changing_classes: 6,
            changing_methods: 12,
            fanout: 7,
            confidence: ImportConfidence::Medium,
            caller_samples: Vec::new(),
            name_collision_count: 0,
            additional_definitions: Vec::new(),
        }),
        representative_snippet: String::new(),
        support: 0,
        file_count: 0,
        idf_score: None,
        action_label: Some("introduce polymorphism or move logic to a single owner"),
        pattern_category: None,
        locality_entropy: None,
        p_value: None,
        locations: Vec::new(),
    }
}

fn cfg(hide_categories: &[&str], hide_smells: &[&str], hide_patterns: &[&str]) -> AuditSuppression {
    let cfg = AuditConfig {
        hide_categories: hide_categories.iter().map(std::string::ToString::to_string).collect(),
        hide_smells: hide_smells.iter().map(std::string::ToString::to_string).collect(),
        hide_patterns: hide_patterns.iter().map(std::string::ToString::to_string).collect(),
    };
    AuditSuppression::from_config(Some(&cfg))
}

#[test]
fn hide_categories_removes_pattern_findings() {
    let findings = vec![
        pattern_finding("foo()", PatternCategory::MethodCall),
        pattern_finding("\"x\"", PatternCategory::LiteralRepetition),
    ];
    let supp = cfg(&["method_call"], &[], &[]);
    let out = format_findings_filtered(&findings, None, &t().audit, false, &supp);
    assert!(!out.contains("foo()"), "method_call category should be hidden");
    assert!(out.contains("\"x\""), "literal_repetition should remain");
    assert!(out.contains("hidden by .pulse.toml"), "header should report suppression");
}

#[test]
fn hide_smells_removes_named_findings() {
    let findings = vec![shotgun_finding("search"), shotgun_finding("save")];
    let supp = cfg(&[], &["shotgun_surgery"], &[]);
    let out = format_findings_filtered(&findings, None, &t().audit, false, &supp);
    assert!(!out.contains("shotgun surgery"), "all shotgun findings hidden");
    assert!(out.contains("hidden by .pulse.toml"));
}

#[test]
fn hide_patterns_glob_matches_representative_snippet() {
    let findings = vec![
        pattern_finding("migrations.RunPython(forwards)", PatternCategory::MethodCall),
        pattern_finding("path('home', views.home)", PatternCategory::MethodCall),
    ];
    let supp = cfg(&[], &[], &["migrations.*"]);
    let out = format_findings_filtered(&findings, None, &t().audit, false, &supp);
    assert!(!out.contains("RunPython"), "migrations glob should hide RunPython");
    assert!(out.contains("path('home'"), "path() pattern remains");
}

#[test]
fn empty_suppression_passes_everything_through() {
    let findings = vec![pattern_finding("foo()", PatternCategory::MethodCall)];
    let supp = AuditSuppression::new();
    let out = format_findings_filtered(&findings, None, &t().audit, false, &supp);
    assert!(out.contains("foo()"));
    assert!(!out.contains("hidden by .pulse.toml"));
}

#[test]
fn suppression_applies_to_json_output() {
    let findings = vec![shotgun_finding("search"), shotgun_finding("save")];
    let supp = cfg(&[], &["shotgun_surgery"], &[]);
    let out = format_findings_json_filtered(&findings, None, false, &supp);
    let parsed: serde_json::Value = serde_json::from_str(&out).expect("valid json");
    let findings_array = parsed.get("findings").and_then(|v| v.as_array()).expect("findings array");
    assert!(findings_array.is_empty(), "json should also filter hidden findings");
}
