use std::path::{Path, PathBuf};

use super::{line_of, DeclaredDep, DepScope, Ecosystem, Lockfile, Manifest};

pub(super) fn parse_manifest(path: &Path, source: &str) -> Option<Manifest> {
    let doc: toml::Value = toml::from_str(source).ok()?;
    let mut deps = Vec::new();
    for (table, scope) in [
        ("dependencies", DepScope::Deployed),
        ("dev-dependencies", DepScope::Dev),
        ("build-dependencies", DepScope::Build),
    ] {
        collect_dep_table(doc.get(table), scope, source, &mut deps);
        if let Some(workspace) = doc.get("workspace") {
            collect_dep_table(workspace.get(table), scope, source, &mut deps);
        }
        if let Some(targets) = doc.get("target").and_then(|t| t.as_table()) {
            for cfg in targets.values() {
                collect_dep_table(cfg.get(table), scope, source, &mut deps);
            }
        }
    }
    if let Some(own) = doc.get("package").and_then(|p| p.get("name")).and_then(|n| n.as_str()) {
        deps.push(DeclaredDep { name: own.to_string(), constraint: String::new(), scope: DepScope::Deployed, line: 0 });
    }
    let workspace_members = doc
        .get("workspace")
        .and_then(|w| w.get("members"))
        .and_then(|m| m.as_array())
        .into_iter()
        .flatten()
        .filter_map(|m| m.as_str())
        .filter(|m| !m.contains('*'))
        .map(PathBuf::from)
        .collect();
    Some(Manifest { path: path.to_path_buf(), ecosystem: Ecosystem::Cargo, deps, workspace_members })
}

fn collect_dep_table(table: Option<&toml::Value>, scope: DepScope, source: &str, out: &mut Vec<DeclaredDep>) {
    let Some(table) = table.and_then(|t| t.as_table()) else { return };
    for (name, spec) in table {
        let constraint = match spec {
            toml::Value::String(v) => v.clone(),
            toml::Value::Table(t) => t.get("version").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            _ => String::new(),
        };
        let renamed = spec.as_table().and_then(|t| t.get("package")).and_then(|p| p.as_str());
        let resolved_name = renamed.unwrap_or(name);
        out.push(DeclaredDep { name: resolved_name.to_string(), constraint, scope, line: line_of(source, name) });
    }
}

pub(super) fn parse_lockfile(path: &Path, source: &str) -> Option<Lockfile> {
    let doc: toml::Value = toml::from_str(source).ok()?;
    let resolved = doc
        .get("package")
        .and_then(|p| p.as_array())
        .into_iter()
        .flatten()
        .filter_map(|pkg| {
            let name = pkg.get("name")?.as_str()?.to_string();
            let version = pkg.get("version").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            Some((name, version))
        })
        .collect();
    Some(Lockfile { path: path.to_path_buf(), ecosystem: Ecosystem::Cargo, resolved })
}
