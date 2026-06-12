use std::path::Path;

use super::{DeclaredDep, DepScope, Ecosystem, Lockfile, Manifest};

pub(super) fn parse_manifest(path: &Path, source: &str) -> Option<Manifest> {
    let mut deps = Vec::new();
    let mut in_require_block = false;
    for (i, raw) in source.lines().enumerate() {
        let line = raw.split("//").next().unwrap_or(raw).trim();
        if line.starts_with("require (") {
            in_require_block = true;
            continue;
        }
        if in_require_block && line.starts_with(')') {
            in_require_block = false;
            continue;
        }
        let spec = if in_require_block {
            line
        } else if let Some(rest) = line.strip_prefix("require ") {
            rest.trim()
        } else {
            continue;
        };
        if let Some(dep) = module_dep(spec, i as u32 + 1) {
            deps.push(dep);
        }
    }
    (!deps.is_empty()).then(|| Manifest {
        path: path.to_path_buf(),
        ecosystem: Ecosystem::Go,
        deps,
        workspace_members: Vec::new(),
    })
}

fn module_dep(spec: &str, line: u32) -> Option<DeclaredDep> {
    let mut parts = spec.split_whitespace();
    let name = parts.next()?;
    if !name.contains('/') && !name.contains('.') {
        return None;
    }
    let constraint = parts.next().unwrap_or_default().to_string();
    Some(DeclaredDep { name: name.to_string(), constraint, scope: DepScope::Deployed, line })
}

pub(super) fn parse_lockfile(path: &Path, source: &str) -> Option<Lockfile> {
    let mut resolved = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for line in source.lines() {
        let mut parts = line.split_whitespace();
        let (Some(name), Some(version)) = (parts.next(), parts.next()) else { continue };
        let version = version.trim_end_matches("/go.mod");
        if seen.insert(name.to_string()) {
            resolved.push((name.to_string(), version.to_string()));
        }
    }
    (!resolved.is_empty()).then(|| Lockfile { path: path.to_path_buf(), ecosystem: Ecosystem::Go, resolved })
}
