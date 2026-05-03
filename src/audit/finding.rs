use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AuditLocation {
    pub file: PathBuf,
    pub line: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditKind {
    UncategorizedPattern { fingerprint: u64 },
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
