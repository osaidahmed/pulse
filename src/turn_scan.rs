use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::analyze::{self, AnalysisResultFull, ScanOptions};
use crate::baseline_ratchet;
use crate::baselines;
use crate::config;
use crate::parse;
use crate::smells::{Finding, Location};
use crate::test_detection;

const MAX_TURN_FILES: usize = 100;

pub fn stamp_turn_head() {
    let Some(sha) = git(Path::new("."), &["rev-parse", "HEAD"]) else { return };
    let _ = std::fs::create_dir_all(baselines::baseline_dir());
    let _ = std::fs::write(sha_path(), sha.trim());
}

fn sha_path() -> PathBuf {
    baselines::baseline_dir().join("turn-head.sha")
}

pub fn canonical_path(path: &str) -> String {
    std::fs::canonicalize(path).ok().and_then(|p| p.to_str().map(String::from)).unwrap_or_else(|| path.to_string())
}

pub(crate) fn repo_and_base() -> Option<(PathBuf, String)> {
    let base_sha = std::fs::read_to_string(sha_path()).ok()?;
    let top = git(Path::new("."), &["rev-parse", "--show-toplevel"])?;
    Some((PathBuf::from(top.trim()), base_sha.trim().to_string()))
}

pub(crate) fn base_blob(repo: &Path, base_sha: &str, rel: &str) -> Option<String> {
    git(repo, &["show", &format!("{}:{}", base_sha.trim(), rel)])
}

pub fn turn_regressions(already_checked: &HashSet<String>) -> Vec<(String, Vec<Finding>)> {
    let Ok(base_sha) = std::fs::read_to_string(sha_path()) else { return Vec::new() };
    let Some(top) = git(Path::new("."), &["rev-parse", "--show-toplevel"]) else { return Vec::new() };
    let repo = PathBuf::from(top.trim());
    let mut out = Vec::new();
    for rel in changed_files(&repo, &base_sha) {
        let file =
            TurnFile { abs: repo.join(&rel).to_string_lossy().into_owned(), rel, repo: &repo, base_sha: &base_sha };
        if already_checked.contains(&canonical_path(&file.abs))
            || test_detection::is_test_file(&file.abs)
            || baselines::is_fixture_file(&file.abs)
        {
            continue;
        }
        if let Some(found) = file_regressions(&file) {
            out.push(found);
        }
    }
    out
}

struct TurnFile<'a> {
    abs: String,
    rel: String,
    repo: &'a Path,
    base_sha: &'a str,
}

pub(crate) fn changed_files(repo: &Path, base_sha: &str) -> Vec<String> {
    let mut files = git_lines(repo, &["diff", "--name-only", base_sha.trim(), "HEAD"]);
    files.extend(git_lines(repo, &["diff", "--name-only", "HEAD"]));
    files.extend(git_lines(repo, &["ls-files", "--others", "--exclude-standard"]));
    let mut seen = HashSet::new();
    files.retain(|f| !f.trim().is_empty() && seen.insert(f.clone()));
    files.truncate(MAX_TURN_FILES);
    files
}

fn file_regressions(file: &TurnFile) -> Option<(String, Vec<Finding>)> {
    let path = Path::new(&file.abs);
    let lang = parse::detect_language(path)?;
    let cfg = config::load_config(path);
    if cfg.as_ref().is_some_and(|c| config::is_ignored_for_file(c, path)) {
        return None;
    }
    let current = analyze::analyze_file(&file.abs, cfg.as_ref(), ScanOptions::hook(None))?;
    let baseline = git(file.repo, &["show", &format!("{}:{}", file.base_sha.trim(), file.rel)])
        .and_then(|src| analyze::analyze_source(&file.abs, &src, lang, cfg.as_ref(), ScanOptions::hook(None)));
    let novel = crate::interaction::suppress_subsumed(novel_findings(&current, baseline.as_ref()));
    let filename = path.file_name()?.to_string_lossy().into_owned();
    (!novel.is_empty()).then_some((filename, novel))
}

fn novel_findings(current: &AnalysisResultFull, baseline: Option<&AnalysisResultFull>) -> Vec<Finding> {
    let func_base = baseline.map_or_else(Default::default, |b| {
        baseline_ratchet::baseline_from_entries(baseline_ratchet::entries_from(&b.findings, &b.metrics))
    });
    let module_counts = baseline.map_or_else(HashMap::new, |b| module_counts(&b.findings));
    let func_findings: Vec<Finding> =
        current.findings.iter().filter(|f| !matches!(f.location, Location::Module)).cloned().collect();
    let mut novel = baseline_ratchet::filter_worsened(func_findings, &func_base, &current.metrics);
    novel.extend(analyze::module_regressions(&current.findings, &module_counts));
    novel
}

fn module_counts(findings: &[Finding]) -> HashMap<String, usize> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for f in findings.iter().filter(|f| matches!(f.location, Location::Module)) {
        *counts.entry(f.smell.as_str().to_string()).or_default() += 1;
    }
    counts
}

fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let out = std::process::Command::new("git").arg("-C").arg(dir).args(args).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

fn git_lines(dir: &Path, args: &[&str]) -> Vec<String> {
    git(dir, args).map(|o| o.lines().map(str::to_string).collect()).unwrap_or_default()
}
