#![allow(clippy::assigning_clones)]

mod cli;
mod hook_run;
mod setup;

use pulse::{analyze, audit, baselines, calibrate, config, history, hook, output, parse, test_detection};

use std::path::{Path, PathBuf};
use std::process;

const SKIP_DIRS: &[&str] = &["node_modules", "target", "vendor", "build", "dist"];

fn read_session_id_from_stdin() -> Option<String> {
    let mut input = String::new();
    std::io::Read::read_to_string(&mut std::io::stdin(), &mut input).ok()?;
    let v: serde_json::Value = serde_json::from_str(&input).ok()?;
    v.get("session_id")?.as_str().map(String::from)
}

fn main() {
    let Some(d) = dispatch_session(cli::parse()) else {
        return;
    };
    dispatch_subcommand(d);
}

fn dispatch_session(d: cli::Dispatch) -> Option<cli::Dispatch> {
    if matches!(d, cli::Dispatch::Hook) {
        if let Some(input) = hook::parse_hook_input() {
            baselines::init_session_dir(input.session_id.as_deref());
            hook_run::run_hook(input);
        }
        return None;
    }
    if matches!(d, cli::Dispatch::Stop) {
        baselines::init_session_dir(read_session_id_from_stdin().as_deref());
        hook_run::run_stop();
        return None;
    }
    if matches!(d, cli::Dispatch::Cleanup) {
        baselines::init_session_dir(read_session_id_from_stdin().as_deref());
        hook_run::run_cleanup();
        return None;
    }
    if matches!(d, cli::Dispatch::UsageError) {
        eprintln!("{}", cli::USAGE);
        process::exit(1);
    }
    Some(d)
}

fn dispatch_subcommand(d: cli::Dispatch) {
    match d {
        cli::Dispatch::Setup { uninstall } => {
            setup::run_setup(uninstall);
            process::exit(0);
        }
        cli::Dispatch::Check { file, json } => run_check(&file, json),
        cli::Dispatch::CheckAll { include_tests, json } => run_check_all(include_tests, json),
        cli::Dispatch::Debug(p) => run_debug(&p),
        cli::Dispatch::Budget(p) => p.as_deref().map_or_else(run_budget_new, run_budget),
        other => dispatch_analysis(other),
    }
}

fn dispatch_analysis(d: cli::Dispatch) {
    match d {
        cli::Dispatch::Audit { args, include_tests } => run_audit_cmd(args, include_tests),
        cli::Dispatch::History { args, include_tests } => run_history_cmd(args, include_tests),
        cli::Dispatch::Calibrate(args) => run_calibrate_cmd(args),
        _ => unreachable!(),
    }
}

fn run_calibrate_cmd(args: cli::CalibrateArgs) {
    let root = args.root.as_deref().map_or_else(|| PathBuf::from("."), PathBuf::from);
    validate_audit_root(&root);
    let cfg_with_root = config::load_config_with_root(&root);
    let (cfg_ref, base) = match &cfg_with_root {
        Some((c, b)) => (Some(c), b.clone()),
        None => (None, root.clone()),
    };
    let thresholds = config::resolve_base_thresholds(cfg_ref);
    let matcher = config::IgnoreMatcher::from_patterns(cfg_ref.map_or(&[][..], |c| &c.ignore.paths));
    let filter = audit::IgnoreFilter::new(&matcher, &base);
    let census = calibrate::collect(&root, &thresholds, &filter);
    let est_cfg = calibrate::estimator::EstimatorConfig {
        warn_percentile: args.warn_percentile.unwrap_or(0.75),
        alert_percentile: args.alert_percentile.unwrap_or(0.95),
    };
    let calibrated = calibrate::estimator::estimate(&census, calibrate::priors::corpus_priors(), &est_cfg);
    let rendered = calibrate::emit::render(&calibrated);
    emit_calibration(&root, &rendered, args.write);
}

fn emit_calibration(root: &Path, rendered: &calibrate::emit::Rendered, write: bool) {
    if !write {
        print!("{}", rendered.main);
        eprintln!("calibrate: dry run — pass --write to save .pulse.toml (and tests/.pulse.toml)");
        return;
    }
    if let Err(e) = std::fs::write(root.join(".pulse.toml"), &rendered.main) {
        eprintln!("calibrate: failed to write .pulse.toml: {e}");
        process::exit(1);
    }
    eprintln!("calibrate: wrote {}", root.join(".pulse.toml").display());
    let tests_dir = root.join("tests");
    if tests_dir.is_dir() {
        let _ = std::fs::write(tests_dir.join(".pulse.toml"), &rendered.tests);
        eprintln!("calibrate: wrote {}", tests_dir.join(".pulse.toml").display());
    }
}

fn run_history_cmd(args: cli::HistoryArgs, include_tests: bool) {
    let calibrate = args.jit_calibrate;
    let run_args = history::cmd::RunArgs {
        root: args.root,
        json: args.json,
        since: args.since,
        max_commits: args.max_commits,
        overrides: config::HistoryCliOverrides {
            co_change_top: args.co_change_top,
            hotspot_top: args.hotspot_top,
            contributors_top: args.contributors_top,
            hist: args.hist,
            arch_trend: args.arch_trend,
            no_szz: args.no_szz,
        },
        include_tests,
    };
    if calibrate {
        history::cmd::calibrate(run_args);
    } else {
        history::cmd::run(run_args);
    }
}

