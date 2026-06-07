use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub mod arch_trend;
pub mod cmd;
pub mod co_change;
pub mod contributors;
pub mod edges;
pub mod finding;
pub mod git;
pub mod hist_smells;
pub mod hotspots;
pub mod jit_risk;
pub mod jit_thresholds;
pub mod output;
pub mod thresholds;

use finding::HistoryFinding;
use thresholds::HistoryThresholds;

#[derive(Debug, Clone)]
pub struct HistoryOpts {
    pub root: PathBuf,
    pub include_tests: bool,
    pub since: Option<String>,
    pub max_commits: Option<u32>,
}

#[derive(Debug)]
pub enum HistoryError {
    NotAGitRepo(PathBuf),
    GitFailed { stderr: String, code: Option<i32> },
    GitNotInstalled,
}

#[allow(dead_code)]
pub fn run(opts: &HistoryOpts, t: &HistoryThresholds) -> Result<Vec<HistoryFinding>, HistoryError> {
    let matcher = crate::config::IgnoreMatcher::from_patterns(&[]);
    let filter = crate::audit::IgnoreFilter::new(&matcher, &opts.root);
    run_with_filter(opts, t, &filter)
}

pub fn run_with_filter(
    opts: &HistoryOpts,
    t: &HistoryThresholds,
    filter: &crate::audit::IgnoreFilter<'_>,
) -> Result<Vec<HistoryFinding>, HistoryError> {
    if !git::is_git_repo(&opts.root) {
        return Err(HistoryError::NotAGitRepo(opts.root.clone()));
    }
    let git_opts = git::GitOpts {
        root: &opts.root,
        since: opts.since.as_deref(),
        max_commits: opts.max_commits,
        max_commit_files: t.max_commit_files,
    };
    let commits_rel = git::collect_commits(&git_opts)?;
    let typed_files = crate::audit::walk_typed_source_files_filtered(&opts.root, opts.include_tests, filter);
    let commits = absolutize_commits(commits_rel, &opts.root);
    let typed_paths: HashSet<PathBuf> = typed_files.iter().map(|(p, _)| p.clone()).collect();
    let graph = edges::build_graph(&typed_files, &opts.root);
    let pairs = co_change::mine(&commits, t);
    let scope = co_change::revisions_in_scope(&commits, t);
    let hist = hist_smells::rank(&commits, &pairs, &scope, &typed_paths, t);
    let mut findings = co_change::rank_drift(pairs, &scope, &graph, &typed_paths, t);
    findings.extend(hotspots::rank(&commits, &typed_files, t));
    findings.extend(contributors::rank(&commits, &typed_paths, t));
    findings.extend(hist);
    if t.arch_trend {
        findings.extend(arch_trend::catalyst_findings(&opts.root, &commits));
    }
    Ok(findings)
}

pub fn calibrate_with_filter(
    opts: &HistoryOpts,
    t: &HistoryThresholds,
    filter: &crate::audit::IgnoreFilter<'_>,
    now_secs: i64,
) -> Result<jit_risk::JitCalibration, HistoryError> {
    if !git::is_git_repo(&opts.root) {
        return Err(HistoryError::NotAGitRepo(opts.root.clone()));
    }
    let git_opts = git::GitOpts {
        root: &opts.root,
        since: opts.since.as_deref(),
        max_commits: opts.max_commits,
        max_commit_files: t.max_commit_files,
    };
    let commits = absolutize_commits(git::collect_commits(&git_opts)?, &opts.root);
    let typed_files = crate::audit::walk_typed_source_files_filtered(&opts.root, opts.include_tests, filter);
    Ok(jit_risk::calibrate(&typed_files, &commits, now_secs, t.jit))
}

#[allow(dead_code)]
pub fn calibrate(
    opts: &HistoryOpts,
    t: &HistoryThresholds,
    now_secs: i64,
) -> Result<jit_risk::JitCalibration, HistoryError> {
    let matcher = crate::config::IgnoreMatcher::from_patterns(&[]);
    let filter = crate::audit::IgnoreFilter::new(&matcher, &opts.root);
    calibrate_with_filter(opts, t, &filter, now_secs)
}

#[allow(dead_code)]
pub fn changeshotgun_files(
    opts: &HistoryOpts,
    t: &HistoryThresholds,
    filter: &crate::audit::IgnoreFilter<'_>,
) -> Option<HashSet<PathBuf>> {
    if !git::is_git_repo(&opts.root) {
        return None;
    }
    let git_opts = git::GitOpts {
        root: &opts.root,
        since: opts.since.as_deref(),
        max_commits: opts.max_commits,
        max_commit_files: t.max_commit_files,
    };
    let commits = absolutize_commits(git::collect_commits(&git_opts).ok()?, &opts.root);
    if commits.is_empty() {
        return None;
    }
    let mut ht = *t;
    ht.hist.enabled = true;
    let typed_files = crate::audit::walk_typed_source_files_filtered(&opts.root, opts.include_tests, filter);
    let typed_paths: HashSet<PathBuf> = typed_files.iter().map(|(p, _)| p.clone()).collect();
    let pairs = co_change::mine(&commits, &ht);
    let scope = co_change::revisions_in_scope(&commits, &ht);
    let hist = hist_smells::rank(&commits, &pairs, &scope, &typed_paths, &ht);
    Some(
        hist.iter()
            .filter_map(|f| match &f.kind {
                finding::HistoryKind::ChangeShotgun(e) => Some(e.file.clone()),
                _ => None,
            })
            .collect(),
    )
}

fn absolutize_commits(commits: Vec<git::Commit>, root: &Path) -> Vec<git::Commit> {
    commits
        .into_iter()
        .map(|mut c| {
            c.files = c.files.iter().map(|f| root.join(f)).collect();
            c
        })
        .collect()
}
