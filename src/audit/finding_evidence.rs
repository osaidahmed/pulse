use std::path::PathBuf;

use super::finding::{AuditLocation, ImportConfidence};

#[derive(Debug, Clone, PartialEq)]
pub struct InjectionEvidence {
    pub file: PathBuf,
    pub function: String,
    pub source_name: String,
    pub source_line: u32,
    pub sink_name: String,
    pub sink_line: u32,
    pub tainted_var: String,
    pub crossed_opacity: bool,
    pub confidence: ImportConfidence,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CloneClusterEvidence {
    pub members: Vec<AuditLocation>,
    pub member_count: u32,
    pub max_loc: u32,
    pub representative: String,
    pub confidence: ImportConfidence,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NaturalnessEvidence {
    pub file: PathBuf,
    pub function: String,
    pub line: u32,
    pub surprisal: f64,
    pub zscore: f64,
    pub confidence: ImportConfidence,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HubLikeEvidence {
    pub component: PathBuf,
    pub afferent: u32,
    pub efferent: u32,
    pub imbalance: u32,
    pub centrality: f64,
    pub confidence: ImportConfidence,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GodComponentEvidence {
    pub component: PathBuf,
    pub loc: u32,
    pub file_count: u32,
    pub density: f64,
    pub centrality: f64,
    pub confidence: ImportConfidence,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompoundEvidence {
    pub component: PathBuf,
    pub constituent_kinds: Vec<&'static str>,
    pub combined_severity: f64,
    pub confidence: ImportConfidence,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SplitComponentEvidence {
    pub component: PathBuf,
    pub file_count: u32,
    pub community_count: u32,
    pub cohesion: f64,
    pub confidence: ImportConfidence,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MoveFileEvidence {
    pub file: PathBuf,
    pub current_dir: PathBuf,
    pub target_dir: PathBuf,
    pub community_size: u32,
    pub home_share: f64,
    pub confidence: ImportConfidence,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MergeComponentsEvidence {
    pub components: Vec<PathBuf>,
    pub community_files: u32,
    pub confidence: ImportConfidence,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnstableDepEvidence {
    pub component: PathBuf,
    pub instability: f64,
    pub strength: f64,
    pub gap: f64,
    pub unstable_deps: u32,
    pub total_deps: u32,
    pub centrality: f64,
    pub confidence: ImportConfidence,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VulnCloneEvidence {
    pub file: PathBuf,
    pub line: u32,
    pub vuln_file: PathBuf,
    pub vuln_line: u32,
    pub sink_name: String,
    pub confidence: ImportConfidence,
}