fn run_audit_cmd(args: cli::AuditArgs, include_tests: bool) {
    let root = args.root.as_deref().map_or_else(|| PathBuf::from("."), PathBuf::from);
    validate_audit_root(&root);
    let cfg_with_root = config::load_config_with_root(&root);
    let (cfg_ref, ignore_base) = match &cfg_with_root {
        Some((c, base)) => (Some(c), base.clone()),
        None => (None, root.clone()),
    };
    let thresholds = config::resolve_base_thresholds(cfg_ref);
    let ignore_patterns: &[String] = cfg_ref.map_or(&[][..], |c| &c.ignore.paths);
    let matcher = config::IgnoreMatcher::from_patterns(ignore_patterns);
    let filter = audit::IgnoreFilter::new(&matcher, &ignore_base);
    let suppression = audit::suppression::AuditSuppression::from_config(cfg_ref.map(|c| &c.audit));
    let opts = audit::AuditOpts {
        root: root.clone(),
        pass: args.pass,
        json: args.json,
        include_tests,
        show_noise: args.show_noise,
        suppression,
    };
    let cache_dir = pulse::registry::cache_dir();
    let cross_validator = |root: &std::path::Path, include_tests: bool, filter: &audit::IgnoreFilter| {
        let hist_opts =
            history::HistoryOpts { root: root.to_path_buf(), include_tests, since: None, max_commits: None };
        history::changeshotgun_files(&hist_opts, &history::thresholds::HistoryThresholds::DEFAULTS, filter)
    };
    let run = audit::RunCtx { online: args.online, cache_dir: &cache_dir, cross_validator: Some(&cross_validator) };
    let findings = audit::run_with_filter_online(&opts, &thresholds.audit, &filter, &run);
    let ctx = audit::output::RenderCtx {
        root: Some(&root),
        show_noise: opts.show_noise,
        suppression: &opts.suppression,
        cpg_enabled: thresholds.cpg.enabled,
    };
    let rendered = if args.json {
        audit::output::format_findings_json_filtered(&findings, &ctx)
    } else {
        audit::output::format_findings_filtered(&findings, &thresholds.audit, &ctx)
    };
    if !rendered.is_empty() {
        print!("{rendered}");
    }
    process::exit(i32::from(!findings.is_empty()));
}

fn validate_audit_root(root: &Path) {
    if !root.exists() {
        eprintln!("audit: root path does not exist: {}", root.display());
        process::exit(1);
    }
    if !root.is_dir() {
        eprintln!("audit: root path is not a directory: {}", root.display());
        process::exit(1);
    }
}

fn run_debug(file_path: &str) {
    let path = Path::new(file_path);
    let cfg = config::load_config(path);
    if cfg.as_ref().is_some_and(|c| config::is_ignored_for_file(c, path)) {
        eprintln!("debug: {file_path} — ignored by .pulse.toml");
        return;
    }
    let lang = parse::detect_language(path).expect("unsupported language");
    let source = std::fs::read_to_string(path).expect("can't read file");
    let metrics = parse::parse_and_walk_guarded(&source, lang).expect("parse failed");
    eprintln!(
        "Module: {} LOC, {} functions, sum_cc={} declarations={}",
        metrics.module.total_loc,
        metrics.module.total_functions,
        metrics.module.sum_cc,
        metrics.module.declaration_count
    );
    for f in &metrics.functions {
        eprintln!(
            "  {} (L{}-{}): loc={} cc={} cogc={} nesting={} bumps={} args={} conditions={} embedded={} asserts={} primitives={}/{} short_vars={} str_match={} fields={:?}",
            f.name, f.start_line, f.end_line, f.loc, f.cc, f.cognitive_complexity, f.max_nesting, f.bump_count,
            f.arg_count, f.compound_condition_count, f.max_embedded_block_loc,
            f.consecutive_asserts, f.primitive_type_count, f.typed_param_count,
            f.short_var_count, f.string_match_arms, f.field_accesses
        );
    }
}

fn run_check(file_path: &str, json: bool) {
    let cfg = config::load_config(Path::new(file_path));
    let result = analyze::analyze_file(file_path, cfg.as_ref(), analyze::ScanOptions::check());
    if json {
        let records: Vec<output::CheckFinding> = result
            .as_ref()
            .map_or_else(Vec::new, |r| r.findings.iter().map(|f| output::to_check_finding(&r.filename, f)).collect());
        println!("{}", output::format_check_json(&records));
        process::exit(i32::from(!records.is_empty()));
    }
    let Some(result) = result else {
        process::exit(0);
    };
    if result.findings.is_empty() {
        process::exit(0);
    }
    print!("{}", output::format(&result.findings, &result.filename));
}

