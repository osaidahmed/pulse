use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use super::co_change::{CoChangeAgg, RevisionScope};
use super::finding::{BuildCoChangeEvidence, HistoryFinding, HistoryKind};
use super::thresholds::{BuildCoChangeThresholds, HistoryThresholds};

const BUILD_FILE_NAMES: &[&str] = &[
    "Cargo.toml",
    "Cargo.lock",
    "package.json",
    "package-lock.json",
    "yarn.lock",
    "pnpm-lock.yaml",
    "pyproject.toml",
    "requirements.txt",
    "setup.py",
    "setup.cfg",
    "Pipfile",
    "Pipfile.lock",
    "poetry.lock",
    "go.mod",
    "go.sum",
    "Gemfile",
    "Gemfile.lock",
    "Package.swift",
    "Package.resolved",
    "pom.xml",
    "build.gradle",
    "build.gradle.kts",
    "settings.gradle",
    "settings.gradle.kts",
    "Makefile",
    "makefile",
    "GNUmakefile",
    "CMakeLists.txt",
    "build.zig",
    "composer.json",
    "composer.lock",
    "BUILD",
    "BUILD.bazel",
    "WORKSPACE",
];

pub fn is_build_file(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
    BUILD_FILE_NAMES.contains(&name) || matches!(path.extension().and_then(|e| e.to_str()), Some("csproj" | "sln"))
}

struct BuildCtx<'a> {
    scope: &'a RevisionScope,
    source_paths: &'a HashSet<PathBuf>,
    bt: &'a BuildCoChangeThresholds,
}

pub fn rank_build_drift(
    pairs: &HashMap<(PathBuf, PathBuf), CoChangeAgg>,
    scope: &RevisionScope,
    source_paths: &HashSet<PathBuf>,
    t: &HistoryThresholds,
) -> Vec<HistoryFinding> {
    let ctx = BuildCtx { scope, source_paths, bt: &t.build_co_change };
    let mut findings: Vec<(f64, u32, HistoryFinding)> =
        pairs.iter().filter_map(|((a, b), agg)| build_pair_finding(a, b, agg, &ctx)).collect();
    findings.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap_or(std::cmp::Ordering::Equal).then(y.1.cmp(&x.1)));
    findings.truncate(ctx.bt.max_findings_reported as usize);
    findings.into_iter().map(|(_, _, f)| f).collect()
}

fn build_pair_finding(a: &Path, b: &Path, agg: &CoChangeAgg, ctx: &BuildCtx) -> Option<(f64, u32, HistoryFinding)> {
    if agg.support < ctx.bt.min_support {
        return None;
    }
    let (build, source) = build_and_source(a, b, ctx.source_paths)?;
    let source_revs = ctx.scope.per_file.get(source).copied().unwrap_or(agg.support).max(agg.support);
    let ratio = f64::from(agg.support) / f64::from(source_revs);
    if ratio < ctx.bt.min_ratio {
        return None;
    }
    let evidence = BuildCoChangeEvidence {
        build_file: build.to_path_buf(),
        source_file: source.to_path_buf(),
        support: agg.support,
        source_revisions: source_revs,
        ratio,
        last_seen_unix: agg.last_seen,
        distinct_authors: u32::try_from(agg.authors.len()).unwrap_or(u32::MAX),
    };
    Some((ratio, agg.support, HistoryFinding { kind: HistoryKind::BuildCoChange(evidence), action_label: None }))
}

fn build_and_source<'a>(a: &'a Path, b: &'a Path, source_paths: &HashSet<PathBuf>) -> Option<(&'a Path, &'a Path)> {
    let (build, source) = if is_build_file(a) {
        (a, b)
    } else if is_build_file(b) {
        (b, a)
    } else {
        return None;
    };
    (!is_build_file(source) && source_paths.contains(source)).then_some((build, source))
}
