use std::path::PathBuf;
use std::time::Instant;

use pulse::audit::call_graph::{CallGraph, MethodIdentity};
use pulse::audit::call_walker::LocatedCall;
use pulse::audit::calls::RawCall;
use pulse::audit::class_registry::ClassRegistry;
use pulse::audit::definitions::DefinitionRecord;
use pulse::audit::detector_divergent_change as dc;
use pulse::audit::detector_feature_envy as fe;
use pulse::audit::detector_god_class as gc;
use pulse::audit::detector_parallel_inheritance as pi;
use pulse::audit::detector_refused_bequest as rb;
use pulse::audit::inheritance::build_inheritance_graph;

use crate::audit_common::t;

#[allow(clippy::too_many_arguments)]
fn def_full(
    file: &str,
    class: Option<&str>,
    name: &str,
    line: u32,
    cc: u32,
    fields: Vec<&str>,
    foreign: Vec<(&str, &str)>,
    parent: Option<&str>,
    is_ctor: bool,
) -> DefinitionRecord {
    DefinitionRecord {
        identity: MethodIdentity {
            file: PathBuf::from(file),
            class: class.map(String::from),
            name: name.to_string(),
            line,
        },
        cc,
        field_accesses: fields.into_iter().map(String::from).collect(),
        foreign_field_accesses: foreign.into_iter().map(|(r, f)| (r.to_string(), f.to_string())).collect(),
        parent_class: parent.map(String::from),
        is_constructor: is_ctor,
    }
}

fn empty_call(caller: &DefinitionRecord, callee_name: &str, hint: Option<&str>) -> LocatedCall {
    LocatedCall {
        call: RawCall {
            callee_name: callee_name.to_string(),
            receiver_hint: hint.map(String::from),
            line: caller.identity.line + 1,
        },
        caller: Some(caller.identity.clone()),
        file: caller.identity.file.clone(),
    }
}

#[test]
fn ten_thousand_methods_call_graph_build_completes() {
    let mut defs = Vec::new();
    for i in 0..10_000 {
        defs.push(def_full(
            &format!("f_{}.py", i / 100),
            Some(&format!("C{}", i / 100)),
            &format!("m{}", i % 100),
            (i % 1000) as u32 + 1,
            1,
            vec![],
            vec![],
            None,
            false,
        ));
    }
    let started = Instant::now();
    let graph = CallGraph::build(defs.clone(), Vec::new());
    assert!(started.elapsed().as_secs_f64() < 5.0);
    assert_eq!(graph.registry.methods.len(), 10_000);
}

#[test]
fn one_thousand_class_registry_built_quickly() {
    let mut defs = Vec::new();
    for c in 0..1_000 {
        let class_name = format!("Cls{c}");
        let file = format!("c_{c}.py");
        for m in 0..5 {
            defs.push(def_full(
                &file,
                Some(&class_name),
                &format!("m{m}"),
                m as u32 + 1,
                1,
                vec![],
                vec![],
                None,
                false,
            ));
        }
    }
    let graph = CallGraph::build(defs.clone(), Vec::new());
    let started = Instant::now();
    let registry = ClassRegistry::from_definitions(&defs, &graph.registry);
    assert!(started.elapsed().as_secs_f64() < 5.0);
    assert_eq!(registry.count(), 1_000);
}

#[test]
fn deep_inheritance_chain_50_levels_descendants_terminate() {
    let mut defs = Vec::new();
    defs.push(def_full("c0.py", Some("C0"), "m", 1, 1, vec![], vec![], None, false));
    for i in 1..50 {
        defs.push(def_full(
            &format!("c{i}.py"),
            Some(&format!("C{i}")),
            "m",
            1,
            1,
            vec![],
            vec![],
            Some(&format!("C{}", i - 1)),
            false,
        ));
    }
    let graph = CallGraph::build(defs.clone(), Vec::new());
    let registry = ClassRegistry::from_definitions(&defs, &graph.registry);
    let started = Instant::now();
    let _ = build_inheritance_graph(&registry);
    assert!(started.elapsed().as_secs_f64() < 1.0);
}

