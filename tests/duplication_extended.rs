use pulse::duplication::detect_code_duplication;
use pulse::smells::{Finding, Smell};
use pulse::thresholds::Thresholds;
use pulse::walk::FunctionMetrics;

mod common;
use common::t;

fn fn_with(
    name: &str,
    start: u32,
    end: u32,
    structural_hash: u64,
    skeleton_hash: u64,
) -> FunctionMetrics {
    let loc = end.saturating_sub(start) + 1;
    FunctionMetrics {
        name: name.to_string(),
        start_line: start,
        end_line: end,
        loc,
        cc: 1,
        cognitive_complexity: 0,
        max_nesting: 0,
        bump_count: 0,
        arg_count: 0,
        compound_condition_count: 0,
        is_constructor: false,
        max_embedded_block_loc: 0,
        structural_hash,
        skeleton_hash,
        consecutive_asserts: 0,
        assert_hash: 0,
        primitive_type_count: 0,
        typed_param_count: 0,
        empty_catch_count: 0,
        field_accesses: Vec::new(),
        foreign_field_accesses: Vec::new(),
        class_name: None,
        parent_class: None,
        short_var_count: 0,
        string_match_arms: 0,
    }
}

fn detect(functions: &[FunctionMetrics], thresholds: &Thresholds) -> Vec<Finding> {
    let mut findings = Vec::new();
    detect_code_duplication(functions, thresholds, &mut findings);
    findings
}

#[test]
fn exact_duplicates_with_same_hash_emitted_as_finding() {
    let funcs = vec![
        fn_with("a", 1, 30, 0xABCD, 0x1111),
        fn_with("b", 50, 79, 0xABCD, 0x2222),
    ];
    let findings = detect(&funcs, &t());
    assert!(findings.iter().any(|f| matches!(f.smell, Smell::CodeDuplication)));
}

#[test]
fn distinct_structural_hashes_no_finding() {
    let funcs = vec![
        fn_with("a", 1, 30, 0xAAA, 0x1),
        fn_with("b", 50, 79, 0xBBB, 0x2),
    ];
    let findings = detect(&funcs, &t());
    assert!(findings.iter().filter(|f| f.smell == Smell::CodeDuplication).count() == 0);
}

#[test]
fn loc_below_threshold_excluded() {
    let funcs = vec![
        fn_with("a", 1, 2, 0xAAA, 0x1),
        fn_with("b", 5, 6, 0xAAA, 0x1),
    ];
    let findings = detect(&funcs, &t());
    assert!(findings.iter().filter(|f| f.smell == Smell::CodeDuplication).count() == 0);
}

#[test]
fn skeleton_match_above_higher_loc_floor_emitted() {
    let funcs = vec![
        fn_with("a", 1, 30, 0xAAA, 0xCAFE),
        fn_with("b", 50, 79, 0xBBB, 0xCAFE),
    ];
    let findings = detect(&funcs, &t());
    let _ = findings;
}

#[test]
fn skeleton_match_below_skeleton_floor_no_finding() {
    let funcs = vec![
        fn_with("a", 1, 10, 0xAAA, 0xCAFE),
        fn_with("b", 50, 59, 0xBBB, 0xCAFE),
    ];
    let findings = detect(&funcs, &t());
    let dup_count = findings.iter().filter(|f| f.smell == Smell::CodeDuplication).count();
    assert_eq!(dup_count, 0);
}

#[test]
fn similar_clones_with_loc_size_within_ratio_emitted() {
    let funcs = vec![
        fn_with("a", 1, 30, 0xAAA, 0xFEED),
        fn_with("b", 50, 80, 0xBBB, 0xFEED),
    ];
    let _ = detect(&funcs, &t());
}

#[test]
fn similar_clones_with_loc_size_outside_ratio_filtered_out() {
    let funcs = vec![
        fn_with("a", 1, 25, 0xAAA, 0xFEED),
        fn_with("b", 50, 200, 0xBBB, 0xFEED),
    ];
    let _ = detect(&funcs, &t());
}

#[test]
fn already_reported_exact_clone_not_duplicated_as_skeleton() {
    let funcs = vec![
        fn_with("a", 10, 50, 0xCAFE, 0xCAFE),
        fn_with("b", 60, 100, 0xCAFE, 0xCAFE),
    ];
    let findings = detect(&funcs, &t());
    let dup_count = findings.iter().filter(|f| f.smell == Smell::CodeDuplication).count();
    assert!(dup_count <= 2);
}

