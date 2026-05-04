use pulse::thresholds::{AuditThresholds, Thresholds};

mod audit_common;
use audit_common::t;

#[test]
fn shotgun_surgery_changing_classes_default_is_five() {
    assert_eq!(t().audit.named_smells.shotgun_surgery.changing_classes, 5);
}

#[test]
fn shotgun_surgery_changing_methods_default_is_ten() {
    assert_eq!(t().audit.named_smells.shotgun_surgery.changing_methods, 10);
}

#[test]
fn shotgun_surgery_fanout_default_is_five() {
    assert_eq!(t().audit.named_smells.shotgun_surgery.fanout, 5);
}

#[test]
fn max_named_smell_findings_reported_default_is_fifty() {
    assert_eq!(t().audit.named_smells.max_findings_reported, 50);
}

#[test]
fn max_caller_samples_per_finding_default_is_twenty() {
    assert_eq!(t().audit.named_smells.max_caller_samples_per_finding, 20);
}

#[test]
fn audit_thresholds_struct_remains_copy() {
    fn assert_copy<T: Copy>(_: T) {}
    let a = AuditThresholds::DEFAULTS;
    let _b = a;
    assert_copy(a);
}

#[test]
fn defaults_const_matches_default_trait_impl() {
    let dflt = AuditThresholds::default();
    assert_eq!(dflt, AuditThresholds::DEFAULTS);
}

#[test]
fn shotgun_changing_classes_is_strictly_less_than_changing_methods() {
    let a = AuditThresholds::DEFAULTS;
    assert!(a.named_smells.shotgun_surgery.changing_classes < a.named_smells.shotgun_surgery.changing_methods);
}

#[test]
fn fanout_threshold_is_positive() {
    assert!(AuditThresholds::DEFAULTS.named_smells.shotgun_surgery.fanout > 0);
}

#[test]
fn caller_sample_cap_does_not_exceed_findings_cap() {
    let a = AuditThresholds::DEFAULTS;
    assert!(a.named_smells.max_caller_samples_per_finding <= a.named_smells.max_findings_reported);
}

#[test]
fn parent_thresholds_default_propagates_audit_subfields() {
    let parent = Thresholds::default();
    assert_eq!(parent.audit.named_smells.shotgun_surgery.changing_classes, 5);
    assert_eq!(parent.audit.named_smells.shotgun_surgery.fanout, 5);
}

#[test]
fn audit_thresholds_equal_after_clone() {
    let a = AuditThresholds::DEFAULTS;
    let b = a;
    assert_eq!(a, b);
}