fn run_budget(file_path: &str) {
    let path = Path::new(file_path);
    let cfg = config::load_config(path);

    if cfg.as_ref().is_some_and(|c| config::is_ignored_for_file(c, path)) {
        eprintln!("budget: {file_path} — ignored by .pulse.toml");
        return;
    }

    let Some(lang) = parse::detect_language(path) else {
        eprintln!("budget: {file_path} — unsupported or unreadable");
        return;
    };
    let Some(metrics) = std::fs::read_to_string(path).ok().and_then(|s| parse::parse_and_walk_guarded(&s, lang)) else {
        eprintln!("budget: {file_path} — unsupported or unreadable");
        return;
    };

    let t = config::resolve_thresholds(cfg.as_ref(), lang);
    let fn_count = metrics.functions.len() as u32;
    let fn_room = t.module.file_function_count.saturating_sub(fn_count);
    let loc_room = t.module.file_loc_warning.saturating_sub(metrics.module.total_loc);
    let cc_room = t.module.file_total_cc.saturating_sub(metrics.module.sum_cc);

    eprintln!("budget: {file_path}");
    eprintln!("  functions: {fn_count}/{} (room: {fn_room})", t.module.file_function_count);
    eprintln!("  LOC:       {}/{} (room: {loc_room})", metrics.module.total_loc, t.module.file_loc_warning);
    eprintln!("  total cc:  {}/{} (room: {cc_room})", metrics.module.sum_cc, t.module.file_total_cc);
    eprintln!(
        "  per-function limits: cc<{}, cogc<{}, loc<{}, args≤{}",
        t.function.cc_warning, t.function.cogc_warning, t.function.fn_loc_warning, t.function.arg_max
    );
}

fn run_budget_new() {
    let cfg = config::load_config(Path::new("."));
    let t = config::resolve_base_thresholds(cfg.as_ref());
    eprintln!("budget: new file thresholds");
    eprintln!("  max functions: {}", t.module.file_function_count);
    eprintln!("  max LOC:       {}", t.module.file_loc_warning);
    eprintln!("  max total cc:  {}", t.module.file_total_cc);
    eprintln!(
        "  per-function:  cc<{}, cogc<{}, loc<{}, args≤{}",
        t.function.cc_warning, t.function.cogc_warning, t.function.fn_loc_warning, t.function.arg_max
    );
}

fn run_check_all(include_tests: bool, json: bool) {
    let (cfg, root) = config::load_config_with_root(Path::new(".")).map_or((None, None), |(c, r)| (Some(c), Some(r)));
    let matcher = cfg.as_ref().map(|c| config::IgnoreMatcher::from_patterns(&c.ignore.paths));
    let mut total = 0;
    let mut records: Vec<output::CheckFinding> = Vec::new();
    for entry in walk_source_files(Path::new(".")) {
        let path_str = entry.to_string_lossy();
        if should_skip_walk_entry(&entry, &path_str, include_tests, matcher.as_ref(), root.as_deref()) {
            continue;
        }
        let Some(result) = analyze::analyze_file(&path_str, cfg.as_ref(), analyze::ScanOptions::check()) else {
            continue;
        };
        if result.findings.is_empty() {
            continue;
        }
        total += result.findings.len();
        if json {
            records.extend(result.findings.iter().map(|f| output::to_check_finding(&result.filename, f)));
        } else {
            print!("{}", output::format(&result.findings, &result.filename));
        }
    }
    if json {
        println!("{}", output::format_check_json(&records));
    }
    if total > 0 {
        process::exit(1);
    }
}

fn should_skip_walk_entry(
    entry: &Path,
    path_str: &str,
    include_tests: bool,
    matcher: Option<&config::IgnoreMatcher>,
    root: Option<&Path>,
) -> bool {
    if !include_tests && test_detection::is_test_file(path_str) {
        return true;
    }
    matcher.zip(root).is_some_and(|(m, r)| m.matches_file(r, entry))
}

const MAX_WALK_FILES: usize = 100_000;

fn walk_source_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut visited = std::collections::HashSet::new();
    walk_source_dir(dir, &mut files, &mut visited);
    files.sort();
    files
}

fn walk_source_dir(dir: &Path, files: &mut Vec<PathBuf>, visited: &mut std::collections::HashSet<(u64, u64)>) {
    if !mark_dir_visited(dir, visited) {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if files.len() >= MAX_WALK_FILES {
            return;
        }
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.starts_with('.') || SKIP_DIRS.contains(&name) {
                continue;
            }
            walk_source_dir(&path, files, visited);
        } else if parse::detect_language(&path).is_some() {
            files.push(path);
        }
    }
}

#[cfg(unix)]
fn mark_dir_visited(dir: &Path, visited: &mut std::collections::HashSet<(u64, u64)>) -> bool {
    use std::os::unix::fs::MetadataExt;
    let Ok(meta) = std::fs::metadata(dir) else {
        return false;
    };
    visited.insert((meta.dev(), meta.ino()))
}

#[cfg(not(unix))]
fn mark_dir_visited(_dir: &Path, _visited: &mut std::collections::HashSet<(u64, u64)>) -> bool {
    true
}
