use std::fmt::Write;
use std::path::Path;

use super::finding::{
    AuditFinding, AuditKind, BloatedDepEvidence, ConstraintEvidence, PhantomDepEvidence, UndeclaredModuleDepEvidence,
    UnusedDeclaredDepEvidence,
};
use super::output_helpers::{confidence_str, display_path};

pub fn dispatch_human(out: &mut String, f: &AuditFinding, root: Option<&Path>, action: &'static str) -> bool {
    match &f.kind {
        AuditKind::BloatedDependency(e) => write_bloated(out, e, root, action),
        AuditKind::PhantomDependency(e) => write_phantom(out, e, root, action),
        AuditKind::ConstraintSmell(e) => write_constraint(out, e, root, action),
        AuditKind::UndeclaredModuleDependency(e) => write_undeclared(out, e, root, action),
        AuditKind::UnusedDeclaredDependency(e) => write_unused(out, e, root, action),
        _ => return false,
    }
    true
}

pub fn dispatch_json(f: &AuditFinding, root: Option<&Path>) -> Option<serde_json::Value> {
    match &f.kind {
        AuditKind::BloatedDependency(e) => Some(serde_json::json!({
            "kind": "BloatedDependency",
            "manifest": display_path(&e.manifest, root),
            "line": e.line,
            "name": e.name,
            "constraint": e.constraint,
            "confidence": confidence_str(e.confidence),
        })),
        AuditKind::PhantomDependency(e) => Some(serde_json::json!({
            "kind": "PhantomDependency",
            "file": display_path(&e.file, root),
            "line": e.line,
            "name": e.name,
            "confidence": confidence_str(e.confidence),
        })),
        AuditKind::ConstraintSmell(e) => Some(serde_json::json!({
            "kind": "ConstraintSmell",
            "manifest": display_path(&e.manifest, root),
            "line": e.line,
            "name": e.name,
            "constraint": e.constraint,
            "problem": e.problem,
            "confidence": confidence_str(e.confidence),
        })),
        AuditKind::UndeclaredModuleDependency(e) => Some(serde_json::json!({
            "kind": "UndeclaredModuleDependency",
            "from_component": e.from_component,
            "to_component": e.to_component,
            "file": display_path(&e.file, root),
            "line": e.line,
            "import": e.import_target,
            "confidence": confidence_str(e.confidence),
        })),
        AuditKind::UnusedDeclaredDependency(e) => Some(serde_json::json!({
            "kind": "UnusedDeclaredDependency",
            "manifest": display_path(&e.manifest, root),
            "line": e.line,
            "from_component": e.from_component,
            "to_component": e.to_component,
            "confidence": confidence_str(e.confidence),
        })),
        _ => None,
    }
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

fn write_undeclared(out: &mut String, e: &UndeclaredModuleDepEvidence, root: Option<&Path>, action: &'static str) {
    let _ = writeln!(out, "audit: undeclared module dependency — {} → {}", e.from_component, e.to_component);
    let _ = writeln!(out, "  imported at:   {}:{}", display_path(&e.file, root), e.line);
    let _ = writeln!(out, "  import:        {}", e.import_target);
    let _ = writeln!(out, "  declared:      `{}` declares no dependency on `{}`", e.from_component, e.to_component);
    let _ = writeln!(out, "  confidence:    {}", confidence_str(e.confidence));
    if !action.is_empty() {
        let _ = writeln!(out, "  action:        {action}");
    }
    let _ = writeln!(out);
}

fn write_unused(out: &mut String, e: &UnusedDeclaredDepEvidence, root: Option<&Path>, action: &'static str) {
    let _ = writeln!(out, "audit: unused declared dependency — {} → {}", e.from_component, e.to_component);
    let _ = writeln!(out, "  declared at:   {}:{}", display_path(&e.manifest, root), e.line);
    let _ = writeln!(
        out,
        "  usage:         no source file in `{}` imports or references `{}`",
        e.from_component, e.to_component
    );
    let _ = writeln!(out, "  confidence:    {}", confidence_str(e.confidence));
    if !action.is_empty() {
        let _ = writeln!(out, "  action:        {action}");
    }
    let _ = writeln!(out);
}
