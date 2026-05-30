use pulse::applicability::{filter_by_applicability, smell_applies, Surface};
use pulse::smells::{Finding, Location, Smell};

fn finding(smell: Smell) -> Finding {
    Finding {
        smell,
        location: Location::Function { name: "f".to_string(), start_line: 1, end_line: 2 },
        detail: String::new(),
    }
}

#[test]
fn short_variable_names_suppressed_on_hook() {
    assert!(!smell_applies(Smell::ShortVariableNames, Surface::Hook));
}

#[test]
fn short_variable_names_kept_on_check() {
    assert!(smell_applies(Smell::ShortVariableNames, Surface::Check));
}

#[test]
fn other_smells_apply_on_every_surface() {
    for surface in [Surface::Hook, Surface::Check] {
        assert!(smell_applies(Smell::ComplexMethod, surface));
        assert!(smell_applies(Smell::PrimitiveObsession, surface));
        assert!(smell_applies(Smell::ConstructorOverInjection, surface));
        assert!(smell_applies(Smell::StringlyTypedSwitch, surface));
    }
}

#[test]
fn filter_drops_short_var_on_hook_keeps_others() {
    let mut findings = vec![finding(Smell::ShortVariableNames), finding(Smell::ComplexMethod)];
    filter_by_applicability(&mut findings, Surface::Hook);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].smell, Smell::ComplexMethod);
}

#[test]
fn filter_keeps_short_var_on_check() {
    let mut findings = vec![finding(Smell::ShortVariableNames)];
    filter_by_applicability(&mut findings, Surface::Check);
    assert_eq!(findings.len(), 1);
}
