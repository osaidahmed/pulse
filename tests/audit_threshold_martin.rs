use pulse::thresholds::{AuditThresholds, Thresholds};

fn t() -> Thresholds {
    Thresholds::default()
}

#[test]
fn martin_distance_warning_default_is_zero_point_seven() {
    assert!((t().audit.martin_distance_warning - 0.7).abs() < 1e-12);
}

#[test]
fn martin_distance_alert_default_is_zero_point_eight_five() {
    assert!((t().audit.martin_distance_alert - 0.85).abs() < 1e-12);
}

#[test]
fn include_tests_in_graph_default_is_false() {
    assert!(!t().audit.include_tests_in_graph);
}

#[test]
fn martin_cycle_min_size_default_is_two() {
    assert_eq!(t().audit.martin_cycle_min_size, 2);
}

#[test]
fn max_cycle_findings_reported_default_is_fifty() {
    assert_eq!(t().audit.max_cycle_findings_reported, 50);
}

#[test]
fn max_martin_findings_reported_default_is_one_hundred() {
    assert_eq!(t().audit.max_martin_findings_reported, 100);
}

#[test]
fn audit_thresholds_default_round_trips_via_const() {
    let direct = AuditThresholds::DEFAULTS;
    let via_default = AuditThresholds::default();
    assert_eq!(direct.martin_distance_warning, via_default.martin_distance_warning);
    assert_eq!(direct.martin_distance_alert, via_default.martin_distance_alert);
    assert_eq!(direct.martin_cycle_min_size, via_default.martin_cycle_min_size);
    assert_eq!(
        direct.max_cycle_findings_reported,
        via_default.max_cycle_findings_reported
    );
    assert_eq!(
        direct.max_martin_findings_reported,
        via_default.max_martin_findings_reported
    );
}

#[test]
fn martin_thresholds_can_be_overridden_locally() {
    let mut th = t().audit;
    th.martin_distance_warning = 0.5;
    th.martin_distance_alert = 0.6;
    assert!((th.martin_distance_warning - 0.5).abs() < 1e-12);
    assert!((th.martin_distance_alert - 0.6).abs() < 1e-12);
}

#[test]
fn audit_thresholds_is_copy() {
    let original = t().audit;
    let copy = original;
    assert_eq!(original.martin_distance_warning, copy.martin_distance_warning);
}

#[test]
fn legacy_layer_three_thresholds_unchanged_alongside_new_fields() {
    let th = t().audit;
    assert_eq!(th.freqt_min_support, 5);
    assert_eq!(th.subtree_min_depth, 3);
    assert_eq!(th.subtree_min_nodes, 5);
    assert!((th.idiom_suppression_threshold - 0.5).abs() < 1e-12);
    assert_eq!(th.max_findings_reported, 50);
    assert_eq!(th.max_locations_per_finding, 20);
}

#[test]
fn warning_threshold_is_lower_than_alert_threshold_by_default() {
    let th = t().audit;
    assert!(th.martin_distance_warning < th.martin_distance_alert);
}

#[test]
fn warning_threshold_is_within_zero_one_range() {
    let th = t().audit;
    assert!(th.martin_distance_warning >= 0.0 && th.martin_distance_warning <= 1.0);
}

#[test]
fn alert_threshold_is_within_zero_one_range() {
    let th = t().audit;
    assert!(th.martin_distance_alert >= 0.0 && th.martin_distance_alert <= 1.0);
}

#[test]
fn cycle_min_size_is_at_least_one() {
    let th = t().audit;
    assert!(th.martin_cycle_min_size >= 1);
}

#[test]
fn max_findings_reported_caps_are_positive() {
    let th = t().audit;
    assert!(th.max_cycle_findings_reported > 0);
    assert!(th.max_martin_findings_reported > 0);
}
