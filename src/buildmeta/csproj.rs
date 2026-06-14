use std::path::Path;

use super::{line_of, DeclaredDep, DepScope, Ecosystem, Manifest};

pub(super) fn parse_manifest(path: &Path, source: &str) -> Option<Manifest> {
    let mut deps = Vec::new();
    let cleaned = strip_comments(source);
    let mut rest = cleaned.as_str();
    while let Some(pos) = rest.find("<PackageReference") {
        let after = &rest[pos + "<PackageReference".len()..];
        let end = after.find('>').unwrap_or(after.len());
        let attrs = &after[..end];
        if let Some(name) = attr_value(attrs, "Include") {
            deps.push(DeclaredDep {
                name: name.to_string(),
                constraint: attr_value(attrs, "Version").unwrap_or_default().to_string(),
                scope: DepScope::Deployed,
                line: line_of(source, name),
                own: false,
            });
        }
        rest = &after[end..];
    }
    if deps.is_empty() {
        return None;
    }
    Some(Manifest { path: path.to_path_buf(), ecosystem: Ecosystem::NuGet, deps, workspace_members: Vec::new() })
}

fn attr_value<'a>(attrs: &'a str, key: &str) -> Option<&'a str> {
    let start = attrs.find(&format!("{key}=\""))? + key.len() + 2;
    let val = &attrs[start..];
    val.find('"').map(|end| &val[..end])
}

fn strip_comments(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut rest = source;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        match rest[start..].find("-->") {
            Some(end) => rest = &rest[start + end + 3..],
            None => return out,
        }
    }
    out.push_str(rest);
    out
}
