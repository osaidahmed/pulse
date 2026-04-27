mod analytics;
mod baselines;
mod config;
mod duplication;
mod hook;
mod module_smells;
mod output;
mod parse;
mod setup;
mod smells;
mod test_detection;
mod thresholds;
mod walk;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process;

use smells::{Finding, Location};

enum Command {
    Hook(hook::HookInput),
    Check(String),
    CheckAll { include_tests: bool },
    Debug(String),
    Budget(Option<String>),
    Stop,
    Cleanup,
}

const CHECKPOINT_INTERVAL: u32 = 5;
const CHECKPOINT_INTERVAL_NEW: u32 = 2;

const SKIP_DIRS: &[&str] = &[
    "node_modules",
    "target",
    "vendor",
    "build",
    "dist",
];

fn parse_args() -> Command {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map_or("", std::string::String::as_str);
    let second = args.get(2).map(String::as_str);

    if cmd == "setup" {
        setup::run_setup();
        process::exit(0);
    }

    if let Some(early) = parse_early_command(cmd, second) {
        return early;
    }

    match cmd {
        "--hook" => hook::parse_hook_input().map_or_else(|| process::exit(0), Command::Hook),
        "--stop" => Command::Stop,
        "--cleanup" => Command::Cleanup,
        "check" | "debug" | "budget" if second.is_some() => file_command(cmd, args[2].clone()),
        _ => {
            eprintln!("usage: pulse setup | --hook | --stop | --cleanup | check <file> | debug <file> | budget <file> | -a/--all [--include-tests]");
            process::exit(1);
        }
    }
}

fn parse_early_command(cmd: &str, second: Option<&str>) -> Option<Command> {
    if matches!(cmd, "-a" | "--all") || (cmd == "check" && second.is_some_and(|a| a == "-a" || a == "--all")) {
        let include_tests = std::env::args().any(|a| a == "--include-tests" || a == "-t");
        return Some(Command::CheckAll { include_tests });
    }
    if cmd == "budget" && second.is_some_and(|a| a == "--new") {
        return Some(Command::Budget(None));
    }
    None
}

fn file_command(cmd: &str, path: String) -> Command {
    if cmd == "debug" { return Command::Debug(path); }
    if cmd == "budget" { return Command::Budget(Some(path)); }
    Command::Check(path)
}

fn read_session_id_from_stdin() -> Option<String> {
    let mut input = String::new();
    std::io::Read::read_to_string(&mut std::io::stdin(), &mut input).ok()?;
    let v: serde_json::Value = serde_json::from_str(&input).ok()?;
    v.get("session_id")?.as_str().map(String::from)
}

fn main() {
    match parse_args() {
        Command::Debug(p) => run_debug(&p),
        Command::Check(p) => run_check(&p),
        Command::CheckAll { include_tests } => run_check_all(include_tests),
        Command::Budget(p) => p.as_deref().map_or_else(run_budget_new, run_budget),
        Command::Hook(h) => {
            baselines::init_session_dir(h.session_id.as_deref());
            run_hook(h);
        }
        Command::Stop => {
            baselines::init_session_dir(read_session_id_from_stdin().as_deref());
            run_stop();
        }
        Command::Cleanup => {
            baselines::init_session_dir(read_session_id_from_stdin().as_deref());
            run_cleanup();
        }
    }
}

