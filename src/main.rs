mod output;
mod parse;
mod smells;
mod thresholds;
mod walk;

use std::io::Read;
use std::path::Path;
use std::process;

use smells::{Finding, Location};

struct HookInput {
    file_path: String,
    edit_range: Option<(u32, u32)>,
}

enum Command {
    Hook(HookInput),
    Check(String),
    Debug(String),
}

fn parse_args() -> Command {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 && args[1] == "--hook" {
        let hook = match parse_hook_input() {
            Some(h) => h,
            None => process::exit(0),
        };
        return Command::Hook(hook);
    }

    if args.len() > 2 && args[1] == "check" {
        return Command::Check(args[2].clone());
    }

    if args.len() > 2 && args[1] == "debug" {
        return Command::Debug(args[2].clone());
    }

    eprintln!("usage: pulse --hook  (reads hook JSON from stdin)");
    eprintln!("       pulse check <file>");
    process::exit(1);
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
        Command::Debug(p) => {
            run_debug(&p);
        }
        Command::Check(p) => {
            run_check(&p);
        }
        Command::Hook(h) => {
            run_hook(h);
        }
    }
}

fn parse_hook_input() -> Option<HookInput> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).ok()?;
    let v: serde_json::Value = serde_json::from_str(&input).ok()?;
    let tool_input = v.get("tool_input")?;
    let file_path = tool_input.get("file_path")?.as_str()?.to_string();

    let edit_range = compute_edit_range(tool_input, &file_path);

    Some(HookInput {
        file_path,
        edit_range,
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
        None => return findings,
    };

    findings
        .into_iter()
        .filter(|f| match &f.location {
            Location::Function {
                start_line,
                end_line,
                ..
            } => *start_line <= end && *end_line >= start,
            Location::Module => true,
        })
        .collect()
}
