use std::collections::{HashMap, HashSet};
use std::io::Write;

use crate::baselines;
use crate::hook::HookInput;
use crate::interaction::tier_for;
use crate::smells::{Finding, Location, Smell};
use crate::walk::FunctionMetrics;

const PRESERVED_THRESHOLD: f64 = 0.8;

pub use pulse_core::analytics_dir;

pub fn save_session_id(hook: &HookInput) {
    let path = baselines::baseline_dir().join("session_id");
    if path.exists() {
        return;
    }
    let sid = hook.session_id.as_deref().unwrap_or("unknown");
    let _ = std::fs::create_dir_all(baselines::baseline_dir());
    let _ = std::fs::write(path, sid);
}

pub fn log_findings(
    hook: &HookInput,
    findings: &[Finding],
    filename: &str,
    functions: &[FunctionMetrics],
    source: &str,
) {
    let log_path = baselines::baseline_dir().join("findings.jsonl");
    let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(log_path) else {
        return;
    };

    let ts = timestamp_secs();
    let lang =
        crate::parse::detect_language(std::path::Path::new(&hook.file_path)).map(crate::parse::Language::to_config_key);

    for f in findings {
        let meta = function_meta(&f.location, functions);
        if let Some(fm) = meta {
            maybe_store_body(f.smell, fm, source, body_key(&hook.file_path, &fm.name, fm.start_line));
        }
        let (func_name, start_line) = match &f.location {
            Location::Function { name, start_line, .. } => (Some(name.as_str()), Some(*start_line)),
            Location::Module => (None, None),
        };
        let record = serde_json::json!({
            "ts": ts,
            "file": filename,
            "path": hook.file_path,
            "lang": lang,
            "smell": f.smell.as_str(),
            "tier": tier_for(f.smell).as_str(),
            "fn": func_name,
            "line": start_line,
            "hash": meta.map(|fm| fm.structural_hash),
            "detail": f.detail,
            "rarity": serde_json::Value::Null,
        });
        let _ = writeln!(file, "{record}");
    }
}

fn function_meta<'a>(location: &Location, functions: &'a [FunctionMetrics]) -> Option<&'a FunctionMetrics> {
    let Location::Function { name, start_line, .. } = location else {
        return None;
    };
    functions.iter().find(|fm| &fm.name == name && fm.start_line == *start_line)
}

fn maybe_store_body(smell: Smell, fm: &FunctionMetrics, source: &str, key: u64) {
    if !matches!(smell, Smell::GodMethod | Smell::ComplexMethod | Smell::LargeMethod | Smell::DeepNestedComplexity) {
        return;
    }
    let lines: Vec<&str> = source.lines().collect();
    let start = (fm.start_line as usize).saturating_sub(1);
    let end = (fm.end_line as usize).min(lines.len());
    if start < end {
        crate::refmine::store_body(key, &lines[start..end].join("\n"));
    }
}

fn body_key(path: &str, name: &str, start_line: u32) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    (path, name, start_line).hash(&mut hasher);
    hasher.finish()
}

pub fn resolve(
    moved_pool: &HashMap<u64, HashSet<String>>,
    analyze_fn: impl Fn(&str) -> Option<(Vec<Finding>, Vec<String>)>,
) {
    let log_path = baselines::baseline_dir().join("findings.jsonl");
    let Ok(log_content) = std::fs::read_to_string(&log_path) else {
        return;
    };

    let session_id = std::fs::read_to_string(baselines::baseline_dir().join("session_id"))
        .unwrap_or_else(|_| "unknown".into())
        .trim()
        .to_string();

    let dir = analytics_dir();
    let _ = std::fs::create_dir_all(&dir);
    let Ok(mut out) = std::fs::OpenOptions::new().create(true).append(true).open(dir.join("analytics.jsonl")) else {
        return;
    };

    let entries = parse_and_dedup(&log_content);
    let by_file = group_by_file(&entries);
    let ts = timestamp_secs();

    for (file_path, file_entries) in &by_file {
        let (current, functions) = analyze_fn(file_path).unwrap_or_default();
        for entry in file_entries {
            let base = outcome_for(entry, &current, &functions, moved_pool);
            let outcome = refine_outcome(base, entry, file_path);
            write_outcome(&mut out, entry, outcome, &session_id, ts);
        }
    }
}