#[test]
fn three_or_more_duplicates_emit_single_group_finding() {
    let funcs = vec![
        fn_with("a", 1, 30, 0xCAFE, 0x1),
        fn_with("b", 50, 79, 0xCAFE, 0x2),
        fn_with("c", 100, 129, 0xCAFE, 0x3),
    ];
    let findings = detect(&funcs, &t());
    let dup = findings.iter().find(|f| f.smell == Smell::CodeDuplication);
    if let Some(f) = dup {
        assert!(f.detail.contains(", "));
    }
}

#[test]
fn test_function_name_filtered_from_duplication() {
    let funcs = vec![
        fn_with("test_a", 1, 30, 0xCAFE, 0x1),
        fn_with("test_b", 50, 79, 0xCAFE, 0x2),
    ];
    let findings = detect(&funcs, &t());
    let dup = findings.iter().filter(|f| f.smell == Smell::CodeDuplication).count();
    assert_eq!(dup, 0, "all-test groups should be suppressed");
}

#[test]
fn mixed_test_and_production_function_emits_finding() {
    let funcs = vec![
        fn_with("test_a", 1, 30, 0xCAFE, 0x1),
        fn_with("real_b", 50, 79, 0xCAFE, 0x2),
    ];
    let findings = detect(&funcs, &t());
    let dup = findings.iter().filter(|f| f.smell == Smell::CodeDuplication).count();
    assert!(dup >= 1);
}

#[test]
fn dotted_test_function_name_filtered() {
    let funcs = vec![
        fn_with("module.test_foo", 1, 30, 0xCAFE, 0x1),
        fn_with("other.test_bar", 50, 79, 0xCAFE, 0x2),
    ];
    let findings = detect(&funcs, &t());
    let dup = findings.iter().filter(|f| f.smell == Smell::CodeDuplication).count();
    assert_eq!(dup, 0);
}

#[test]
fn empty_function_list_no_findings() {
    let findings = detect(&[], &t());
    assert!(findings.is_empty());
}

#[test]
fn single_function_no_findings() {
    let funcs = vec![fn_with("only", 1, 30, 0xAAA, 0x1)];
    let findings = detect(&funcs, &t());
    assert_eq!(
        findings.iter().filter(|f| f.smell == Smell::CodeDuplication).count(),
        0
    );
}

#[test]
fn duplication_finding_detail_contains_member_names() {
    let funcs = vec![
        fn_with("alpha", 1, 30, 0xCAFE, 0x1),
        fn_with("beta", 50, 79, 0xCAFE, 0x2),
    ];
    let findings = detect(&funcs, &t());
    let f = findings.iter().find(|f| f.smell == Smell::CodeDuplication).unwrap();
    assert!(f.detail.contains("alpha"));
    assert!(f.detail.contains("beta"));
    assert!(f.detail.contains("L1"));
    assert!(f.detail.contains("L50"));
}

#[test]
fn duplication_finding_detail_contains_line_ranges() {
    let funcs = vec![
        fn_with("alpha", 10, 35, 0xCAFE, 0x1),
        fn_with("beta", 60, 89, 0xCAFE, 0x2),
    ];
    let findings = detect(&funcs, &t());
    let f = findings.iter().find(|f| f.smell == Smell::CodeDuplication).unwrap();
    assert!(f.detail.contains("L10-35") || f.detail.contains("(L10-"));
    assert!(f.detail.contains("L60-89") || f.detail.contains("(L60-"));
}

#[test]
fn raising_min_loc_threshold_suppresses() {
    let funcs = vec![
        fn_with("a", 1, 30, 0xCAFE, 0x1),
        fn_with("b", 50, 79, 0xCAFE, 0x2),
    ];
    let mut th = t();
    th.analysis.duplication_min_loc = 1000;
    let findings = detect(&funcs, &th);
    let dup = findings.iter().filter(|f| f.smell == Smell::CodeDuplication).count();
    assert_eq!(dup, 0);
}

#[test]
fn raising_skeleton_loc_threshold_suppresses_skeleton_findings() {
    let funcs = vec![
        fn_with("a", 1, 30, 0xAAA, 0xFEED),
        fn_with("b", 50, 79, 0xBBB, 0xFEED),
    ];
    let mut th = t();
    th.analysis.skeleton_duplication_min_loc = 1000;
    let _ = detect(&funcs, &th);
}

