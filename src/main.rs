mod output;
mod parse;
mod smells;
mod thresholds;
mod walk;

use std::io::Read;
use std::path::Path;
use std::process;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let file_path = if args.len() > 1 && args[1] == "--hook" {
        match read_hook_file_path() {
            Some(p) => p,
            None => process::exit(0),
        }
    } else if args.len() > 2 && args[1] == "check" {
        args[2].clone()
    } else if args.len() > 2 && args[1] == "debug" {
        let file_path = args[2].clone();
        let path = Path::new(&file_path);
        let lang = parse::detect_language(path).expect("unsupported language");
        let source = std::fs::read_to_string(path).expect("can't read file");
        let (functions, module) = parse::parse_and_walk(&source, lang).expect("parse failed");
        eprintln!("Module: {} LOC, {} functions, sum_cc={}", module.total_loc, module.total_functions, module.sum_cc);
        for f in &functions {
            eprintln!("  {} (L{}-{}): loc={} cc={} nesting={} bumps={} args={} conditions={} embedded={}",
                f.name, f.start_line, f.end_line, f.loc, f.cc, f.max_nesting, f.bump_count, f.arg_count, f.compound_condition_count, f.max_embedded_block_loc);
        }
        process::exit(0);
    } else {
        eprintln!("usage: pulse --hook  (reads hook JSON from stdin)");
        eprintln!("       pulse check <file>");
        process::exit(1);
    };

    let path = Path::new(&file_path);
    if !path.exists() {
        process::exit(0);
    }

    let lang = match parse::detect_language(path) {
        Some(l) => l,
        None => process::exit(0),
    };

    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => process::exit(0),
    };

    let functions = match parse::parse_and_walk(&source, lang) {
        Some(f) => f,
        None => process::exit(0),
    };

    let thresholds = thresholds::Thresholds::default();
    let findings = smells::detect(&functions, &source, &thresholds);

    if findings.is_empty() {
        process::exit(0);
    }

    let filename = path.file_name().unwrap_or_default().to_string_lossy();
    print!("{}", output::format(&findings, &filename));
}

fn read_hook_file_path() -> Option<String> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).ok()?;
    let v: serde_json::Value = serde_json::from_str(&input).ok()?;
    v.get("tool_input")?.get("file_path")?.as_str().map(String::from)
}