#[test]
fn wide_hierarchy_500_siblings_completes() {
    let mut defs = Vec::new();
    defs.push(def_full("root.py", Some("Root"), "m", 1, 1, vec![], vec![], None, false));
    for i in 0..500 {
        defs.push(def_full(
            &format!("c{i}.py"),
            Some(&format!("C{i}")),
            "m",
            1,
            1,
            vec![],
            vec![],
            Some("Root"),
            false,
        ));
    }
    let graph = CallGraph::build(defs.clone(), Vec::new());
    let registry = ClassRegistry::from_definitions(&defs, &graph.registry);
    let inh = build_inheritance_graph(&registry);
    let started = Instant::now();
    let _ = pi::detect(&registry, &inh, &t().audit);
    assert!(started.elapsed().as_secs_f64() < 5.0);
}

#[test]
fn diamond_inheritance_5_levels_handled() {
    let mut defs = Vec::new();
    defs.push(def_full("top.py", Some("Top"), "m", 1, 1, vec![], vec![], None, false));
    for level in 0..5 {
        for branch in 0..(1 << level) {
            let parent = if level == 0 { "Top".to_string() } else { format!("L{}_B{}", level - 1, branch / 2) };
            let cls = format!("L{level}_B{branch}");
            defs.push(def_full(&format!("{cls}.py"), Some(&cls), "m", 1, 1, vec![], vec![], Some(&parent), false));
        }
    }
    let graph = CallGraph::build(defs.clone(), Vec::new());
    let registry = ClassRegistry::from_definitions(&defs, &graph.registry);
    let inh = build_inheritance_graph(&registry);
    let started = Instant::now();
    let _ = pi::detect(&registry, &inh, &t().audit);
    assert!(started.elapsed().as_secs_f64() < 5.0);
}

#[test]
fn self_cycle_inheritance_does_not_loop_forever() {
    let defs = vec![def_full("a.py", Some("A"), "m", 1, 1, vec![], vec![], Some("A"), false)];
    let graph = CallGraph::build(defs.clone(), Vec::new());
    let registry = ClassRegistry::from_definitions(&defs, &graph.registry);
    let started = Instant::now();
    let _ = build_inheritance_graph(&registry);
    assert!(started.elapsed().as_secs_f64() < 1.0);
}

#[test]
fn three_cycle_does_not_loop_forever() {
    let defs = vec![
        def_full("a.py", Some("A"), "m", 1, 1, vec![], vec![], Some("B"), false),
        def_full("b.py", Some("B"), "m", 1, 1, vec![], vec![], Some("C"), false),
        def_full("c.py", Some("C"), "m", 1, 1, vec![], vec![], Some("A"), false),
    ];
    let graph = CallGraph::build(defs.clone(), Vec::new());
    let registry = ClassRegistry::from_definitions(&defs, &graph.registry);
    let started = Instant::now();
    let _ = build_inheritance_graph(&registry);
    assert!(started.elapsed().as_secs_f64() < 1.0);
}

#[test]
fn determinism_property_call_graph_build_is_idempotent() {
    let defs: Vec<_> = (0..20)
        .map(|i| def_full(&format!("f_{i}.py"), Some(&format!("C{i}")), "m", 1, 1, vec![], vec![], None, false))
        .collect();
    let calls: Vec<LocatedCall> = (1..20).map(|i| empty_call(&defs[i - 1], "m", Some(&format!("C{i}")))).collect();
    let g1 = CallGraph::build(defs.clone(), calls.clone());
    let g2 = CallGraph::build(defs.clone(), calls);
    assert_eq!(g1.registry.methods.len(), g2.registry.methods.len());
    for i in 0..g1.registry.methods.len() {
        assert_eq!(g1.registry.methods[i], g2.registry.methods[i]);
    }
}

