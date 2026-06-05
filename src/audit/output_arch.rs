use std::fmt::Write;
use std::path::Path;

use super::finding::AuditKind;
use super::output_helpers::{confidence_str, display_path};

pub fn write_arch(out: &mut String, kind: &AuditKind, root: Option<&Path>, action: &'static str) -> bool {
    match kind {
        AuditKind::UnstableDependency(e) => {
            let _ = writeln!(out, "audit: unstable dependency — {}", display_path(&e.component, root));
            let _ = writeln!(out, "  instability:   {:.3}", e.instability);
            let _ = writeln!(
                out,
                "  strength:      {:.3} ({} of {} deps less stable)",
                e.strength, e.unstable_deps, e.total_deps
            );
            let _ = writeln!(out, "  gap:           {:.3}", e.gap);
            let _ = writeln!(out, "  confidence:    {}", confidence_str(e.confidence));
        }
        AuditKind::HubLikeDependency(e) => {
            let _ = writeln!(out, "audit: hub-like dependency — {}", display_path(&e.component, root));
            let _ = writeln!(out, "  Ca:            {}", e.afferent);
            let _ = writeln!(out, "  Ce:            {}", e.efferent);
            let _ = writeln!(out, "  imbalance:     {}", e.imbalance);
            let _ = writeln!(out, "  confidence:    {}", confidence_str(e.confidence));
        }
        _ => return false,
    }
    if !action.is_empty() {
        let _ = writeln!(out, "  action:        {action}");
    }
    let _ = writeln!(out);
    true
}

pub fn arch_json(kind: &AuditKind, root: Option<&Path>) -> Option<serde_json::Value> {
    match kind {
        AuditKind::UnstableDependency(e) => Some(serde_json::json!({
            "kind": "UnstableDependency",
            "component": display_path(&e.component, root),
            "instability": e.instability,
            "strength": e.strength,
            "gap": e.gap,
            "unstable_deps": e.unstable_deps,
            "total_deps": e.total_deps,
            "confidence": confidence_str(e.confidence),
        })),
        AuditKind::HubLikeDependency(e) => Some(serde_json::json!({
            "kind": "HubLikeDependency",
            "component": display_path(&e.component, root),
            "afferent": e.afferent,
            "efferent": e.efferent,
            "imbalance": e.imbalance,
            "confidence": confidence_str(e.confidence),
        })),
        _ => None,
    }
}
