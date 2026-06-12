use std::path::Path;

use super::{line_of, DeclaredDep, DepScope, Ecosystem, Manifest};

pub(super) fn parse_pyproject(path: &Path, source: &str) -> Option<Manifest> {
    let doc: toml::Value = toml::from_str(source).ok()?;
    let mut deps = Vec::new();
    let project = doc.get("project");
    collect_requirement_list(project.and_then(|p| p.get("dependencies")), DepScope::Deployed, source, &mut deps);
    if let Some(optional) = project.and_then(|p| p.get("optional-dependencies")).and_then(|o| o.as_table()) {
        for group in optional.values() {
            collect_requirement_list(Some(group), DepScope::Dev, source, &mut deps);
        }
    }
    let poetry = doc.get("tool").and_then(|t| t.get("poetry"));
    collect_poetry_table(poetry.and_then(|p| p.get("dependencies")), DepScope::Deployed, source, &mut deps);
    collect_poetry_table(poetry.and_then(|p| p.get("dev-dependencies")), DepScope::Dev, source, &mut deps);
    if let Some(groups) = poetry.and_then(|p| p.get("group")).and_then(|g| g.as_table()) {
        for group in groups.values() {
            collect_poetry_table(group.get("dependencies"), DepScope::Dev, source, &mut deps);
        }
    }
    Some(Manifest { path: path.to_path_buf(), ecosystem: Ecosystem::Pip, deps, workspace_members: Vec::new() })
}

fn collect_requirement_list(list: Option<&toml::Value>, scope: DepScope, source: &str, out: &mut Vec<DeclaredDep>) {
    let Some(list) = list.and_then(|l| l.as_array()) else { return };
    for spec in list.iter().filter_map(|s| s.as_str()) {
        if let Some(dep) = requirement_dep(spec, scope, source) {
            out.push(dep);
        }
    }
}

fn collect_poetry_table(table: Option<&toml::Value>, scope: DepScope, source: &str, out: &mut Vec<DeclaredDep>) {
    let Some(table) = table.and_then(|t| t.as_table()) else { return };
    for (name, spec) in table {
        if name == "python" {
            continue;
        }
        let constraint = match spec {
            toml::Value::String(v) => v.clone(),
            toml::Value::Table(t) => t.get("version").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            _ => String::new(),
        };
        out.push(DeclaredDep { name: name.clone(), constraint, scope, line: line_of(source, name) });
    }
}

pub(super) fn parse_requirements(path: &Path, source: &str) -> Option<Manifest> {
    let mut deps = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('-') {
            continue;
        }
        if let Some(dep) = requirement_dep(trimmed, DepScope::Deployed, source) {
            deps.push(dep);
        }
    }
    (!deps.is_empty()).then(|| Manifest {
        path: path.to_path_buf(),
        ecosystem: Ecosystem::Pip,
        deps,
        workspace_members: Vec::new(),
    })
}

fn requirement_dep(spec: &str, scope: DepScope, source: &str) -> Option<DeclaredDep> {
    let body = spec.split(';').next().unwrap_or(spec).trim();
    let name_end =
        body.find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')).unwrap_or(body.len());
    let name = &body[..name_end];
    if name.is_empty() {
        return None;
    }
    let constraint = body[name_end..].trim_start_matches('[').trim().to_string();
    Some(DeclaredDep { name: name.to_string(), constraint, scope, line: line_of(source, name) })
}
