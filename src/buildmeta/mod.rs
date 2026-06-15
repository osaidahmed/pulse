use std::path::{Path, PathBuf};

mod cargo;
mod csproj;
pub mod declared;
mod gemfile;
mod golang;
mod npm;
mod python;
pub mod stdlib;
mod swiftpm;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ecosystem {
    Cargo,
    Npm,
    Pip,
    Go,
    RubyGems,
    NuGet,
    Swift,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepScope {
    Deployed,
    Dev,
    Build,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeclaredDep {
    pub name: String,
    pub constraint: String,
    pub scope: DepScope,
    pub line: u32,
    pub own: bool,
}

#[derive(Debug, Clone)]
pub struct Manifest {
    pub path: PathBuf,
    pub ecosystem: Ecosystem,
    pub deps: Vec<DeclaredDep>,
    pub workspace_members: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct Lockfile {
    pub path: PathBuf,
    pub ecosystem: Ecosystem,
    pub resolved: Vec<(String, String)>,
}

#[derive(Debug, Default)]
pub struct BuildMeta {
    pub manifests: Vec<Manifest>,
    pub lockfiles: Vec<Lockfile>,
}

impl BuildMeta {
    pub fn declared_names(&self) -> std::collections::HashSet<&str> {
        let from_manifests = self.manifests.iter().flat_map(|m| m.deps.iter().map(|d| d.name.as_str()));
        let from_locks = self.lockfiles.iter().flat_map(|l| l.resolved.iter().map(|(n, _)| n.as_str()));
        from_manifests.chain(from_locks).collect()
    }
}

type ManifestParser = fn(&Path, &str) -> Option<Manifest>;
type LockfileParser = fn(&Path, &str) -> Option<Lockfile>;

const MANIFEST_FILES: &[(&str, ManifestParser)] = &[
    ("Cargo.toml", cargo::parse_manifest),
    ("package.json", npm::parse_manifest),
    ("pyproject.toml", python::parse_pyproject),
    ("requirements.txt", python::parse_requirements),
    ("go.mod", golang::parse_manifest),
    ("Gemfile", gemfile::parse_manifest),
    ("Package.swift", swiftpm::parse_manifest),
];

const LOCKFILE_FILES: &[(&str, LockfileParser)] = &[
    ("Cargo.lock", cargo::parse_lockfile),
    ("package-lock.json", npm::parse_lockfile),
    ("go.sum", golang::parse_lockfile),
    ("Gemfile.lock", gemfile::parse_lockfile),
    ("Package.resolved", swiftpm::parse_lockfile),
];

pub fn discover(root: &Path) -> BuildMeta {
    let mut meta = BuildMeta::default();
    collect_dir(root, &mut meta);
    let members: Vec<PathBuf> = meta.manifests.iter().flat_map(|m| m.workspace_members.clone()).collect();
    for member in members {
        let dir = root.join(member);
        if dir.is_dir() {
            collect_dir(&dir, &mut meta);
        }
    }
    meta
}

fn collect_dir(dir: &Path, meta: &mut BuildMeta) {
    for (name, parser) in MANIFEST_FILES {
        let path = dir.join(name);
        if let Some(m) = read_capped(&path).and_then(|s| parser(&path, &s)) {
            meta.manifests.push(m);
        }
    }
    for (name, parser) in LOCKFILE_FILES {
        let path = dir.join(name);
        if let Some(l) = read_capped(&path).and_then(|s| parser(&path, &s)) {
            meta.lockfiles.push(l);
        }
    }
}

const CSPROJ_SKIP_DIRS: &[&str] =
    &["bin", "obj", "node_modules", "target", "packages", "vendor", "third_party", "dist", "build", "out", "Pods"];

pub fn csproj_manifests(root: &Path) -> Vec<Manifest> {
    let mut out = Vec::new();
    collect_csproj(root, 0, &mut out);
    out
}

fn collect_csproj(dir: &Path, depth: usize, out: &mut Vec<Manifest>) {
    if depth > 12 || out.len() >= 2000 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        handle_csproj_entry(&entry.path(), depth, out);
    }
}

fn handle_csproj_entry(path: &Path, depth: usize, out: &mut Vec<Manifest>) {
    if path.is_dir() {
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if !CSPROJ_SKIP_DIRS.contains(&name.as_str()) && !name.starts_with('.') {
            collect_csproj(path, depth + 1, out);
        }
    } else if path.extension().is_some_and(|e| e == "csproj") {
        if let Some(m) = read_capped(path).and_then(|s| csproj::parse_manifest(path, &s)) {
            out.push(m);
        }
    }
}

const MAX_MANIFEST_BYTES: u64 = 8 * 1024 * 1024;

fn read_capped(path: &Path) -> Option<String> {
    let size = std::fs::metadata(path).ok()?.len();
    if size > MAX_MANIFEST_BYTES {
        return None;
    }
    std::fs::read_to_string(path).ok()
}

pub(crate) fn line_of(source: &str, needle: &str) -> u32 {
    for (i, line) in source.lines().enumerate() {
        let mut rest = line;
        while let Some(pos) = rest.find(needle) {
            let before_ok = pos == 0 || !is_ident_byte(rest.as_bytes()[pos - 1]);
            let after = pos + needle.len();
            let after_ok = after >= rest.len() || !is_ident_byte(rest.as_bytes()[after]);
            if before_ok && after_ok {
                return i as u32 + 1;
            }
            rest = &rest[after..];
        }
    }
    0
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-'
}
