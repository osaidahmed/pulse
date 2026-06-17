use super::finding::{AuditFinding, AuditKind, ImportConfidence, PatternCategory};

pub fn finding_confidence(f: &AuditFinding) -> ImportConfidence {
    evidence_confidence(&f.kind).unwrap_or_else(|| pattern_confidence(f))
}

fn evidence_confidence(kind: &AuditKind) -> Option<ImportConfidence> {
    package_metric_confidence(kind)
        .or_else(|| named_smell_confidence(kind))
        .or_else(|| advisory_confidence(kind))
        .or_else(|| deps_confidence(kind))
        .or_else(|| ifdef_confidence(kind))
}

fn package_metric_confidence(kind: &AuditKind) -> Option<ImportConfidence> {
    if matches!(kind, AuditKind::ZeroEdgeProject { .. }) {
        return Some(ImportConfidence::Medium);
    }
    package_metric_evidence_confidence(kind)
}

macro_rules! confidence_lookup {
    ($name:ident { $($variant:ident),+ $(,)? }) => {
        fn $name(kind: &AuditKind) -> Option<ImportConfidence> {
            match kind {
                $( AuditKind::$variant(e) => Some(e.confidence), )+
                _ => None,
            }
        }
    };
}

confidence_lookup!(package_metric_evidence_confidence {
    DistanceFromMainSequence,
    ImportCycle,
    UnstableDependency,
    HubLikeDependency,
    GodComponent,
    OverFragmentation,
    CompoundArchSmell,
    SplitComponent,
    MoveFile,
    MergeComponents
});

confidence_lookup!(named_smell_confidence {
    ShotgunSurgery,
    DivergentChange,
    FeatureEnvy,
    GodClass,
    ParallelInheritance,
    RefusedBequest,
    LowConceptualCohesion,
    MultivariateAnomaly
});

confidence_lookup!(advisory_confidence { InjectionShape, NearDuplicate, UnnaturalCode, VulnerableCloneSibling });

confidence_lookup!(deps_confidence {
    BloatedDependency,
    PhantomDependency,
    ConstraintSmell,
    UndeclaredModuleDependency,
    UnusedDeclaredDependency,
    StrictnessDebt,
    OutdatedDependency,
    VulnerableDependency
});

confidence_lookup!(ifdef_confidence { IfdefDensity });

fn pattern_confidence(f: &AuditFinding) -> ImportConfidence {
    let category = f.pattern_category.unwrap_or(PatternCategory::Other);
    if category.is_noise() {
        return ImportConfidence::Low;
    }
    match f.support {
        s if s >= 50 => ImportConfidence::High,
        s if s >= 10 => ImportConfidence::Medium,
        _ => ImportConfidence::Low,
    }
}
