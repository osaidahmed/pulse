use std::path::Path;

use super::finding::{
    self, BlobEvidence, ChangeShotgunEvidence, DriftEvidence, FragmentationEvidence, HistoryFinding,
    HistoryKind, HistoryPillar, HotspotEvidence,
};

const PILLAR_ORDER: [(HistoryPillar, &str); 4] = [
    (HistoryPillar::Drift, "architectural drift"),
    (HistoryPillar::Complexity, "hotspots"),
    (HistoryPillar::Ownership, "knowledge fragmentation"),
    (HistoryPillar::Evolution, "evolutionary smells"),
];

#[derive(Default)]
struct PillarCounts {
    drift: usize,
    complexity: usize,
    ownership: usize,
    evolution: usize,
}

pub fn format_findings(findings: &[HistoryFinding], root: Option<&Path>) -> String {
    let mut out = String::new();
    write_header(&mut out, findings, root);
    for &(pillar, _) in &PILLAR_ORDER {
        write_section(&mut out, findings, pillar);
    }
    out
}

fn write_header(out: &mut String, findings: &[HistoryFinding], root: Option<&Path>) {
    let root_label = root.map_or_else(|| ".".to_string(), |p| p.display().to_string());
    out.push_str(&format!("history: {} — {} findings\n", root_label, findings.len()));
    let c = pillar_counts(findings);
    out.push_str(&format!(
        "  {} drift · {} hotspots · {} ownership · {} evolution\n\n",
        c.drift, c.complexity, c.ownership, c.evolution
    ));
}

fn pillar_counts(findings: &[HistoryFinding]) -> PillarCounts {
    let mut c = PillarCounts::default();
    for f in findings {
        match finding::variant_info(&f.kind).pillar {
            HistoryPillar::Drift => c.drift += 1,
            HistoryPillar::Complexity => c.complexity += 1,
            HistoryPillar::Ownership => c.ownership += 1,
            HistoryPillar::Evolution => c.evolution += 1,
        }
    }
    c
}

fn pillar_label(p: HistoryPillar) -> &'static str {
    PILLAR_ORDER
        .iter()
        .find(|(pi, _)| *pi == p)
        .map_or("", |(_, l)| l)
}

fn write_section(out: &mut String, findings: &[HistoryFinding], pillar: HistoryPillar) {
    let in_section: Vec<&HistoryFinding> = findings
        .iter()
        .filter(|f| finding::variant_info(&f.kind).pillar == pillar)
        .collect();
    out.push_str(&format!(
        "{} — {} findings\n",
        pillar_label(pillar),
        in_section.len()
    ));
    if in_section.is_empty() {
        out.push_str("  (none)\n\n");
        return;
    }
    for f in &in_section {
        match &f.kind {
            HistoryKind::ArchitecturalDrift(e) => out.push_str(&format!(
                "  {} ↔ {}   ({} co-changes, conf {:.2}, lift {:.1}, {} authors)\n",
                e.file_a.display(),
                e.file_b.display(),
                e.support,
                e.confidence,
                e.lift,
                e.distinct_authors
            )),
            HistoryKind::Hotspot(e) => out.push_str(&format!(
                "  {}   {} revisions × cc={} = {}\n",
                e.file.display(),
                e.revisions,
                e.sum_cc,
                e.score
            )),
            HistoryKind::KnowledgeFragmentation(e) => render_ownership(e, out),
            HistoryKind::FileBlob(e) => out.push_str(&format!(
                "  {}   {} of {} multi-file commits ({:.0}%)\n",
                e.file.display(),
                e.multi_file_commits,
                e.total_multi_file_commits,
                e.blob_ratio * 100.0
            )),
            HistoryKind::ChangeShotgun(e) => out.push_str(&format!(
                "  {}   {} confident partners across {} packages\n",
                e.file.display(),
                e.partner_count,
                e.package_count
            )),
        }
    }
    out.push('\n');
}

fn render_ownership(e: &FragmentationEvidence, out: &mut String) {
    let pct = (e.minor_contributor_pct * 100.0).round() as u32;
    let authors = if e.top_minor_authors.is_empty() {
        String::new()
    } else {
        format!(" — {}", e.top_minor_authors.join(", "))
    };
    out.push_str(&format!(
        "  {}   total={}, minor={} ({}%){}\n",
        e.file.display(),
        e.total_contributors,
        e.minor_contributor_count,
        pct,
        authors
    ));
}

pub fn format_findings_json(findings: &[HistoryFinding], root: Option<&Path>) -> String {
    let c = pillar_counts(findings);
    let summary = serde_json::json!({
        "root": root.map_or_else(|| ".".to_string(), |p| p.display().to_string()),
        "findings_total": findings.len(),
        "by_pillar": {
            "drift": c.drift,
            "complexity": c.complexity,
            "ownership": c.ownership,
            "evolution": c.evolution,
        },
    });
    let arr: Vec<serde_json::Value> = findings.iter().map(finding_to_json).collect();
    let envelope = serde_json::json!({"summary": summary, "findings": arr});
    serde_json::to_string_pretty(&envelope).unwrap_or_else(|_| String::from("{}"))
}

fn finding_to_json(f: &HistoryFinding) -> serde_json::Value {
    let mut obj = match &f.kind {
        HistoryKind::ArchitecturalDrift(e) => drift_to_json(e),
        HistoryKind::Hotspot(e) => hotspot_to_json(e),
        HistoryKind::KnowledgeFragmentation(e) => ownership_to_json(e),
        HistoryKind::FileBlob(e) => blob_to_json(e),
        HistoryKind::ChangeShotgun(e) => shotgun_to_json(e),
    };
    if let (serde_json::Value::Object(ref mut map), Some(action)) = (&mut obj, f.action_label) {
        map.insert("action".to_string(), serde_json::Value::String(action.to_string()));
    }
    obj
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
    let fields = [("multi_file_commits", e.multi_file_commits.into()), ("total_multi_file_commits", e.total_multi_file_commits.into()), ("blob_ratio", e.blob_ratio.into())];
    evidence_json("FileBlob", &e.file, &fields)
}

fn shotgun_to_json(e: &ChangeShotgunEvidence) -> serde_json::Value {
    let pkgs: Vec<String> = e.packages.iter().map(|p| p.display().to_string()).collect();
    let fields = [("partner_count", e.partner_count.into()), ("package_count", e.package_count.into()), ("packages", pkgs.into())];
    evidence_json("ChangeShotgun", &e.file, &fields)
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