fn run_debug(file_path: &str) {
    let path = Path::new(file_path);
    let lang = parse::detect_language(path).expect("unsupported language");
    let source = std::fs::read_to_string(path).expect("can't read file");
    let metrics = parse::parse_and_walk(&source, lang).expect("parse failed");
    eprintln!(
        "Module: {} LOC, {} functions, sum_cc={} declarations={}",
        metrics.module.total_loc, metrics.module.total_functions, metrics.module.sum_cc, metrics.module.declaration_count
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

struct AnalysisResult {
    findings: Vec<Finding>,
    filename: String,
    fn_count: u32,
    total_loc: u32,
    sum_cc: u32,
}

fn analyze_file(file_path: &str, cfg: Option<&config::PulseConfig>) -> Option<AnalysisResult> {
    let path = Path::new(file_path);
    if !path.exists() {
        return None;
    }
    let lang = parse::detect_language(path)?;
    let source = std::fs::read_to_string(path).ok()?;
    let metrics = parse::parse_and_walk(&source, lang)?;
    let t = config::resolve_thresholds(cfg, lang);
    let disabled = config::resolve_disabled(cfg);
    let mut findings = smells::detect(&metrics, &source, &t);
    config::filter_disabled(&mut findings, &disabled);
    let filename = path.file_name()?.to_string_lossy().into_owned();
    Some(AnalysisResult {
        findings,
        filename,
        fn_count: metrics.functions.len() as u32,
        total_loc: metrics.module.total_loc,
        sum_cc: metrics.module.sum_cc,
    })
}

fn run_check(file_path: &str) {
    let cfg = config::load_config(Path::new(file_path));
    let Some(result) = analyze_file(file_path, cfg.as_ref()) else {
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

    let Some(lang) = parse::detect_language(path) else {
        eprintln!("budget: {file_path} — unsupported or unreadable");
        return;
    };
    let Some(metrics) = std::fs::read_to_string(path).ok().and_then(|s| parse::parse_and_walk(&s, lang))
    else {
        eprintln!("budget: {file_path} — unsupported or unreadable");
        return;
    };

    let t = config::resolve_thresholds(cfg.as_ref(), lang);
    let fn_count = metrics.functions.len() as u32;
    let fn_room = t.file_function_count.saturating_sub(fn_count);
    let loc_room = t.file_loc_warning.saturating_sub(metrics.module.total_loc);
    let cc_room = t.file_total_cc.saturating_sub(metrics.module.sum_cc);

    eprintln!("budget: {file_path}");
    eprintln!("  functions: {fn_count}/{} (room: {fn_room})", t.file_function_count);
    eprintln!("  LOC:       {}/{} (room: {loc_room})", metrics.module.total_loc, t.file_loc_warning);
    eprintln!("  total cc:  {}/{} (room: {cc_room})", metrics.module.sum_cc, t.file_total_cc);
    eprintln!("  per-function limits: cc<{}, cogc<{}, loc<{}, args≤{}", t.cc_warning, t.cogc_warning, t.fn_loc_warning, t.arg_max);
}

fn run_budget_new() {
    let cfg = config::load_config(Path::new("."));
    let t = config::resolve_base_thresholds(cfg.as_ref());
    eprintln!("budget: new file thresholds");
    eprintln!("  max functions: {}", t.file_function_count);
    eprintln!("  max LOC:       {}", t.file_loc_warning);
    eprintln!("  max total cc:  {}", t.file_total_cc);
    eprintln!("  per-function:  cc<{}, cogc<{}, loc<{}, args≤{}", t.cc_warning, t.cogc_warning, t.fn_loc_warning, t.arg_max);
}

fn run_check_all(include_tests: bool) {
    let cfg = config::load_config(Path::new("."));
    let mut total = 0;
    for entry in walk_source_files(Path::new(".")) {
        let path_str = entry.to_string_lossy();
        if !include_tests && test_detection::is_test_file(&path_str) {
            continue;
        }
        if let Some(result) = analyze_file(&path_str, cfg.as_ref()) {
            if !result.findings.is_empty() {
                total += result.findings.len();
                print!("{}", output::format(&result.findings, &result.filename));
            }
        }
    }
    if total > 0 {
        process::exit(1);
    }
}

fn walk_source_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return files;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.starts_with('.') || SKIP_DIRS.contains(&name) {
                continue;
            }
            files.extend(walk_source_files(&path));
        } else if parse::detect_language(&path).is_some() {
            files.push(path);
        }
    }
    files.sort();
    files
}

fn run_hook(h: hook::HookInput) {
    if std::env::var("PULSE_DISABLE").is_ok() || test_detection::is_test_file(&h.file_path) {
        return;
    }
    let cfg = config::load_config(Path::new(&h.file_path));
    analytics::save_session_id(&h);
    baselines::cache_baseline(&h, cfg.as_ref());
    let Some(result) = analyze_file(&h.file_path, cfg.as_ref()) else {
        process::exit(0);
    };

    let edit_count = baselines::increment_edit_count(&h.file_path);
    let func_baseline = baselines::load_function_baseline(&h.file_path);

    let (module_findings, func_findings): (Vec<_>, Vec<_>) = result.findings
        .into_iter()
        .partition(|f| matches!(f.location, Location::Module));

    let mut findings: Vec<Finding> = hook::filter_by_edit_range(func_findings, h.edit_range)
        .into_iter()
        .filter(|f| !baselines::is_preexisting_finding(f, &func_baseline))
        .collect();

    collect_module_findings(&h.file_path, edit_count, module_findings, &mut findings, cfg.as_ref());

    if findings.is_empty() {
        process::exit(0);
    }
    analytics::log_findings(&h, &findings, &result.filename);
    let lang = parse::detect_language(Path::new(&h.file_path)).unwrap_or(parse::Language::Python);
    let t = config::resolve_thresholds(cfg.as_ref(), lang);
    let budget = format!(
        "[budget] fn={}/{} loc={}/{} cc={}/{}",
        result.fn_count, t.file_function_count,
        result.total_loc, t.file_loc_warning,
        result.sum_cc, t.file_total_cc,
    );
    let conflict = detect_constraint_conflict(&findings, result.fn_count, &t);
    let reason = match conflict {
        Some(note) => format!("{}\n{}\n{}", output::format_compact(&findings, &result.filename).trim(), note, budget),
        None => format!("{}\n{}", output::format_compact(&findings, &result.filename).trim(), budget),
    };
    let decision = serde_json::json!({
        "decision": "block",
        "reason": reason.trim()
    });
    println!("{decision}");
}

