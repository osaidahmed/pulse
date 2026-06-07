use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct HistoryFinding {
    pub kind: HistoryKind,
    pub action_label: Option<&'static str>,
}

#[derive(Debug, Clone)]
pub enum HistoryKind {
    ArchitecturalDrift(DriftEvidence),
    Hotspot(HotspotEvidence),
    KnowledgeFragmentation(FragmentationEvidence),
    FileBlob(BlobEvidence),
}

#[derive(Debug, Clone)]
pub struct DriftEvidence {
    pub file_a: PathBuf,
    pub file_b: PathBuf,
    pub support: u32,
    pub commits: u32,
    pub confidence: f64,
    pub lift: f64,
    pub jaccard: f64,
    pub last_seen_unix: i64,
    pub distinct_authors: u32,
}

#[derive(Debug, Clone)]
pub struct HotspotEvidence {
    pub file: PathBuf,
    pub revisions: u32,
    pub sum_cc: u32,
    pub score: u64,
}

#[derive(Debug, Clone)]
pub struct FragmentationEvidence {
    pub file: PathBuf,
    pub total_contributors: u32,
    pub minor_contributor_count: u32,
    pub minor_contributor_pct: f64,
    pub top_minor_authors: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct BlobEvidence {
    pub file: PathBuf,
    pub multi_file_commits: u32,
    pub total_multi_file_commits: u32,
    pub blob_ratio: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryPillar {
    Drift,
    Complexity,
    Ownership,
    Evolution,
}

#[allow(dead_code)]
pub struct VariantInfo {
    pub pillar: HistoryPillar,
    pub slug: &'static str,
    pub label: &'static str,
    pub action: &'static str,
}

pub fn variant_info(k: &HistoryKind) -> VariantInfo {
    match k {
        HistoryKind::ArchitecturalDrift(_) => VariantInfo {
            pillar: HistoryPillar::Drift,
            slug: "architectural_drift",
            label: "architectural drift",
            action: "introduce a static link or co-locate",
        },
        HistoryKind::Hotspot(_) => VariantInfo {
            pillar: HistoryPillar::Complexity,
            slug: "hotspot",
            label: "hotspot",
            action: "refactor for testability — 80% of value in top decile",
        },
        HistoryKind::KnowledgeFragmentation(_) => VariantInfo {
            pillar: HistoryPillar::Ownership,
            slug: "knowledge_fragmentation",
            label: "knowledge fragmentation",
            action: "assign a code owner or schedule pair-programming",
        },
        HistoryKind::FileBlob(_) => VariantInfo {
            pillar: HistoryPillar::Evolution,
            slug: "file_blob",
            label: "file blob",
            action: "split this file's responsibilities — it changes with everything",
        },
    }
}