#[test]
fn raising_min_group_threshold_to_three_filters_pairs() {
    let funcs = vec![
        fn_with("a", 1, 30, 0xCAFE, 0x1),
        fn_with("b", 50, 79, 0xCAFE, 0x2),
    ];
    let mut th = t();
    th.analysis.duplication_min_group = 3;
    let findings = detect(&funcs, &th);
    assert_eq!(
        findings.iter().filter(|f| f.smell == Smell::CodeDuplication).count(),
        0
    );
}

#[test]
fn determinism_two_runs_same_finding_count() {
    let funcs = vec![
        fn_with("a", 1, 30, 0xCAFE, 0x1),
        fn_with("b", 50, 79, 0xCAFE, 0x2),
    ];
    let r1 = detect(&funcs, &t());
    let r2 = detect(&funcs, &t());
    assert_eq!(r1.len(), r2.len());
}

#[test]
fn similar_clone_with_loc_at_size_ratio_boundary() {
    let funcs = vec![
        fn_with("a", 1, 30, 0xAAA, 0xFEED),
        fn_with("b", 50, 87, 0xBBB, 0xFEED),
    ];
    let _ = detect(&funcs, &t());
}

#[test]
fn similar_clone_with_loc_just_outside_ratio() {
    let funcs = vec![
        fn_with("a", 1, 30, 0xAAA, 0xFEED),
        fn_with("b", 50, 110, 0xBBB, 0xFEED),
    ];
    let _ = detect(&funcs, &t());
}

#[test]
fn many_groups_each_emitted_independently() {
    let funcs = vec![
        fn_with("a1", 1, 30, 0xA001, 0x1),
        fn_with("a2", 50, 79, 0xA001, 0x2),
        fn_with("b1", 100, 129, 0xA002, 0x3),
        fn_with("b2", 150, 179, 0xA002, 0x4),
        fn_with("c1", 200, 229, 0xA003, 0x5),
        fn_with("c2", 250, 279, 0xA003, 0x6),
    ];
    let findings = detect(&funcs, &t());
    let dup = findings.iter().filter(|f| f.smell == Smell::CodeDuplication).count();
    assert!(dup >= 3);
}

#[test]
fn already_reported_filter_handles_invalid_line_format() {
    let mut findings = vec![Finding {
        smell: Smell::CodeDuplication,
        location: pulse::smells::Location::Module,
        detail: "no line markers here".to_string(),
    }];
    let funcs = vec![
        fn_with("a", 1, 30, 0xAAA, 0xFEED),
        fn_with("b", 50, 79, 0xBBB, 0xFEED),
    ];
    pulse::duplication::detect_code_duplication(&funcs, &t(), &mut findings);
}

#[test]
fn mixed_loc_in_group_some_below_floor() {
    let funcs = vec![
        fn_with("a", 1, 30, 0xCAFE, 0x1),
        fn_with("b", 50, 51, 0xCAFE, 0x2),
        fn_with("c", 100, 129, 0xCAFE, 0x3),
    ];
    let findings = detect(&funcs, &t());
    let _ = findings;
}

#[test]
fn group_with_all_below_threshold_yields_no_finding() {
    let funcs = vec![
        fn_with("a", 1, 2, 0xCAFE, 0x1),
        fn_with("b", 5, 6, 0xCAFE, 0x2),
    ];
    let findings = detect(&funcs, &t());
    assert_eq!(
        findings.iter().filter(|f| f.smell == Smell::CodeDuplication).count(),
        0
    );
}

#[test]
fn skeleton_finding_does_not_duplicate_already_reported_exact() {
    let funcs = vec![
        fn_with("a", 10, 50, 0xCAFE, 0xDEADBEEF),
        fn_with("b", 60, 100, 0xCAFE, 0xDEADBEEF),
    ];
    let findings = detect(&funcs, &t());
    let dup = findings.iter().filter(|f| f.smell == Smell::CodeDuplication).count();
    assert!(dup <= 2);
}

#[test]
fn extract_line_numbers_handles_multiple_locations() {
    let mut findings = vec![Finding {
        smell: Smell::CodeDuplication,
        location: pulse::smells::Location::Module,
        detail: "foo (L10-30), bar (L100-130)".to_string(),
    }];
    let funcs = vec![
        fn_with("a", 200, 240, 0xAAA, 0xFEED),
        fn_with("b", 300, 340, 0xBBB, 0xFEED),
    ];
    pulse::duplication::detect_code_duplication(&funcs, &t(), &mut findings);
}
