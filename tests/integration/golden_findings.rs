use pulse::analyze::{analyze_source, ScanOptions};
use pulse::config::PulseConfig;
use pulse::parse;
use pulse::smells::Location;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const LANG_DIRS: &[&str] = &[
    "python",
    "typescript",
    "javascript",
    "rust",
    "c",
    "cpp",
    "java",
    "csharp",
    "go",
    "swift",
    "zig",
    "ruby",
    "objc",
    "tcl",
    "kotlin",
    "haskell",
    "lua",
    "r",
    "php",
    "cobol",
    "d",
    "groovy",
    "parity",
];

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct GoldenFinding {
    smell: String,
    location: String,
    detail: String,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct GoldenFile {
    path: String,
    findings: Vec<GoldenFinding>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct GoldenSection {
    cpg_enabled: bool,
    files: Vec<GoldenFile>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct GoldenCorpus {
    sections: Vec<GoldenSection>,
}

fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures")
}

fn corpus_files() -> Vec<PathBuf> {
    let root = fixtures_root();
    let mut files: Vec<PathBuf> = LANG_DIRS
        .iter()
        .flat_map(|d| {
            std::fs::read_dir(root.join(d))
                .into_iter()
                .flatten()
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.is_file() && parse::detect_language(p).is_some())
        })
        .collect();
    files.sort();
    files
}

fn render_location(loc: &Location) -> String {
    match loc {
        Location::Function { name, start_line, end_line } => format!("{name} L{start_line}-{end_line}"),
        Location::Module => "module".to_string(),
    }
}

fn snapshot_section(cpg_enabled: bool) -> GoldenSection {
    let cfg: Option<PulseConfig> =
        cpg_enabled.then(|| toml::from_str("[thresholds.cpg]\nenabled = true\n").expect("cpg config"));
    let root = fixtures_root();
    let mut files = Vec::new();
    for path in corpus_files() {
        let Some(lang) = parse::detect_language(&path) else { continue };
        let Ok(source) = std::fs::read_to_string(&path) else { continue };
        let rel = path.strip_prefix(&root).unwrap_or(&path).to_string_lossy().replace('\\', "/");
        let Some(result) = analyze_source(&rel, &source, lang, cfg.as_ref(), ScanOptions::check()) else {
            continue;
        };
        if result.findings.is_empty() {
            continue;
        }
        let findings = result
            .findings
            .iter()
            .map(|f| GoldenFinding {
                smell: f.smell.snake_name().to_string(),
                location: render_location(&f.location),
                detail: f.detail.clone(),
            })
            .collect();
        files.push(GoldenFile { path: rel, findings });
    }
    GoldenSection { cpg_enabled, files }
}

#[test]
fn golden_corpus_findings_are_stable() {
    let live = GoldenCorpus { sections: vec![snapshot_section(false), snapshot_section(true)] };
    let golden_path = fixtures_root().join("golden_findings.json");
    if std::env::var("UPDATE_GOLDEN").is_ok() || !golden_path.exists() {
        let json = serde_json::to_string_pretty(&live).expect("serialize golden");
        std::fs::write(&golden_path, json).expect("write golden");
        return;
    }
    let raw = std::fs::read_to_string(&golden_path).expect("read golden");
    let golden: GoldenCorpus = serde_json::from_str(&raw).expect("parse golden");
    for (live_s, gold_s) in live.sections.iter().zip(&golden.sections) {
        assert_eq!(live_s.cpg_enabled, gold_s.cpg_enabled, "section order drifted");
        let live_paths: Vec<&String> = live_s.files.iter().map(|f| &f.path).collect();
        let gold_paths: Vec<&String> = gold_s.files.iter().map(|f| &f.path).collect();
        assert_eq!(
            live_paths, gold_paths,
            "flagged-file set drifted (cpg={}); regenerate deliberately with UPDATE_GOLDEN=1",
            live_s.cpg_enabled
        );
        for (lf, gf) in live_s.files.iter().zip(&gold_s.files) {
            assert_eq!(
                lf.findings, gf.findings,
                "findings drifted for {} (cpg={}); regenerate deliberately with UPDATE_GOLDEN=1",
                lf.path, live_s.cpg_enabled
            );
        }
    }
    assert_eq!(live.sections.len(), golden.sections.len());
}