fn parse_and_dedup(log_content: &str) -> Vec<serde_json::Value> {
    let mut seen = std::collections::HashSet::new();
    let mut entries = Vec::new();
    for line in log_content.lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let key = format!(
            "{}:{}:{}",
            v.get("path").and_then(|x| x.as_str()).unwrap_or(""),
            v.get("fn").and_then(|x| x.as_str()).unwrap_or("_module_"),
            v.get("smell").and_then(|x| x.as_str()).unwrap_or(""),
        );
        if seen.insert(key) {
            entries.push(v);
        }
    }
    entries
}

fn group_by_file(entries: &[serde_json::Value]) -> HashMap<String, Vec<&serde_json::Value>> {
    let mut by_file: HashMap<String, Vec<&serde_json::Value>> = HashMap::new();
    for entry in entries {
        if let Some(fp) = entry.get("path").and_then(|v| v.as_str()) {
            by_file.entry(fp.to_string()).or_default().push(entry);
        }
    }
    by_file
}

fn outcome_for(
    entry: &serde_json::Value,
    current_findings: &[Finding],
    functions: &[String],
    moved_pool: &HashMap<u64, HashSet<String>>,
) -> &'static str {
    if smell_still_present(entry, current_findings) {
        return "ignored";
    }
    let Some(name) = entry.get("fn").and_then(|v| v.as_str()) else {
        return "addressed";
    };
    if functions.iter().any(|f| f == name) {
        return "addressed";
    }
    if moved_to_other_file(entry, moved_pool) {
        return "moved";
    }
    "removed"
}

fn moved_to_other_file(entry: &serde_json::Value, moved_pool: &HashMap<u64, HashSet<String>>) -> bool {
    let Some(hash) = entry.get("hash").and_then(serde_json::Value::as_u64) else {
        return false;
    };
    let path = canonical(entry.get("path").and_then(|v| v.as_str()).unwrap_or(""));
    moved_pool.get(&hash).is_some_and(|files| files.iter().any(|f| f != &path))
}

fn canonical(path: &str) -> String {
    std::fs::canonicalize(path).ok().and_then(|p| p.to_str().map(String::from)).unwrap_or_else(|| path.to_string())
}

fn smell_still_present(entry: &serde_json::Value, current_findings: &[Finding]) -> bool {
    let smell = entry.get("smell").and_then(|v| v.as_str()).unwrap_or("");
    let func = entry.get("fn").and_then(|v| v.as_str());
    current_findings.iter().any(|cf| {
        cf.smell.as_str() == smell
            && match (&cf.location, func) {
                (Location::Function { name, .. }, Some(fn_name)) => name == fn_name,
                (Location::Module, None) => true,
                _ => false,
            }
    })
}

fn refine_outcome(base: &'static str, entry: &serde_json::Value, file_path: &str) -> &'static str {
    if base != "addressed" && base != "removed" {
        return base;
    }
    match body_preservation(entry, file_path) {
        Some(ratio) if ratio >= PRESERVED_THRESHOLD => "redistributed",
        _ => base,
    }
}

fn body_preservation(entry: &serde_json::Value, file_path: &str) -> Option<f64> {
    let name = entry.get("fn").and_then(|v| v.as_str())?;
    let start = entry.get("line").and_then(serde_json::Value::as_u64)? as u32;
    let pre_body = crate::refmine::load_body(body_key(file_path, name, start))?;
    let cur = std::fs::read_to_string(file_path).ok()?;
    let lang = crate::parse::detect_language(std::path::Path::new(file_path))?;
    crate::refmine::body_match_ratio(&pre_body, &cur, lang)
}

fn write_outcome(out: &mut std::fs::File, entry: &serde_json::Value, outcome: &str, session_id: &str, ts: u64) {
    let record = serde_json::json!({
        "ts": ts,
        "session": session_id,
        "file": entry.get("file").and_then(|v| v.as_str()).unwrap_or(""),
        "lang": entry.get("lang").and_then(|v| v.as_str()),
        "smell": entry.get("smell").and_then(|v| v.as_str()).unwrap_or(""),
        "tier": entry.get("tier").and_then(|v| v.as_str()).unwrap_or(""),
        "fn": entry.get("fn").and_then(|v| v.as_str()),
        "detail": entry.get("detail").and_then(|v| v.as_str()).unwrap_or(""),
        "rarity": entry.get("rarity").cloned().unwrap_or(serde_json::Value::Null),
        "outcome": outcome,
    });
    let _ = writeln!(out, "{record}");
}

fn timestamp_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
}
