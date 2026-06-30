use std::path::PathBuf;

use pulse_audit::call_graph::{CallGraph, MethodIdentity};
use pulse_audit::class_registry::ClassRegistry;
use pulse_audit::definitions::DefinitionRecord;
use pulse_audit::detector_god_class::build_method_idx_lookup;
use pulse_audit::finding::AuditKind;
use pulse_audit::mcd;
use pulse_thresholds::AuditThresholds;

use crate::audit_common::t;

fn def(
    file: &str,
    class: &str,
    name: &str,
    line: u32,
    cc: u32,
    fields: Vec<&str>,
    foreign: Vec<(&str, &str)>,
) -> DefinitionRecord {
    DefinitionRecord {
        identity: MethodIdentity {
            file: PathBuf::from(file),
            class: Some(class.to_string()),
            name: name.to_string(),
            line,
        },
        cc,
        field_accesses: fields.into_iter().map(String::from).collect(),
        foreign_field_accesses: foreign.into_iter().map(|(r, f)| (r.to_string(), f.to_string())).collect(),
        parent_class: None,
        is_constructor: false,
    }
}

fn inlier_class(file: &str, class: &str, offset: u32, seed: u32) -> Vec<DefinitionRecord> {
    let cc_a = 2 + seed % 5;
    let cc_b = 2 + (seed + 1) % 5;
    let cc_c = 2 + (seed + 2) % 5;
    let foreign: Vec<(&str, &str)> = match seed % 4 {
        0 => vec![],
        1 => vec![("Helper", "x")],
        2 => vec![("Util", "y"), ("Util", "z")],
        _ => vec![("Dep", "a"), ("Dep", "b"), ("Dep", "c")],
    };
    match seed % 3 {
        0 => vec![
            def(file, class, "alpha", offset, cc_a, vec!["a", "b"], foreign.clone()),
            def(file, class, "beta", offset + 1, cc_b, vec!["b", "c"], vec![]),
            def(file, class, "gamma", offset + 2, cc_c, vec!["a", "c"], vec![]),
        ],
        1 => vec![
            def(file, class, "alpha", offset, cc_a, vec!["x", "y"], foreign.clone()),
            def(file, class, "beta", offset + 1, cc_b, vec!["p", "q"], vec![]),
            def(file, class, "gamma", offset + 2, cc_c, vec!["p", "r"], vec![]),
        ],
        _ => vec![
            def(file, class, "alpha", offset, cc_a, vec!["a1", "a2"], foreign.clone()),
            def(file, class, "beta", offset + 1, cc_b, vec!["b1", "b2"], vec![]),
            def(file, class, "gamma", offset + 2, cc_c, vec!["c1", "c2"], vec![]),
        ],
    }
}

fn detect_with(defs: Vec<DefinitionRecord>, at: &AuditThresholds) -> Vec<pulse_audit::finding::AuditFinding> {
    let graph = CallGraph::build(defs.clone(), Vec::new());
    let registry = ClassRegistry::from_definitions(&defs, &graph.registry);
    let lookup = build_method_idx_lookup(&graph, &defs);
    mcd::detect(&registry, &defs, &lookup, at)
}

#[test]
fn below_min_classes_produces_no_findings() {
    let mut defs = Vec::new();
    for i in 0..5u32 {
        defs.extend(inlier_class(&format!("f{i}.py"), &format!("C{i}"), i * 10, i));
    }
    let findings = detect_with(defs, &t().audit);
    assert!(findings.is_empty(), "fewer than min_classes should produce no findings");
}

#[test]
fn disabled_produces_no_findings() {
    let mut at = t().audit;
    at.named_smells.multivariate_anomaly.enabled = false;

    let mut defs = Vec::new();
    for i in 0..12u32 {
        defs.extend(inlier_class(&format!("f{i}.py"), &format!("C{i}"), i * 10, i));
    }
    let findings = detect_with(defs, &at);
    assert!(findings.is_empty(), "disabled detector must emit nothing");
}

#[test]
fn clear_outlier_is_flagged_among_inlier_classes() {
    let mut defs = Vec::new();
    for i in 0..10u32 {
        defs.extend(inlier_class(&format!("f{i}.py"), &format!("Normal{i}"), i * 10, i));
    }
    let foreign_heavy: Vec<(&str, &str)> = (0..40)
        .map(|j| {
            (
                if j % 4 == 0 {
                    "E1"
                } else if j % 4 == 1 {
                    "E2"
                } else if j % 4 == 2 {
                    "E3"
                } else {
                    "E4"
                },
                "v",
            )
        })
        .collect();
    for k in 0u32..8 {
        defs.push(def(
            "outlier.py",
            "Outlier",
            &format!("op{k}"),
            200 + k,
            50,
            vec![&format!("own{k}")],
            foreign_heavy.clone(),
        ));
    }
    let findings = detect_with(defs, &t().audit);
    assert!(!findings.is_empty(), "extreme outlier must produce at least one finding");
    let has_outlier = findings.iter().any(|f| {
        if let AuditKind::MultivariateAnomaly(e) = &f.kind {
            e.class_name == "Outlier"
        } else {
            false
        }
    });
    assert!(has_outlier, "outlier class must appear in findings");
}

