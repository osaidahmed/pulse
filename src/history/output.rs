use std::path::Path;

use super::finding::{self, FragmentationEvidence, HistoryFinding, HistoryKind, HistoryPillar};
use super::output_json::finding_to_json;

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
    PILLAR_ORDER.iter().find(|(pi, _)| *pi == p).map_or("", |(_, l)| l)
}

fn write_section(out: &mut String, findings: &[HistoryFinding], pillar: HistoryPillar) {
    let in_section: Vec<&HistoryFinding> =
        findings.iter().filter(|f| finding::variant_info(&f.kind).pillar == pillar).collect();
    out.push_str(&format!("{} — {} findings\n", pillar_label(pillar), in_section.len()));
    if in_section.is_empty() {
        out.push_str("  (none)\n\n");
        return;
    }
    for f in &in_section {
        render_finding_line(out, f);
    }
    out.push('\n');
}

fn render_finding_line(out: &mut String, f: &HistoryFinding) {
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
        HistoryKind::DefectProneFile(e) => out.push_str(&format!(
            "  {} :: {}   blamed by {} bug-fixes (from {} introducing commits)\n",
            e.file.display(),
            e.function,
            e.fix_count,
            e.introducer_count
        )),
        _ => render_evolution_line(out, f),
    }
}

fn render_evolution_line(out: &mut String, f: &HistoryFinding) {
    match &f.kind {
        HistoryKind::ChangeShotgun(e) => out.push_str(&format!(
            "  {}   {} confident partners across {} packages\n",
            e.file.display(),
            e.partner_count,
            e.package_count
        )),
        HistoryKind::CatalystWarning(e) => out.push_str(&format!(
            "  fresh cycle ({} modules): {}\n",
            e.members.len(),
            e.members.iter().map(|m| m.display().to_string()).collect::<Vec<_>>().join(" → ")
        )),
        HistoryKind::DecayTrend(e) => out.push_str(&format!(
            "  growing cycle ({} → {} modules): {}\n",
            e.previous_size,
            e.current_size,
            e.members.iter().map(|m| m.display().to_string()).collect::<Vec<_>>().join(" → ")
        )),
        HistoryKind::BuildCoChange(e) => out.push_str(&format!(
            "  {} ↔ {}   ({} of {} revisions, {:.0}% co-change, {} authors)\n",
            e.source_file.display(),
            e.build_file.display(),
            e.support,
            e.source_revisions,
            e.ratio * 100.0,
            e.distinct_authors
        )),
        _ => {}
    }
}

fn render_ownership(e: &FragmentationEvidence, out: &mut String) {
    let pct = (e.minor_contributor_pct * 100.0).round() as u32;
    let authors =
        if e.top_minor_authors.is_empty() { String::new() } else { format!(" — {}", e.top_minor_authors.join(", ")) };
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
