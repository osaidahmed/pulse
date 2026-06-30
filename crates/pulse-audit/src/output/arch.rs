use std::fmt::Write;
use std::path::Path;

use crate::finding::AuditKind;
use crate::output::helpers::{confidence_str, display_path};

pub fn write_arch(out: &mut String, kind: &AuditKind, root: Option<&Path>, action: &'static str) -> bool {
    if !render_arch(out, kind, root) {
        return false;
    }
    if !action.is_empty() {
        let _ = writeln!(out, "  action:        {action}");
    }
    let _ = writeln!(out);
    true
}

fn render_arch(out: &mut String, kind: &AuditKind, root: Option<&Path>) -> bool {
    render_component_smell(out, kind, root) || render_remod(out, kind, root)
}

fn render_component_smell(out: &mut String, kind: &AuditKind, root: Option<&Path>) -> bool {
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
            let _ = writeln!(out, "  centrality:    {:.4}", e.centrality);
            let _ = writeln!(out, "  confidence:    {}", confidence_str(e.confidence));
        }
        AuditKind::HubLikeDependency(e) => {
            let _ = writeln!(out, "audit: hub-like dependency — {}", display_path(&e.component, root));
            let _ = writeln!(out, "  Ca:            {}", e.afferent);
            let _ = writeln!(out, "  Ce:            {}", e.efferent);
            let _ = writeln!(out, "  imbalance:     {}", e.imbalance);
            let _ = writeln!(out, "  centrality:    {:.4}", e.centrality);
            let _ = writeln!(out, "  confidence:    {}", confidence_str(e.confidence));
        }
        AuditKind::GodComponent(e) => {
            let _ = writeln!(out, "audit: god component — {}", display_path(&e.component, root));
            let _ = writeln!(out, "  LOC:           {}", e.loc);
            let _ = writeln!(out, "  files:         {}", e.file_count);
            let _ = writeln!(out, "  density:       {:.1} LOC/file", e.density);
            let _ = writeln!(out, "  centrality:    {:.4}", e.centrality);
            let _ = writeln!(out, "  confidence:    {}", confidence_str(e.confidence));
        }
        AuditKind::OverFragmentation(e) => {
            let _ = writeln!(out, "audit: over-fragmented directory — {}", display_path(&e.component, root));
            let _ = writeln!(out, "  coupled files: {}", e.coupled_files);
            let _ = writeln!(out, "  avg size:      {} LOC/file", e.avg_loc);
            let _ = writeln!(out, "  intra-imports: {}", e.internal_edges);
            let _ = writeln!(out, "  density:       {:.2} imports/file", e.density);
            let _ = writeln!(out, "  confidence:    {}", confidence_str(e.confidence));
        }
        AuditKind::CompoundArchSmell(e) => {
            let _ = writeln!(out, "audit: compound architecture smell — {}", display_path(&e.component, root));
            let _ = writeln!(out, "  smells:        {}", e.constituent_kinds.join(" + "));
            let _ = writeln!(out, "  severity:      {:.4}", e.combined_severity);
            let _ = writeln!(out, "  confidence:    {}", confidence_str(e.confidence));
        }
        AuditKind::SplitComponent(e) => {
            let _ = writeln!(out, "audit: component to split — {}", display_path(&e.component, root));
            let _ = writeln!(out, "  files:         {}", e.file_count);
            let _ = writeln!(out, "  communities:   {}", e.community_count);
            let _ = writeln!(out, "  cohesion:      {:.2}", e.cohesion);
            let _ = writeln!(out, "  confidence:    {}", confidence_str(e.confidence));
        }
        _ => return false,
    }
    true
}

