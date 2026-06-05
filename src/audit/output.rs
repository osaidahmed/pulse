use std::fmt::Write;
use std::path::{Path, PathBuf};

use crate::config::AuditSuppression;
use crate::thresholds::AuditThresholds;

use super::finding::{
    action_for_kind, finding_confidence, pass_for, AuditFinding, AuditKind, PatternCategory,
    ShotgunSurgeryEvidence,
};
use super::output_helpers::{
    confidence_str, display_path, write_capped_list, ListLayout,
};
use super::output_named_smells::WriterCtx;
use super::output_package_metrics::{
    cycle_json, martin_json, write_cycle, write_martin, write_zero_edge, zero_edge_json,
};

pub fn format_findings(
    findings: &[AuditFinding],
    root: Option<&Path>,
    thresholds: &AuditThresholds,
) -> String {
    let mut out = String::new();
    let collected: Vec<&AuditFinding> = findings.iter().collect();
    let (pattern_findings, other_findings) = super::output_grouped::split(&collected);
    super::output_grouped::render(&mut out, &pattern_findings, root, thresholds, render_pattern_human);
    for f in other_findings {
        render_human(&mut out, f, root, thresholds);
    }
    out
}

pub struct RenderCtx<'a> {
    pub root: Option<&'a Path>,
    pub show_noise: bool,
    pub suppression: &'a AuditSuppression,
}

pub fn format_findings_filtered(
    findings: &[AuditFinding],
    root: Option<&Path>,
    thresholds: &AuditThresholds,
    show_noise: bool,
    suppression: &AuditSuppression,
) -> String {
    let ctx = RenderCtx { root, show_noise, suppression };
    let visible = filter_visible(findings, show_noise, suppression);
    if findings.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    write_human_header(&mut out, findings, &visible, &ctx);
    let pillar_ctx = super::output_sections::PillarCtx {
        root,
        thresholds,
        render_pattern: render_pattern_human,
        render_other: render_human,
    };
    super::output_sections::render_pillars(&mut out, &visible, &pillar_ctx);
    out
}

pub fn format_findings_json(findings: &[AuditFinding], root: Option<&Path>) -> String {
    let entries: Vec<serde_json::Value> = findings
        .iter()
        .map(|f| enrich_json(render_json(f, root), f))
        .collect();
    serde_json::Value::Array(entries).to_string()
}

pub fn format_findings_json_filtered(
    findings: &[AuditFinding],
    root: Option<&Path>,
    show_noise: bool,
    suppression: &AuditSuppression,
) -> String {
    let visible = filter_visible(findings, show_noise, suppression);
    let entries: Vec<serde_json::Value> = visible
        .iter()
        .map(|f| enrich_json(render_json(f, root), f))
        .collect();
    if findings.is_empty() {
        return serde_json::Value::Array(entries).to_string();
    }
    let summary = build_summary(findings, &visible, root);
    serde_json::json!({"summary": summary, "findings": entries}).to_string()
}

fn filter_visible<'a>(
    findings: &'a [AuditFinding],
    show_noise: bool,
    suppression: &AuditSuppression,
) -> Vec<&'a AuditFinding> {
    findings
        .iter()
        .filter(|f| show_noise || !is_hidden_noise(f))
        .filter(|f| !suppression.is_hidden(f))
        .collect()
}

fn is_hidden_noise(f: &AuditFinding) -> bool {
    f.pattern_category.is_some_and(PatternCategory::is_noise)
}

fn enrich_json(mut v: serde_json::Value, f: &AuditFinding) -> serde_json::Value {
    if let Some(obj) = v.as_object_mut() {
        obj.insert("pass".into(), serde_json::Value::String(pass_for(&f.kind).into()));
        obj.insert(
            "category".into(),
            f.pattern_category.map_or(serde_json::Value::Null, |c| {
                serde_json::Value::String(c.slug().into())
            }),
        );
        obj.insert(
            "confidence".into(),
            serde_json::Value::String(confidence_str(finding_confidence(f)).into()),
        );
        obj.insert(
            "action".into(),
            f.action_label.map_or(serde_json::Value::Null, |a| {
                serde_json::Value::String(a.into())
            }),
        );
    }
    v
}

fn build_summary(
    all: &[AuditFinding],
    visible: &[&AuditFinding],
    root: Option<&Path>,
) -> serde_json::Value {
    let hidden = all.len().saturating_sub(visible.len());
    let mut by_pass: std::collections::BTreeMap<&'static str, u32> = std::collections::BTreeMap::new();
    let mut by_confidence: std::collections::BTreeMap<&'static str, u32> = std::collections::BTreeMap::new();
    for f in visible {
        *by_pass.entry(pass_for(&f.kind)).or_insert(0) += 1;
        *by_confidence.entry(confidence_str(finding_confidence(f))).or_insert(0) += 1;
    }
    serde_json::json!({
        "root": root.map_or(String::new(), |r| r.display().to_string()),
        "findings_total": visible.len(),
        "noise_hidden": hidden,
        "by_pass": by_pass,
        "by_confidence": by_confidence,
    })
}

