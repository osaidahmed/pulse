use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::buildmeta::{self, BuildMeta, DeclaredDep, DepScope, Ecosystem, Manifest};
use crate::import_check::normalize;
use crate::registry::{self, FreshnessThresholds, PackageInfo, VersionInfo};

use super::finding::{AuditFinding, AuditKind, ImportConfidence, OutdatedDepEvidence};

pub struct FreshnessVerdict {
    pub current_version: String,
    pub latest_version: String,
    pub missed_releases: u32,
    pub abandoned: bool,
}

const PROGRESS_MIN_DEPS: usize = 20;

pub(super) fn for_each_deployed_locked<F>(root: &Path, max_findings: usize, build: F) -> Vec<AuditFinding>
where
    F: Fn(&Manifest, &DeclaredDep, &str) -> Option<AuditFinding> + Sync,
{
    use rayon::prelude::*;
    let meta = buildmeta::discover(root);
    let candidates: Vec<(&Manifest, &DeclaredDep, String)> = meta
        .manifests
        .iter()
        .flat_map(|m| m.deps.iter().map(move |d| (m, d)))
        .filter(|(_, d)| !d.own && d.scope == DepScope::Deployed)
        .filter_map(|(m, d)| locked_version(&meta, m.ecosystem, &d.name).map(|v| (m, d, v)))
        .collect();
    let total = candidates.len();
    let progress = should_show_progress(total, super::progress::is_active());
    let done = std::sync::Mutex::new(0usize);
    let mut findings: Vec<AuditFinding> = candidates
        .par_iter()
        .filter_map(|(m, d, v)| {
            let finding = build(m, d, v);
            if progress {
                let mut n = done.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                *n += 1;
                emit_progress(*n, total);
            }
            finding
        })
        .collect();
    if progress {
        super::progress::clear();
    }
    findings.sort_by(|a, b| a.representative_snippet.cmp(&b.representative_snippet));
    findings.truncate(max_findings);
    findings
}

pub fn should_show_progress(total: usize, stderr_is_terminal: bool) -> bool {
    total > PROGRESS_MIN_DEPS && stderr_is_terminal
}

fn emit_progress(done: usize, total: usize) {
    super::progress::show(&format!("  checking dependencies {done}/{total}"));
}

pub fn run_from(root: &Path, cache_dir: &Path, online: bool, t: &FreshnessThresholds) -> Vec<AuditFinding> {
    let now_year = current_year();
    for_each_deployed_locked(root, t.max_findings, |manifest, dep, current| {
        let pkg = registry::lookup(manifest.ecosystem, &dep.name, cache_dir, online)?;
        let verdict = assess(&pkg, current, now_year, t)?;
        Some(finding(&manifest.path, dep.line, &dep.name, verdict))
    })
}

pub fn assess(pkg: &PackageInfo, current: &str, now_year: i64, t: &FreshnessThresholds) -> Option<FreshnessVerdict> {
    let cur = pkg.versions.iter().find(|v| v.version_key.version == current)?;
    let latest = latest_version(pkg)?;
    let missed = pkg
        .versions
        .iter()
        .filter(|v| !v.published_at.is_empty() && !cur.published_at.is_empty() && v.published_at > cur.published_at)
        .count() as u32;
    let abandoned = year_of(&latest.published_at).is_some_and(|y| now_year - y >= t.abandon_years);
    if missed < t.min_missed && !abandoned {
        return None;
    }
    Some(FreshnessVerdict {
        current_version: current.to_string(),
        latest_version: latest.version_key.version.clone(),
        missed_releases: missed,
        abandoned,
    })
}

fn latest_version(pkg: &PackageInfo) -> Option<&VersionInfo> {
    pkg.versions.iter().find(|v| v.is_default).or_else(|| {
        pkg.versions.iter().filter(|v| !v.published_at.is_empty()).max_by(|a, b| a.published_at.cmp(&b.published_at))
    })
}

fn year_of(rfc3339: &str) -> Option<i64> {
    rfc3339.get(0..4)?.parse().ok()
}

fn current_year() -> i64 {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    1970 + i64::try_from(secs / 31_557_600).unwrap_or(0)
}

fn locked_version(meta: &BuildMeta, eco: Ecosystem, name: &str) -> Option<String> {
    let norm = normalize(name);
    meta.lockfiles
        .iter()
        .filter(|l| l.ecosystem == eco)
        .flat_map(|l| &l.resolved)
        .find(|(n, _)| normalize(n) == norm)
        .map(|(_, v)| v.clone())
}

fn finding(manifest: &Path, line: u32, name: &str, v: FreshnessVerdict) -> AuditFinding {
    super::deps_reconcile::wrap(
        name.to_string(),
        AuditKind::OutdatedDependency(OutdatedDepEvidence {
            manifest: manifest.to_path_buf(),
            line,
            name: name.to_string(),
            current_version: v.current_version,
            latest_version: v.latest_version,
            missed_releases: v.missed_releases,
            abandoned: v.abandoned,
            confidence: ImportConfidence::Medium,
        }),
    )
}
