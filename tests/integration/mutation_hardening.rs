use crate::common::t;
use pulse::calibrate::priors::corpus_priors;
use pulse::intensity::{for_finding, normalize_exceedance, rank_findings};
use pulse::parse::Language;
use pulse::smells::{detect, Finding, Location, Smell};
use pulse::walk::{FileMetrics, FunctionMetrics, ModuleMetrics};

const RUST: Language = Language::Rust;

fn corpus_tail(metric: &str) -> f64 {
    corpus_priors().main["rust"].metrics[metric].quantile(0.995)
}

fn fm(name: &str) -> FunctionMetrics {
    FunctionMetrics {
        name: name.to_string(),
        start_line: 1,
        end_line: 2,
        loc: 2,
        cc: 1,
        cognitive_complexity: 0,
        max_nesting: 0,
        bump_count: 0,
        arg_count: 0,
        compound_condition_count: 0,
        is_constructor: false,
        max_embedded_block_loc: 0,
        structural_hash: 0,
        distinct_node_kinds: 0,
        skeleton_hash: 0,
        consecutive_asserts: 0,
        assert_hash: 0,
        primitive_type_count: 0,
        typed_param_count: 0,
        max_same_primitive_count: 0,
        empty_catch_count: 0,
        field_accesses: Vec::new(),
        foreign_field_accesses: Vec::new(),
        class_name: None,
        parent_class: None,
        short_var_count: 0,
        string_match_arms: 0,
        cpg: None,
    }
}

fn module() -> ModuleMetrics {
    ModuleMetrics {
        total_loc: 10,
        total_functions: 1,
        sum_cc: 1,
        global_conditional_count: 0,
        global_max_nesting: 0,
        declaration_count: 0,
        struct_fields: Vec::new(),
    }
}

fn detect_one(f: FunctionMetrics) -> Vec<Finding> {
    detect(&FileMetrics { functions: vec![f], module: module() }, "", &t())
}

fn detail_of(findings: &[Finding], smell: Smell) -> String {
    findings.iter().find(|f| f.smell == smell).map(|f| f.detail.clone()).unwrap_or_default()
}

#[test]
fn severity_labels_flip_strictly_above_the_alert_thresholds() {
    let th = t().function;
    let cases: &[(u32, u32, u32, &str)] = &[
        (th.cc_alert, 0, 2, "warning"),
        (th.cc_alert + 1, 0, 2, "alert"),
        (1, th.cogc_alert, 2, "warning"),
        (1, th.cogc_alert + 1, 2, "alert"),
        (th.cc_alert, th.cogc_alert, 2, "warning"),
        (th.cc_alert + 1, th.cogc_alert, 2, "alert"),
    ];
    for &(cc, cogc, loc, expected) in cases {
        let mut f = fm("worker");
        f.cc = cc;
        f.cognitive_complexity = cogc;
        f.loc = loc;
        let detail = detail_of(&detect_one(f), Smell::ComplexMethod);
        assert!(detail.contains(&format!("[{expected}]")), "cc={cc} cogc={cogc} should label [{expected}]: {detail}");
    }
}

#[test]
fn large_method_label_flips_strictly_above_the_loc_alert() {
    let th = t().function;
    for (loc, expected) in [(th.fn_loc_alert, "warning"), (th.fn_loc_alert + 1, "alert")] {
        let mut f = fm("pager");
        f.loc = loc;
        f.end_line = loc;
        let detail = detail_of(&detect_one(f), Smell::LargeMethod);
        assert!(detail.contains(&format!("[{expected}]")), "loc={loc} should label [{expected}]: {detail}");
    }
}

#[test]
fn god_class_requires_strictly_more_functions_than_the_ceiling() {
    let th = t();
    let mut god = fm("titan");
    god.cc = th.function.cc_alert + 5;
    god.cognitive_complexity = th.function.cogc_alert + 5;
    god.loc = th.function.fn_loc_alert + 5;
    god.end_line = god.loc;
    for (count, expected) in [(th.module.file_function_count, false), (th.module.file_function_count + 1, true)] {
        let mut m = module();
        m.total_loc = th.module.file_loc_warning;
        m.total_functions = count;
        let findings = detect(&FileMetrics { functions: vec![god.clone()], module: m }, "", &th);
        let has = findings.iter().any(|f| f.smell == Smell::GodClass);
        assert_eq!(has, expected, "fn count {count} god-class expectation failed: {findings:?}");
    }
}

fn cohesion_methods(n: usize) -> Vec<FunctionMetrics> {
    (0..n)
        .map(|i| {
            let mut f = fm(&format!("Svc.m{i}"));
            f.class_name = Some("Svc".to_string());
            f.field_accesses = vec![format!("field_{i}")];
            f
        })
        .collect()
}

