use std::path::PathBuf;

use pulse::audit::finding::{ImportConfidence, MartinTier};
use pulse::audit::graph::{ImportGraph, InputEdge, NodeIndex};
use pulse::audit::martin::{classify, compute, distance, instability, AbstractnessRecord};
use pulse::parse::Language;
use pulse::thresholds::Thresholds;

fn t() -> Thresholds {
    Thresholds::default()
}

fn p(s: &str) -> PathBuf {
    PathBuf::from(s)
}

fn edge(src: &str, dst: &str) -> InputEdge {
    InputEdge {
        source: p(src),
        target: p(dst),
        source_lang: Language::Rust,
        target_lang: Language::Rust,
    }
}

fn build(edges: &[InputEdge]) -> ImportGraph {
    ImportGraph::build(edges)
}

fn pure_abstract() -> AbstractnessRecord {
    AbstractnessRecord { abstractness: 1.0, confidence: ImportConfidence::High }
}

fn pure_concrete() -> AbstractnessRecord {
    AbstractnessRecord { abstractness: 0.0, confidence: ImportConfidence::High }
}

fn half_abstract() -> AbstractnessRecord {
    AbstractnessRecord { abstractness: 0.5, confidence: ImportConfidence::High }
}

#[test]
fn instability_is_zero_when_only_incoming() {
    assert_eq!(instability(5, 0), 0.0);
}

#[test]
fn instability_is_one_when_only_outgoing() {
    assert_eq!(instability(0, 5), 1.0);
}

#[test]
fn instability_is_zero_when_isolated() {
    assert_eq!(instability(0, 0), 0.0);
}

#[test]
fn instability_is_half_when_symmetric() {
    assert_eq!(instability(5, 5), 0.5);
}

#[test]
fn instability_is_f64_precise_for_three_quarters() {
    assert!((instability(1, 3) - 0.75).abs() < 1e-12);
}

#[test]
fn instability_handles_large_counts_without_overflow() {
    let big = u32::MAX / 2;
    let i = instability(big, big);
    assert!((i - 0.5).abs() < 1e-9);
}

#[test]
fn distance_at_a_zero_i_zero_is_one() {
    assert!((distance(0.0, 0.0) - 1.0).abs() < 1e-12);
}

#[test]
fn distance_at_a_one_i_one_is_one() {
    assert!((distance(1.0, 1.0) - 1.0).abs() < 1e-12);
}

#[test]
fn distance_at_a_zero_i_one_is_zero() {
    assert!(distance(0.0, 1.0).abs() < 1e-12);
}

#[test]
fn distance_at_a_one_i_zero_is_zero() {
    assert!(distance(1.0, 0.0).abs() < 1e-12);
}

#[test]
fn distance_at_main_sequence_a_half_i_half_is_zero() {
    assert!(distance(0.5, 0.5).abs() < 1e-12);
}

#[test]
fn distance_is_at_most_one() {
    for a in 0..=10 {
        for i in 0..=10 {
            let d = distance(f64::from(a) / 10.0, f64::from(i) / 10.0);
            assert!(d <= 1.0 + 1e-12);
        }
    }
}

#[test]
fn classify_below_warning_is_healthy() {
    let mut th = t().audit;
    th.package_metrics.martin_distance_warning = 0.7;
    th.package_metrics.martin_distance_alert = 0.85;
    assert_eq!(classify(0.5, &th), MartinTier::Healthy);
    assert_eq!(classify(0.69, &th), MartinTier::Healthy);
}

#[test]
fn classify_at_warning_threshold_is_warning() {
    let mut th = t().audit;
    th.package_metrics.martin_distance_warning = 0.7;
    th.package_metrics.martin_distance_alert = 0.85;
    assert_eq!(classify(0.7, &th), MartinTier::Warning);
}

#[test]
fn classify_at_alert_threshold_is_alert() {
    let mut th = t().audit;
    th.package_metrics.martin_distance_warning = 0.7;
    th.package_metrics.martin_distance_alert = 0.85;
    assert_eq!(classify(0.85, &th), MartinTier::Alert);
    assert_eq!(classify(0.95, &th), MartinTier::Alert);
}

#[test]
fn compute_for_isolated_module_yields_distance_one_with_concrete() {
    let g = build(&[edge("a.rs", "b.rs")]);
    let isolated = NodeIndex(0);
    let m = compute(&g, isolated, pure_concrete(), ImportConfidence::High, &t().audit);
    assert!((m.instability - 1.0).abs() < 1e-12);
    assert!(m.distance < 1e-12);
}

