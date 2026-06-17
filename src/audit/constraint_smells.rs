use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::buildmeta::{self, BuildMeta, DeclaredDep, Ecosystem, Manifest};
use crate::import_check::normalize;
use crate::thresholds::AuditThresholds;

use super::finding::{AuditFinding, AuditKind, ConstraintEvidence, ImportConfidence};

const MIN_DEPS_FOR_PINNING_SMELL: usize = 3;

pub fn run_from(root: &Path, _thresholds: &AuditThresholds) -> Vec<AuditFinding> {
    let meta = buildmeta::discover(root);
    let internal = super::reflexion::workspace_internal_names(&meta);
    let mut findings = Vec::new();
    for manifest in &meta.manifests {
        findings.extend(wildcard_findings(manifest, &internal));
        findings.extend(pinned_without_lockfile(manifest, &meta));
        findings.extend(divergence_findings(manifest, &meta));
    }
    findings.sort_by(|a, b| a.representative_snippet.cmp(&b.representative_snippet));
    findings
}

fn declared(manifest: &Manifest) -> impl Iterator<Item = &DeclaredDep> {
    manifest.deps.iter().filter(|d| !d.own)
}

fn wildcard_findings(manifest: &Manifest, internal: &HashSet<String>) -> Vec<AuditFinding> {
    let own = own_vendor(manifest);
    declared(manifest)
        .filter(|d| {
            is_wildcard(&d.constraint)
                && !internal.contains(&normalize(&d.name))
                && !same_vendor_sibling(manifest.ecosystem, &d.name, own.as_deref())
        })
        .map(|d| {
            finding(
                manifest,
                d,
                format!("`{}` accepts any version (`{}`)", d.name, d.constraint),
                ImportConfidence::High,
            )
        })
        .collect()
}

fn own_vendor(manifest: &Manifest) -> Option<String> {
    if manifest.ecosystem != Ecosystem::Composer {
        return None;
    }
    manifest.deps.iter().find(|d| d.own).and_then(|d| d.name.split('/').next()).map(str::to_string)
}

fn same_vendor_sibling(eco: Ecosystem, name: &str, own_vendor: Option<&str>) -> bool {
    eco == Ecosystem::Composer && own_vendor.is_some_and(|vendor| name.split('/').next() == Some(vendor))
}

fn is_wildcard(constraint: &str) -> bool {
    let c = constraint.trim();
    matches!(c, "*" | "latest" | "x") || c.starts_with(">=0.0.0") || c == ">=0"
}

fn pinned_without_lockfile(manifest: &Manifest, meta: &BuildMeta) -> Vec<AuditFinding> {
    if manifest.ecosystem == Ecosystem::Go || has_lockfile(meta, manifest.ecosystem) {
        return Vec::new();
    }
    let deps: Vec<&DeclaredDep> = declared(manifest).collect();
    if deps.len() < MIN_DEPS_FOR_PINNING_SMELL || !deps.iter().all(|d| exact_pin(manifest.ecosystem, &d.constraint)) {
        return Vec::new();
    }
    let lead = deps[0];
    vec![finding(
        manifest,
        lead,
        format!(
            "all {} dependencies are exactly pinned but no lockfile exists — transitive resolution still floats",
            deps.len()
        ),
        ImportConfidence::Medium,
    )]
}

fn has_lockfile(meta: &BuildMeta, eco: Ecosystem) -> bool {
    meta.lockfiles.iter().any(|l| l.ecosystem == eco)
}

fn exact_pin(eco: Ecosystem, constraint: &str) -> bool {
    exact_version(eco, constraint).is_some()
}

fn npm_exact(c: &str) -> bool {
    let core = c.split(['-', '+']).next().unwrap_or(c);
    c.starts_with(|ch: char| ch.is_ascii_digit())
        && !core.is_empty()
        && core.chars().all(|ch| ch.is_ascii_digit() || ch == '.')
}

fn strip_build(version: &str) -> &str {
    version.split('+').next().unwrap_or(version)
}

fn normalize_version(v: &str) -> &str {
    strip_build(v.trim().trim_start_matches('v'))
}

fn versions_consistent(pin: &str, resolved: &str) -> bool {
    let (p, r) = (normalize_version(pin), normalize_version(resolved));
    p == r || boundary_prefix(p, r) || boundary_prefix(r, p)
}

fn boundary_prefix(short: &str, long: &str) -> bool {
    long.strip_prefix(short).is_some_and(|rest| rest.starts_with('.'))
}

fn maven_exact(c: &str) -> Option<&str> {
    let inner = c.strip_prefix('[')?.strip_suffix(']')?;
    (!inner.is_empty() && !inner.contains(',')).then_some(inner)
}

fn divergence_findings(manifest: &Manifest, meta: &BuildMeta) -> Vec<AuditFinding> {
    let locked: HashMap<String, &str> = meta
        .lockfiles
        .iter()
        .filter(|l| l.ecosystem == manifest.ecosystem)
        .flat_map(|l| l.resolved.iter().map(|(n, v)| (normalize(n), v.as_str())))
        .collect();
    declared(manifest)
        .filter_map(|d| {
            let pin = exact_version(manifest.ecosystem, &d.constraint)?;
            let resolved = locked.get(&normalize(&d.name))?;
            (!resolved.is_empty() && !versions_consistent(pin, resolved)).then(|| {
                finding(
                    manifest,
                    d,
                    format!("`{}` is pinned to {pin} but the lockfile resolved {resolved}", d.name),
                    ImportConfidence::High,
                )
            })
        })
        .collect()
}

fn exact_version(eco: Ecosystem, constraint: &str) -> Option<&str> {
    let c = constraint.trim();
    match eco {
        Ecosystem::Npm | Ecosystem::Composer => npm_exact(c).then_some(c),
        Ecosystem::Cargo => c.strip_prefix('='),
        Ecosystem::Pip => c.strip_prefix("=="),
        Ecosystem::Maven => maven_exact(c),
        Ecosystem::RubyGems
        | Ecosystem::Go
        | Ecosystem::NuGet
        | Ecosystem::Swift
        | Ecosystem::Zig
        | Ecosystem::Gradle => None,
    }
    .map(str::trim)
}

fn finding(manifest: &Manifest, dep: &DeclaredDep, problem: String, confidence: ImportConfidence) -> AuditFinding {
    AuditFinding {
        kind: AuditKind::ConstraintSmell(ConstraintEvidence {
            manifest: manifest.path.clone(),
            line: dep.line,
            name: dep.name.clone(),
            constraint: dep.constraint.clone(),
            problem,
            confidence,
        }),
        representative_snippet: dep.name.clone(),
        support: 1,
        file_count: 1,
        idf_score: None,
        action_label: None,
        pattern_category: None,
        locality_entropy: None,
        p_value: None,
        locations: Vec::new(),
    }
}
