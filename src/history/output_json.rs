use std::path::Path;

use super::finding::{
    BlobEvidence, BuildCoChangeEvidence, CatalystEvidence, ChangeShotgunEvidence, DecayEvidence, DefectProneEvidence,
    DriftEvidence, FragmentationEvidence, HistoryFinding, HistoryKind, HotspotEvidence,
};

pub fn finding_to_json(f: &HistoryFinding) -> serde_json::Value {
    let mut obj = kind_to_json(&f.kind);
    if let (serde_json::Value::Object(ref mut map), Some(action)) = (&mut obj, f.action_label) {
        map.insert("action".to_string(), serde_json::Value::String(action.to_string()));
    }
    obj
}

fn kind_to_json(kind: &HistoryKind) -> serde_json::Value {
    match kind {
        HistoryKind::ArchitecturalDrift(e) => drift_to_json(e),
        HistoryKind::Hotspot(e) => hotspot_to_json(e),
        HistoryKind::KnowledgeFragmentation(e) => ownership_to_json(e),
        HistoryKind::FileBlob(e) => blob_to_json(e),
        HistoryKind::DefectProneFile(e) => defect_prone_to_json(e),
        _ => evolution_to_json(kind),
    }
}

fn defect_prone_to_json(e: &DefectProneEvidence) -> serde_json::Value {
    serde_json::json!({
        "kind": "defect_prone_file",
        "file": e.file.display().to_string(),
        "fix_count": e.fix_count,
        "introducer_count": e.introducer_count,
    })
}

fn evolution_to_json(kind: &HistoryKind) -> serde_json::Value {
    match kind {
        HistoryKind::ChangeShotgun(e) => shotgun_to_json(e),
        HistoryKind::CatalystWarning(e) => catalyst_to_json(e),
        HistoryKind::DecayTrend(e) => decay_to_json(e),
        HistoryKind::BuildCoChange(e) => build_cochange_to_json(e),
        _ => serde_json::Value::Null,
    }
}

fn drift_to_json(e: &DriftEvidence) -> serde_json::Value {
    serde_json::json!({
        "kind": "ArchitecturalDrift",
        "file_a": e.file_a.display().to_string(),
        "file_b": e.file_b.display().to_string(),
        "support": e.support,
        "commits": e.commits,
        "confidence": e.confidence,
        "lift": e.lift,
        "jaccard": e.jaccard,
        "last_seen_unix": e.last_seen_unix,
        "distinct_authors": e.distinct_authors,
    })
}

fn evidence_json(kind: &str, file: &Path, fields: &[(&str, serde_json::Value)]) -> serde_json::Value {
    let mut obj = serde_json::Map::new();
    obj.insert("kind".to_string(), serde_json::Value::String(kind.to_string()));
    obj.insert("file".to_string(), serde_json::Value::String(file.display().to_string()));
    for (key, value) in fields {
        obj.insert((*key).to_string(), value.clone());
    }
    serde_json::Value::Object(obj)
}

fn hotspot_to_json(e: &HotspotEvidence) -> serde_json::Value {
    let fields = [("revisions", e.revisions.into()), ("sum_cc", e.sum_cc.into()), ("score", e.score.into())];
    evidence_json("Hotspot", &e.file, &fields)
}

fn blob_to_json(e: &BlobEvidence) -> serde_json::Value {
    let fields = [
        ("multi_file_commits", e.multi_file_commits.into()),
        ("total_multi_file_commits", e.total_multi_file_commits.into()),
        ("blob_ratio", e.blob_ratio.into()),
    ];
    evidence_json("FileBlob", &e.file, &fields)
}

fn shotgun_to_json(e: &ChangeShotgunEvidence) -> serde_json::Value {
    let pkgs: Vec<String> = e.packages.iter().map(|p| p.display().to_string()).collect();
    let fields = [
        ("partner_count", e.partner_count.into()),
        ("package_count", e.package_count.into()),
        ("packages", pkgs.into()),
    ];
    evidence_json("ChangeShotgun", &e.file, &fields)
}

fn catalyst_to_json(e: &CatalystEvidence) -> serde_json::Value {
    let members: Vec<String> = e.members.iter().map(|m| m.display().to_string()).collect();
    serde_json::json!({ "kind": "CatalystWarning", "members": members })
}

fn decay_to_json(e: &DecayEvidence) -> serde_json::Value {
    let members: Vec<String> = e.members.iter().map(|m| m.display().to_string()).collect();
    serde_json::json!({
        "kind": "DecayTrend",
        "members": members,
        "previous_size": e.previous_size,
        "current_size": e.current_size,
    })
}

fn build_cochange_to_json(e: &BuildCoChangeEvidence) -> serde_json::Value {
    serde_json::json!({
        "kind": "BuildCoChange",
        "build_file": e.build_file.display().to_string(),
        "source_file": e.source_file.display().to_string(),
        "support": e.support,
        "source_revisions": e.source_revisions,
        "ratio": e.ratio,
        "last_seen_unix": e.last_seen_unix,
        "distinct_authors": e.distinct_authors,
    })
}

fn ownership_to_json(e: &FragmentationEvidence) -> serde_json::Value {
    serde_json::json!({
        "kind": "KnowledgeFragmentation",
        "file": e.file.display().to_string(),
        "total_contributors": e.total_contributors,
        "minor_contributor_count": e.minor_contributor_count,
        "minor_contributor_pct": e.minor_contributor_pct,
        "top_minor_authors": e.top_minor_authors,
    })
}
