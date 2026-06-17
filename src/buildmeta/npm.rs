use std::path::Path;

use super::{line_of, DeclaredDep, DepScope, Ecosystem, Lockfile, Manifest};

pub(super) fn parse_manifest(path: &Path, source: &str) -> Option<Manifest> {
    let doc: serde_json::Value = serde_json::from_str(source).ok()?;
    let mut deps = Vec::new();
    for (key, scope) in [
        ("dependencies", DepScope::Deployed),
        ("peerDependencies", DepScope::Deployed),
        ("optionalDependencies", DepScope::Deployed),
        ("devDependencies", DepScope::Dev),
    ] {
        let Some(table) = doc.get(key).and_then(|t| t.as_object()) else { continue };
        for (name, constraint) in table {
            deps.push(DeclaredDep {
                name: name.clone(),
                constraint: constraint.as_str().unwrap_or_default().to_string(),
                scope,
                line: line_of(source, name),
                own: false,
            });
        }
    }
    if let Some(own) = doc.get("name").and_then(|n| n.as_str()) {
        deps.push(DeclaredDep {
            name: own.to_string(),
            constraint: String::new(),
            scope: DepScope::Deployed,
            line: 0,
            own: true,
        });
    }
    let base = path.parent().unwrap_or_else(|| std::path::Path::new("."));
    let mut workspace_members = doc
        .get("workspaces")
        .and_then(|w| w.as_array().or_else(|| w.get("packages").and_then(|p| p.as_array())))
        .map(|arr| super::expand_members(arr.iter().filter_map(|m| m.as_str()), base, "package.json"))
        .unwrap_or_default();
    if workspace_members.is_empty() {
        workspace_members = pnpm_members(base);
    }
    Some(Manifest { path: path.to_path_buf(), ecosystem: Ecosystem::Npm, deps, workspace_members })
}

fn pnpm_members(base: &Path) -> Vec<std::path::PathBuf> {
    let Ok(source) = std::fs::read_to_string(base.join("pnpm-workspace.yaml")) else { return Vec::new() };
    let globs = pnpm_package_globs(&source);
    super::expand_members(globs.iter().map(String::as_str), base, "package.json")
}

fn pnpm_package_globs(source: &str) -> Vec<String> {
    let mut globs = Vec::new();
    let mut in_packages = false;
    for line in source.lines() {
        if line.trim_start().starts_with("packages:") {
            in_packages = true;
        } else if in_packages {
            if let Some(glob) = package_list_glob(line) {
                globs.push(glob);
            } else if ends_packages_block(line) {
                break;
            }
        }
    }
    globs
}

fn package_list_glob(line: &str) -> Option<String> {
    let item = line.trim().strip_prefix("- ")?.trim();
    let value = match item.strip_prefix(['\'', '"']) {
        Some(rest) => rest.split(item.as_bytes()[0] as char).next().unwrap_or(rest),
        None => item.split(" #").next().unwrap_or(item),
    };
    let glob = value.trim();
    (!glob.is_empty() && !glob.starts_with('!')).then(|| glob.to_string())
}

fn ends_packages_block(line: &str) -> bool {
    !line.trim().is_empty() && !line.starts_with(char::is_whitespace)
}

pub(super) fn parse_lockfile(path: &Path, source: &str) -> Option<Lockfile> {
    let doc: serde_json::Value = serde_json::from_str(source).ok()?;
    let mut resolved = Vec::new();
    if let Some(packages) = doc.get("packages").and_then(|p| p.as_object()) {
        for (key, pkg) in packages {
            let Some(name) = key.rsplit("node_modules/").next().filter(|n| !n.is_empty() && *n != *key) else {
                continue;
            };
            let version = pkg.get("version").and_then(|v| v.as_str()).unwrap_or_default();
            resolved.push((name.to_string(), version.to_string()));
        }
    }
    if let Some(dependencies) = doc.get("dependencies").and_then(|d| d.as_object()) {
        for (name, pkg) in dependencies {
            let version = pkg.get("version").and_then(|v| v.as_str()).unwrap_or_default();
            resolved.push((name.clone(), version.to_string()));
        }
    }
    Some(Lockfile { path: path.to_path_buf(), ecosystem: Ecosystem::Npm, resolved })
}
