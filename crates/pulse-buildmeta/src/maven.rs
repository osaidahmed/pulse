use std::path::{Path, PathBuf};

use super::{line_of, strip_xml_comments, DeclaredDep, DepScope, Ecosystem, Manifest};

pub(super) fn parse_manifest(path: &Path, source: &str) -> Option<Manifest> {
    let cleaned = strip_xml_comments(source);
    let deps = collect_deps(&cleaned, source);
    let workspace_members = collect_modules(&cleaned);
    if deps.is_empty() && workspace_members.is_empty() {
        return None;
    }
    Some(Manifest { path: path.to_path_buf(), ecosystem: Ecosystem::Maven, deps, workspace_members })
}

fn collect_deps(cleaned: &str, source: &str) -> Vec<DeclaredDep> {
    let mut deps = Vec::new();
    let mut rest = cleaned;
    while let Some(start) = rest.find("<dependency>") {
        let after = &rest[start + "<dependency>".len()..];
        let end = after.find("</dependency>").unwrap_or(after.len());
        if let Some(dep) = dep_from_block(&after[..end], source) {
            deps.push(dep);
        }
        rest = &after[end..];
    }
    deps
}

fn dep_from_block(block: &str, source: &str) -> Option<DeclaredDep> {
    let group = element(block, "groupId")?;
    let artifact = element(block, "artifactId")?;
    let scope = match element(block, "scope") {
        Some("test") => DepScope::Dev,
        _ => DepScope::Deployed,
    };
    Some(DeclaredDep {
        name: format!("{group}:{artifact}"),
        constraint: element(block, "version").unwrap_or_default().to_string(),
        scope,
        line: line_of(source, artifact),
        own: false,
    })
}

fn collect_modules(cleaned: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut rest = cleaned;
    while let Some(start) = rest.find("<module>") {
        let after = &rest[start + "<module>".len()..];
        let end = after.find("</module>").unwrap_or(after.len());
        let member = after[..end].trim();
        if !member.is_empty() {
            out.push(PathBuf::from(member));
        }
        rest = &after[end..];
    }
    out
}

fn element<'a>(block: &'a str, tag: &str) -> Option<&'a str> {
    let start = block.find(&format!("<{tag}>"))? + tag.len() + 2;
    let val = &block[start..];
    val.find(&format!("</{tag}>")).map(|end| val[..end].trim())
}
