use std::path::{Path, PathBuf};
use std::process::Command;

use super::HistoryError;

#[derive(Debug, Clone)]
pub struct Commit {
    #[allow(dead_code)]
    pub hash: String,
    pub author: String,
    pub timestamp: i64,
    pub files: Vec<PathBuf>,
}

pub struct GitOpts<'a> {
    pub root: &'a Path,
    pub since: Option<&'a str>,
    pub max_commits: Option<u32>,
    pub max_commit_files: u32,
}

const SENTINEL: &str = "__COMMIT__";

fn git_check_succeeds(root: &Path, args: &[&str]) -> bool {
    Command::new("git").arg("-C").arg(root).args(args).output().map(|o| o.status.success()).unwrap_or(false)
}

pub fn is_git_repo(root: &Path) -> bool {
    git_check_succeeds(root, &["rev-parse", "--git-dir"])
}

pub fn repo_toplevel(root: &Path) -> Option<PathBuf> {
    let output = Command::new("git").arg("-C").arg(root).args(["rev-parse", "--show-toplevel"]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let top = String::from_utf8(output.stdout).ok()?;
    let trimmed = top.trim();
    (!trimmed.is_empty()).then(|| PathBuf::from(trimmed))
}

fn git_stdout(dir: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git").arg("-C").arg(dir).args(args).output().ok()?;
    output.status.success().then(|| String::from_utf8(output.stdout).ok()).flatten()
}

pub fn last_commit_unix(dir: &Path, pathspec: &Path) -> Option<i64> {
    let spec = pathspec.to_string_lossy();
    git_stdout(dir, &["log", "-1", "--format=%at", "--", &spec])?.trim().parse::<i64>().ok()
}

pub fn working_tree_numstat(dir: &Path) -> Option<Vec<u64>> {
    Some(git_stdout(dir, &["diff", "HEAD", "--numstat"])?.lines().filter_map(numstat_line_changes).collect())
}

fn numstat_line_changes(line: &str) -> Option<u64> {
    let mut cols = line.split('\t');
    let added = cols.next()?.parse::<u64>().ok()?;
    let deleted = cols.next()?.parse::<u64>().ok()?;
    Some(added + deleted)
}

fn has_any_commit(root: &Path) -> bool {
    git_check_succeeds(root, &["rev-parse", "--verify", "HEAD"])
}

pub fn collect_commits(opts: &GitOpts) -> Result<Vec<Commit>, HistoryError> {
    if !is_git_repo(opts.root) {
        return Err(HistoryError::NotAGitRepo(opts.root.to_path_buf()));
    }
    if !has_any_commit(opts.root) {
        return Ok(Vec::new());
    }
    let args = build_log_args(opts);
    let output = Command::new("git").args(&args).output().map_err(|_| HistoryError::GitNotInstalled)?;
    if !output.status.success() {
        return Err(HistoryError::GitFailed {
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            code: output.status.code(),
        });
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_log_output(&stdout, opts.max_commit_files))
}

pub fn files_at_commit(root: &Path, rev: &str) -> Vec<PathBuf> {
    let Ok(output) = Command::new("git").arg("-C").arg(root).args(["ls-tree", "-r", "--name-only", rev]).output()
    else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&output.stdout).lines().map(PathBuf::from).collect()
}

pub fn file_at_commit(root: &Path, rev: &str, path: &Path) -> Option<String> {
    let spec = format!("{rev}:{}", path.to_string_lossy());
    let output = Command::new("git").arg("-C").arg(root).args(["show", &spec]).output().ok()?;
    if output.status.success() {
        String::from_utf8(output.stdout).ok()
    } else {
        None
    }
}

fn build_log_args(opts: &GitOpts) -> Vec<String> {
    let mut args = vec![
        "-C".to_string(),
        opts.root.to_string_lossy().into_owned(),
        "log".to_string(),
        "--name-only".to_string(),
        "--use-mailmap".to_string(),
        "--no-renames".to_string(),
        format!("--pretty=format:{SENTINEL}%n%H%n%ae%n%at"),
    ];
    if let Some(since) = opts.since {
        args.push(format!("--since={since}"));
    }
    if let Some(max) = opts.max_commits {
        args.push("-n".to_string());
        args.push(max.to_string());
    }
    args
}

pub fn parse_log_output(stdout: &str, max_files: u32) -> Vec<Commit> {
    let split_token = format!("{SENTINEL}\n");
    stdout.split(split_token.as_str()).skip(1).filter_map(|chunk| parse_chunk(chunk, max_files)).collect()
}

fn parse_chunk(chunk: &str, max_files: u32) -> Option<Commit> {
    let mut lines = chunk.lines();
    let hash = lines.next()?.to_string();
    let author = lines.next()?.to_string();
    let timestamp = lines.next()?.parse::<i64>().ok()?;
    let files: Vec<PathBuf> = lines.filter(|l| !l.trim().is_empty()).map(PathBuf::from).collect();
    if files.is_empty() || files.len() > max_files as usize {
        return None;
    }
    Some(Commit { hash, author, timestamp, files })
}
