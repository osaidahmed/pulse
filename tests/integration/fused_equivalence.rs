use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use pulse::parse;
use pulse::smells::{self, Finding, Location};
use pulse::thresholds::Thresholds;
use pulse::walk::{FunctionMetrics, ModuleMetrics};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
struct FnSnap {
    name: String,
    start_line: u32,
    end_line: u32,
    loc: u32,
    cc: u32,
    cognitive_complexity: u32,
    max_nesting: u32,
    bump_count: u32,
    arg_count: u32,
    compound_condition_count: u32,
    is_constructor: bool,
    max_embedded_block_loc: u32,
    consecutive_asserts: u32,
    primitive_type_count: u32,
    typed_param_count: u32,
    empty_catch_count: u32,
    short_var_count: u32,
    string_match_arms: u32,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
struct ModSnap {
    total_loc: u32,
    total_functions: u32,
    sum_cc: u32,
    global_conditional_count: u32,
    global_max_nesting: u32,
    declaration_count: u32,
    struct_fields: Vec<(String, u32)>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct FindingSnap {
    smell: String,
    location: String,
    detail: String,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
struct FileSnap {
    path: String,
    module: ModSnap,
    functions: Vec<FnSnap>,
    findings: Vec<FindingSnap>,
}

fn fn_snap(f: &FunctionMetrics) -> FnSnap {
    FnSnap {
        name: f.name.clone(),
        start_line: f.start_line,
        end_line: f.end_line,
        loc: f.loc,
        cc: f.cc,
        cognitive_complexity: f.cognitive_complexity,
        max_nesting: f.max_nesting,
        bump_count: f.bump_count,
        arg_count: f.arg_count,
        compound_condition_count: f.compound_condition_count,
        is_constructor: f.is_constructor,
        max_embedded_block_loc: f.max_embedded_block_loc,
        consecutive_asserts: f.consecutive_asserts,
        primitive_type_count: f.primitive_type_count,
        typed_param_count: f.typed_param_count,
        empty_catch_count: f.empty_catch_count,
        short_var_count: f.short_var_count,
        string_match_arms: f.string_match_arms,
    }
}

fn mod_snap(m: &ModuleMetrics) -> ModSnap {
    ModSnap {
        total_loc: m.total_loc,
        total_functions: m.total_functions,
        sum_cc: m.sum_cc,
        global_conditional_count: m.global_conditional_count,
        global_max_nesting: m.global_max_nesting,
        declaration_count: m.declaration_count,
        struct_fields: m.struct_fields.clone(),
    }
}

fn location_key(loc: &Location) -> String {
    match loc {
        Location::Function { name, start_line, end_line } => {
            format!("fn:{name}:{start_line}:{end_line}")
        }
        Location::Module => "module".to_string(),
    }
}

fn finding_snaps(findings: &[Finding]) -> Vec<FindingSnap> {
    let mut out: Vec<FindingSnap> = findings
        .iter()
        .map(|f| FindingSnap {
            smell: f.smell.as_str().to_string(),
            location: location_key(&f.location),
            detail: f.detail.clone(),
        })
        .collect();
    out.sort();
    out
}

fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

fn collect_source_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            collect_source_files(&path, out);
        } else if parse::detect_language(&path).is_some() {
            out.push(path);
        }
    }
}

fn build_snapshot() -> Vec<FileSnap> {
    let root = fixtures_root();
    let mut files = Vec::new();
    collect_source_files(&root, &mut files);
    let thresholds = Thresholds::default();
    let mut snaps = Vec::new();
    for path in &files {
        let Some(lang) = parse::detect_language(path) else {
            continue;
        };
        let Ok(source) = std::fs::read_to_string(path) else {
            continue;
        };
        let Some(metrics) = parse::parse_and_walk_guarded(&source, lang) else {
            continue;
        };
        let findings = smells::detect(&metrics, &source, &thresholds);
        let rel = path.strip_prefix(&root).unwrap_or(path);
        snaps.push(FileSnap {
            path: rel.to_string_lossy().replace('\\', "/"),
            module: mod_snap(&metrics.module),
            functions: metrics.functions.iter().map(fn_snap).collect(),
            findings: finding_snaps(&findings),
        });
    }
    snaps.sort_by(|a, b| a.path.cmp(&b.path));
    snaps
}

#[test]
fn fused_equivalence_matches_legacy_golden() {
    let live = build_snapshot();
    let golden_path = fixtures_root().join("golden_legacy.json");
    if std::env::var("UPDATE_GOLDEN").is_ok() || !golden_path.exists() {
        let json = serde_json::to_string_pretty(&live).expect("serialize golden");
        std::fs::write(&golden_path, json).expect("write golden");
        return;
    }
    let raw = std::fs::read_to_string(&golden_path).expect("read golden");
    let golden: Vec<FileSnap> = serde_json::from_str(&raw).expect("parse golden");
    assert_eq!(
        live.len(),
        golden.len(),
        "golden file-count drift: live {} vs golden {}",
        live.len(),
        golden.len()
    );
    for (l, g) in live.iter().zip(&golden) {
        assert_eq!(l, g, "metric/finding drift in fixture: {}", l.path);
    }
}
