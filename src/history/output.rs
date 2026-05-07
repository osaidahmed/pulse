use std::path::Path;

use super::finding::{
    self, DriftEvidence, FragmentationEvidence, HistoryFinding, HistoryKind, HistoryPillar,
    HotspotEvidence,
};

const PILLAR_ORDER: [(HistoryPillar, &str); 3] = [
    (HistoryPillar::Drift, "architectural drift"),
    (HistoryPillar::Complexity, "hotspots"),
    (HistoryPillar::Ownership, "knowledge fragmentation"),
];

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
    let (drift, complexity, ownership) = pillar_counts(findings);
    out.push_str(&format!(
        "  {drift} drift · {complexity} hotspots · {ownership} ownership\n\n"
    ));
}

fn pillar_counts(findings: &[HistoryFinding]) -> (usize, usize, usize) {
    let mut drift = 0;
    let mut complexity = 0;
    let mut ownership = 0;
    for f in findings {
        match finding::variant_info(&f.kind).pillar {
            HistoryPillar::Drift => drift += 1,
            HistoryPillar::Complexity => complexity += 1,
            HistoryPillar::Ownership => ownership += 1,
        }
    }
    (drift, complexity, ownership)
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
                "  {} ↔ {}   ({} co-changes, {} authors)\n",
                e.file_a.display(),
                e.file_b.display(),
                e.support,
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
    let (drift, complexity, ownership) = pillar_counts(findings);
    let summary = serde_json::json!({
        "root": root.map_or_else(|| ".".to_string(), |p| p.display().to_string()),
        "findings_total": findings.len(),
        "by_pillar": {
            "drift": drift,
            "complexity": complexity,
            "ownership": ownership,
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
        "last_seen_unix": e.last_seen_unix,
        "distinct_authors": e.distinct_authors,
    })
}

fn hotspot_to_json(e: &HotspotEvidence) -> serde_json::Value {
    serde_json::json!({
        "kind": "Hotspot",
        "file": e.file.display().to_string(),
        "revisions": e.revisions,
        "sum_cc": e.sum_cc,
        "score": e.score,
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
