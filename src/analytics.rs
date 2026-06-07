use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::PathBuf;

use crate::baselines;
use crate::hook::HookInput;
use crate::interaction::tier_for;
use crate::smells::{Finding, Location};
use crate::walk::FunctionMetrics;

pub fn analytics_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("PULSE_ANALYTICS_DIR") {
        return PathBuf::from(dir);
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".local/share/pulse");
    }
    PathBuf::from("/tmp/pulse-analytics")
}

pub fn save_session_id(hook: &HookInput) {
    let path = baselines::baseline_dir().join("session_id");
    if path.exists() {
        return;
    }
    let sid = hook.session_id.as_deref().unwrap_or("unknown");
    let _ = std::fs::create_dir_all(baselines::baseline_dir());
    let _ = std::fs::write(path, sid);
}

pub fn log_findings(hook: &HookInput, findings: &[Finding], filename: &str, functions: &[FunctionMetrics]) {
    let log_path = baselines::baseline_dir().join("findings.jsonl");
    let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(log_path) else {
        return;
    };

    let ts = timestamp_secs();

    for f in findings {
        let (func_name, start_line) = match &f.location {
            Location::Function { name, start_line, .. } => (Some(name.as_str()), Some(*start_line)),
            Location::Module => (None, None),
        };
        let record = serde_json::json!({
            "ts": ts,
            "file": filename,
            "path": hook.file_path,
            "smell": f.smell.as_str(),
            "tier": tier_for(f.smell).as_str(),
            "fn": func_name,
            "line": start_line,
            "hash": structural_hash_of(&f.location, functions),
            "detail": f.detail,
        });
        let _ = writeln!(file, "{record}");
    }
}

fn structural_hash_of(location: &Location, functions: &[FunctionMetrics]) -> Option<u64> {
    match location {
        Location::Function { name, start_line, .. } => {
            functions.iter().find(|fm| &fm.name == name && fm.start_line == *start_line).map(|fm| fm.structural_hash)
        }
        Location::Module => None,
    }
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
            let outcome = outcome_for(entry, &current, &functions, moved_pool);
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

fn write_outcome(out: &mut std::fs::File, entry: &serde_json::Value, outcome: &str, session_id: &str, ts: u64) {
    let record = serde_json::json!({
        "ts": ts,
        "session": session_id,
        "file": entry.get("file").and_then(|v| v.as_str()).unwrap_or(""),
        "smell": entry.get("smell").and_then(|v| v.as_str()).unwrap_or(""),
        "tier": entry.get("tier").and_then(|v| v.as_str()).unwrap_or(""),
        "fn": entry.get("fn").and_then(|v| v.as_str()),
        "detail": entry.get("detail").and_then(|v| v.as_str()).unwrap_or(""),
        "outcome": outcome,
    });
    let _ = writeln!(out, "{record}");
}

fn timestamp_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
}
