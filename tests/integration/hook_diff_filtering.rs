use pulse::hook::filter_by_edit_range;
use pulse::smells::{Finding, Location, Smell};

fn func_finding(name: &str, start_line: u32, end_line: u32) -> Finding {
    Finding {
        smell: Smell::ComplexMethod,
        location: Location::Function {
            name: name.to_string(),
            start_line,
            end_line,
        },
        detail: String::new(),
    }
}

fn module_finding() -> Finding {
    Finding {
        smell: Smell::FileTooLarge,
        location: Location::Module,
        detail: String::new(),
    }
}

fn names(findings: &[Finding]) -> Vec<String> {
    findings
        .iter()
        .map(|f| match &f.location {
            Location::Function { name, .. } => name.clone(),
            Location::Module => "<module>".into(),
        })
        .collect()
}

#[test]
fn finding_strictly_inside_range_is_kept() {
    let edit_start = 30;
    let edit_end = 40;
    let findings = vec![func_finding("inside", 32, 38)];
    let kept = filter_by_edit_range(findings, Some((edit_start, edit_end)));
    assert_eq!(names(&kept), vec!["inside"]);
}

#[test]
fn finding_strictly_outside_range_is_dropped() {
    let findings = vec![
        func_finding("before", 5, 10),
        func_finding("after", 100, 110),
    ];
    let kept = filter_by_edit_range(findings, Some((30, 40)));
    assert!(kept.is_empty(), "no findings should overlap, got: {:?}", names(&kept));
}

#[test]
fn finding_starting_at_edit_end_is_kept() {
    let findings = vec![func_finding("at_end", 40, 50)];
    let kept = filter_by_edit_range(findings, Some((30, 40)));
    assert_eq!(names(&kept), vec!["at_end"], "boundary start_line == edit_end included");
}

#[test]
fn finding_starting_one_past_edit_end_is_dropped() {
    let findings = vec![func_finding("just_past", 41, 50)];
    let kept = filter_by_edit_range(findings, Some((30, 40)));
    assert!(kept.is_empty(), "boundary start_line == edit_end + 1 excluded, got: {:?}", names(&kept));
}

#[test]
fn finding_ending_at_edit_start_is_kept() {
    let findings = vec![func_finding("ending_at_start", 1, 30)];
    let kept = filter_by_edit_range(findings, Some((30, 40)));
    assert_eq!(names(&kept), vec!["ending_at_start"], "boundary end_line == edit_start included");
}

#[test]
fn finding_ending_one_before_edit_start_is_dropped() {
    let findings = vec![func_finding("ending_just_before", 1, 29)];
    let kept = filter_by_edit_range(findings, Some((30, 40)));
    assert!(kept.is_empty(), "boundary end_line == edit_start - 1 excluded, got: {:?}", names(&kept));
}

#[test]
fn finding_overlapping_edit_range_is_kept() {
    let findings = vec![func_finding("overlaps", 20, 50)];
    let kept = filter_by_edit_range(findings, Some((30, 40)));
    assert_eq!(names(&kept), vec!["overlaps"]);
}

#[test]
fn module_finding_dropped_when_range_provided() {
    let findings = vec![
        module_finding(),
        func_finding("inside", 32, 38),
    ];
    let kept = filter_by_edit_range(findings, Some((30, 40)));
    assert_eq!(names(&kept), vec!["inside"], "module finding must be dropped when range provided");
}

#[test]
fn mixed_findings_only_overlapping_kept() {
    let findings = vec![
        func_finding("before", 5, 10),
        func_finding("inside", 32, 38),
        func_finding("at_boundary_start", 1, 30),
        func_finding("at_boundary_end", 40, 60),
        func_finding("after", 100, 110),
        module_finding(),
    ];
    let mut got = names(&filter_by_edit_range(findings, Some((30, 40))));
    got.sort();
    let mut expected = vec![
        "at_boundary_end".to_string(),
        "at_boundary_start".to_string(),
        "inside".to_string(),
    ];
    expected.sort();
    assert_eq!(got, expected);
}

#[test]
fn none_range_keeps_all_function_findings() {
    let findings = vec![
        func_finding("a", 1, 10),
        func_finding("b", 100, 200),
        func_finding("c", 1000, 2000),
    ];
    let kept = filter_by_edit_range(findings, None);
    let mut got = names(&kept);
    got.sort();
    assert_eq!(got, vec!["a", "b", "c"]);
}

#[test]
fn none_range_drops_module_findings_only() {
    let findings = vec![
        func_finding("fn1", 1, 10),
        module_finding(),
        func_finding("fn2", 50, 60),
        module_finding(),
    ];
    let kept = filter_by_edit_range(findings, None);
    let mut got = names(&kept);
    got.sort();
    assert_eq!(got, vec!["fn1", "fn2"]);
}

#[test]
fn none_range_with_no_module_findings_is_identity() {
    let findings = vec![func_finding("only", 5, 15)];
    let kept = filter_by_edit_range(findings.clone(), None);
    assert_eq!(names(&kept), names(&findings));
}

#[test]
fn empty_input_returns_empty() {
    let kept = filter_by_edit_range(Vec::new(), Some((10, 20)));
    assert!(kept.is_empty());
    let kept = filter_by_edit_range(Vec::new(), None);
    assert!(kept.is_empty());
}

#[test]
fn range_with_zero_length_keeps_findings_at_that_line() {
    let findings = vec![
        func_finding("at_line_50", 50, 50),
        func_finding("nearby", 49, 51),
        func_finding("far_away", 1, 5),
    ];
    let kept = filter_by_edit_range(findings, Some((50, 50)));
    let mut got = names(&kept);
    got.sort();
    assert_eq!(got, vec!["at_line_50", "nearby"]);
}