fn write_human_header(
    out: &mut String,
    all: &[AuditFinding],
    visible: &[&AuditFinding],
    ctx: &RenderCtx,
) {
    let total = visible.len();
    let hidden_total = all.len().saturating_sub(total);
    let suppressed = if ctx.suppression.is_empty() {
        0
    } else {
        all.iter().filter(|f| ctx.suppression.is_hidden(f)).count()
    };
    let noise_hidden = hidden_total.saturating_sub(suppressed);
    let counts = aggregate_counts(visible);
    let root_label = ctx.root.map_or_else(String::new, |r| r.display().to_string());
    let _ = writeln!(out, "audit: {root_label} — {total} findings");
    let _ = writeln!(
        out,
        "       {} high · {} medium · {} low confidence",
        counts.high, counts.medium, counts.low
    );
    let _ = writeln!(
        out,
        "       {} pattern-mining · {} package-metrics · {} named-smells",
        counts.pattern, counts.package, counts.named
    );
    if noise_hidden > 0 && !ctx.show_noise {
        let _ = writeln!(
            out,
            "       {noise_hidden} hidden under noise categories  (run with --show-noise to surface)"
        );
    }
    if suppressed > 0 {
        let _ = writeln!(
            out,
            "       {suppressed} hidden by .pulse.toml [audit] suppression rules"
        );
    }
    let _ = writeln!(out);
}

struct HeaderCounts {
    high: u32,
    medium: u32,
    low: u32,
    pattern: u32,
    package: u32,
    named: u32,
}

fn aggregate_counts(visible: &[&AuditFinding]) -> HeaderCounts {
    let mut c = HeaderCounts { high: 0, medium: 0, low: 0, pattern: 0, package: 0, named: 0 };
    for f in visible {
        match confidence_str(finding_confidence(f)) {
            "high" => c.high += 1,
            "medium" => c.medium += 1,
            _ => c.low += 1,
        }
        match pass_for(&f.kind) {
            "pattern-mining" => c.pattern += 1,
            "package-metrics" => c.package += 1,
            _ => c.named += 1,
        }
    }
    c
}


fn render_human(out: &mut String, f: &AuditFinding, root: Option<&Path>, t: &AuditThresholds) {
    if dispatch_known_variants_human(out, f, root, t) {
        return;
    }
    render_pattern_human(out, f, root, t);
}

fn dispatch_known_variants_human(
    out: &mut String,
    f: &AuditFinding,
    root: Option<&Path>,
    t: &AuditThresholds,
) -> bool {
    let action = action_for_kind(&f.kind, f.pattern_category);
    if let AuditKind::DistanceFromMainSequence(m) = &f.kind {
        write_martin(out, m, root, action);
        return true;
    }
    if let AuditKind::ImportCycle(c) = &f.kind {
        write_cycle(out, c, root, t, action);
        return true;
    }
    if let AuditKind::ZeroEdgeProject { module_count } = &f.kind {
        write_zero_edge(out, *module_count, action);
        return true;
    }
    if let AuditKind::ShotgunSurgery(e) = &f.kind {
        write_shotgun(out, e, root, t, action);
        return true;
    }
    if super::output_arch::write_arch(out, &f.kind, root, action) {
        return true;
    }
    if super::output_advisory::dispatch_human(out, f, root, t, action) {
        return true;
    }
    let ctx = WriterCtx { root, confidence_str, display_path, action };
    super::output_named_smells::dispatch_human(out, &f.kind, &ctx)
}

fn render_pattern_human(out: &mut String, f: &AuditFinding, root: Option<&Path>, t: &AuditThresholds) {
    let _ = writeln!(
        out,
        "audit: cross-file pattern in {} files ({} occurrences)",
        f.file_count, f.support
    );
    let _ = writeln!(out, "  representative: {}", f.representative_snippet);
    let unique = unique_file_locations(f);
    let layout = ListLayout {
        prefix_first: "  files:        ",
        prefix_rest: "                ",
        cap: t.max_locations_per_finding,
    };
    write_capped_list(out, &layout, unique.len(), |i| {
        let (path, line) = &unique[i];
        format!("{}:{}", display_path(path, root), line)
    });
    let _ = writeln!(out, "  confidence:    {}", confidence_str(finding_confidence(f)));
    if let Some(label) = f.action_label {
        let _ = writeln!(out, "  pattern action: {label}");
    }
    let _ = writeln!(out);
}

fn render_json(f: &AuditFinding, root: Option<&Path>) -> serde_json::Value {
    if let Some(v) = dispatch_known_variants_json(f, root) {
        return v;
    }
    if let Some(v) = super::output_named_smells::dispatch_json(&f.kind, root, confidence_str, display_path) {
        return v;
    }
    let AuditKind::UncategorizedPattern { fingerprint } = &f.kind else {
        unreachable!()
    };
    pattern_json(f, *fingerprint, root)
}

