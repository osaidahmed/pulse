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

#[derive(Debug, Clone, Deserialize)]
pub struct AdvisoryKey {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VersionDetail {
    #[serde(rename = "advisoryKeys", default)]
    pub advisory_keys: Vec<AdvisoryKey>,
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
        Ecosystem::RubyGems | Ecosystem::Swift | Ecosystem::Composer | Ecosystem::Maven => None,
    }
}

pub fn lookup(eco: Ecosystem, name: &str, cache_dir: &Path, online: bool) -> Option<PackageInfo> {
    let system = depsdev_system(eco)?;
    let path = cache_dir.join(format!("{system}__{}.json", encode(name)));
    let url = format!("https://api.deps.dev/v3/systems/{system}/packages/{}", encode(name));
    parse_package(&cached(&path, online, &url)?)
}

pub fn lookup_version(
    eco: Ecosystem,
    name: &str,
    version: &str,
    cache_dir: &Path,
    online: bool,
) -> Option<VersionDetail> {
    let system = depsdev_system(eco)?;
    let path = cache_dir.join(format!("{system}__{}__{}.json", encode(name), encode(version)));
    let url =
        format!("https://api.deps.dev/v3/systems/{system}/packages/{}/versions/{}", encode(name), encode(version));
    parse_version_detail(&cached(&path, online, &url)?)
}

pub fn parse_version_detail(json: &str) -> Option<VersionDetail> {
    serde_json::from_str(json).ok()
}

fn cached(cache_path: &Path, online: bool, url: &str) -> Option<String> {
    if let Ok(body) = std::fs::read_to_string(cache_path) {
        return Some(body);
    }
    if !online {
        return None;
    }
    let body = fetch_url(url)?;
    write_atomic(cache_path, &body);
    Some(body)
}

pub fn write_atomic(cache_path: &Path, body: &str) {
    let Some(parent) = cache_path.parent() else { return };
    if std::fs::create_dir_all(parent).is_err() {
        return;
    }
    let seq = TMP_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = cache_path.with_file_name(format!(".pulse-tmp.{}.{seq}", std::process::id()));
    if std::fs::write(&tmp, body).is_ok() {
        let _ = std::fs::rename(&tmp, cache_path);
    } else {
        let _ = std::fs::remove_file(&tmp);
    }
}

static TMP_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn fetch_url(url: &str) -> Option<String> {
    ureq::get(url).timeout(Duration::from_secs(10)).call().ok()?.into_string().ok()
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