#[test]
fn max_findings_cap_is_respected() {
    let mut at = t().audit;
    at.named_smells.multivariate_anomaly.max_findings = 3;
    at.named_smells.multivariate_anomaly.min_classes = 5;

    let mut defs = Vec::new();
    for i in 0..6u32 {
        defs.extend(inlier_class(&format!("f{i}.py"), &format!("Normal{i}"), i * 10, i));
    }
    let foreign: Vec<(&str, &str)> = (0..15).map(|_| ("Ext", "x")).collect();
    for k in 0u32..8 {
        for m in 0u32..4 {
            defs.push(def(
                &format!("out{k}.py"),
                &format!("Out{k}"),
                &format!("m{m}"),
                k * 100 + m,
                25 + k,
                vec![&format!("f{m}")],
                foreign.clone(),
            ));
        }
    }
    let findings = detect_with(defs, &at);
    assert!(findings.len() <= 3, "max_findings cap must be respected: got {}", findings.len());
}

#[test]
fn inlier_only_population_produces_no_or_few_findings() {
    let mut defs = Vec::new();
    for i in 0..16u32 {
        let cc_a = 2 + i % 5;
        let cc_b = 2 + (i + 1) % 5;
        let cc_c = 2 + (i + 2) % 5;
        let foreign: Vec<(&str, &str)> = match i % 4 {
            0 => vec![],
            1 => vec![("Helper", "x")],
            2 => vec![("Util", "y"), ("Util", "z")],
            _ => vec![("Dep", "a")],
        };
        match i % 4 {
            0 => defs.extend(vec![
                def(&format!("f{i}.py"), &format!("Clean{i}"), "alpha", i * 10, cc_a, vec!["a", "b"], foreign.clone()),
                def(&format!("f{i}.py"), &format!("Clean{i}"), "beta", i * 10 + 1, cc_b, vec!["b", "c"], vec![]),
                def(&format!("f{i}.py"), &format!("Clean{i}"), "gamma", i * 10 + 2, cc_c, vec!["a", "c"], vec![]),
            ]),
            1 => defs.extend(vec![
                def(&format!("f{i}.py"), &format!("Clean{i}"), "alpha", i * 10, cc_a, vec!["a", "b"], foreign.clone()),
                def(&format!("f{i}.py"), &format!("Clean{i}"), "beta", i * 10 + 1, cc_b, vec!["b", "c"], vec![]),
                def(&format!("f{i}.py"), &format!("Clean{i}"), "gamma", i * 10 + 2, cc_c, vec!["c", "d"], vec![]),
            ]),
            2 => defs.extend(vec![
                def(&format!("f{i}.py"), &format!("Clean{i}"), "alpha", i * 10, cc_a, vec!["x", "y"], foreign.clone()),
                def(&format!("f{i}.py"), &format!("Clean{i}"), "beta", i * 10 + 1, cc_b, vec!["p", "q"], vec![]),
                def(&format!("f{i}.py"), &format!("Clean{i}"), "gamma", i * 10 + 2, cc_c, vec!["p", "r"], vec![]),
            ]),
            _ => defs.extend(vec![
                def(
                    &format!("f{i}.py"),
                    &format!("Clean{i}"),
                    "alpha",
                    i * 10,
                    cc_a,
                    vec!["a1", "a2"],
                    foreign.clone(),
                ),
                def(&format!("f{i}.py"), &format!("Clean{i}"), "beta", i * 10 + 1, cc_b, vec!["b1", "b2"], vec![]),
                def(&format!("f{i}.py"), &format!("Clean{i}"), "gamma", i * 10 + 2, cc_c, vec!["c1", "c2"], vec![]),
            ]),
        }
    }
    let findings = detect_with(defs, &t().audit);
    let flagged: Vec<_> = findings.iter().filter(|f| matches!(f.kind, AuditKind::MultivariateAnomaly(_))).collect();
    assert!(flagged.len() <= 4, "near-uniform inlier population must not flag more than 4 classes: {flagged:?}");
}

#[test]
fn finding_carries_correct_class_metadata() {
    let mut defs = Vec::new();
    for i in 0..10u32 {
        defs.extend(inlier_class(&format!("f{i}.py"), &format!("N{i}"), i * 10, i));
    }
    let foreign: Vec<(&str, &str)> = (0..35)
        .map(|j| {
            (
                if j % 3 == 0 {
                    "A"
                } else if j % 3 == 1 {
                    "B"
                } else {
                    "C"
                },
                "v",
            )
        })
        .collect();
    for m in 0u32..6 {
        defs.push(def(
            "monster.py",
            "Monster",
            &format!("act{m}"),
            500 + m,
            45,
            vec![&format!("own{m}")],
            foreign.clone(),
        ));
    }
    let findings = detect_with(defs, &t().audit);
    let finding =
        findings.iter().find(|f| matches!(&f.kind, AuditKind::MultivariateAnomaly(e) if e.class_name == "Monster"));
    assert!(finding.is_some(), "Monster class must be flagged");
    if let Some(f) = finding {
        if let AuditKind::MultivariateAnomaly(e) = &f.kind {
            assert_eq!(e.class_file, PathBuf::from("monster.py"));
            assert!(
                e.mahalanobis_sq > t().audit.named_smells.multivariate_anomaly.distance_quantile,
                "distance² must exceed threshold: {}",
                e.mahalanobis_sq
            );
        }
    }
}