fn render_remod(out: &mut String, kind: &AuditKind, root: Option<&Path>) -> bool {
    match kind {
        AuditKind::MoveFile(e) => {
            let _ = writeln!(out, "audit: file to relocate — {}", display_path(&e.file, root));
            let _ = writeln!(out, "  current dir:   {}", display_path(&e.current_dir, root));
            let _ = writeln!(out, "  target dir:    {}", display_path(&e.target_dir, root));
            let _ =
                writeln!(out, "  community:     {} files, {:.0}% in target", e.community_size, e.home_share * 100.0);
            let _ = writeln!(out, "  confidence:    {}", confidence_str(e.confidence));
        }
        AuditKind::MergeComponents(e) => {
            let dirs: Vec<String> = e.components.iter().map(|c| display_path(c, root)).collect();
            let _ = writeln!(out, "audit: components to merge — {}", dirs.join(", "));
            let _ = writeln!(out, "  shared community: {} files", e.community_files);
            let _ = writeln!(out, "  confidence:    {}", confidence_str(e.confidence));
        }
        _ => return false,
    }
    true
}

pub fn arch_json(kind: &AuditKind, root: Option<&Path>) -> Option<serde_json::Value> {
    component_json(kind, root).or_else(|| remod_json(kind, root))
}

fn component_json(kind: &AuditKind, root: Option<&Path>) -> Option<serde_json::Value> {
    match kind {
        AuditKind::UnstableDependency(e) => Some(serde_json::json!({
            "kind": "UnstableDependency",
            "component": display_path(&e.component, root),
            "instability": e.instability,
            "strength": e.strength,
            "gap": e.gap,
            "unstable_deps": e.unstable_deps,
            "total_deps": e.total_deps,
            "centrality": e.centrality,
            "confidence": confidence_str(e.confidence),
        })),
        AuditKind::HubLikeDependency(e) => Some(serde_json::json!({
            "kind": "HubLikeDependency",
            "component": display_path(&e.component, root),
            "afferent": e.afferent,
            "efferent": e.efferent,
            "imbalance": e.imbalance,
            "centrality": e.centrality,
            "confidence": confidence_str(e.confidence),
        })),
        AuditKind::GodComponent(e) => Some(serde_json::json!({
            "kind": "GodComponent",
            "component": display_path(&e.component, root),
            "loc": e.loc,
            "file_count": e.file_count,
            "density": e.density,
            "centrality": e.centrality,
            "confidence": confidence_str(e.confidence),
        })),
        AuditKind::OverFragmentation(e) => Some(serde_json::json!({
            "kind": "OverFragmentation",
            "component": display_path(&e.component, root),
            "coupled_files": e.coupled_files,
            "avg_loc": e.avg_loc,
            "internal_edges": e.internal_edges,
            "density": e.density,
            "confidence": confidence_str(e.confidence),
        })),
        AuditKind::CompoundArchSmell(e) => Some(serde_json::json!({
            "kind": "CompoundArchSmell",
            "component": display_path(&e.component, root),
            "constituent_kinds": e.constituent_kinds,
            "combined_severity": e.combined_severity,
            "confidence": confidence_str(e.confidence),
        })),
        AuditKind::SplitComponent(e) => Some(serde_json::json!({
            "kind": "SplitComponent",
            "component": display_path(&e.component, root),
            "file_count": e.file_count,
            "community_count": e.community_count,
            "cohesion": e.cohesion,
            "confidence": confidence_str(e.confidence),
        })),
        _ => None,
    }
}

fn remod_json(kind: &AuditKind, root: Option<&Path>) -> Option<serde_json::Value> {
    match kind {
        AuditKind::MoveFile(e) => Some(serde_json::json!({
            "kind": "MoveFile",
            "file": display_path(&e.file, root),
            "current_dir": display_path(&e.current_dir, root),
            "target_dir": display_path(&e.target_dir, root),
            "community_size": e.community_size,
            "home_share": e.home_share,
            "confidence": confidence_str(e.confidence),
        })),
        AuditKind::MergeComponents(e) => Some(serde_json::json!({
            "kind": "MergeComponents",
            "components": e.components.iter().map(|c| display_path(c, root)).collect::<Vec<_>>(),
            "community_files": e.community_files,
            "confidence": confidence_str(e.confidence),
        })),
        _ => None,
    }
}