#[test]
fn registry_built_from_same_defs_yields_same_class_count() {
    let defs: Vec<_> = (0..50)
        .map(|i| def_full(&format!("f_{i}.py"), Some(&format!("C{i}")), "m", 1, 1, vec![], vec![], None, false))
        .collect();
    let g1 = CallGraph::build(defs.clone(), Vec::new());
    let r1 = ClassRegistry::from_definitions(&defs, &g1.registry);
    let g2 = CallGraph::build(defs.clone(), Vec::new());
    let r2 = ClassRegistry::from_definitions(&defs, &g2.registry);
    assert_eq!(r1.count(), r2.count());
}

#[test]
fn lowering_threshold_monotonically_increases_findings_divergent() {
    let mut defs = Vec::new();
    let mut calls = Vec::new();
    for m in 0..6 {
        defs.push(def_full("c.py", Some("C"), &format!("m{m}"), 10 + m, 1, vec![], vec![], None, false));
    }
    for caller_idx in 0..7 {
        let caller = def_full(
            &format!("ca_{caller_idx}.py"),
            Some(&format!("Ca{caller_idx}")),
            "t",
            1,
            1,
            vec![],
            vec![],
            None,
            false,
        );
        defs.push(caller.clone());
        calls.push(empty_call(&caller, "m0", Some("C")));
    }
    for dep_idx in 0..7 {
        defs.push(def_full(
            &format!("d_{dep_idx}.py"),
            Some(&format!("Dep{dep_idx}")),
            "do",
            1,
            1,
            vec![],
            vec![],
            None,
            false,
        ));
        let target = def_full("c.py", Some("C"), "m0", 10, 1, vec![], vec![], None, false);
        calls.push(empty_call(&target, "do", Some(&format!("Dep{dep_idx}"))));
    }
    let g = CallGraph::build(defs.clone(), calls);
    let r = ClassRegistry::from_definitions(&defs, &g.registry);
    let baseline = dc::detect(&r, &g, &t().audit).len();
    let mut lowered = t().audit;
    lowered.named_smells.divergent_change.changing_classes = 0;
    lowered.named_smells.divergent_change.fanout = 0;
    lowered.named_smells.divergent_change.method_count = 0;
    let lowered_count = dc::detect(&r, &g, &lowered).len();
    assert!(lowered_count >= baseline);
}

#[test]
fn raising_threshold_monotonically_decreases_findings_feature_envy() {
    let foreign = vec![("b", "f1"), ("b", "f2"), ("b", "f3"), ("b", "f4"), ("b", "f5"), ("b", "f6")];
    let m = def_full("a.py", Some("A"), "m", 1, 1, vec![], foreign, None, false);
    let other = def_full("b.py", Some("B"), "do", 1, 1, vec![], vec![], None, false);
    let calls: Vec<LocatedCall> = (0..4).map(|_| empty_call(&m, "do", Some("B"))).collect();
    let defs = vec![m, other];
    let g = CallGraph::build(defs.clone(), calls);
    let baseline = fe::detect(&defs, &g, &t().audit).len();
    let mut raised = t().audit;
    raised.named_smells.feature_envy.atfd = 1000;
    let raised_count = fe::detect(&defs, &g, &raised).len();
    assert!(raised_count <= baseline);
}

#[test]
fn empty_inputs_never_panic_across_all_detectors() {
    let g = CallGraph::build(Vec::new(), Vec::new());
    let r = ClassRegistry::from_definitions(&[], &g.registry);
    let inh = build_inheritance_graph(&r);
    let _ = dc::detect(&r, &g, &t().audit);
    let _ = fe::detect(&[], &g, &t().audit);
    let _ = gc::detect(&r, &[], &gc::build_method_idx_lookup(&g, &[]), &t().audit);
    let _ = pi::detect(&r, &inh, &t().audit);
    let _ = rb::detect(&r, &[], &|_: &std::path::Path| -> Option<f64> { None }, &t().audit);
}

