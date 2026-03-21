mod analytics;
mod baselines;
mod hook;
mod module_smells;
mod output;
mod parse;
mod smells;
mod thresholds;
mod walk;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process;

use smells::{Finding, Location};

enum Command {
    Hook(hook::HookInput),
    Check(String),
    CheckAll,
    Debug(String),
    Stop,
    Cleanup,
}

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

    match cmd {
        "--hook" => {
            let Some(h) = hook::parse_hook_input() else {
                process::exit(0);
            };
            Command::Hook(h)
        }
        "--stop" => Command::Stop,
        "--cleanup" => Command::Cleanup,
        "check" if args.get(2).is_some_and(|a| a == "-a" || a == "--all") => Command::CheckAll,
        "check" if args.len() > 2 => Command::Check(args[2].clone()),
        "debug" if args.len() > 2 => Command::Debug(args[2].clone()),
        "-a" | "--all" => Command::CheckAll,
        _ => {
            eprintln!("usage: pulse --hook | --stop | --cleanup | check <file> | debug <file> | -a/--all");
            process::exit(1);
        }
    }
}

fn main() {
    match parse_args() {
        Command::Debug(p) => run_debug(&p),
        Command::Check(p) => run_check(&p),
        Command::CheckAll => run_check_all(),
        Command::Hook(h) => run_hook(h),
        Command::Stop => run_stop(),
        Command::Cleanup => run_cleanup(),
    }
}

fn run_debug(file_path: &str) {
    let path = Path::new(file_path);
    let lang = parse::detect_language(path).expect("unsupported language");
    let source = std::fs::read_to_string(path).expect("can't read file");
    let (functions, module) = parse::parse_and_walk(&source, lang).expect("parse failed");
    eprintln!(
        "Module: {} LOC, {} functions, sum_cc={} declarations={}",
        module.total_loc, module.total_functions, module.sum_cc, module.declaration_count
    );
    for f in &functions {
        eprintln!(
            "  {} (L{}-{}): loc={} cc={} cogc={} nesting={} bumps={} args={} conditions={} embedded={} asserts={} primitives={}/{} fields={:?}",
            f.name, f.start_line, f.end_line, f.loc, f.cc, f.cognitive_complexity, f.max_nesting, f.bump_count,
            f.arg_count, f.compound_condition_count, f.max_embedded_block_loc,
            f.consecutive_asserts, f.primitive_type_count, f.typed_param_count, f.field_accesses
        );
    }
}

fn analyze_file(file_path: &str) -> Option<(Vec<Finding>, String)> {
    let path = Path::new(file_path);
    if !path.exists() {
        return None;
    }
    let lang = parse::detect_language(path)?;
    let source = std::fs::read_to_string(path).ok()?;
    let metrics = parse::parse_and_walk(&source, lang)?;
    let t = thresholds::Thresholds::default();
    let findings = smells::detect(&metrics, &source, &t);
    let filename = path.file_name()?.to_string_lossy().into_owned();
    Some((findings, filename))
}

fn run_check(file_path: &str) {
    let Some((findings, filename)) = analyze_file(file_path) else {
        process::exit(0);
    };
    if findings.is_empty() {
        process::exit(0);
    }
    print!("{}", output::format(&findings, &filename));
}

fn run_check_all() {
    let mut total = 0;
    for entry in walk_source_files(Path::new(".")) {
        let path_str = entry.to_string_lossy();
        if let Some((findings, filename)) = analyze_file(&path_str) {
            if !findings.is_empty() {
                total += findings.len();
                print!("{}", output::format(&findings, &filename));
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
    if std::env::var("PULSE_DISABLE").is_ok() {
        return;
    }
    analytics::save_session_id(&h);
    baselines::cache_baseline(&h);
    let Some((all_findings, filename)) = analyze_file(&h.file_path) else {
        process::exit(0);
    };
    let findings = hook::filter_by_edit_range(all_findings, h.edit_range);
    if findings.is_empty() {
        process::exit(0);
    }
    analytics::log_findings(&h, &findings, &filename);
    let reason = output::format_compact(&findings, &filename);
    let decision = serde_json::json!({
        "decision": "block",
        "reason": reason.trim()
    });
    println!("{decision}");
}

fn run_stop() {
    let Ok(manifest) = std::fs::read_to_string(baselines::baseline_dir().join("manifest.txt"))
    else {
        return;
    };

    let mut all_regressions: Vec<(String, Vec<Finding>)> = Vec::new();
    for file_path in manifest.lines().filter(|l| !l.trim().is_empty()) {
        if let Some((filename, regressions)) = detect_regressions(file_path) {
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

    analytics::resolve(analyze_file);
    let _ = std::fs::remove_dir_all(baselines::baseline_dir());
}

fn detect_regressions(file_path: &str) -> Option<(String, Vec<Finding>)> {
    let baseline = baselines::load_baseline(file_path);
    let (findings, filename) = analyze_file(file_path)?;

    let mut current_counts: HashMap<&str, usize> = HashMap::new();
    for f in findings.iter().filter(|f| matches!(f.location, Location::Module)) {
        *current_counts.entry(f.smell).or_default() += 1;
    }

    let regressions: Vec<Finding> = findings
        .into_iter()
        .filter(|f| matches!(f.location, Location::Module))
        .filter(|f| {
            let baseline_count = baseline.get(f.smell).copied().unwrap_or(0);
            let current_count = current_counts.get(f.smell).copied().unwrap_or(0);
            current_count > baseline_count
        })
        .collect();

    if regressions.is_empty() {
        return None;
    }
    Some((filename, regressions))
}

fn run_cleanup() {
    let _ = std::fs::remove_dir_all(baselines::baseline_dir());
}
