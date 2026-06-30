use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Path, PathBuf};

use pulse_smells::{self as smells, Location, Smell};
use pulse_syntax::parse::{self, Language};
use pulse_thresholds::Thresholds;

const MAX_FILE_BYTES: u64 = 2_000_000;
const SKIP_DIRS: &[&str] =
    &[".git", "node_modules", "target", "vendor", ".venv", "build", "dist", "__pycache__", ".tox", "third_party"];

fn cpg_lang(lang: Language) -> bool {
    matches!(
        lang,
        Language::Rust
            | Language::Go
            | Language::Python
            | Language::C
            | Language::Cpp
            | Language::Java
            | Language::CSharp
            | Language::TypeScript
            | Language::JavaScript
            | Language::Php
            | Language::Swift
            | Language::Kotlin
            | Language::Ruby
    )
}

fn cpg_thresholds() -> Thresholds {
    let mut t = Thresholds::default();
    t.cpg.enabled = true;
    t
}

fn smell_key(s: Smell) -> Option<&'static str> {
    match s {
        Smell::DeadStore => Some("dead_store"),
        Smell::UseBeforeDef => Some("use_before_def"),
        Smell::UnreachableCode => Some("unreachable_code"),
        _ => None,
    }
}

fn sorted_subdirs(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> =
        std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
    v.sort();
    v
}

