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
    let workspace_members = doc
        .get("workspaces")
        .and_then(|w| w.as_array().or_else(|| w.get("packages").and_then(|p| p.as_array())))
        .into_iter()
        .flatten()
        .filter_map(|m| m.as_str())
        .filter(|m| !m.contains('*'))
        .map(std::path::PathBuf::from)
        .collect();
    Some(Manifest { path: path.to_path_buf(), ecosystem: Ecosystem::Npm, deps, workspace_members })
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
