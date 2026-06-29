use std::path::{Path, PathBuf};

pub fn declared_test_roots(root: &Path) -> Vec<PathBuf> {
    let mut names: Vec<String> = Vec::new();
    if let Ok(src) = std::fs::read_to_string(root.join("pyproject.toml")) {
        names.extend(pyproject_testpaths(&src));
    }
    for ini in ["pytest.ini", "tox.ini", "setup.cfg"] {
        if let Ok(src) = std::fs::read_to_string(root.join(ini)) {
            names.extend(ini_testpaths(&src));
        }
    }
    let mut out: Vec<PathBuf> = names.into_iter().map(|n| root.join(n)).filter(|p| p.is_dir()).collect();
    out.sort();
    out.dedup();
    out
}

pub fn is_under_test_root(path: &Path, roots: &[PathBuf]) -> bool {
    roots.iter().any(|r| path.starts_with(r))
}

fn pyproject_testpaths(source: &str) -> Vec<String> {
    let Ok(doc) = toml::from_str::<toml::Value>(source) else { return Vec::new() };
    let tp = doc
        .get("tool")
        .and_then(|t| t.get("pytest"))
        .and_then(|p| p.get("ini_options"))
        .and_then(|i| i.get("testpaths"));
    match tp {
        Some(toml::Value::Array(a)) => a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect(),
        Some(toml::Value::String(s)) => s.split_whitespace().map(str::to_string).collect(),
        _ => Vec::new(),
    }
}

fn ini_testpaths(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut collecting = false;
    for line in source.lines() {
        if collecting && collect_continuation(line, &mut out) {
            continue;
        }
        collecting = false;
        if let Some(values) = testpaths_value(line.trim()) {
            collecting = values.is_empty();
            out.extend(values.split_whitespace().map(str::to_string));
        }
    }
    out
}

fn collect_continuation(line: &str, out: &mut Vec<String>) -> bool {
    if line.starts_with(char::is_whitespace) && !line.trim().is_empty() {
        out.extend(line.split_whitespace().map(str::to_string));
        return true;
    }
    false
}

fn testpaths_value(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("testpaths")?.trim_start();
    rest.strip_prefix('=').or_else(|| rest.strip_prefix(':')).map(str::trim)
}
