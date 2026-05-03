use std::fmt::Write;
use std::path::{Path, PathBuf};

use crate::thresholds::AuditThresholds;

use super::finding::{AuditFinding, AuditLocation};

pub fn format_findings(
    findings: &[AuditFinding],
    root: Option<&Path>,
    thresholds: &AuditThresholds,
) -> String {
    let mut out = String::new();
    for f in findings {
        write_finding(&mut out, f, root, thresholds);
    }
    out
}

pub fn format_findings_json(findings: &[AuditFinding], root: Option<&Path>) -> String {
    let entries: Vec<serde_json::Value> = findings
        .iter()
        .map(|f| finding_to_json(f, root))
        .collect();
    serde_json::Value::Array(entries).to_string()
}

fn write_finding(out: &mut String, f: &AuditFinding, root: Option<&Path>, t: &AuditThresholds) {
    let _ = writeln!(
        out,
        "audit: cross-file pattern in {} files ({} occurrences)",
        f.file_count, f.support
    );
    let _ = writeln!(out, "  representative: {}", f.representative_snippet);
    write_locations(out, &f.locations, root, t);
    if let Some(label) = f.action_label {
        let _ = writeln!(out, "  pattern action: {label}");
    }
    let _ = writeln!(out);
}

fn write_locations(out: &mut String, locs: &[AuditLocation], root: Option<&Path>, t: &AuditThresholds) {
    let cap = t.max_locations_per_finding;
    let total = locs.len();
    let shown = total.min(cap);
    for (i, loc) in locs.iter().take(shown).enumerate() {
        let prefix = if i == 0 { "  locations:    " } else { "                " };
        let _ = writeln!(out, "{prefix}{}:{}", display_path(&loc.file, root), loc.line);
    }
    if total > shown {
        let _ = writeln!(out, "                ... ({} more)", total - shown);
    }
}

fn display_path(file: &Path, root: Option<&Path>) -> String {
    let rel = root.and_then(|r| file.strip_prefix(r).ok()).map_or_else(
        || file.to_path_buf(),
        Path::to_path_buf,
    );
    rel.display().to_string()
}

fn finding_to_json(f: &AuditFinding, root: Option<&Path>) -> serde_json::Value {
    let kind_str = match f.kind {
        super::finding::AuditKind::UncategorizedPattern { .. } => "UncategorizedPattern",
    };
    let fp = match f.kind {
        super::finding::AuditKind::UncategorizedPattern { fingerprint } => fingerprint,
    };
    let locations: Vec<serde_json::Value> = f
        .locations
        .iter()
        .map(|loc| {
            serde_json::json!({
                "file": display_path(&loc.file, root),
                "line": loc.line,
            })
        })
        .collect();
    serde_json::json!({
        "kind": kind_str,
        "fingerprint": fp,
        "representative_snippet": f.representative_snippet,
        "support": f.support,
        "file_count": f.file_count,
        "idf_score": f.idf_score,
        "action_label": f.action_label,
        "locations": locations,
    })
}

pub fn relative_to(root: &Path, file: &Path) -> PathBuf {
    file.strip_prefix(root).map_or_else(|_| file.to_path_buf(), Path::to_path_buf)
}
