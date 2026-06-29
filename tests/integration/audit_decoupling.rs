use std::path::PathBuf;

use pulse::audit::finding::{
    kind_pillar, AuditKind, AuditPillar, CompoundEvidence, GodClassEvidence, GodComponentEvidence, HubLikeEvidence,
    ImportConfidence, UnstableDepEvidence,
};

const ARCH_MODULES: &[&str] = &[
    "arch_smells.rs",
    "components.rs",
    "compound.rs",
    "centrality.rs",
    "cycle_shapes.rs",
    "cycles.rs",
    "martin.rs",
    "graph.rs",
    "package_metrics.rs",
    "community.rs",
    "remodularization.rs",
];

const CLASS_SMELL_SYMBOLS: &[&str] = &[
    "named_smells",
    "detector_god_class",
    "detector_feature_envy",
    "detector_divergent_change",
    "detector_parallel_inheritance",
    "detector_refused_bequest",
    "class_registry",
    "ClassRegistry",
    "GodClassEvidence",
    "FeatureEnvyEvidence",
    "DivergentChangeEvidence",
    "RefusedBequestEvidence",
    "ParallelInheritanceEvidence",
    "ShotgunSurgeryEvidence",
];

fn audit_source(file: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("crates").join("pulse-audit").join("src").join(file);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn medium() -> ImportConfidence {
    ImportConfidence::Medium
}

fn architecture_kinds() -> Vec<AuditKind> {
    let component = PathBuf::from("pkg/sub");
    vec![
        AuditKind::UnstableDependency(UnstableDepEvidence {
            component: component.clone(),
            instability: 0.4,
            strength: 0.5,
            gap: -0.2,
            unstable_deps: 2,
            total_deps: 4,
            centrality: 0.1,
            confidence: medium(),
        }),
        AuditKind::HubLikeDependency(HubLikeEvidence {
            component: component.clone(),
            afferent: 3,
            efferent: 3,
            imbalance: 0,
            centrality: 0.1,
            confidence: medium(),
        }),
        AuditKind::GodComponent(GodComponentEvidence {
            component: component.clone(),
            loc: 900,
            file_count: 1,
            density: 900.0,
            centrality: 0.1,
            confidence: medium(),
        }),
        AuditKind::CompoundArchSmell(CompoundEvidence {
            component,
            constituent_kinds: vec!["unstable_dependency", "hub_like_dependency"],
            combined_severity: 1.0,
            confidence: medium(),
        }),
    ]
}

fn god_class_kind() -> AuditKind {
    AuditKind::GodClass(GodClassEvidence {
        class_file: PathBuf::from("pkg/sub/foo.rs"),
        class_name: "Foo".into(),
        wmc: 60,
        tcc: 0.1,
        atfd: 8,
        method_count: 30,
        confidence: medium(),
    })
}

#[test]
fn architecture_modules_never_reference_class_smells() {
    assert!(
        !ARCH_MODULES.is_empty() && !CLASS_SMELL_SYMBOLS.is_empty(),
        "the guardrail must check at least one module against at least one symbol"
    );
    for module in ARCH_MODULES {
        let source = audit_source(module);
        for symbol in CLASS_SMELL_SYMBOLS {
            assert!(
                !source.contains(symbol),
                "architecture module {module} references class-smell symbol `{symbol}` — \
                 architecture (P9) and code smells (P10) must stay decoupled"
            );
        }
    }
}

#[test]
fn every_architecture_kind_stays_on_the_architecture_pillar() {
    for kind in architecture_kinds() {
        assert_eq!(
            kind_pillar(&kind),
            AuditPillar::Architecture,
            "architecture finding leaked off the Architecture pillar"
        );
    }
}

#[test]
fn god_component_is_never_conflated_with_god_class() {
    let god_component = &architecture_kinds()[2];
    let god_class = god_class_kind();
    assert_eq!(kind_pillar(god_component), AuditPillar::Architecture);
    assert_eq!(kind_pillar(&god_class), AuditPillar::ClassSmells);
    assert_ne!(
        kind_pillar(god_component),
        kind_pillar(&god_class),
        "a god component is an architecture smell, not a class smell"
    );
}

#[test]
fn architecture_and_class_pillars_are_disjoint() {
    let arch = architecture_kinds();
    let class = god_class_kind();
    assert!(arch.iter().all(|k| kind_pillar(k) != AuditPillar::ClassSmells));
    assert_ne!(kind_pillar(&class), AuditPillar::Architecture);
}