#[test]
fn compute_returns_zero_distance_for_balanced_abstract_unstable_pair() {
    let g = build(&[edge("a.rs", "b.rs"), edge("a.rs", "c.rs")]);
    let a = g.registry.lookup(&p("a.rs")).unwrap();
    let m = compute(&g, a, pure_concrete(), ImportConfidence::High, &t().audit);
    assert!((m.instability - 1.0).abs() < 1e-12);
    assert!(m.distance < 1e-12);
}

#[test]
fn compute_distance_for_stable_concrete_module_is_one() {
    let g = build(&[edge("dep1.rs", "core.rs"), edge("dep2.rs", "core.rs")]);
    let core = g.registry.lookup(&p("core.rs")).unwrap();
    let m = compute(&g, core, pure_concrete(), ImportConfidence::High, &t().audit);
    assert!((m.instability - 0.0).abs() < 1e-12);
    assert!((m.distance - 1.0).abs() < 1e-12);
}

#[test]
fn compute_records_module_path_correctly() {
    let g = build(&[edge("foo.rs", "bar.rs")]);
    let foo = g.registry.lookup(&p("foo.rs")).unwrap();
    let m = compute(&g, foo, pure_concrete(), ImportConfidence::High, &t().audit);
    assert_eq!(m.module, p("foo.rs"));
}

#[test]
fn compute_propagates_abstractness_value() {
    let g = build(&[edge("a.rs", "b.rs")]);
    let a = g.registry.lookup(&p("a.rs")).unwrap();
    let m = compute(&g, a, half_abstract(), ImportConfidence::High, &t().audit);
    assert!((m.abstractness - 0.5).abs() < 1e-12);
}

#[test]
fn compute_takes_min_of_import_and_abstractness_confidence() {
    let g = build(&[edge("a.rs", "b.rs")]);
    let a = g.registry.lookup(&p("a.rs")).unwrap();
    let abs_low = AbstractnessRecord { abstractness: 0.0, confidence: ImportConfidence::Low };
    let m = compute(&g, a, abs_low, ImportConfidence::High, &t().audit);
    assert_eq!(m.confidence, ImportConfidence::Low);
}

#[test]
fn compute_min_confidence_handles_na_abstraction_lower_than_low() {
    let g = build(&[edge("a.rs", "b.rs")]);
    let a = g.registry.lookup(&p("a.rs")).unwrap();
    let abs_na = AbstractnessRecord {
        abstractness: 0.0,
        confidence: ImportConfidence::NaAbstraction,
    };
    let m = compute(&g, a, abs_na, ImportConfidence::Low, &t().audit);
    assert_eq!(m.confidence, ImportConfidence::NaAbstraction);
}

#[test]
fn compute_classifies_pure_abstract_isolated_as_main_sequence() {
    let g = build(&[edge("z.rs", "a.rs")]);
    let a = g.registry.lookup(&p("a.rs")).unwrap();
    let m = compute(&g, a, pure_abstract(), ImportConfidence::High, &t().audit);
    assert_eq!(m.tier, MartinTier::Healthy);
    assert!(m.distance < 1e-12);
}

#[test]
fn compute_classifies_concrete_zero_couplings_as_alert() {
    let mut th = t().audit;
    th.package_metrics.martin_distance_alert = 0.85;
    let g = build(&[edge("a.rs", "b.rs"), edge("c.rs", "d.rs")]);
    let isolated = g.registry.lookup(&p("d.rs")).unwrap();
    let m = compute(&g, isolated, pure_concrete(), ImportConfidence::High, &th);
    assert_eq!(m.tier, MartinTier::Alert);
}

#[test]
fn compute_is_deterministic_across_calls() {
    let g = build(&[edge("a.rs", "b.rs"), edge("c.rs", "a.rs")]);
    let a = g.registry.lookup(&p("a.rs")).unwrap();
    let m1 = compute(&g, a, half_abstract(), ImportConfidence::High, &t().audit);
    let m2 = compute(&g, a, half_abstract(), ImportConfidence::High, &t().audit);
    assert_eq!(m1.instability, m2.instability);
    assert_eq!(m1.abstractness, m2.abstractness);
    assert_eq!(m1.distance, m2.distance);
}

