use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::audit::walk_typed_source_files;
use crate::baselines;
use crate::parse::{self, Language};
use crate::smells::{Finding, Location, Smell};
use crate::test_detection;
use crate::turn_scan;
use crate::walk::FunctionMetrics;

const MAX_SCAN_FILES: usize = 5000;

struct Candidate {
    filename: String,
    bare: String,
    start_line: u32,
    end_line: u32,
}

pub fn unused_added_functions() -> Vec<(String, Vec<Finding>)> {
    let Some((repo, base_sha)) = turn_scan::repo_and_base() else { return Vec::new() };
    let candidates = collect_candidates(&repo, &base_sha);
    if candidates.is_empty() {
        return Vec::new();
    }
    let names: HashSet<&str> = candidates.iter().map(|c| c.bare.as_str()).collect();
    let Some(counts) = reference_counts(&repo, &names) else { return Vec::new() };
    group_findings(candidates, &counts)
}

fn collect_candidates(repo: &Path, base_sha: &str) -> Vec<Candidate> {
    let mut out = Vec::new();
    for rel in turn_scan::changed_files(repo, base_sha) {
        let abs = repo.join(&rel).to_string_lossy().into_owned();
        if test_detection::is_test_file(&abs) || baselines::is_fixture_file(&abs) {
            continue;
        }
        let path = Path::new(&abs);
        let Some(lang) = parse::detect_language(path) else { continue };
        let Ok(source) = std::fs::read_to_string(path) else { continue };
        let Some(current) = parse::parse_and_walk_guarded(&source, lang) else { continue };
        let base = base_function_names(repo, base_sha, &rel, lang);
        let filename = path.file_name().map_or_else(|| rel.clone(), |n| n.to_string_lossy().into_owned());
        for f in &current.functions {
            if let Some(bare) = candidate_name(f, &base, &source) {
                out.push(Candidate {
                    filename: filename.clone(),
                    bare,
                    start_line: f.start_line,
                    end_line: f.end_line,
                });
            }
        }
    }
    out
}

fn base_function_names(repo: &Path, base_sha: &str, rel: &str, lang: Language) -> HashSet<String> {
    turn_scan::base_blob(repo, base_sha, rel)
        .and_then(|src| parse::parse_and_walk_guarded(&src, lang))
        .map(|m| m.functions.iter().map(|f| f.name.clone()).collect())
        .unwrap_or_default()
}

fn candidate_name(f: &FunctionMetrics, base_names: &HashSet<String>, source: &str) -> Option<String> {
    if base_names.contains(&f.name) || f.is_constructor {
        return None;
    }
    let ident = f.name.rsplit(['.', ':']).next().unwrap_or(&f.name);
    if !is_plain_identifier(ident) || is_entry_point(ident) || is_decorated(f.start_line, source) {
        return None;
    }
    Some(ident.to_string())
}

fn is_plain_identifier(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn is_entry_point(bare: &str) -> bool {
    matches!(bare, "main" | "init") || (bare.starts_with("__") && bare.ends_with("__"))
}

fn is_decorated(start_line: u32, source: &str) -> bool {
    let lines: Vec<&str> = source.lines().collect();
    let mut i = (start_line as usize).saturating_sub(1);
    while i > 0 {
        i -= 1;
        let trimmed = lines.get(i).map_or("", |l| l.trim());
        if trimmed.is_empty() {
            continue;
        }
        return trimmed.starts_with('@') || trimmed.starts_with("#[");
    }
    false
}

fn reference_counts(repo: &Path, names: &HashSet<&str>) -> Option<HashMap<String, u32>> {
    let files = walk_typed_source_files(repo, true);
    if files.len() > MAX_SCAN_FILES {
        return None;
    }
    let mut counts: HashMap<String, u32> = names.iter().map(|n| ((*n).to_string(), 0)).collect();
    for (path, _) in &files {
        let Ok(text) = std::fs::read_to_string(path) else { continue };
        for name in names {
            let entry = counts.entry((*name).to_string()).or_insert(0);
            if *entry < 2 {
                *entry = (*entry + count_word_occurrences(&text, name, 2)).min(2);
            }
        }
    }
    Some(counts)
}

fn count_word_occurrences(text: &str, name: &str, cap: u32) -> u32 {
    let bytes = text.as_bytes();
    let mut count = 0u32;
    let mut offset = 0;
    while let Some(rel) = text[offset..].find(name) {
        let pos = offset + rel;
        let end = pos + name.len();
        let before = pos == 0 || !is_ident_byte(bytes[pos - 1]);
        let after = end >= bytes.len() || !is_ident_byte(bytes[end]);
        if before && after {
            count += 1;
            if count >= cap {
                break;
            }
        }
        offset = end;
    }
    count
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn group_findings(candidates: Vec<Candidate>, counts: &HashMap<String, u32>) -> Vec<(String, Vec<Finding>)> {
    let mut by_file: HashMap<String, Vec<Finding>> = HashMap::new();
    for c in candidates {
        if counts.get(&c.bare).copied().unwrap_or(0) == 1 {
            by_file.entry(c.filename.clone()).or_default().push(Finding {
                smell: Smell::UnusedFunction,
                location: Location::Function { name: c.bare.clone(), start_line: c.start_line, end_line: c.end_line },
                detail: format!("`{}` was added this session but is referenced nowhere in the project", c.bare),
            });
        }
    }
    let mut out: Vec<(String, Vec<Finding>)> = by_file.into_iter().collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    for (_, findings) in &mut out {
        findings.sort_by_key(finding_line);
    }
    out
}

fn finding_line(f: &Finding) -> u32 {
    match &f.location {
        Location::Function { start_line, .. } => *start_line,
        Location::Module => 0,
    }
}