fn detect_constraint_conflict(
    findings: &[Finding],
    fn_count: u32,
    t: &thresholds::Thresholds,
) -> Option<&'static str> {
    let fn_tight = fn_count + 2 >= t.file_function_count;
    let has_cc_finding = findings.iter().any(|f| matches!(
        f.smell,
        smells::Smell::ComplexMethod | smells::Smell::GodMethod
    ));
    if fn_tight && has_cc_finding {
        return Some("[conflict] fn count and per-function complexity are both constrained — merge only low-cc functions");
    }
    None
}

fn collect_module_findings(
    file_path: &str,
    edit_count: u32,
    module_findings: Vec<Finding>,
    findings: &mut Vec<Finding>,
    cfg: Option<&config::PulseConfig>,
) {
    if edit_count == 1 {
        let baseline = baselines::load_baseline(file_path);
        findings.extend(module_findings.into_iter().filter(|f| {
            baseline.get(f.smell.as_str()).copied().unwrap_or(0) == 0
        }));
    }

    let interval = if edit_count <= CHECKPOINT_INTERVAL_NEW { CHECKPOINT_INTERVAL_NEW } else { CHECKPOINT_INTERVAL };
    if edit_count.is_multiple_of(interval) && !test_detection::is_test_file(file_path) {
        if let Some((_, regressions)) = detect_regressions(file_path, cfg) {
            findings.extend(regressions);
        }
    }
}

fn run_stop() {
    let Ok(manifest) = std::fs::read_to_string(baselines::baseline_dir().join("manifest.txt"))
    else {
        return;
    };

    let cfg = config::load_config(Path::new("."));
    let mut all_regressions: Vec<(String, Vec<Finding>)> = Vec::new();
    for file_path in manifest.lines().filter(|l| !l.trim().is_empty()) {
        if test_detection::is_test_file(file_path) || baselines::is_fixture_file(file_path) { continue; }
        if let Some((filename, regressions)) = detect_regressions(file_path, cfg.as_ref()) {
            all_regressions.push((filename, regressions));
        }
    }

    if !all_regressions.is_empty() {
        let reason = output::format_stop(&all_regressions);
        let decision = serde_json::json!({
            "decision": "block",
            "reason": reason.trim()
        });
        println!("{decision}");
    }

    analytics::resolve(|p| analyze_file(p, cfg.as_ref()).map(|r| (r.findings, r.filename)));
    let _ = std::fs::remove_dir_all(baselines::baseline_dir());
}

fn detect_regressions(file_path: &str, cfg: Option<&config::PulseConfig>) -> Option<(String, Vec<Finding>)> {
    let baseline = baselines::load_baseline(file_path);
    let result = analyze_file(file_path, cfg)?;

    let mut current_counts: HashMap<smells::Smell, usize> = HashMap::new();
    for f in result.findings.iter().filter(|f| matches!(f.location, Location::Module)) {
        *current_counts.entry(f.smell).or_default() += 1;
    }

    let regressions: Vec<Finding> = result.findings
        .into_iter()
        .filter(|f| matches!(f.location, Location::Module))
        .filter(|f| {
            let baseline_count = baseline.get(f.smell.as_str()).copied().unwrap_or(0);
            let current_count = current_counts.get(&f.smell).copied().unwrap_or(0);
            current_count > baseline_count
        })
        .collect();

    if regressions.is_empty() {
        return None;
    }
    Some((result.filename, regressions))
}

fn run_cleanup() {
    let _ = std::fs::remove_dir_all(baselines::baseline_dir());
}
