use std::path::PathBuf;

use pulse::audit::finding::{AuditFinding, AuditKind, PatternCategory};
use pulse::audit::output::format_findings_filtered;
use pulse::config::{AuditConfig, AuditSuppression, IgnoreMatcher};

use crate::audit_common::{plain_ctx, t};

fn pattern_finding(snippet: &str) -> AuditFinding {
    AuditFinding {
        kind: AuditKind::UncategorizedPattern { fingerprint: 1 },
        representative_snippet: snippet.to_string(),
        support: 25,
        file_count: 5,
        idf_score: Some(2.0),
        action_label: Some("wrap repeated literals in a typed object"),
        pattern_category: Some(PatternCategory::MethodCall),
        locality_entropy: None,
        p_value: None,
        locations: vec![pulse::audit::finding::AuditLocation { file: PathBuf::from("a.py"), line: 1 }],
    }
}

fn supp_with_patterns(hide_patterns: &[&str]) -> AuditSuppression {
    let cfg = AuditConfig {
        hide_categories: Vec::new(),
        hide_smells: Vec::new(),
        hide_patterns: hide_patterns.iter().map(std::string::ToString::to_string).collect(),
        cross_validate_history: None,
    };
    AuditSuppression::from_config(Some(&cfg))
}

// Covers src/config.rs:188 — the `continue` branch in AuditSuppression::from_config:
// a whitespace-only hide_patterns entry trims to empty and is skipped, while the
// real glob alongside it is still compiled and applied.
#[test]
fn from_config_skips_blank_hide_pattern_entry() {
    let supp = supp_with_patterns(&["   ", "migrations.*"]);
    assert!(!supp.is_empty(), "non-blank pattern keeps suppression active");

    let findings = vec![pattern_finding("migrations.RunPython(forwards)"), pattern_finding("path('home', views.home)")];
    let out = format_findings_filtered(&findings, &t().audit, &plain_ctx(&supp));
    assert!(!out.contains("RunPython"), "real glob still hides matching finding: {out}");
    assert!(out.contains("path('home'"), "non-matching finding survives: {out}");
}

// Covers src/config.rs:188 in isolation — a lone whitespace-only pattern leaves
// the suppression empty (the only entry was `continue`d past).
#[test]
fn from_config_blank_only_pattern_yields_empty_pattern_set() {
    let supp = supp_with_patterns(&["\t  \n"]);
    assert!(supp.is_empty(), "blank-only hide_patterns must produce empty suppression");

    let findings = vec![pattern_finding("foo()")];
    let out = format_findings_filtered(&findings, &t().audit, &plain_ctx(&supp));
    assert!(out.contains("foo()"), "nothing suppressed: {out}");
    assert!(!out.contains("hidden by .pulse.toml"), "no suppression header: {out}");
}

// Covers src/config.rs:230 — the `return Vec::new()` branch in expand_pattern,
// reached only when a pattern is non-empty but is all slashes (trims to empty
// after trim_end_matches('/')). Driven via the public IgnoreMatcher; a slash-only
// pattern contributes no glob, so the matcher matches nothing.
#[test]
fn matcher_slash_only_pattern_matches_nothing() {
    let m = IgnoreMatcher::from_patterns(&["/".to_string()]);
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("a.py");
    std::fs::write(&file, "").unwrap();
    assert!(!m.matches_file(dir.path(), &file), "slash-only pattern must not match any file");
}

// Covers src/config.rs:230 — a multi-slash-only pattern also strips to empty and
// is dropped, while a valid sibling pattern still matches (proving the slash-only
// one was skipped rather than corrupting the set).
#[test]
fn matcher_multi_slash_pattern_dropped_real_pattern_still_matches() {
    let m = IgnoreMatcher::from_patterns(&["///".to_string(), "vendor/**".to_string()]);
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("vendor");
    std::fs::create_dir(&nested).unwrap();
    let file = nested.join("lib.py");
    std::fs::write(&file, "").unwrap();
    assert!(m.matches_file(dir.path(), &file), "real pattern still matches alongside slash-only");

    let outside = dir.path().join("a.py");
    std::fs::write(&outside, "").unwrap();
    assert!(!m.matches_file(dir.path(), &outside), "slash-only pattern added no match");
}
