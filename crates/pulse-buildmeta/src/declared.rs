use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

use super::{discover, Ecosystem};
use pulse_core::{baseline_dir, file_key};

const ECOSYSTEM_MANIFESTS: &[(Ecosystem, &[&str])] = &[
    (Ecosystem::Cargo, &["Cargo.toml"]),
    (Ecosystem::Npm, &["package.json"]),
    (Ecosystem::Pip, &["pyproject.toml", "requirements.txt"]),
    (Ecosystem::Go, &["go.mod"]),
    (Ecosystem::RubyGems, &["Gemfile"]),
];

pub fn manifest_names(eco: Ecosystem) -> &'static [&'static str] {
    ECOSYSTEM_MANIFESTS.iter().find(|(e, _)| *e == eco).map_or(&[], |(_, names)| names)
}

pub fn nearest_manifest_root(file_path: &Path, eco: Ecosystem) -> Option<PathBuf> {
    let names = manifest_names(eco);
    let mut dir = file_path.parent()?;
    for _ in 0..12 {
        if names.iter().any(|n| dir.join(n).is_file()) {
            return Some(dir.to_path_buf());
        }
        dir = dir.parent()?;
    }
    None
}

#[derive(Serialize, Deserialize)]
struct CachedDeclared {
    root_stamp: u64,
    names: Vec<String>,
}

pub fn declared_names_for(root: &Path) -> HashSet<String> {
    let stamp = dir_stamp(root);
    let cache_path = baseline_dir().join(format!("{}.buildmeta", file_key(&root.to_string_lossy())));
    if let Some(cached) = load_cache(&cache_path, stamp) {
        return cached;
    }
    let meta = discover(root);
    let names: Vec<String> = meta.declared_names().iter().map(|n| (*n).to_string()).collect();
    let record = CachedDeclared { root_stamp: stamp, names: names.clone() };
    if let Ok(json) = serde_json::to_string(&record) {
        let _ = std::fs::create_dir_all(baseline_dir());
        let _ = std::fs::write(&cache_path, json);
    }
    names.into_iter().collect()
}

fn load_cache(cache_path: &Path, stamp: u64) -> Option<HashSet<String>> {
    let raw = std::fs::read_to_string(cache_path).ok()?;
    let cached: CachedDeclared = serde_json::from_str(&raw).ok()?;
    (cached.root_stamp == stamp).then(|| cached.names.into_iter().collect())
}

fn dir_stamp(root: &Path) -> u64 {
    let mut stamp: u64 = 0;
    for names in ECOSYSTEM_MANIFESTS.iter().map(|(_, n)| n) {
        for name in names.iter().chain(["Cargo.lock", "package-lock.json", "go.sum", "Gemfile.lock"].iter()) {
            if let Ok(m) = std::fs::metadata(root.join(name)) {
                let secs = m.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs());
                stamp = stamp.wrapping_mul(31).wrapping_add(secs).wrapping_add(m.len());
            }
        }
    }
    stamp
}
