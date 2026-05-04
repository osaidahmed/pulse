use std::path::PathBuf;

#[derive(Debug, Clone)]
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
