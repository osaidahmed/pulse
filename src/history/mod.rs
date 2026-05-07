use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub mod thresholds;
pub mod git;
pub mod edges;
pub mod co_change;
pub mod hotspots;
pub mod contributors;
pub mod finding;
pub mod output;
pub mod cmd;

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
    let typed_files = crate::audit::walk_typed_source_files_filtered(
        &opts.root,
        opts.include_tests,
        filter,
    );
    let total_commits = u32::try_from(commits_rel.len()).unwrap_or(u32::MAX);
    let commits = absolutize_commits(commits_rel, &opts.root);
    let typed_paths: HashSet<PathBuf> = typed_files.iter().map(|(p, _)| p.clone()).collect();
    let graph = edges::build_graph(&typed_files, &opts.root);
    let pairs = co_change::mine(&commits, t);
    let mut findings = co_change::rank_drift(pairs, &graph, &typed_paths, total_commits, t);
    findings.extend(hotspots::rank(&commits, &typed_files, t));
    findings.extend(contributors::rank(&commits, &typed_paths, t));
    Ok(findings)
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
