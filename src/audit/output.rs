use std::fmt::Write;
use std::path::{Path, PathBuf};

use crate::thresholds::AuditThresholds;

use super::finding::{
    AuditFinding, AuditKind, CycleMembership, ImportConfidence, MartinMetrics, MartinTier,
    ShotgunSurgeryEvidence,
};

pub fn format_findings(
    findings: &[AuditFinding],
    root: Option<&Path>,
    thresholds: &AuditThresholds,
) -> String {
    let mut out = String::new();
    for f in findings {
        render_human(&mut out, f, root, thresholds);
    }
    out
}

pub fn format_findings_json(findings: &[AuditFinding], root: Option<&Path>) -> String {
    let entries: Vec<serde_json::Value> = findings
        .iter()
        .map(|f| render_json(f, root))
        .collect();
    serde_json::Value::Array(entries).to_string()
}

struct ListLayout<'a> {
    prefix_first: &'a str,
    prefix_rest: &'a str,
    cap: usize,
}

fn render_human(out: &mut String, f: &AuditFinding, root: Option<&Path>, t: &AuditThresholds) {
    if dispatch_known_variants_human(out, f, root, t) {
        return;
    }
    render_pattern_human(out, f, root, t);
}

fn dispatch_known_variants_human(
    out: &mut String,
    f: &AuditFinding,
    root: Option<&Path>,
    t: &AuditThresholds,
) -> bool {
    if let AuditKind::DistanceFromMainSequence(m) = &f.kind {
        write_martin(out, m, root);
        return true;
    }
    if let AuditKind::ImportCycle(c) = &f.kind {
        write_cycle(out, c, root, t);
        return true;
    }
    if let AuditKind::ZeroEdgeProject { module_count } = &f.kind {
        write_zero_edge(out, *module_count);
        return true;
    }
    if let AuditKind::ShotgunSurgery(e) = &f.kind {
        write_shotgun(out, e, root, t);
        return true;
    }
    super::output_named_smells::dispatch_human(out, &f.kind, root, confidence_str, display_path)
}

fn render_pattern_human(out: &mut String, f: &AuditFinding, root: Option<&Path>, t: &AuditThresholds) {
    let _ = writeln!(
        out,
        "audit: cross-file pattern in {} files ({} occurrences)",
        f.file_count, f.support
    );
    let _ = writeln!(out, "  representative: {}", f.representative_snippet);
    let layout = ListLayout {
        prefix_first: "  locations:    ",
        prefix_rest: "                ",
        cap: t.max_locations_per_finding,
    };
    write_capped_list(out, &layout, f.locations.len(), |i| {
        let loc = &f.locations[i];
        format!("{}:{}", display_path(&loc.file, root), loc.line)
    });
    if let Some(label) = f.action_label {
        let _ = writeln!(out, "  pattern action: {label}");
    }
    let _ = writeln!(out);
}

fn render_json(f: &AuditFinding, root: Option<&Path>) -> serde_json::Value {
    if let Some(v) = dispatch_known_variants_json(f, root) {
        return v;
    }
    if let Some(v) = super::output_named_smells::dispatch_json(&f.kind, root, confidence_str, display_path) {
        return v;
    }
    let AuditKind::UncategorizedPattern { fingerprint } = &f.kind else {
        unreachable!()
    };
    pattern_json(f, *fingerprint, root)
}

fn dispatch_known_variants_json(f: &AuditFinding, root: Option<&Path>) -> Option<serde_json::Value> {
    if let AuditKind::DistanceFromMainSequence(m) = &f.kind {
        return Some(martin_json(m, root));
    }
    if let AuditKind::ImportCycle(c) = &f.kind {
        return Some(cycle_json(c, root));
    }
    if let AuditKind::ZeroEdgeProject { module_count } = &f.kind {
        return Some(zero_edge_json(*module_count));
    }
    if let AuditKind::ShotgunSurgery(e) = &f.kind {
        return Some(shotgun_json(e, root));
    }
    None
}

fn write_martin(out: &mut String, m: &MartinMetrics, root: Option<&Path>) {
    let tier = match m.tier {
        MartinTier::Healthy => "healthy",
        MartinTier::Warning => "warning",
        MartinTier::Alert => "alert",
    };
    let _ = writeln!(out, "audit: distance from main sequence");
    let _ = writeln!(out, "  module:        {}", display_path(&m.module, root));
    let _ = writeln!(out, "  Ca:            {}", m.afferent);
    let _ = writeln!(out, "  Ce:            {}", m.efferent);
    let _ = writeln!(out, "  instability:   {:.3}", m.instability);
    let _ = writeln!(out, "  abstractness:  {:.3}", m.abstractness);
    let _ = writeln!(out, "  distance:      {:.3}", m.distance);
    let _ = writeln!(out, "  tier:          {tier}");
    let _ = writeln!(out, "  confidence:    {}", confidence_str(m.confidence));
    let _ = writeln!(out);
}

fn write_cycle(out: &mut String, c: &CycleMembership, root: Option<&Path>, t: &AuditThresholds) {
    let _ = writeln!(out, "audit: import cycle (size {})", c.members.len());
    let members_layout = ListLayout {
        prefix_first: "  members:       ",
        prefix_rest: "                 ",
        cap: t.max_locations_per_finding,
    };
    write_capped_list(out, &members_layout, c.members.len(), |i| {
        display_path(&c.members[i], root)
    });
    let edges_layout = ListLayout {
        prefix_first: "  edges:         ",
        prefix_rest: "                 ",
        cap: t.max_locations_per_finding,
    };
    write_capped_list(out, &edges_layout, c.edges.len(), |i| {
        let (src, dst) = &c.edges[i];
        format!("{} -> {}", display_path(src, root), display_path(dst, root))
    });
    let _ = writeln!(out, "  confidence:    {}", confidence_str(c.confidence));
    let _ = writeln!(out);
}

