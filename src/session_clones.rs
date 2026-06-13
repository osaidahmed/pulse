use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::baselines;
use crate::smells::{Finding, Location, Smell};
use crate::thresholds::Thresholds;
use crate::walk::FunctionMetrics;

#[derive(Default, Serialize, Deserialize)]
struct Index {
    by_hash: BTreeMap<u64, Vec<Entry>>,
}

#[derive(Clone, Serialize, Deserialize)]
struct Entry {
    file: String,
    function: String,
}

pub fn check(file_path: &str, functions: &[FunctionMetrics], t: &Thresholds) -> Vec<Finding> {
    let file = canonical(file_path);
    let mut index = load();
    let findings = matches(&index, &file, functions, t);
    refresh(&mut index, &file, functions, t);
    save(&index);
    findings
}

fn eligible(f: &FunctionMetrics, t: &Thresholds) -> bool {
    let d = &t.analysis.duplication;
    f.structural_hash != 0 && f.loc >= d.min_loc && f.distinct_node_kinds >= d.min_distinct_kinds
}

fn matches(index: &Index, file: &str, functions: &[FunctionMetrics], t: &Thresholds) -> Vec<Finding> {
    let mut out = Vec::new();
    for f in functions.iter().filter(|f| eligible(f, t)) {
        let Some(entries) = index.by_hash.get(&f.structural_hash) else { continue };
        let Some(other) = entries.iter().find(|e| e.file != file) else { continue };
        out.push(Finding {
            smell: Smell::CrossFileDuplication,
            location: Location::Function { name: f.name.clone(), start_line: f.start_line, end_line: f.end_line },
            detail: format!("shares an identical structure with `{}` in {}", other.function, basename(&other.file)),
        });
    }
    out
}

fn refresh(index: &mut Index, file: &str, functions: &[FunctionMetrics], t: &Thresholds) {
    for entries in index.by_hash.values_mut() {
        entries.retain(|e| e.file != file);
    }
    index.by_hash.retain(|_, v| !v.is_empty());
    for f in functions.iter().filter(|f| eligible(f, t)) {
        index
            .by_hash
            .entry(f.structural_hash)
            .or_default()
            .push(Entry { file: file.to_string(), function: f.name.clone() });
    }
}

fn index_path() -> PathBuf {
    baselines::baseline_dir().join("session_clones.json")
}

fn load() -> Index {
    std::fs::read_to_string(index_path()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

fn save(index: &Index) {
    let _ = std::fs::create_dir_all(baselines::baseline_dir());
    if let Ok(json) = serde_json::to_string(index) {
        let _ = std::fs::write(index_path(), json);
    }
}

fn canonical(path: &str) -> String {
    std::fs::canonicalize(path).ok().and_then(|p| p.to_str().map(String::from)).unwrap_or_else(|| path.to_string())
}

fn basename(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}
