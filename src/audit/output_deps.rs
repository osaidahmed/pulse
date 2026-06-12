use std::fmt::Write;
use std::path::Path;

use super::finding::{AuditFinding, AuditKind, BloatedDepEvidence, ConstraintEvidence, PhantomDepEvidence};
use super::output_helpers::{confidence_str, display_path};

pub fn dispatch_human(out: &mut String, f: &AuditFinding, root: Option<&Path>, action: &'static str) -> bool {
    if let AuditKind::BloatedDependency(e) = &f.kind {
        write_bloated(out, e, root, action);
        return true;
    }
    if let AuditKind::PhantomDependency(e) = &f.kind {
        write_phantom(out, e, root, action);
        return true;
    }
    if let AuditKind::ConstraintSmell(e) = &f.kind {
        write_constraint(out, e, root, action);
        return true;
    }
    false
}

pub fn dispatch_json(f: &AuditFinding, root: Option<&Path>) -> Option<serde_json::Value> {
    if let AuditKind::BloatedDependency(e) = &f.kind {
        return Some(bloated_json(e, root));
    }
    if let AuditKind::PhantomDependency(e) = &f.kind {
        return Some(phantom_json(e, root));
    }
    if let AuditKind::ConstraintSmell(e) = &f.kind {
        return Some(constraint_json(e, root));
    }
    None
}

fn write_bloated(out: &mut String, e: &BloatedDepEvidence, root: Option<&Path>, action: &'static str) {
    let _ = writeln!(out, "audit: bloated dependency — {}", e.name);
    let _ = writeln!(out, "  declared at:   {}:{}", display_path(&e.manifest, root), e.line);
    if !e.constraint.is_empty() {
        let _ = writeln!(out, "  constraint:    {}", e.constraint);
    }
    let _ = writeln!(out, "  usage:         no analyzed source file imports or references it");
    let _ = writeln!(out, "  confidence:    {}", confidence_str(e.confidence));
    if !action.is_empty() {
        let _ = writeln!(out, "  action:        {action}");
    }
    let _ = writeln!(out);
}

fn write_phantom(out: &mut String, e: &PhantomDepEvidence, root: Option<&Path>, action: &'static str) {
    let _ = writeln!(out, "audit: phantom dependency — {}", e.name);
    let _ = writeln!(out, "  imported at:   {}:{}", display_path(&e.file, root), e.line);
    let _ = writeln!(out, "  resolution:    present only in the lockfile as a transitive dependency");
    let _ = writeln!(out, "  confidence:    {}", confidence_str(e.confidence));
    if !action.is_empty() {
        let _ = writeln!(out, "  action:        {action}");
    }
    let _ = writeln!(out);
}

fn bloated_json(e: &BloatedDepEvidence, root: Option<&Path>) -> serde_json::Value {
    serde_json::json!({
        "kind": "BloatedDependency",
        "manifest": display_path(&e.manifest, root),
        "line": e.line,
        "name": e.name,
        "constraint": e.constraint,
        "confidence": confidence_str(e.confidence),
    })
}

fn phantom_json(e: &PhantomDepEvidence, root: Option<&Path>) -> serde_json::Value {
    serde_json::json!({
        "kind": "PhantomDependency",
        "file": display_path(&e.file, root),
        "line": e.line,
        "name": e.name,
        "confidence": confidence_str(e.confidence),
    })
}

fn write_constraint(out: &mut String, e: &ConstraintEvidence, root: Option<&Path>, action: &'static str) {
    let _ = writeln!(out, "audit: constraint smell — {}", e.name);
    let _ = writeln!(out, "  declared at:   {}:{}", display_path(&e.manifest, root), e.line);
    let _ = writeln!(out, "  problem:       {}", e.problem);
    let _ = writeln!(out, "  confidence:    {}", confidence_str(e.confidence));
    if !action.is_empty() {
        let _ = writeln!(out, "  action:        {action}");
    }
    let _ = writeln!(out);
}

fn constraint_json(e: &ConstraintEvidence, root: Option<&Path>) -> serde_json::Value {
    serde_json::json!({
        "kind": "ConstraintSmell",
        "manifest": display_path(&e.manifest, root),
        "line": e.line,
        "name": e.name,
        "constraint": e.constraint,
        "problem": e.problem,
        "confidence": confidence_str(e.confidence),
    })
}
