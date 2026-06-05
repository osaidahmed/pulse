use std::fmt::Write;
use std::path::Path;

use super::finding::VulnCloneEvidence;
use super::output_helpers::{confidence_str, display_path};

pub fn write_vuln_clone(out: &mut String, e: &VulnCloneEvidence, root: Option<&Path>, action: &'static str) {
    let _ = writeln!(out, "audit: vulnerable clone sibling — {}:{}", display_path(&e.file, root), e.line);
    let _ = writeln!(
        out,
        "  clone of:      {}:{} (reaches {}() sink)",
        display_path(&e.vuln_file, root),
        e.vuln_line,
        e.sink_name
    );
    let _ = writeln!(out, "  confidence:    {}", confidence_str(e.confidence));
    if !action.is_empty() {
        let _ = writeln!(out, "  action:        {action}");
    }
    let _ = writeln!(out);
}

pub fn vuln_clone_json(e: &VulnCloneEvidence, root: Option<&Path>) -> serde_json::Value {
    serde_json::json!({
        "kind": "VulnerableCloneSibling",
        "file": display_path(&e.file, root),
        "line": e.line,
        "vuln_file": display_path(&e.vuln_file, root),
        "vuln_line": e.vuln_line,
        "sink": e.sink_name,
        "confidence": confidence_str(e.confidence),
    })
}
