use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditLocation {
    pub file: PathBuf,
    pub line: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AuditKind {
    UncategorizedPattern { fingerprint: u64 },
    DistanceFromMainSequence(MartinMetrics),
    ImportCycle(CycleMembership),
    ZeroEdgeProject { module_count: u32 },
    ShotgunSurgery(ShotgunSurgeryEvidence),
    DivergentChange(DivergentChangeEvidence),
    FeatureEnvy(FeatureEnvyEvidence),
    GodClass(GodClassEvidence),
    ParallelInheritance(ParallelInheritanceEvidence),
    RefusedBequest(RefusedBequestEvidence),
    InjectionShape(InjectionEvidence),
    NearDuplicate(CloneClusterEvidence),
    UnnaturalCode(NaturalnessEvidence),
}

pub use super::finding_evidence::{CloneClusterEvidence, InjectionEvidence, NaturalnessEvidence};

#[derive(Debug, Clone, PartialEq)]
pub struct DivergentChangeEvidence {
    pub class_file: PathBuf,
    pub class_name: String,
    pub changing_classes: u32,
    pub fanout: u32,
    pub method_count: u32,
    pub confidence: ImportConfidence,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FeatureEnvyEvidence {
    pub method_file: PathBuf,
    pub method_class: Option<String>,
    pub method_name: String,
    pub method_line: u32,
    pub atfd: u32,
    pub foreign_call_count: u32,
    pub intra_call_count: u32,
    pub envied_class: Option<String>,
    pub confidence: ImportConfidence,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GodClassEvidence {
    pub class_file: PathBuf,
    pub class_name: String,
    pub wmc: u32,
    pub tcc: f64,
    pub atfd: u32,
    pub method_count: u32,
    pub confidence: ImportConfidence,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassIdentityRef {
    pub file: PathBuf,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParallelInheritanceEvidence {
    pub root_a: ClassIdentityRef,
    pub root_b: ClassIdentityRef,
    pub matched_descendants: Vec<(String, String)>,
    pub confidence: ImportConfidence,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RefusedBequestEvidence {
    pub subclass_file: PathBuf,
    pub subclass_name: String,
    pub parent_file: PathBuf,
    pub parent_name: String,
    pub override_count: u32,
    pub parent_method_count: u32,
    pub override_ratio: f64,
    pub confidence: ImportConfidence,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShotgunSurgeryEvidence {
    pub method_file: PathBuf,
    pub method_class: Option<String>,
    pub method_name: String,
    pub method_line: u32,
    pub changing_classes: u32,
    pub changing_methods: u32,
    pub fanout: u32,
    pub confidence: ImportConfidence,
    pub caller_samples: Vec<AuditLocation>,
    pub name_collision_count: u32,
    pub additional_definitions: Vec<AuditLocation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MartinMetrics {
    pub module: PathBuf,
    pub afferent: u32,
    pub efferent: u32,
    pub instability: f64,
    pub abstractness: f64,
    pub distance: f64,
    pub tier: MartinTier,
    pub confidence: ImportConfidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MartinTier {
    Healthy,
    Warning,
    Alert,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CycleMembership {
    pub members: Vec<PathBuf>,
    pub edges: Vec<(PathBuf, PathBuf)>,
    pub confidence: ImportConfidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ImportConfidence {
    NaAbstraction,
    BestEffort,
    Low,
    Medium,
    High,
}

impl ImportConfidence {
    #[must_use]
    pub fn min(self, other: Self) -> Self {
        if self < other {
            self
        } else {
            other
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PatternCategory {
    PrimitiveObsession = 0,
    ChainedDictAccess = 1,
    EnumValueAccess = 2,
    AttributeChain = 3,
    LiteralRepetition = 4,
    MethodCall = 5,
    Comparison = 6,
    Assignment = 7,
    DictLiteral = 8,
    ListLiteral = 9,
    Other = 10,
}

struct PatternDescriptor {
    header: &'static str,
    slug: &'static str,
    action: &'static str,
    noise: bool,
}

const PATTERN_DESCRIPTORS: [PatternDescriptor; 11] = [
    PatternDescriptor { header: "PRIMITIVE OBSESSION", slug: "primitive_obsession", action: "wrap repeated literals in a typed object", noise: false },
    PatternDescriptor { header: "CHAINED DICT ACCESS", slug: "chained_dict_access", action: "introduce an intermediate accessor or typed object", noise: false },
    PatternDescriptor { header: "ENUM VALUE ACCESS", slug: "enum_value_access", action: "consider a dispatch table or polymorphism", noise: false },
    PatternDescriptor { header: "ATTRIBUTE CHAIN", slug: "attribute_chain", action: "extract a named accessor or shared helper", noise: false },
    PatternDescriptor { header: "LITERAL REPETITION", slug: "literal_repetition", action: "extract a named constant", noise: false },
    PatternDescriptor { header: "METHOD CALL", slug: "method_call", action: "consider a shared helper or template method", noise: false },
    PatternDescriptor { header: "COMPARISON", slug: "comparison", action: "consolidate the comparison behind a named predicate", noise: false },
    PatternDescriptor { header: "ASSIGNMENT", slug: "assignment", action: "consider a typed config or builder", noise: false },
    PatternDescriptor { header: "DICT LITERAL", slug: "dict_literal", action: "extract a named factory or schema", noise: false },
    PatternDescriptor { header: "LIST LITERAL", slug: "list_literal", action: "extract a named constant or factory", noise: false },
    PatternDescriptor { header: "OTHER STRUCTURAL RECURRENCE", slug: "other", action: "review case-by-case", noise: true },
];

impl PatternCategory {
    fn descriptor(self) -> &'static PatternDescriptor {
        &PATTERN_DESCRIPTORS[self as usize]
    }

    pub fn header(self) -> &'static str {
        self.descriptor().header
    }

    pub fn order(self) -> u8 {
        self as u8
    }

    pub fn slug(self) -> &'static str {
        self.descriptor().slug
    }

    pub fn action(self) -> &'static str {
        self.descriptor().action
    }

    pub fn is_noise(self) -> bool {
        self.descriptor().noise
    }
}

#[derive(Debug, Clone)]
pub struct AuditFinding {
    pub kind: AuditKind,
    pub representative_snippet: String,
    pub support: u32,
    pub file_count: u32,
    pub idf_score: Option<f64>,
    pub action_label: Option<&'static str>,
    pub locations: Vec<AuditLocation>,
    pub pattern_category: Option<PatternCategory>,
    pub locality_entropy: Option<f64>,
    pub p_value: Option<f64>,
}

struct VariantInfo {
    test: fn(&AuditKind) -> bool,
    pass: &'static str,
    slug: &'static str,
    action: &'static str,
    label: &'static str,
    pillar: AuditPillar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AuditPillar {
    Architecture,
    ClassSmells,
    Security,
    Duplication,
    Naturalness,
    Patterns,
}

const VARIANT_TABLE: [VariantInfo; 13] = [
    VariantInfo {
        test: |k| matches!(k, AuditKind::UncategorizedPattern { .. }),
        pass: "pattern-mining",
        slug: "uncategorized_pattern",
        action: "",
        label: "Cross-file pattern",
        pillar: AuditPillar::Patterns,
    },
    VariantInfo {
        test: |k| matches!(k, AuditKind::DistanceFromMainSequence(_)),
        pass: "package-metrics",
        slug: "distance_from_main_sequence",
        action: "depend less on this module or extract an interface",
        label: "Distance from main sequence",
        pillar: AuditPillar::Architecture,
    },
    VariantInfo {
        test: |k| matches!(k, AuditKind::ImportCycle(_)),
        pass: "package-metrics",
        slug: "import_cycle",
        action: "break by hoisting shared symbols to a leaf module",
        label: "Import cycles",
        pillar: AuditPillar::Architecture,
    },
    VariantInfo {
        test: |k| matches!(k, AuditKind::ZeroEdgeProject { .. }),
        pass: "package-metrics",
        slug: "zero_edge_project",
        action: "verify --root points at the project's source tree",
        label: "Zero-edge project",
        pillar: AuditPillar::Architecture,
    },
    VariantInfo {
        test: |k| matches!(k, AuditKind::ShotgunSurgery(_)),
        pass: "named-smells",
        slug: "shotgun_surgery",
        action: "introduce polymorphism or move logic to a single owner",
        label: "Shotgun surgery",
        pillar: AuditPillar::ClassSmells,
    },
    VariantInfo {
        test: |k| matches!(k, AuditKind::DivergentChange(_)),
        pass: "named-smells",
        slug: "divergent_change",
        action: "split this class along its independent change axes",
        label: "Divergent change",
        pillar: AuditPillar::ClassSmells,
    },
    VariantInfo {
        test: |k| matches!(k, AuditKind::FeatureEnvy(_)),
        pass: "named-smells",
        slug: "feature_envy",
        action: "move this method to the class whose data it accesses",
        label: "Feature envy",
        pillar: AuditPillar::ClassSmells,
    },
    VariantInfo {
        test: |k| matches!(k, AuditKind::GodClass(_)),
        pass: "named-smells",
        slug: "god_class",
        action: "extract cohesive subclasses or split responsibilities",
        label: "God classes",
        pillar: AuditPillar::ClassSmells,
    },
    VariantInfo {
        test: |k| matches!(k, AuditKind::ParallelInheritance(_)),
        pass: "named-smells",
        slug: "parallel_inheritance",
        action: "extract a shared abstract base and unify subclass shape",
        label: "Parallel inheritance",
        pillar: AuditPillar::ClassSmells,
    },
    VariantInfo {
        test: |k| matches!(k, AuditKind::RefusedBequest(_)),
        pass: "named-smells",
        slug: "refused_bequest",
        action: "narrow the base class or make subclass concrete",
        label: "Refused bequest",
        pillar: AuditPillar::ClassSmells,
    },
    VariantInfo {
        test: |k| matches!(k, AuditKind::InjectionShape(_)),
        pass: "taint",
        slug: "injection_shape",
        action: "sanitize or validate the tainted value before it reaches the sink",
        label: "Injection shape",
        pillar: AuditPillar::Security,
    },
    VariantInfo {
        test: |k| matches!(k, AuditKind::NearDuplicate(_)),
        pass: "clones",
        slug: "near_duplicate",
        action: "extract the shared fragment into a single reusable definition",
        label: "Near-duplicate clones",
        pillar: AuditPillar::Duplication,
    },
    VariantInfo {
        test: |k| matches!(k, AuditKind::UnnaturalCode(_)),
        pass: "naturalness",
        slug: "unnatural_code",
        action: "review this unusually-structured function for clarity or hidden bugs",
        label: "Unnatural code",
        pillar: AuditPillar::Naturalness,
    },
];

pub fn kind_label(kind: &AuditKind) -> &'static str {
    variant_info(kind).map_or("", |v| v.label)
}

pub fn kind_pillar(kind: &AuditKind) -> AuditPillar {
    variant_info(kind).map_or(AuditPillar::Patterns, |v| v.pillar)
}

fn variant_info(kind: &AuditKind) -> Option<&'static VariantInfo> {
    VARIANT_TABLE.iter().find(|v| (v.test)(kind))
}

pub fn pass_for(kind: &AuditKind) -> &'static str {
    variant_info(kind).map_or("named-smells", |v| v.pass)
}

pub fn kind_slug(kind: &AuditKind) -> &'static str {
    variant_info(kind).map_or("", |v| v.slug)
}

pub fn action_for_kind(kind: &AuditKind, category: Option<PatternCategory>) -> &'static str {
    let action = variant_info(kind).map_or("", |v| v.action);
    if !action.is_empty() {
        return action;
    }
    category.map_or("review case-by-case", PatternCategory::action)
}

pub fn finding_confidence(f: &AuditFinding) -> ImportConfidence {
    evidence_confidence(&f.kind).unwrap_or_else(|| pattern_confidence(f))
}

fn evidence_confidence(kind: &AuditKind) -> Option<ImportConfidence> {
    package_metric_confidence(kind)
        .or_else(|| named_smell_confidence(kind))
        .or_else(|| advisory_confidence(kind))
}

fn package_metric_confidence(kind: &AuditKind) -> Option<ImportConfidence> {
    if let AuditKind::DistanceFromMainSequence(m) = kind {
        return Some(m.confidence);
    }
    if let AuditKind::ImportCycle(c) = kind {
        return Some(c.confidence);
    }
    if matches!(kind, AuditKind::ZeroEdgeProject { .. }) {
        return Some(ImportConfidence::Medium);
    }
    None
}

fn named_smell_confidence(kind: &AuditKind) -> Option<ImportConfidence> {
    match kind {
        AuditKind::ShotgunSurgery(e) => Some(e.confidence),
        AuditKind::DivergentChange(e) => Some(e.confidence),
        AuditKind::FeatureEnvy(e) => Some(e.confidence),
        AuditKind::GodClass(e) => Some(e.confidence),
        AuditKind::ParallelInheritance(e) => Some(e.confidence),
        AuditKind::RefusedBequest(e) => Some(e.confidence),
        _ => None,
    }
}

fn advisory_confidence(kind: &AuditKind) -> Option<ImportConfidence> {
    match kind {
        AuditKind::InjectionShape(e) => Some(e.confidence),
        AuditKind::NearDuplicate(e) => Some(e.confidence),
        AuditKind::UnnaturalCode(e) => Some(e.confidence),
        _ => None,
    }
}

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
