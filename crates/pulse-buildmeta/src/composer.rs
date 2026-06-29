use std::path::Path;

use super::{line_of, DeclaredDep, DepScope, Ecosystem, Lockfile, Manifest};

pub(super) fn parse_manifest(path: &Path, source: &str) -> Option<Manifest> {
    let doc: serde_json::Value = serde_json::from_str(source).ok()?;
    let mut deps = Vec::new();
    for (key, scope) in [("require", DepScope::Deployed), ("require-dev", DepScope::Dev)] {
        let Some(table) = doc.get(key).and_then(|t| t.as_object()) else { continue };
        for (name, constraint) in table {
            if !name.contains('/') {
                continue;
            }
            deps.push(DeclaredDep {
                name: name.clone(),
                constraint: constraint.as_str().unwrap_or_default().to_string(),
                scope,
                line: line_of(source, name),
                own: false,
            });
        }
    }
    if let Some(own) = doc.get("name").and_then(|n| n.as_str()).filter(|n| n.contains('/')) {
        deps.push(DeclaredDep {
            name: own.to_string(),
            constraint: String::new(),
            scope: DepScope::Deployed,
            line: 0,
            own: true,
        });
    }
    Some(Manifest { path: path.to_path_buf(), ecosystem: Ecosystem::Composer, deps, workspace_members: Vec::new() })
}

pub(super) fn parse_lockfile(path: &Path, source: &str) -> Option<Lockfile> {
    let doc: serde_json::Value = serde_json::from_str(source).ok()?;
    let mut resolved = Vec::new();
    for key in ["packages", "packages-dev"] {
        let Some(arr) = doc.get(key).and_then(|p| p.as_array()) else { continue };
        for pkg in arr {
            let version = pkg.get("version").and_then(|v| v.as_str()).unwrap_or_default();
            if let Some(name) = pkg.get("name").and_then(|n| n.as_str()) {
                resolved.push((name.to_string(), version.to_string()));
            }
        }
    }
    (!resolved.is_empty()).then(|| Lockfile { path: path.to_path_buf(), ecosystem: Ecosystem::Composer, resolved })
}
