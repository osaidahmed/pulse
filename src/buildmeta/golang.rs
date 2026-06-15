use std::path::Path;

use super::{DeclaredDep, DepScope, Ecosystem, Lockfile, Manifest};

pub(super) fn parse_manifest(path: &Path, source: &str) -> Option<Manifest> {
    let mut deps = Vec::new();
    let mut in_require_block = false;
    for (i, raw) in source.lines().enumerate() {
        let line = raw.split("//").next().unwrap_or(raw).trim();
        let lineno = i as u32 + 1;
        if let Some(own) = line.strip_prefix("module ") {
            let own = own.trim().to_string();
            deps.push(DeclaredDep {
                name: own,
                constraint: String::new(),
                scope: DepScope::Deployed,
                line: lineno,
                own: true,
            });
        } else if let Some(dep) = block_spec(line, "require", &mut in_require_block).and_then(|s| module_dep(s, lineno))
        {
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

fn block_spec<'a>(line: &'a str, keyword: &str, in_block: &mut bool) -> Option<&'a str> {
    if line.starts_with(&format!("{keyword} (")) {
        *in_block = true;
        return None;
    }
    if *in_block {
        if line.starts_with(')') {
            *in_block = false;
            return None;
        }
        return Some(line);
    }
    line.strip_prefix(&format!("{keyword} ")).map(str::trim)
}

fn module_dep(spec: &str, line: u32) -> Option<DeclaredDep> {
    let mut parts = spec.split_whitespace();
    let name = parts.next()?;
    if !name.contains('/') && !name.contains('.') {
        return None;
    }
    let constraint = parts.next().unwrap_or_default().to_string();
    Some(DeclaredDep { name: name.to_string(), constraint, scope: DepScope::Deployed, line, own: false })
}

pub(super) fn parse_workspace(path: &Path, source: &str) -> Option<Manifest> {
    let mut members = Vec::new();
    let mut in_use_block = false;
    for raw in source.lines() {
        let line = raw.split("//").next().unwrap_or(raw).trim();
        let Some(dir) = block_spec(line, "use", &mut in_use_block).map(member_path) else { continue };
        if !dir.as_os_str().is_empty() && dir != Path::new(".") {
            members.push(dir);
        }
    }
    (!members.is_empty()).then(|| Manifest {
        path: path.to_path_buf(),
        ecosystem: Ecosystem::Go,
        deps: Vec::new(),
        workspace_members: members,
    })
}

fn member_path(dir: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(dir.trim_start_matches("./").trim_end_matches('/'))
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
