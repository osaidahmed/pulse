mod output;
mod parse;
mod smells;
mod thresholds;
mod walk;

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process;

use smells::{Finding, Location};

struct HookInput {
    file_path: String,
    edit_range: Option<(u32, u32)>,
    old_string: Option<String>,
    new_string: Option<String>,
}

enum Command {
    Hook(HookInput),
    Check(String),
    Debug(String),
    Stop,
    Cleanup,
}

fn parse_args() -> Command {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("");

    match cmd {
        "--hook" => {
            let hook = match parse_hook_input() {
                Some(h) => h,
                None => process::exit(0),
            };
            Command::Hook(hook)
        }
        "--stop" => Command::Stop,
        "--cleanup" => Command::Cleanup,
        "check" if args.len() > 2 => Command::Check(args[2].clone()),
        "debug" if args.len() > 2 => Command::Debug(args[2].clone()),
        _ => {
            eprintln!("usage: pulse --hook | --stop | --cleanup | check <file> | debug <file>");
            process::exit(1);
        }
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
            "  {} (L{}-{}): loc={} cc={} nesting={} bumps={} args={} conditions={} embedded={} asserts={} primitives={}/{} fields={:?}",
            f.name, f.start_line, f.end_line, f.loc, f.cc, f.max_nesting, f.bump_count,
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
    let thresholds = thresholds::Thresholds::default();
    let findings = smells::detect(&metrics, &source, &thresholds);
    let filename = path.file_name()?.to_string_lossy().into_owned();
    Some((findings, filename))
}

fn baseline_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("PULSE_BASELINE_DIR") {
        return PathBuf::from(dir);
    }
    PathBuf::from("/tmp/pulse-baselines")
}

fn baseline_path(file_path: &str) -> PathBuf {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    file_path.hash(&mut hasher);
    baseline_dir().join(format!("{:016x}.json", hasher.finish()))
}

fn count_module_findings(findings: &[Finding]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for f in findings {
        if matches!(f.location, Location::Module) {
            *counts.entry(f.smell.to_string()).or_default() += 1;
        }
    }
    counts
}

fn cache_baseline(hook: &HookInput) {
    let bp = baseline_path(&hook.file_path);
    if bp.exists() {
        return;
    }

    let counts = compute_baseline_counts(hook);
    write_baseline(&bp, &counts);
    append_manifest(&hook.file_path);
}

fn compute_baseline_counts(hook: &HookInput) -> HashMap<String, usize> {
    let source = match reconstruct_pre_edit(hook) {
        Some(s) if !s.is_empty() => s,
        _ => return HashMap::new(),
    };
    let lang = match parse::detect_language(Path::new(&hook.file_path)) {
        Some(l) => l,
        None => return HashMap::new(),
    };
    let metrics = match parse::parse_and_walk(&source, lang) {
        Some(m) => m,
        None => return HashMap::new(),
    };
    let thresholds = thresholds::Thresholds::default();
    let findings = smells::detect(&metrics, &source, &thresholds);
    count_module_findings(&findings)
}

fn reconstruct_pre_edit(hook: &HookInput) -> Option<String> {
    if let (Some(old_str), Some(new_str)) = (&hook.old_string, &hook.new_string) {
        let current = std::fs::read_to_string(&hook.file_path).ok()?;
        return Some(current.replacen(new_str, old_str, 1));
    }

    let output = std::process::Command::new("git")
        .args(["show", &format!("HEAD:{}", &hook.file_path)])
        .output()
        .ok()?;
    if output.status.success() {
        return Some(String::from_utf8_lossy(&output.stdout).into_owned());
    }

    None
}

fn write_baseline(path: &Path, counts: &HashMap<String, usize>) {
    let dir = baseline_dir();
    let _ = std::fs::create_dir_all(&dir);
    let json = serde_json::to_string(counts).unwrap_or_default();
    let _ = std::fs::write(path, json);
}

fn load_baseline(file_path: &str) -> HashMap<String, usize> {
    let bp = baseline_path(file_path);
    let json = match std::fs::read_to_string(&bp) {
        Ok(j) => j,
        Err(_) => return HashMap::new(),
    };
    serde_json::from_str(&json).unwrap_or_default()
}