#[test]
fn compute_uses_thresholds_from_argument_not_default() {
    let mut th = t().audit;
    th.package_metrics.martin_distance_warning = 0.99;
    th.package_metrics.martin_distance_alert = 0.999;
    let g = build(&[edge("a.rs", "b.rs"), edge("c.rs", "d.rs")]);
    let d = g.registry.lookup(&p("d.rs")).unwrap();
    let m = compute(&g, d, pure_concrete(), ImportConfidence::High, &th);
    assert_eq!(m.tier, MartinTier::Alert);
}

#[test]
fn classify_strict_inequality_at_warning_minus_epsilon_is_healthy() {
    let mut th = t().audit;
    th.package_metrics.martin_distance_warning = 0.7;
    th.package_metrics.martin_distance_alert = 0.85;
    assert_eq!(classify(0.6999999, &th), MartinTier::Healthy);
}

#[test]
fn classify_strict_inequality_at_alert_minus_epsilon_is_warning() {
    let mut th = t().audit;
    th.package_metrics.martin_distance_warning = 0.7;
    th.package_metrics.martin_distance_alert = 0.85;
    assert_eq!(classify(0.8499999, &th), MartinTier::Warning);
}

#[test]
fn instability_is_f64_not_f32_precision() {
    let i = instability(7, 13);
    let expected: f64 = 13.0 / 20.0;
    assert!((i - expected).abs() < 1e-15);
}

#[test]
fn distance_is_symmetric_under_a_i_swap() {
    for a in 0..=10 {
        for i in 0..=10 {
            let af = f64::from(a) / 10.0;
            let inf = f64::from(i) / 10.0;
            assert!((distance(af, inf) - distance(inf, af)).abs() < 1e-12);
        }
    }
}

#[test]
fn compute_for_pure_abstract_stable_classifies_main_sequence() {
    let mut edges: Vec<InputEdge> = Vec::new();
    for i in 0..5 {
        edges.push(edge(&format!("dep{i}.rs"), "abstract_core.rs"));
    }
    let g = build(&edges);
    let core = g.registry.lookup(&p("abstract_core.rs")).unwrap();
    let m = compute(&g, core, pure_abstract(), ImportConfidence::High, &t().audit);
    assert_eq!(m.tier, MartinTier::Healthy);
}

#[test]
fn compute_for_concrete_unstable_classifies_main_sequence() {
    let mut edges: Vec<InputEdge> = Vec::new();
    for i in 0..5 {
        edges.push(edge("hub.rs", &format!("dep{i}.rs")));
    }
    let g = build(&edges);
    let hub = g.registry.lookup(&p("hub.rs")).unwrap();
    let m = compute(&g, hub, pure_concrete(), ImportConfidence::High, &t().audit);
    assert_eq!(m.tier, MartinTier::Healthy);
}

#[test]
fn compute_for_concrete_stable_module_classifies_alert() {
    let mut th = t().audit;
    th.package_metrics.martin_distance_alert = 0.85;
    let mut edges: Vec<InputEdge> = Vec::new();
    for i in 0..5 {
        edges.push(edge(&format!("dep{i}.rs"), "rigid_core.rs"));
    }
    let g = build(&edges);
    let core = g.registry.lookup(&p("rigid_core.rs")).unwrap();
    let m = compute(&g, core, pure_concrete(), ImportConfidence::High, &th);
    assert_eq!(m.tier, MartinTier::Alert);
}

#[test]
fn compute_records_afferent_and_efferent_counts() {
    let g = build(&[
        edge("dep1.rs", "core.rs"),
        edge("dep2.rs", "core.rs"),
        edge("core.rs", "util.rs"),
    ]);
    let core = g.registry.lookup(&p("core.rs")).unwrap();
    let m = compute(&g, core, pure_concrete(), ImportConfidence::High, &t().audit);
    assert_eq!(m.afferent, 2);
    assert_eq!(m.efferent, 1);
}

#[test]
fn instability_with_max_minus_one_outgoing() {
    let i = instability(0, u32::MAX - 1);
    assert!((i - 1.0).abs() < 1e-12);
}

#[test]
fn instability_zero_zero_documented_choice() {
    assert_eq!(instability(0, 0), 0.0);
}

#[test]
fn distance_zero_zero_with_concrete_is_far_from_main() {
    let d = distance(0.0, 0.0);
    assert!(d > 0.85);
}