fn write_zero_edge(out: &mut String, module_count: u32) {
    let _ = writeln!(
        out,
        "audit: no import edges resolved across {module_count} modules"
    );
    let _ = writeln!(out, "  note: per-module distance findings suppressed.");
    let _ = writeln!(
        out,
        "  hint: confirm --root points at the project's source tree."
    );
    let _ = writeln!(out);
}

fn write_shotgun(
    out: &mut String,
    e: &ShotgunSurgeryEvidence,
    root: Option<&Path>,
    t: &AuditThresholds,
) {
    let label = e.method_class.as_deref().map_or_else(
        || e.method_name.clone(),
        |c| format!("{c}.{}", e.method_name),
    );
    let _ = writeln!(out, "audit: shotgun surgery — {label}");
    let _ = writeln!(
        out,
        "  defined at:    {}:{}",
        display_path(&e.method_file, root),
        e.method_line
    );
    let _ = writeln!(out, "  CC:            {}", e.changing_classes);
    let _ = writeln!(out, "  CM:            {}", e.changing_methods);
    let _ = writeln!(out, "  fanout:        {}", e.fanout);
    let _ = writeln!(out, "  confidence:    {}", confidence_str(e.confidence));
    let layout = ListLayout {
        prefix_first: "  callers:       ",
        prefix_rest: "                 ",
        cap: t.named_smells.max_caller_samples_per_finding,
    };
    write_capped_list(out, &layout, e.caller_samples.len(), |i| {
        let loc = &e.caller_samples[i];
        format!("{}:{}", display_path(&loc.file, root), loc.line)
    });
    let _ = writeln!(out);
}

fn shotgun_json(e: &ShotgunSurgeryEvidence, root: Option<&Path>) -> serde_json::Value {
    let callers: Vec<serde_json::Value> = e
        .caller_samples
        .iter()
        .map(|loc| serde_json::json!({"file": display_path(&loc.file, root), "line": loc.line}))
        .collect();
    serde_json::json!({
        "kind": "ShotgunSurgery",
        "method_file": display_path(&e.method_file, root),
        "method_class": e.method_class,
        "method_name": e.method_name,
        "method_line": e.method_line,
        "changing_classes": e.changing_classes,
        "changing_methods": e.changing_methods,
        "fanout": e.fanout,
        "confidence": confidence_str(e.confidence),
        "callers": callers,
    })
}

fn write_capped_list<F>(out: &mut String, layout: &ListLayout, total: usize, render: F)
where
    F: Fn(usize) -> String,
{
    let shown = total.min(layout.cap);
    for i in 0..shown {
        let prefix = if i == 0 { layout.prefix_first } else { layout.prefix_rest };
        let _ = writeln!(out, "{prefix}{}", render(i));
    }
    if total > shown {
        let _ = writeln!(out, "{}... ({} more)", layout.prefix_rest, total - shown);
    }
}

fn display_path(file: &Path, root: Option<&Path>) -> String {
    let rel = root.and_then(|r| file.strip_prefix(r).ok()).map_or_else(
        || file.to_path_buf(),
        Path::to_path_buf,
    );
    rel.display().to_string()
}

fn confidence_str(c: ImportConfidence) -> &'static str {
    match c {
        ImportConfidence::High => "high",
        ImportConfidence::Medium => "medium",
        ImportConfidence::Low => "low",
        ImportConfidence::BestEffort => "best-effort",
        ImportConfidence::NaAbstraction => "n/a-abstraction",
    }
}

fn pattern_json(f: &AuditFinding, fp: u64, root: Option<&Path>) -> serde_json::Value {
    let locations: Vec<serde_json::Value> = f
        .locations
        .iter()
        .map(|loc| serde_json::json!({"file": display_path(&loc.file, root), "line": loc.line}))
        .collect();
    serde_json::json!({
        "kind": "UncategorizedPattern",
        "fingerprint": fp,
        "representative_snippet": f.representative_snippet,
        "support": f.support,
        "file_count": f.file_count,
        "idf_score": f.idf_score,
        "action_label": f.action_label,
        "locations": locations,
    })
}

fn martin_json(m: &MartinMetrics, root: Option<&Path>) -> serde_json::Value {
    let tier = match m.tier {
        MartinTier::Healthy => "healthy",
        MartinTier::Warning => "warning",
        MartinTier::Alert => "alert",
    };
    serde_json::json!({
        "kind": "DistanceFromMainSequence",
        "module": display_path(&m.module, root),
        "afferent": m.afferent,
        "efferent": m.efferent,
        "instability": m.instability,
        "abstractness": m.abstractness,
        "distance": m.distance,
        "tier": tier,
        "confidence": confidence_str(m.confidence),
    })
}

fn cycle_json(c: &CycleMembership, root: Option<&Path>) -> serde_json::Value {
    let members: Vec<String> = c.members.iter().map(|p| display_path(p, root)).collect();
    let edges: Vec<serde_json::Value> = c
        .edges
        .iter()
        .map(|(src, dst)| {
            serde_json::json!([display_path(src, root), display_path(dst, root)])
        })
        .collect();
    serde_json::json!({
        "kind": "ImportCycle",
        "size": c.members.len(),
        "members": members,
        "edges": edges,
        "confidence": confidence_str(c.confidence),
    })
}

fn zero_edge_json(module_count: u32) -> serde_json::Value {
    serde_json::json!({"kind": "ZeroEdgeProject", "module_count": module_count})
}

#[allow(dead_code)]
pub fn relative_to(root: &Path, file: &Path) -> PathBuf {
    file.strip_prefix(root).map_or_else(|_| file.to_path_buf(), Path::to_path_buf)
}