fn append_manifest(file_path: &str) {
    let manifest = baseline_dir().join("manifest.txt");
    let existing = std::fs::read_to_string(&manifest).unwrap_or_default();
    if existing.lines().any(|l| l == file_path) {
        return;
    }
    use std::io::Write;
    let _ = std::fs::create_dir_all(baseline_dir());
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&manifest)
    {
        let _ = writeln!(f, "{}", file_path);
    }
}

fn run_check(file_path: &str) {
    let (findings, filename) = match analyze_file(file_path) {
        Some(r) => r,
        None => process::exit(0),
    };
    if findings.is_empty() {
        process::exit(0);
    }
    print!("{}", output::format(&findings, &filename));
}

fn run_hook(hook: HookInput) {
    cache_baseline(&hook);
    let (all_findings, filename) = match analyze_file(&hook.file_path) {
        Some(r) => r,
        None => process::exit(0),
    };
    let findings = filter_by_edit_range(all_findings, hook.edit_range);
    if findings.is_empty() {
        process::exit(0);
    }
    print!("{}", output::format_compact(&findings, &filename));
}

fn main() {
    match parse_args() {
        Command::Debug(p) => run_debug(&p),
        Command::Check(p) => run_check(&p),
        Command::Hook(h) => run_hook(h),
        Command::Stop => run_stop(),
        Command::Cleanup => run_cleanup(),
    }
}

fn run_stop() {
    let manifest = match std::fs::read_to_string(baseline_dir().join("manifest.txt")) {
        Ok(m) => m,
        Err(_) => return,
    };

    let mut all_regressions: Vec<(String, Vec<Finding>)> = Vec::new();
    for file_path in manifest.lines().filter(|l| !l.trim().is_empty()) {
        if let Some((filename, regressions)) = detect_regressions(file_path) {
            all_regressions.push((filename, regressions));
        }
    }

    if !all_regressions.is_empty() {
        print!("{}", output::format_stop(&all_regressions));
    }

    let _ = std::fs::remove_dir_all(baseline_dir());
}

fn detect_regressions(file_path: &str) -> Option<(String, Vec<Finding>)> {
    let baseline = load_baseline(file_path);
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
    let _ = std::fs::remove_dir_all(baseline_dir());
}

fn parse_hook_input() -> Option<HookInput> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).ok()?;
    let v: serde_json::Value = serde_json::from_str(&input).ok()?;
    let tool_input = v.get("tool_input")?;
    let file_path = tool_input.get("file_path")?.as_str()?.to_string();

    let old_string = tool_input
        .get("old_string")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let new_string = tool_input
        .get("new_string")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let edit_range = compute_edit_range(tool_input, &file_path);

    Some(HookInput {
        file_path,
        edit_range,
        old_string,
        new_string,
    })
}

fn compute_edit_range(
    tool_input: &serde_json::Value,
    file_path: &str,
) -> Option<(u32, u32)> {
    let new_string = tool_input.get("new_string")?.as_str()?;
    let old_string = tool_input.get("old_string")?.as_str()?;

    let source = std::fs::read_to_string(file_path).ok()?;
    let start_byte = source.find(new_string).or_else(|| source.find(old_string))?;

    let start_line = source[..start_byte].matches('\n').count() as u32 + 1;
    let new_lines = new_string.matches('\n').count() as u32;
    let end_line = start_line + new_lines;

    Some((start_line, end_line))
}

pub fn filter_by_edit_range(
    findings: Vec<Finding>,
    range: Option<(u32, u32)>,
) -> Vec<Finding> {
    let (start, end) = match range {
        Some(r) => r,
        None => {
            return findings
                .into_iter()
                .filter(|f| !matches!(f.location, Location::Module))
                .collect();
        }
    };

    findings
        .into_iter()
        .filter(|f| match &f.location {
            Location::Function {
                start_line,
                end_line,
                ..
            } => *start_line <= end && *end_line >= start,
            Location::Module => false,
        })
        .collect()
}
