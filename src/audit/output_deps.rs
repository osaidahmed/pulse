use std::fmt::Write;
use std::path::Path;

use super::finding::{
    AuditFinding, AuditKind, BloatedDepEvidence, ConstraintEvidence, ImportConfidence, PhantomDepEvidence,
    UndeclaredModuleDepEvidence, UnusedDeclaredDepEvidence,
};
use super::output_helpers::{confidence_str, display_path};

pub fn dispatch_human(out: &mut String, f: &AuditFinding, root: Option<&Path>, action: &'static str) -> bool {
    match &f.kind {
        AuditKind::BloatedDependency(e) => write_bloated(out, e, root, action),
        AuditKind::PhantomDependency(e) => write_phantom(out, e, root, action),
        AuditKind::ConstraintSmell(e) => write_constraint(out, e, root, action),
        AuditKind::UndeclaredModuleDependency(e) => write_undeclared(out, e, root, action),
        AuditKind::UnusedDeclaredDependency(e) => write_unused(out, e, root, action),
        AuditKind::StrictnessDebt(e) => write_metric_block(
            out,
            &MetricBlock {
                title: "type strictness debt",
                file: &e.file,
                line: e.line,
                loc_label: "first `any`:   ",
                summary: format!(
                    "any count:     {} explicit `any` annotations under a non-strict tsconfig",
                    e.any_count
                ),
                confidence: e.confidence,
            },
            root,
            action,
        ),
        AuditKind::IfdefDensity(e) => write_metric_block(
            out,
            &MetricBlock {
                title: "conditional-compilation density",
                file: &e.file,
                line: e.line,
                loc_label: "first `#if`:    ",
                summary: format!("conditionals:  {} preprocessor conditional blocks", e.conditional_count),
                confidence: e.confidence,
            },
            root,
            action,
        ),
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
        AuditKind::StrictnessDebt(e) => Some(serde_json::json!({
            "kind": "StrictnessDebt",
            "file": display_path(&e.file, root),
            "line": e.line,
            "any_count": e.any_count,
            "confidence": confidence_str(e.confidence),
        })),
        AuditKind::IfdefDensity(e) => Some(serde_json::json!({
            "kind": "IfdefDensity",
            "file": display_path(&e.file, root),
            "line": e.line,
            "conditional_count": e.conditional_count,
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

struct MetricBlock<'a> {
    title: &'a str,
    file: &'a Path,
    line: u32,
    loc_label: &'a str,
    summary: String,
    confidence: ImportConfidence,
}

fn write_metric_block(out: &mut String, b: &MetricBlock, root: Option<&Path>, action: &'static str) {
    let path = display_path(b.file, root);
    let _ = writeln!(out, "audit: {} — {}", b.title, path);
    let _ = writeln!(out, "  {}{}:{}", b.loc_label, path, b.line);
    let _ = writeln!(out, "  {}", b.summary);
    let _ = writeln!(out, "  confidence:    {}", confidence_str(b.confidence));
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
