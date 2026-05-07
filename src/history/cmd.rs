use std::path::PathBuf;
use std::process;

use crate::audit;
use crate::config;

use super::{output, HistoryError, HistoryOpts};

pub fn run(
    root: Option<String>,
    json: bool,
    since: Option<String>,
    max_commits: Option<u32>,
    include_tests: bool,
) {
    let root = root.as_deref().map_or_else(|| PathBuf::from("."), PathBuf::from);
    let cfg_with_root = config::load_config_with_root(&root);
    let (cfg_ref, ignore_base) = match &cfg_with_root {
        Some((c, base)) => (Some(c), base.clone()),
        None => (None, root.clone()),
    };
    let thresholds = config::resolve_base_thresholds(cfg_ref);
    let ignore_patterns: &[String] = cfg_ref.map_or(&[][..], |c| &c.ignore.paths);
    let matcher = config::IgnoreMatcher::from_patterns(ignore_patterns);
    let filter = audit::IgnoreFilter::new(&matcher, &ignore_base);
    let opts = HistoryOpts {
        root: root.clone(),
        include_tests,
        since,
        max_commits,
    };
    let findings = match super::run_with_filter(&opts, &thresholds.history, &filter) {
        Ok(f) => f,
        Err(e) => {
            handle_error(&e);
            return;
        }
    };
    let rendered = if json {
        output::format_findings_json(&findings, Some(&root))
    } else {
        output::format_findings(&findings, Some(&root))
    };
    if !rendered.is_empty() {
        print!("{rendered}");
    }
    process::exit(i32::from(!findings.is_empty()));
}

fn handle_error(e: &HistoryError) {
    use HistoryError::{GitFailed, GitNotInstalled, NotAGitRepo};
    match e {
        NotAGitRepo(p) => eprintln!("history: not a git repository: {}", p.display()),
        GitNotInstalled => eprintln!("history: git not found in PATH"),
        GitFailed { stderr, code } => eprintln!("history: git failed (code {code:?}): {stderr}"),
    }
    process::exit(2);
}
