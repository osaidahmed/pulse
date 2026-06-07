use std::fmt::Write;
use std::path::Path;

use crate::thresholds::AuditThresholds;

use super::finding::CloneClusterEvidence;
use super::output_helpers::{confidence_str, display_path, write_capped_list, ListLayout};

pub fn write_clone(
    out: &mut String,
    e: &CloneClusterEvidence,
    root: Option<&Path>,
    t: &AuditThresholds,
    action: &'static str,
) {
    let _ = writeln!(out, "audit: near-duplicate clones — {} fragments (~{} lines)", e.member_count, e.max_loc);
    if !e.representative.is_empty() {
        let _ = writeln!(out, "  representative: {}", e.representative);
    }
    let layout = ListLayout {
        prefix_first: "  members:       ",
        prefix_rest: "                 ",
        cap: t.max_locations_per_finding,
    };
    write_capped_list(out, &layout, e.members.len(), |i| {
        let loc = &e.members[i];
        format!("{}:{}", display_path(&loc.file, root), loc.line)
    });
    let _ = writeln!(out, "  confidence:    {}", confidence_str(e.confidence));
    if !action.is_empty() {
        let _ = writeln!(out, "  action:        {action}");
    }
    let _ = writeln!(out);
}

pub fn clone_json(e: &CloneClusterEvidence, root: Option<&Path>) -> serde_json::Value {
    let members: Vec<serde_json::Value> = e
        .members
        .iter()
        .map(|loc| serde_json::json!({"file": display_path(&loc.file, root), "line": loc.line}))
        .collect();
    serde_json::json!({
        "kind": "NearDuplicate",
        "member_count": e.member_count,
        "max_loc": e.max_loc,
        "representative": e.representative,
        "members": members,
        "confidence": confidence_str(e.confidence),
    })
}
