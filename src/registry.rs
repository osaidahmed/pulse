use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Deserialize;

use crate::buildmeta::Ecosystem;

#[derive(Debug, Clone, Deserialize)]
pub struct VersionKey {
    pub version: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VersionInfo {
    #[serde(rename = "versionKey")]
    pub version_key: VersionKey,
    #[serde(rename = "publishedAt", default)]
    pub published_at: String,
    #[serde(rename = "isDefault", default)]
    pub is_default: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PackageInfo {
    #[serde(default)]
    pub versions: Vec<VersionInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FreshnessThresholds {
    pub min_missed: u32,
    pub abandon_years: i64,
    pub max_findings: usize,
}

impl FreshnessThresholds {
    pub const DEFAULTS: Self = Self { min_missed: 5, abandon_years: 2, max_findings: 30 };
}

pub fn cache_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("PULSE_REGISTRY_CACHE") {
        return PathBuf::from(dir);
    }
    std::env::var("HOME").map_or_else(
        |_| std::env::temp_dir().join("pulse-registry"),
        |h| PathBuf::from(h).join(".cache/pulse/registry"),
    )
}

pub fn parse_package(json: &str) -> Option<PackageInfo> {
    serde_json::from_str(json).ok()
}

pub fn depsdev_system(eco: Ecosystem) -> Option<&'static str> {
    match eco {
        Ecosystem::Cargo => Some("cargo"),
        Ecosystem::Npm => Some("npm"),
        Ecosystem::Pip => Some("pypi"),
        Ecosystem::Go => Some("go"),
        Ecosystem::NuGet => Some("nuget"),
        Ecosystem::RubyGems | Ecosystem::Swift => None,
    }
}

pub fn lookup(eco: Ecosystem, name: &str, cache_dir: &Path, online: bool) -> Option<PackageInfo> {
    let system = depsdev_system(eco)?;
    let cache_path = cache_dir.join(format!("{system}__{}.json", encode(name)));
    if let Ok(cached) = std::fs::read_to_string(&cache_path) {
        return parse_package(&cached);
    }
    if !online {
        return None;
    }
    let body = fetch(system, name)?;
    let _ = std::fs::create_dir_all(cache_dir);
    let _ = std::fs::write(&cache_path, &body);
    parse_package(&body)
}

fn fetch(system: &str, name: &str) -> Option<String> {
    let url = format!("https://api.deps.dev/v3/systems/{system}/packages/{}", encode(name));
    ureq::get(&url).timeout(Duration::from_secs(10)).call().ok()?.into_string().ok()
}

fn encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
