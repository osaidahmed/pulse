use std::fmt::Write;
use std::path::Path;

use super::finding::{
    AuditKind, ConceptualCohesionEvidence, DivergentChangeEvidence, FeatureEnvyEvidence, GodClassEvidence,
    ImportConfidence, MultivariateAnomalyEvidence, ParallelInheritanceEvidence, RefusedBequestEvidence,
    ShotgunSurgeryEvidence,
};

pub struct WriterCtx<'a> {
    pub root: Option<&'a Path>,
    pub confidence_str: fn(ImportConfidence) -> &'static str,
    pub display_path: fn(&Path, Option<&Path>) -> String,
    pub action: &'static str,
}

pub fn dispatch_human(out: &mut String, kind: &AuditKind, ctx: &WriterCtx) -> bool {
    if let AuditKind::DivergentChange(e) = kind {
        write_divergent(out, e, ctx);
        return true;
    }
    if let AuditKind::FeatureEnvy(e) = kind {
        write_feature_envy(out, e, ctx);
        return true;
    }
    if let AuditKind::GodClass(e) = kind {
        write_god_class(out, e, ctx);
        return true;
    }
    if let AuditKind::ParallelInheritance(e) = kind {
        write_parallel(out, e, ctx);
        return true;
    }
    if let AuditKind::RefusedBequest(e) = kind {
        write_refused(out, e, ctx);
        return true;
    }
    if let AuditKind::LowConceptualCohesion(e) = kind {
        write_cohesion(out, e, ctx);
        return true;
    }
    if let AuditKind::MultivariateAnomaly(e) = kind {
        write_multivariate(out, e, ctx);
        return true;
    }
    false
}

pub fn dispatch_json(
    kind: &AuditKind,
    root: Option<&Path>,
    confidence_str: fn(ImportConfidence) -> &'static str,
    display_path_for: fn(&Path, Option<&Path>) -> String,
) -> Option<serde_json::Value> {
    if let AuditKind::DivergentChange(e) = kind {
        return Some(divergent_json(e, root, confidence_str, display_path_for));
    }
    if let AuditKind::FeatureEnvy(e) = kind {
        return Some(feature_envy_json(e, root, confidence_str, display_path_for));
    }
    if let AuditKind::GodClass(e) = kind {
        return Some(god_class_json(e, root, confidence_str, display_path_for));
    }
    if let AuditKind::ParallelInheritance(e) = kind {
        return Some(parallel_json(e, root, confidence_str, display_path_for));
    }
    if let AuditKind::RefusedBequest(e) = kind {
        return Some(refused_json(e, root, confidence_str, display_path_for));
    }
    if let AuditKind::LowConceptualCohesion(e) = kind {
        return Some(cohesion_json(e, root, confidence_str, display_path_for));
    }
    if let AuditKind::MultivariateAnomaly(e) = kind {
        return Some(multivariate_json(e, root, confidence_str, display_path_for));
    }
    None
}

fn write_action_row(out: &mut String, action: &str) {
    if !action.is_empty() {
        let _ = writeln!(out, "  action:        {action}");
    }
}

fn write_divergent(out: &mut String, e: &DivergentChangeEvidence, ctx: &WriterCtx) {
    let _ = writeln!(out, "audit: divergent change — {}", e.class_name);
    let _ = writeln!(out, "  file:          {}", (ctx.display_path)(&e.class_file, ctx.root));
    let _ = writeln!(out, "  CC:            {}", e.changing_classes);
    let _ = writeln!(out, "  fanout:        {}", e.fanout);
    let _ = writeln!(out, "  method count:  {}", e.method_count);
    let _ = writeln!(out, "  confidence:    {}", (ctx.confidence_str)(e.confidence));
    write_action_row(out, ctx.action);
    let _ = writeln!(out);
}

fn write_feature_envy(out: &mut String, e: &FeatureEnvyEvidence, ctx: &WriterCtx) {
    let label = qualified_method_name(e.method_class.as_deref(), &e.method_name);
    let _ = writeln!(out, "audit: feature envy — {label}");
    let _ = writeln!(out, "  defined at:    {}:{}", (ctx.display_path)(&e.method_file, ctx.root), e.method_line);
    let _ = writeln!(out, "  ATFD:          {}", e.atfd);
    let _ = writeln!(out, "  foreign calls: {}", e.foreign_call_count);
    let _ = writeln!(out, "  intra calls:   {}", e.intra_call_count);
    if let Some(envied) = &e.envied_class {
        let _ = writeln!(out, "  envied class:  {envied}");
    } else {
        let _ = writeln!(out, "  envied class:  (unresolved)");
    }
    let _ = writeln!(out, "  confidence:    {}", (ctx.confidence_str)(e.confidence));
    write_action_row(out, ctx.action);
    let _ = writeln!(out);
}