fn collect_source_files(dir: &Path, cap: usize, out: &mut Vec<PathBuf>) {
    if out.len() >= cap {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    paths.sort();
    for p in paths {
        if out.len() >= cap {
            return;
        }
        if p.is_dir() {
            let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            if SKIP_DIRS.contains(&name.as_str()) || name.starts_with('.') {
                continue;
            }
            collect_source_files(&p, cap, out);
        } else if parse::detect_language(&p).is_some_and(cpg_lang) {
            out.push(p);
        }
    }
}

fn readable_source(path: &Path) -> Option<(Language, String)> {
    let lang = parse::detect_language(path)?;
    if std::fs::metadata(path).map_or(true, |m| m.len() > MAX_FILE_BYTES) {
        return None;
    }
    Some((lang, std::fs::read_to_string(path).ok()?))
}

fn cpg_findings(src: &str, lang: Language, t: &Thresholds) -> Vec<(&'static str, u32, String)> {
    let Some(metrics) = parse::parse_and_walk_scoped(src, lang, None, true) else { return Vec::new() };
    smells::detect(&metrics, src, t)
        .into_iter()
        .filter_map(|f| {
            let key = smell_key(f.smell)?;
            let line = match &f.location {
                Location::Function { start_line, .. } => *start_line,
                Location::Module => 0,
            };
            Some((key, line, f.detail))
        })
        .collect()
}

fn lang_dirs(root: &Path, only: Option<&String>) -> Vec<(String, PathBuf)> {
    sorted_subdirs(root)
        .into_iter()
        .map(|d| (d.file_name().unwrap_or_default().to_string_lossy().into_owned(), d))
        .filter(|(name, _)| only.is_none_or(|l| l == name))
        .collect()
}

fn write_samples(samples: &[String]) {
    let out_path = std::env::var("SCAN_CPG_OUT").unwrap_or_else(|_| "/tmp/cpg_scan.tsv".into());
    if let Ok(mut f) = std::fs::File::create(&out_path) {
        let _ = f.write_all(samples.join("\n").as_bytes());
    }
}

#[test]
fn scan_corpus_for_cpg_findings() {
    let Ok(corpus) = std::env::var("SCAN_CPG_CORPUS") else { return };
    if std::env::var("SCAN_CPG_TRANSIENT").is_ok() {
        return;
    }
    let only = std::env::var("SCAN_CPG_LANG").ok();
    let cap: usize = std::env::var("SCAN_CPG_FILECAP").ok().and_then(|s| s.parse().ok()).unwrap_or(4000);
    let t = cpg_thresholds();
    let mut samples: Vec<String> = Vec::new();

    for (lang, lang_dir) in lang_dirs(&PathBuf::from(&corpus), only.as_ref()) {
        let (mut files, mut functions) = (0u64, 0u64);
        let mut by_smell: BTreeMap<&'static str, u64> = BTreeMap::new();
        for repo in sorted_subdirs(&lang_dir) {
            let slug = repo.file_name().unwrap_or_default().to_string_lossy().into_owned();
            let mut paths = Vec::new();
            collect_source_files(&repo, cap, &mut paths);
            for p in &paths {
                let Some((detected, src)) = readable_source(p) else { continue };
                let Some(metrics) = parse::parse_and_walk_scoped(&src, detected, None, true) else { continue };
                files += 1;
                functions += metrics.functions.len() as u64;
                for f in smells::detect(&metrics, &src, &t) {
                    let Some(key) = smell_key(f.smell) else { continue };
                    *by_smell.entry(key).or_default() += 1;
                    let line = match &f.location {
                        Location::Function { start_line, .. } => *start_line,
                        Location::Module => 0,
                    };
                    if samples.len() < 4000 {
                        samples.push(format!("{lang}\t{slug}\t{}\t{line}\t{key}\t{}", p.display(), f.detail));
                    }
                }
            }
        }
        let total: u64 = by_smell.values().sum();
        let per_kfn = if functions > 0 { total as f64 * 1000.0 / functions as f64 } else { 0.0 };
        eprintln!("{lang}: {files} files, {functions} fns, {total} cpg-findings ({per_kfn:.2}/1k fns) {by_smell:?}");
    }
    write_samples(&samples);
}

#[test]
fn scan_corpus_for_transient_cpg_findings() {
    let Ok(corpus) = std::env::var("SCAN_CPG_CORPUS") else { return };
    if std::env::var("SCAN_CPG_TRANSIENT").is_err() {
        return;
    }
    let only = std::env::var("SCAN_CPG_LANG").ok();
    let cap: usize = std::env::var("SCAN_CPG_FILECAP").ok().and_then(|s| s.parse().ok()).unwrap_or(500);
    let step: usize = std::env::var("SCAN_CPG_STEP").ok().and_then(|s| s.parse().ok()).unwrap_or(5);
    let max_lines: usize = std::env::var("SCAN_CPG_MAXLINES").ok().and_then(|s| s.parse().ok()).unwrap_or(400);
    let t = cpg_thresholds();
    let mut samples: Vec<String> = Vec::new();

    for (lang, lang_dir) in lang_dirs(&PathBuf::from(&corpus), only.as_ref()) {
        let mut files = 0u64;
        let mut by_smell: BTreeMap<&'static str, u64> = BTreeMap::new();
        for repo in sorted_subdirs(&lang_dir) {
            let slug = repo.file_name().unwrap_or_default().to_string_lossy().into_owned();
            let mut paths = Vec::new();
            collect_source_files(&repo, cap, &mut paths);
            for p in &paths {
                let Some((detected, src)) = readable_source(p) else { continue };
                let lines: Vec<&str> = src.lines().collect();
                if lines.len() < step || lines.len() > max_lines {
                    continue;
                }
                let final_set: BTreeSet<(&'static str, String)> =
                    cpg_findings(&src, detected, &t).into_iter().map(|(k, _, d)| (k, d)).collect();
                files += 1;
                let mut seen: BTreeSet<(&'static str, String)> = BTreeSet::new();
                let mut cut = step;
                while cut < lines.len() {
                    let partial = lines[..cut].join("\n");
                    for (key, _, detail) in cpg_findings(&partial, detected, &t) {
                        let id = (key, detail.clone());
                        if !final_set.contains(&id) && seen.insert(id) {
                            *by_smell.entry(key).or_default() += 1;
                            if samples.len() < 4000 {
                                samples.push(format!("{lang}\t{slug}\t{}\t@{cut}\t{key}\t{detail}", p.display()));
                            }
                        }
                    }
                    cut += step;
                }
            }
        }
        let total: u64 = by_smell.values().sum();
        let per_file = if files > 0 { total as f64 / files as f64 } else { 0.0 };
        eprintln!("{lang}: {files} files, {total} TRANSIENT cpg-findings ({per_file:.3}/file) {by_smell:?}");
    }
    write_samples(&samples);
}
