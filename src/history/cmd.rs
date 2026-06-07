use std::path::PathBuf;
use std::process;

use crate::audit;
use crate::config;

use super::{output, HistoryError, HistoryOpts};

#[derive(Debug, Default)]
pub struct RunArgs {
    pub root: Option<String>,
    pub json: bool,
    pub since: Option<String>,
    pub max_commits: Option<u32>,
    pub overrides: config::HistoryCliOverrides,
    pub include_tests: bool,
}

pub fn run(args: RunArgs) {
    let root = args.root.as_deref().map_or_else(|| PathBuf::from("."), PathBuf::from);
    let cfg_with_root = config::load_config_with_root(&root);
    let (cfg_ref, ignore_base) = match &cfg_with_root {
        Some((c, base)) => (Some(c), base.clone()),
        None => (None, root.clone()),
    };
    let history_thresholds = config::resolve_history_thresholds(cfg_ref, args.overrides);
    let ignore_patterns = config::combined_history_ignore_patterns(cfg_ref);
    let matcher = config::IgnoreMatcher::from_patterns(&ignore_patterns);
    let filter = audit::IgnoreFilter::new(&matcher, &ignore_base);
    let opts = HistoryOpts {
        root: root.clone(),
        include_tests: args.include_tests,
        since: args.since,
        max_commits: args.max_commits,
    };
    let findings = match super::run_with_filter(&opts, &history_thresholds, &filter) {
        Ok(f) => f,
        Err(e) => {
            handle_error(&e);
            return;
        }
    };
    let rendered = if args.json {
        output::format_findings_json(&findings, Some(&root))
    } else {
        output::format_findings(&findings, Some(&root))
    };
    if !rendered.is_empty() {
        print!("{rendered}");
    }
    process::exit(i32::from(!findings.is_empty()));
}

pub fn calibrate(args: RunArgs) {
    let root = args.root.as_deref().map_or_else(|| PathBuf::from("."), PathBuf::from);
    let cfg_with_root = config::load_config_with_root(&root);
    let (cfg_ref, ignore_base) = match &cfg_with_root {
        Some((c, base)) => (Some(c), base.clone()),
        None => (None, root.clone()),
    };
    let history_thresholds = config::resolve_history_thresholds(cfg_ref, args.overrides);
    let ignore_patterns = config::combined_history_ignore_patterns(cfg_ref);
    let matcher = config::IgnoreMatcher::from_patterns(&ignore_patterns);
    let filter = audit::IgnoreFilter::new(&matcher, &ignore_base);
    let opts = HistoryOpts {
        root: root.clone(),
        include_tests: args.include_tests,
        since: args.since,
        max_commits: args.max_commits,
    };
    match super::calibrate_with_filter(&opts, &history_thresholds, &filter, now_secs()) {
        Ok(calib) => match super::jit_risk::write_calibration(&root, &calib) {
            Ok(()) => println!("jit calibration written: {}", super::jit_risk::calib_path(&root).display()),
            Err(e) => {
                eprintln!("history: failed to write jit calibration: {e}");
                process::exit(2);
            }
        },
        Err(e) => handle_error(&e),
    }
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
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