fn write_god_class(out: &mut String, e: &GodClassEvidence, ctx: &WriterCtx) {
    let _ = writeln!(out, "audit: god class — {}", e.class_name);
    let _ = writeln!(out, "  file:          {}", (ctx.display_path)(&e.class_file, ctx.root));
    let _ = writeln!(out, "  WMC:           {}", e.wmc);
    let _ = writeln!(out, "  TCC:           {:.3}", e.tcc);
    let _ = writeln!(out, "  ATFD:          {}", e.atfd);
    let _ = writeln!(out, "  method count:  {}", e.method_count);
    let _ = writeln!(out, "  confidence:    {}", (ctx.confidence_str)(e.confidence));
    write_action_row(out, ctx.action);
    let _ = writeln!(out);
}

fn write_parallel(out: &mut String, e: &ParallelInheritanceEvidence, ctx: &WriterCtx) {
    let _ = writeln!(out, "audit: parallel inheritance — {} ↔ {}", e.root_a.name, e.root_b.name);
    let _ = writeln!(out, "  root A file:   {}", (ctx.display_path)(&e.root_a.file, ctx.root));
    let _ = writeln!(out, "  root B file:   {}", (ctx.display_path)(&e.root_b.file, ctx.root));
    let _ = writeln!(out, "  matched pairs: {}", e.matched_descendants.len());
    for (a, b) in e.matched_descendants.iter().take(20) {
        let _ = writeln!(out, "                 {a} ↔ {b}");
    }
    let _ = writeln!(out, "  confidence:    {}", (ctx.confidence_str)(e.confidence));
    write_action_row(out, ctx.action);
    let _ = writeln!(out);
}

fn write_refused(out: &mut String, e: &RefusedBequestEvidence, ctx: &WriterCtx) {
    let _ = writeln!(out, "audit: refused bequest — {} extends {}", e.subclass_name, e.parent_name);
    let _ = writeln!(out, "  subclass file: {}", (ctx.display_path)(&e.subclass_file, ctx.root));
    let _ = writeln!(out, "  parent file:   {}", (ctx.display_path)(&e.parent_file, ctx.root));
    let _ = writeln!(out, "  overrides:     {} of {}", e.override_count, e.parent_method_count);
    let _ = writeln!(out, "  ratio:         {:.3}", e.override_ratio);
    let _ = writeln!(out, "  confidence:    {}", (ctx.confidence_str)(e.confidence));
    write_action_row(out, ctx.action);
    let _ = writeln!(out);
}

fn divergent_json(
    e: &DivergentChangeEvidence,
    root: Option<&Path>,
    conf_fn: fn(ImportConfidence) -> &'static str,
    path_fn: fn(&Path, Option<&Path>) -> String,
) -> serde_json::Value {
    serde_json::json!({
        "kind": "DivergentChange",
        "class_file": path_fn(&e.class_file, root),
        "class_name": e.class_name,
        "changing_classes": e.changing_classes,
        "fanout": e.fanout,
        "method_count": e.method_count,
        "confidence": conf_fn(e.confidence),
    })
}

fn feature_envy_json(
    e: &FeatureEnvyEvidence,
    root: Option<&Path>,
    conf_fn: fn(ImportConfidence) -> &'static str,
    path_fn: fn(&Path, Option<&Path>) -> String,
) -> serde_json::Value {
    serde_json::json!({
        "kind": "FeatureEnvy",
        "method_file": path_fn(&e.method_file, root),
        "method_class": e.method_class,
        "method_name": e.method_name,
        "method_line": e.method_line,
        "atfd": e.atfd,
        "foreign_call_count": e.foreign_call_count,
        "intra_call_count": e.intra_call_count,
        "envied_class": e.envied_class,
        "confidence": conf_fn(e.confidence),
    })
}

fn god_class_json(
    e: &GodClassEvidence,
    root: Option<&Path>,
    conf_fn: fn(ImportConfidence) -> &'static str,
    path_fn: fn(&Path, Option<&Path>) -> String,
) -> serde_json::Value {
    serde_json::json!({
        "kind": "GodClass",
        "class_file": path_fn(&e.class_file, root),
        "class_name": e.class_name,
        "wmc": e.wmc,
        "tcc": e.tcc,
        "atfd": e.atfd,
        "method_count": e.method_count,
        "confidence": conf_fn(e.confidence),
    })
}

