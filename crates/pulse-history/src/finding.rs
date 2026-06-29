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
    ChangeShotgun(ChangeShotgunEvidence),
    CatalystWarning(CatalystEvidence),
    DecayTrend(DecayEvidence),
    BuildCoChange(BuildCoChangeEvidence),
    DefectProneFile(DefectProneEvidence),
}

#[derive(Debug, Clone)]
pub struct DefectProneEvidence {
    pub file: PathBuf,
    pub function: String,
    pub fix_count: u32,
    pub introducer_count: u32,
}

#[derive(Debug, Clone)]
pub struct BuildCoChangeEvidence {
    pub build_file: PathBuf,
    pub source_file: PathBuf,
    pub support: u32,
    pub source_revisions: u32,
    pub ratio: f64,
    pub last_seen_unix: i64,
    pub distinct_authors: u32,
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

#[derive(Debug, Clone)]
pub struct ChangeShotgunEvidence {
    pub file: PathBuf,
    pub partner_count: u32,
    pub package_count: u32,
    pub packages: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct CatalystEvidence {
    pub members: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct DecayEvidence {
    pub members: Vec<PathBuf>,
    pub previous_size: u32,
    pub current_size: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryPillar {
    Drift,
    Complexity,
    Ownership,
    Evolution,
}

#[derive(Clone, Copy)]
pub struct VariantInfo {
    pub pillar: HistoryPillar,
    pub slug: &'static str,
    pub label: &'static str,
    pub action: &'static str,
}

const VARIANT_TABLE: [VariantInfo; 9] = [
    VariantInfo {
        pillar: HistoryPillar::Drift,
        slug: "architectural_drift",
        label: "architectural drift",
        action: "introduce a static link or co-locate",
    },
    VariantInfo {
        pillar: HistoryPillar::Complexity,
        slug: "hotspot",
        label: "hotspot",
        action: "refactor for testability — 80% of value in top decile",
    },
    VariantInfo {
        pillar: HistoryPillar::Ownership,
        slug: "knowledge_fragmentation",
        label: "knowledge fragmentation",
        action: "assign a code owner or schedule pair-programming",
    },
    VariantInfo {
        pillar: HistoryPillar::Evolution,
        slug: "file_blob",
        label: "file blob",
        action: "split this file's responsibilities — it changes with everything",
    },
    VariantInfo {
        pillar: HistoryPillar::Evolution,
        slug: "change_shotgun",
        label: "change shotgun",
        action: "consolidate this responsibility — its changes ripple across many packages",
    },
    VariantInfo {
        pillar: HistoryPillar::Evolution,
        slug: "catalyst_cycle",
        label: "newly-introduced cycle",
        action: "break this fresh dependency cycle early — cycles tend to precede other smells",
    },
    VariantInfo {
        pillar: HistoryPillar::Evolution,
        slug: "decay_trend",
        label: "decaying cycle",
        action: "this dependency cycle is growing across history — break it before it absorbs more modules",
    },
    VariantInfo {
        pillar: HistoryPillar::Evolution,
        slug: "build_co_change",
        label: "build co-change coupling",
        action: "when you change this source file, check whether the build manifest needs updating too",
    },
    VariantInfo {
        pillar: HistoryPillar::Complexity,
        slug: "defect_prone_file",
        label: "defect-prone file",
        action: "this file is the blamed source of repeated bug fixes — stabilize it with tests and review",
    },
];

pub fn variant_info(k: &HistoryKind) -> VariantInfo {
    VARIANT_TABLE[variant_index(k)]
}

fn variant_index(k: &HistoryKind) -> usize {
    match k {
        HistoryKind::ArchitecturalDrift(_) => 0,
        HistoryKind::Hotspot(_) => 1,
        HistoryKind::KnowledgeFragmentation(_) => 2,
        HistoryKind::FileBlob(_) => 3,
        _ => late_variant_index(k),
    }
}

fn late_variant_index(k: &HistoryKind) -> usize {
    match k {
        HistoryKind::ChangeShotgun(_) => 4,
        HistoryKind::CatalystWarning(_) => 5,
        HistoryKind::DecayTrend(_) => 6,
        HistoryKind::BuildCoChange(_) => 7,
        _ => 8,
    }
}