fn dispatch_known_variants_json(f: &AuditFinding, root: Option<&Path>) -> Option<serde_json::Value> {
    if let AuditKind::DistanceFromMainSequence(m) = &f.kind {
        return Some(martin_json(m, root));
    }
    if let AuditKind::ImportCycle(c) = &f.kind {
        return Some(cycle_json(c, root));
    }
    if let AuditKind::ZeroEdgeProject { module_count } = &f.kind {
        return Some(zero_edge_json(*module_count));
    }
    if let AuditKind::ShotgunSurgery(e) = &f.kind {
        return Some(shotgun_json(e, root));
    }
    if let Some(v) = super::output_arch::arch_json(&f.kind, root) {
        return Some(v);
    }
    if let Some(v) = super::output_advisory::dispatch_json(f, root) {
        return Some(v);
    }
    None
}

fn write_shotgun(
    out: &mut String,
    e: &ShotgunSurgeryEvidence,
    root: Option<&Path>,
    t: &AuditThresholds,
    action: &'static str,
) {
    let label = e.method_class.as_deref().map_or_else(
        || e.method_name.clone(),
        |c| format!("{c}.{}", e.method_name),
    );
    if e.name_collision_count > 1 {
        let _ = writeln!(
            out,
            "audit: shotgun surgery — {label} ({} same-named definitions)",
            e.name_collision_count
        );
        let _ = writeln!(out, "  note: name-collision; review per-definition fanout");
    } else {
        let _ = writeln!(out, "audit: shotgun surgery — {label}");
    }
    let _ = writeln!(
        out,
        "  defined at:    {}:{}",
        display_path(&e.method_file, root),
        e.method_line
    );
    if !e.additional_definitions.is_empty() {
        let layout = ListLayout {
            prefix_first: "  also defined:  ",
            prefix_rest: "                 ",
            cap: t.named_smells.max_caller_samples_per_finding,
        };
        write_capped_list(out, &layout, e.additional_definitions.len(), |i| {
            let loc = &e.additional_definitions[i];
            format!("{}:{}", display_path(&loc.file, root), loc.line)
        });
    }
    let _ = writeln!(out, "  CC:            {}", e.changing_classes);
    let _ = writeln!(out, "  CM:            {}", e.changing_methods);
    let _ = writeln!(out, "  fanout:        {}", e.fanout);
    let _ = writeln!(out, "  confidence:    {}", confidence_str(e.confidence));
    let layout = ListLayout {
        prefix_first: "  callers:       ",
        prefix_rest: "                 ",
        cap: t.named_smells.max_caller_samples_per_finding,
    };
    write_capped_list(out, &layout, e.caller_samples.len(), |i| {
        let loc = &e.caller_samples[i];
        format!("{}:{}", display_path(&loc.file, root), loc.line)
    });
    if !action.is_empty() {
        let _ = writeln!(out, "  action:        {action}");
    }
    let _ = writeln!(out);
}

fn shotgun_json(e: &ShotgunSurgeryEvidence, root: Option<&Path>) -> serde_json::Value {
    let callers: Vec<serde_json::Value> = e
        .caller_samples
        .iter()
        .map(|loc| serde_json::json!({"file": display_path(&loc.file, root), "line": loc.line}))
        .collect();
    let additional: Vec<serde_json::Value> = e
        .additional_definitions
        .iter()
        .map(|loc| serde_json::json!({"file": display_path(&loc.file, root), "line": loc.line}))
        .collect();
    serde_json::json!({
        "kind": "ShotgunSurgery",
        "method_class": e.method_class,
        "method_name": e.method_name,
        "method_file": display_path(&e.method_file, root),
        "method_line": e.method_line,
        "changing_classes": e.changing_classes,
        "changing_methods": e.changing_methods,
        "fanout": e.fanout,
        "confidence": confidence_str(e.confidence),
        "callers": callers,
        "name_collision_count": e.name_collision_count,
        "additional_definitions": additional,
    })
}

fn unique_file_locations(f: &AuditFinding) -> Vec<(PathBuf, u32)> {
    let mut seen: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();
    let mut out: Vec<(PathBuf, u32)> = Vec::new();
    for loc in &f.locations {
        if seen.insert(loc.file.clone()) {
            out.push((loc.file.clone(), loc.line));
        }
    }
    out
}

fn pattern_json(f: &AuditFinding, fingerprint: u64, root: Option<&Path>) -> serde_json::Value {
    let locations: Vec<serde_json::Value> = f
        .locations
        .iter()
        .map(|loc| serde_json::json!({"file": display_path(&loc.file, root), "line": loc.line}))
        .collect();
    serde_json::json!({
        "kind": "UncategorizedPattern",
        "fingerprint": fingerprint,
        "representative_snippet": f.representative_snippet,
        "support": f.support,
        "file_count": f.file_count,
        "idf_score": f.idf_score,
        "action_label": f.action_label,
        "locations": locations,
    })
}

#[allow(dead_code)]
pub fn relative_to(root: &Path, file: &Path) -> PathBuf {
    file.strip_prefix(root).map_or_else(|_| file.to_path_buf(), Path::to_path_buf)
}