#[test]
fn unicode_identifiers_round_trip_through_all_detectors() {
    let defs = vec![
        def_full("κ.py", Some("Κλάσση"), "μ", 1, 1, vec![], vec![], None, false),
        def_full("κ.py", Some("Κλάσση"), "ν", 5, 1, vec![], vec![], None, false),
        def_full("κ.py", Some("Κλάσση"), "ξ", 10, 1, vec![], vec![], None, false),
    ];
    let g = CallGraph::build(defs.clone(), Vec::new());
    let r = ClassRegistry::from_definitions(&defs, &g.registry);
    let inh = build_inheritance_graph(&r);
    let _ = dc::detect(&r, &g, &t().audit);
    let _ = fe::detect(&defs, &g, &t().audit);
    let _ = gc::detect(&r, &defs, &gc::build_method_idx_lookup(&g, &defs), &t().audit);
    let _ = pi::detect(&r, &inh, &t().audit);
    let _ = rb::detect(&r, &defs, &|_: &std::path::Path| -> Option<f64> { None }, &t().audit);
}

#[test]
fn five_run_determinism_each_detector_yields_same_count() {
    let defs = vec![
        def_full(
            "g.py",
            Some("G"),
            "m1",
            10,
            10,
            vec!["f0"],
            vec![("a", "x"), ("b", "y"), ("c", "z"), ("d", "w"), ("e", "v"), ("f", "u")],
            None,
            false,
        ),
        def_full(
            "g.py",
            Some("G"),
            "m2",
            11,
            10,
            vec!["f1"],
            vec![("a", "x"), ("b", "y"), ("c", "z"), ("d", "w"), ("e", "v"), ("f", "u")],
            None,
            false,
        ),
        def_full(
            "g.py",
            Some("G"),
            "m3",
            12,
            10,
            vec!["f2"],
            vec![("a", "x"), ("b", "y"), ("c", "z"), ("d", "w"), ("e", "v"), ("f", "u")],
            None,
            false,
        ),
        def_full(
            "g.py",
            Some("G"),
            "m4",
            13,
            10,
            vec!["f3"],
            vec![("a", "x"), ("b", "y"), ("c", "z"), ("d", "w"), ("e", "v"), ("f", "u")],
            None,
            false,
        ),
        def_full(
            "g.py",
            Some("G"),
            "m5",
            14,
            10,
            vec!["f4"],
            vec![("a", "x"), ("b", "y"), ("c", "z"), ("d", "w"), ("e", "v"), ("f", "u")],
            None,
            false,
        ),
        def_full(
            "g.py",
            Some("G"),
            "m6",
            15,
            10,
            vec!["f5"],
            vec![("a", "x"), ("b", "y"), ("c", "z"), ("d", "w"), ("e", "v"), ("f", "u")],
            None,
            false,
        ),
    ];
    let mut counts = Vec::new();
    for _ in 0..5 {
        let g = CallGraph::build(defs.clone(), Vec::new());
        let r = ClassRegistry::from_definitions(&defs, &g.registry);
        let lookup = gc::build_method_idx_lookup(&g, &defs);
        counts.push(gc::detect(&r, &defs, &lookup, &t().audit).len());
    }
    for c in &counts[1..] {
        assert_eq!(*c, counts[0]);
    }
}

#[test]
fn malformed_inputs_never_panic() {
    let weird_paths = ["/", "..", "../../etc/passwd", "a\nb", "::"];
    for path in weird_paths {
        let defs = vec![
            def_full(path, Some("C"), "m1", 1, 1, vec![], vec![], None, false),
            def_full(path, Some("C"), "m2", 5, 1, vec![], vec![], None, false),
            def_full(path, Some("C"), "m3", 10, 1, vec![], vec![], None, false),
        ];
        let g = CallGraph::build(defs.clone(), Vec::new());
        let r = ClassRegistry::from_definitions(&defs, &g.registry);
        let _ = dc::detect(&r, &g, &t().audit);
    }
}