#[test]
fn lcom4_needs_at_least_three_methods_and_fires_at_three_components() {
    let th = t();
    let two = detect(&FileMetrics { functions: cohesion_methods(2), module: module() }, "", &th);
    assert!(!two.iter().any(|f| f.smell == Smell::LowCohesion), "two methods never trip lcom4: {two:?}");
    let three = detect(&FileMetrics { functions: cohesion_methods(3), module: module() }, "", &th);
    assert!(three.iter().any(|f| f.smell == Smell::LowCohesion), "three disconnected methods must trip: {three:?}");
}

#[test]
fn complexity_intensity_scores_each_satisfied_band() {
    let th = t();
    let mut by_cc = fm("a");
    by_cc.cc = th.function.cc_warning + 1;
    let expected = normalize_exceedance(f64::from(by_cc.cc), f64::from(th.function.cc_warning), corpus_tail("cc"));
    assert!(expected > 0.0);
    assert!((for_finding(Smell::ComplexMethod, &by_cc, RUST, &th) - expected).abs() < 1e-9);

    let mut by_cogc = fm("b");
    by_cogc.cognitive_complexity = th.function.cogc_warning + 3;
    let expected = normalize_exceedance(
        f64::from(by_cogc.cognitive_complexity),
        f64::from(th.function.cogc_warning),
        corpus_tail("cogc"),
    );
    assert!(expected > 0.0);
    assert!((for_finding(Smell::ComplexMethod, &by_cogc, RUST, &th) - expected).abs() < 1e-9);

    let mut by_loc = fm("c");
    by_loc.loc = th.function.fn_loc_warning + 7;
    let expected =
        normalize_exceedance(f64::from(by_loc.loc), f64::from(th.function.fn_loc_warning), corpus_tail("fn_loc"));
    assert!(expected > 0.0);
    assert!((for_finding(Smell::LargeMethod, &by_loc, RUST, &th) - expected).abs() < 1e-9);
}

type SetMetric = fn(&mut FunctionMetrics, u32);

#[test]
fn structural_intensity_normalizes_against_the_corpus_tail() {
    let th = t();
    let cases: Vec<(Smell, u32, &str, SetMetric)> = vec![
        (Smell::DeepNestedComplexity, th.function.nesting_depth, "nesting", |f, v| f.max_nesting = v),
        (Smell::ComplexConditional, th.function.compound_conditions, "compound_conditions", |f, v| {
            f.compound_condition_count = v;
        }),
        (Smell::ExcessArguments, th.function.arg_max, "args", |f, v| f.arg_count = v),
        (Smell::ConstructorOverInjection, th.function.constructor_arg_max, "args", |f, v| f.arg_count = v),
        (Smell::LargeEmbeddedBlock, th.function.embedded_block_loc, "embedded_block_loc", |f, v| {
            f.max_embedded_block_loc = v;
        }),
        (Smell::NestedConditionalChunks, th.function.bump_count, "bump", |f, v| f.bump_count = v),
    ];
    for (smell, floor, metric, set) in cases {
        let mut f = fm("s");
        let value = floor + 1;
        set(&mut f, value);
        let expected = normalize_exceedance(f64::from(value), f64::from(floor), corpus_tail(metric));
        let got = for_finding(smell, &f, RUST, &th);
        assert!((got - expected).abs() < 1e-9, "{smell:?}: got {got}, expected {expected}");
    }
}

#[test]
fn rank_findings_orders_by_descending_intensity() {
    let th = t();
    let mut mild = fm("mild");
    mild.cc = th.function.cc_warning + 1;
    let mut severe = fm("severe");
    severe.cc = th.function.cc_alert;
    severe.start_line = 10;
    severe.end_line = 11;
    let findings = vec![
        Finding {
            smell: Smell::ComplexMethod,
            location: Location::Function { name: "mild".into(), start_line: 1, end_line: 2 },
            detail: String::new(),
        },
        Finding {
            smell: Smell::ComplexMethod,
            location: Location::Function { name: "severe".into(), start_line: 10, end_line: 11 },
            detail: String::new(),
        },
    ];
    let metrics = FileMetrics { functions: vec![mild, severe], module: module() };
    let ranked = rank_findings(&findings, &metrics, RUST, &th);
    let first = match &ranked[0].location {
        Location::Function { name, .. } => name.clone(),
        Location::Module => String::new(),
    };
    assert_eq!(first, "severe", "higher-intensity finding must rank first");
}

#[test]
fn every_smell_is_registered_across_all_tables() {
    for &smell in pulse::smells::ALL_SMELLS {
        assert!(!pulse::output::action_for(smell, "").is_empty(), "{smell:?} lacks an action");
        assert!(!smell.as_str().is_empty(), "{smell:?} lacks a display name");
        assert_eq!(
            pulse::smells::smell_from_snake_case(smell.snake_name()),
            Some(smell),
            "{smell:?} snake name must round-trip"
        );
    }
}
