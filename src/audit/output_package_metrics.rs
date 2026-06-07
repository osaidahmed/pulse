use std::fmt::Write;
use std::path::Path;

use crate::thresholds::AuditThresholds;

use super::finding::{CycleMembership, MartinMetrics, MartinTier};
use super::output_helpers::{confidence_str, display_path, write_capped_list, ListLayout};

fn write_action_row(out: &mut String, action: &str) {
    if !action.is_empty() {
        let _ = writeln!(out, "  action:        {action}");
    }
}

pub fn write_martin(out: &mut String, m: &MartinMetrics, root: Option<&Path>, action: &'static str) {
    let tier = tier_str(m.tier);
    let _ = writeln!(out, "audit: distance from main sequence");
    let _ = writeln!(out, "  module:        {}", display_path(&m.module, root));
    let _ = writeln!(out, "  Ca:            {}", m.afferent);
    let _ = writeln!(out, "  Ce:            {}", m.efferent);
    let _ = writeln!(out, "  instability:   {:.3}", m.instability);
    let _ = writeln!(out, "  abstractness:  {:.3}", m.abstractness);
    let _ = writeln!(out, "  distance:      {:.3}", m.distance);
    let _ = writeln!(out, "  tier:          {tier}");
    let _ = writeln!(out, "  confidence:    {}", confidence_str(m.confidence));
    write_action_row(out, action);
    let _ = writeln!(out);
}

pub fn write_cycle(
    out: &mut String,
    c: &CycleMembership,
    root: Option<&Path>,
    t: &AuditThresholds,
    action: &'static str,
) {
    let _ = writeln!(
        out,
        "audit: import cycle (size {}, {} shape)",
        c.members.len(),
        super::cycle_shapes::shape_label(c.shape)
    );
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
    let _ = writeln!(out, "  centrality:    {:.4}", c.centrality);
    if let Some((src, dst)) = &c.feedback_edge {
        let _ = writeln!(
            out,
            "  suggested cut: {} -> {}",
            display_path(src, root),
            display_path(dst, root)
        );
    }
    let _ = writeln!(out, "  confidence:    {}", confidence_str(c.confidence));
    write_action_row(out, action);
    let _ = writeln!(out);
}

pub fn write_zero_edge(out: &mut String, module_count: u32, action: &'static str) {
    let _ = writeln!(
        out,
        "audit: no import edges resolved across {module_count} modules"
    );
    let _ = writeln!(out, "  note: per-module distance findings suppressed.");
    let _ = writeln!(
        out,
        "  hint: confirm --root points at the project's source tree."
    );
    write_action_row(out, action);
    let _ = writeln!(out);
}

pub fn martin_json(m: &MartinMetrics, root: Option<&Path>) -> serde_json::Value {
    let tier = tier_str(m.tier);
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

pub fn cycle_json(c: &CycleMembership, root: Option<&Path>) -> serde_json::Value {
    let members: Vec<String> = c.members.iter().map(|p| display_path(p, root)).collect();
    let edges: Vec<serde_json::Value> = c
        .edges
        .iter()
        .map(|(src, dst)| serde_json::json!([display_path(src, root), display_path(dst, root)]))
        .collect();
    let feedback = c
        .feedback_edge
        .as_ref()
        .map(|(src, dst)| serde_json::json!([display_path(src, root), display_path(dst, root)]));
    serde_json::json!({
        "kind": "ImportCycle",
        "size": c.members.len(),
        "shape": super::cycle_shapes::shape_label(c.shape),
        "members": members,
        "edges": edges,
        "centrality": c.centrality,
        "feedback_edge": feedback,
        "confidence": confidence_str(c.confidence),
    })
}

pub fn zero_edge_json(module_count: u32) -> serde_json::Value {
    serde_json::json!({"kind": "ZeroEdgeProject", "module_count": module_count})
}

fn tier_str(t: MartinTier) -> &'static str {
    match t {
        MartinTier::Healthy => "healthy",
        MartinTier::Warning => "warning",
        MartinTier::Alert => "alert",
    }
}
