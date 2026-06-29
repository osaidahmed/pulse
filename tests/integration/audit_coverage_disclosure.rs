use pulse::audit::coverage::{disclosure, language_coverage};
use pulse::audit::finding::{AuditFinding, AuditKind, AuditLocation, PatternCategory};
use pulse::audit::output::{format_findings_filtered, format_findings_json_filtered};
use pulse::audit::suppression::AuditSuppression;
use pulse::parse::Language;
use std::path::PathBuf;

use crate::audit_common::{plain_ctx, t};

fn sample_finding() -> AuditFinding {
    AuditFinding {
        kind: AuditKind::UncategorizedPattern { fingerprint: 1 },
        representative_snippet: "foo()".to_string(),
        support: 25,
        file_count: 5,
        idf_score: Some(2.0),
        action_label: Some("wrap repeated literals in a typed object"),
        pattern_category: Some(PatternCategory::MethodCall),
        locality_entropy: None,
        p_value: None,
        locations: vec![AuditLocation { file: PathBuf::from("a.py"), line: 1 }],
    }
}

#[test]
fn coverage_counts_pin_current_dispatch_tables() {
    let c = language_coverage();
    assert_eq!(c.languages, Language::COUNT);
    assert_eq!(c.imports, Language::COUNT, "every language has an import extractor");
    assert_eq!(c.abstractness, 18, "abstractness profile table covers 18 languages");
    assert_eq!(c.taint, 12, "taint lang table covers 12 languages");
    assert_eq!(c.cpg, 13, "cfg lang consts cover 13 languages");
}

#[test]
fn disclosure_reports_cpg_off_when_disabled() {
    let c = language_coverage();
    let line = disclosure(false);
    assert!(line.contains(&format!("imports {}/{}", c.imports, c.languages)), "{line}");
    assert!(line.contains(&format!("abstractness {}/{}", c.abstractness, c.languages)), "{line}");
    assert!(line.contains(&format!("taint {}/{}", c.taint, c.languages)), "{line}");
    assert!(line.contains("cpg off"), "{line}");
}

#[test]
fn disclosure_reports_cpg_count_when_enabled() {
    let c = language_coverage();
    let line = disclosure(true);
    assert!(line.contains(&format!("cpg {}/{}", c.cpg, c.languages)), "{line}");
    assert!(!line.contains("cpg off"), "{line}");
}

#[test]
fn human_header_carries_disclosure_line() {
    let findings = vec![sample_finding()];
    let supp = AuditSuppression::new();
    let out = format_findings_filtered(&findings, &t().audit, &plain_ctx(&supp));
    assert!(out.contains(&format!("analyzed: {}", disclosure(false))), "{out}");
}

#[test]
fn json_summary_carries_language_coverage() {
    let findings = vec![sample_finding()];
    let supp = AuditSuppression::new();
    let s = format_findings_json_filtered(&findings, &plain_ctx(&supp));
    let parsed: serde_json::Value = serde_json::from_str(&s).expect("valid json");
    let cov =
        parsed.get("summary").and_then(|s| s.get("language_coverage")).expect("summary.language_coverage present");
    let c = language_coverage();
    assert_eq!(cov.get("languages").and_then(serde_json::Value::as_u64), Some(c.languages as u64));
    assert_eq!(cov.get("imports").and_then(serde_json::Value::as_u64), Some(c.imports as u64));
    assert_eq!(cov.get("abstractness").and_then(serde_json::Value::as_u64), Some(c.abstractness as u64));
    assert_eq!(cov.get("taint").and_then(serde_json::Value::as_u64), Some(c.taint as u64));
    assert_eq!(cov.get("cpg").and_then(serde_json::Value::as_u64), Some(c.cpg as u64));
    assert_eq!(cov.get("cpg_enabled").and_then(serde_json::Value::as_bool), Some(false));
}
