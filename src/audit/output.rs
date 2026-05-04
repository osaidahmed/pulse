use std::fmt::Write;
use std::path::{Path, PathBuf};

use crate::thresholds::AuditThresholds;

use super::finding::{AuditFinding, AuditKind, ShotgunSurgeryEvidence};
use super::output_helpers::{
    confidence_str, display_path, write_capped_list, ListLayout,
};
use super::output_package_metrics::{
    cycle_json, martin_json, write_cycle, write_martin, write_zero_edge, zero_edge_json,
};

pub fn format_findings(
    findings: &[AuditFinding],
    root: Option<&Path>,
    thresholds: &AuditThresholds,
) -> String {
    let mut out = String::new();
    let (pattern_findings, other_findings) = super::output_grouped::split(findings);
    super::output_grouped::render(&mut out, &pattern_findings, root, thresholds, render_pattern_human);
    for f in other_findings {
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
    let unique = unique_file_locations(f);
    let layout = ListLayout {
        prefix_first: "  files:        ",
        prefix_rest: "                ",
        cap: t.max_locations_per_finding,
    };
    write_capped_list(out, &layout, unique.len(), |i| {
        let (path, line) = &unique[i];
        format!("{}:{}", display_path(path, root), line)
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
        "method_class": e.method_class,
        "method_name": e.method_name,
        "method_file": display_path(&e.method_file, root),
        "method_line": e.method_line,
        "changing_classes": e.changing_classes,
        "changing_methods": e.changing_methods,
        "fanout": e.fanout,
        "confidence": confidence_str(e.confidence),
        "callers": callers,
    })
}

fn unique_file_locations(f: &AuditFinding) -> Vec<(PathBuf, u32)> {
    let mut seen: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();
    let mut out: Vec<(PathBuf, u32)> = Vec::new();
    for loc in &f.locations {
        if seen.insert(loc.file.clone()) {
            out.push((loc.file.clone(), loc.line));
        }
    }
    out
}

fn pattern_json(f: &AuditFinding, fingerprint: u64, root: Option<&Path>) -> serde_json::Value {
    let locations: Vec<serde_json::Value> = f
        .locations
        .iter()
        .map(|loc| serde_json::json!({"file": display_path(&loc.file, root), "line": loc.line}))
        .collect();
    serde_json::json!({
        "kind": "UncategorizedPattern",
        "fingerprint": fingerprint,
        "representative_snippet": f.representative_snippet,
        "support": f.support,
        "file_count": f.file_count,
        "idf_score": f.idf_score,
        "action_label": f.action_label,
        "locations": locations,
    })
}

#[allow(dead_code)]
pub fn relative_to(root: &Path, file: &Path) -> PathBuf {
    file.strip_prefix(root).map_or_else(|_| file.to_path_buf(), Path::to_path_buf)
}