#[test]
fn one_hundred_god_classes_all_emit() {
    let mut defs = Vec::new();
    for c in 0..100 {
        let class_name = format!("God{c}");
        let file = format!("g_{c}.py");
        for m in 0..10 {
            defs.push(def_full(
                &file,
                Some(&class_name),
                &format!("m{m}"),
                m as u32 + 10,
                10,
                vec![&format!("f{m}")],
                vec![("a", "x"), ("b", "y"), ("c", "z"), ("d", "w"), ("e", "v"), ("f", "u")],
                None,
                false,
            ));
        }
    }
    let g = CallGraph::build(defs.clone(), Vec::new());
    let r = ClassRegistry::from_definitions(&defs, &g.registry);
    let lookup = gc::build_method_idx_lookup(&g, &defs);
    let started = Instant::now();
    let findings = gc::detect(&r, &defs, &lookup, &t().audit);
    assert!(started.elapsed().as_secs_f64() < 5.0);
    assert!(findings.len() >= 100);
}

#[test]
fn confidence_aggregation_across_mixed_edges() {
    use pulse::audit::finding::ImportConfidence;
    assert_eq!(ImportConfidence::High.min(ImportConfidence::Low), ImportConfidence::Low);
    assert_eq!(ImportConfidence::Medium.min(ImportConfidence::BestEffort), ImportConfidence::BestEffort);
    assert_eq!(ImportConfidence::High.min(ImportConfidence::NaAbstraction), ImportConfidence::NaAbstraction);
}

#[test]
fn property_findings_are_sorted_total_order_god_class() {
    use pulse::audit::finding::AuditKind;
    let mut defs = Vec::new();
    for c in 0..30 {
        let class_name = format!("G{c}");
        let file = format!("g_{c}.py");
        let cc = (c % 5 + 5) as u32;
        for m in 0..10 {
            defs.push(def_full(
                &file,
                Some(&class_name),
                &format!("m{m}"),
                m + 10,
                cc,
                vec![&format!("f{m}")],
                vec![("a", "x"), ("b", "y"), ("c", "z"), ("d", "w"), ("e", "v"), ("f", "u")],
                None,
                false,
            ));
        }
    }
    let g = CallGraph::build(defs.clone(), Vec::new());
    let r = ClassRegistry::from_definitions(&defs, &g.registry);
    let lookup = gc::build_method_idx_lookup(&g, &defs);
    let findings = gc::detect(&r, &defs, &lookup, &t().audit);
    for w in findings.windows(2) {
        let wmc_a = if let AuditKind::GodClass(e) = &w[0].kind { e.wmc } else { 0 };
        let wmc_b = if let AuditKind::GodClass(e) = &w[1].kind { e.wmc } else { 0 };
        assert!(wmc_a >= wmc_b);
    }
}

#[test]
fn property_no_negative_metrics_in_findings() {
    use pulse::audit::finding::AuditKind;
    let mut defs = Vec::new();
    for c in 0..3 {
        for m in 0..10 {
            defs.push(def_full(
                &format!("g_{c}.py"),
                Some(&format!("G{c}")),
                &format!("m{m}"),
                m + 10,
                10,
                vec![&format!("f{m}")],
                vec![("a", "x"), ("b", "y"), ("c", "z"), ("d", "w"), ("e", "v"), ("f", "u")],
                None,
                false,
            ));
        }
    }
    let g = CallGraph::build(defs.clone(), Vec::new());
    let r = ClassRegistry::from_definitions(&defs, &g.registry);
    let lookup = gc::build_method_idx_lookup(&g, &defs);
    let findings = gc::detect(&r, &defs, &lookup, &t().audit);
    for f in &findings {
        if let AuditKind::GodClass(e) = &f.kind {
            assert!(e.wmc > 0);
            assert!(e.method_count > 0);
            assert!(e.tcc >= 0.0 && e.tcc <= 1.0);
        }
    }
}
