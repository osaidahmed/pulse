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
}

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

#[derive(Debug, Clone)]
pub struct AuditFinding {
    pub kind: AuditKind,
    pub representative_snippet: String,
    pub support: u32,
    pub file_count: u32,
    pub idf_score: Option<f64>,
    pub action_label: Option<&'static str>,
    pub locations: Vec<AuditLocation>,
}
