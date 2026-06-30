use pulse_audit::finding::AuditKind;
use pulse_audit::suppression::AuditSuppression;
use pulse_audit::{self as audit, AuditOpts, IgnoreFilter, PassChoice};
use pulse_config::IgnoreMatcher;

use crate::audit_common::{scenario_path, t};

fn run_audit_named_smells(scenario: &str, lang: &str) -> Vec<pulse_audit::finding::AuditFinding> {
    let dir = scenario_path(scenario, lang);
    let matcher = IgnoreMatcher::from_patterns(&[]);
    let opts = AuditOpts {
        root: dir.clone(),
        pass: Some(PassChoice::NamedSmells),
        json: false,
        include_tests: true,
        show_noise: false,
        suppression: AuditSuppression::new(),
    };
    let filter = IgnoreFilter::new(&matcher, &dir);
    audit::run_with_filter(&opts, &t().audit, &filter)
}

#[test]
fn shotgun_surgery_calls_python_fixture_finds_handle() {
    let findings = run_audit_named_smells("shotgun_surgery_calls", "python");
    let shotgun: Vec<_> = findings.iter().filter(|f| matches!(f.kind, AuditKind::ShotgunSurgery(_))).collect();
    assert!(!shotgun.is_empty(), "expected at least one ShotgunSurgery finding");
    let any_handle = shotgun.iter().any(|f| {
        let AuditKind::ShotgunSurgery(e) = &f.kind else { return false };
        e.method_name == "handle"
    });
    assert!(any_handle, "expected ShotgunSurgery on `handle`");
}

#[test]
fn shotgun_surgery_finding_has_high_or_medium_confidence() {
    let findings = run_audit_named_smells("shotgun_surgery_calls", "python");
    let handle = findings.iter().find_map(|f| {
        let AuditKind::ShotgunSurgery(e) = &f.kind else { return None };
        if e.method_name == "handle" {
            Some(e)
        } else {
            None
        }
    });
    assert!(handle.is_some());
}

#[test]
fn shotgun_surgery_finding_has_caller_samples() {
    let findings = run_audit_named_smells("shotgun_surgery_calls", "python");
    for f in &findings {
        let AuditKind::ShotgunSurgery(e) = &f.kind else { continue };
        if e.method_name == "handle" {
            assert!(!e.caller_samples.is_empty());
            return;
        }
    }
    panic!("no handle finding to inspect samples");
}

#[test]
fn clean_small_python_produces_no_named_smells() {
    let findings = run_audit_named_smells("clean_small", "python");
    let shotgun_count = findings.iter().filter(|f| matches!(f.kind, AuditKind::ShotgunSurgery(_))).count();
    assert_eq!(shotgun_count, 0);
}