fn parallel_json(
    e: &ParallelInheritanceEvidence,
    root: Option<&Path>,
    conf_fn: fn(ImportConfidence) -> &'static str,
    path_fn: fn(&Path, Option<&Path>) -> String,
) -> serde_json::Value {
    let pairs: Vec<serde_json::Value> = e.matched_descendants.iter().map(|(a, b)| serde_json::json!([a, b])).collect();
    serde_json::json!({
        "kind": "ParallelInheritance",
        "root_a_file": path_fn(&e.root_a.file, root),
        "root_a_name": e.root_a.name,
        "root_b_file": path_fn(&e.root_b.file, root),
        "root_b_name": e.root_b.name,
        "matched_descendants": pairs,
        "confidence": conf_fn(e.confidence),
    })
}

fn refused_json(
    e: &RefusedBequestEvidence,
    root: Option<&Path>,
    conf_fn: fn(ImportConfidence) -> &'static str,
    path_fn: fn(&Path, Option<&Path>) -> String,
) -> serde_json::Value {
    serde_json::json!({
        "kind": "RefusedBequest",
        "subclass_file": path_fn(&e.subclass_file, root),
        "subclass_name": e.subclass_name,
        "parent_file": path_fn(&e.parent_file, root),
        "parent_name": e.parent_name,
        "override_count": e.override_count,
        "parent_method_count": e.parent_method_count,
        "override_ratio": e.override_ratio,
        "confidence": conf_fn(e.confidence),
    })
}

fn write_cohesion(out: &mut String, e: &ConceptualCohesionEvidence, ctx: &WriterCtx) {
    let _ = writeln!(out, "audit: low conceptual cohesion — {}", e.class_name);
    let _ = writeln!(out, "  file:          {}", (ctx.display_path)(&e.class_file, ctx.root));
    let _ = writeln!(out, "  cohesion:      {:.3}", e.cohesion);
    let _ = writeln!(out, "  method count:  {}", e.method_count);
    let _ = writeln!(out, "  confidence:    {}", (ctx.confidence_str)(e.confidence));
    write_action_row(out, ctx.action);
    let _ = writeln!(out);
}

fn cohesion_json(
    e: &ConceptualCohesionEvidence,
    root: Option<&Path>,
    conf_fn: fn(ImportConfidence) -> &'static str,
    path_fn: fn(&Path, Option<&Path>) -> String,
) -> serde_json::Value {
    serde_json::json!({
        "kind": "LowConceptualCohesion",
        "class_file": path_fn(&e.class_file, root),
        "class_name": e.class_name,
        "cohesion": e.cohesion,
        "method_count": e.method_count,
        "confidence": conf_fn(e.confidence),
    })
}

fn write_multivariate(out: &mut String, e: &MultivariateAnomalyEvidence, ctx: &WriterCtx) {
    let _ = writeln!(out, "audit: multivariate class anomaly — {}", e.class_name);
    let _ = writeln!(out, "  file:          {}", (ctx.display_path)(&e.class_file, ctx.root));
    let _ = writeln!(out, "  WMC:           {}", e.wmc);
    let _ = writeln!(out, "  TCC:           {:.3}", e.tcc);
    let _ = writeln!(out, "  ATFD:          {}", e.atfd);
    let _ = writeln!(out, "  N4:            {}", e.n4);
    let _ = writeln!(out, "  distance²:     {:.2}", e.mahalanobis_sq);
    let _ = writeln!(out, "  confidence:    {}", (ctx.confidence_str)(e.confidence));
    write_action_row(out, ctx.action);
    let _ = writeln!(out);
}

fn multivariate_json(
    e: &MultivariateAnomalyEvidence,
    root: Option<&Path>,
    conf_fn: fn(ImportConfidence) -> &'static str,
    path_fn: fn(&Path, Option<&Path>) -> String,
) -> serde_json::Value {
    serde_json::json!({
        "kind": "MultivariateAnomaly",
        "class_file": path_fn(&e.class_file, root),
        "class_name": e.class_name,
        "wmc": e.wmc,
        "tcc": e.tcc,
        "atfd": e.atfd,
        "n4": e.n4,
        "mahalanobis_sq": e.mahalanobis_sq,
        "confidence": conf_fn(e.confidence),
    })
}

fn qualified_method_name(class: Option<&str>, name: &str) -> String {
    class.map_or_else(|| name.to_string(), |c| format!("{c}.{name}"))
}

pub fn dummy_unused_shotgun(_e: &ShotgunSurgeryEvidence) {}
