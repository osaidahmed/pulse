use std::collections::HashSet;
use std::path::Path;

use crate::audit::imports::{extract_imports, RawImport};
use crate::buildmeta::{declared, stdlib, Ecosystem};
use crate::imports::{ecosystem_for, external_root, normalize, PYTHON_IMPORT_ALIASES};
use crate::parse::{parse_guarded, Language};
use crate::smells::{Finding, Location, Smell};

pub fn run(file_path: &str, source: &str, lang: Language, original: Option<&str>) -> Vec<Finding> {
    let Some(eco) = ecosystem_for(lang) else { return Vec::new() };
    let path = Path::new(file_path);
    let Some(root) = declared::nearest_manifest_root(path, eco) else { return Vec::new() };
    let Some(tree) = parse_guarded(source, lang) else { return Vec::new() };
    let declared = normalize_set(&declared::declared_names_for(&root));
    let mut findings = Vec::new();
    let mut seen = HashSet::new();
    for import in extract_imports(&tree, source, lang) {
        let Some(root_name) = external_root(eco, &import.target) else { continue };
        if !seen.insert(root_name.clone()) {
            continue;
        }
        if is_accounted_for(eco, &root_name, &import.target, &declared) {
            continue;
        }
        if resolves_internally(&root_name, &root, path, eco) {
            continue;
        }
        if original.is_some_and(|pre| pre.contains(&import.target)) {
            continue;
        }
        findings.push(finding_for(&root_name, &import));
    }
    findings
}

fn is_accounted_for(eco: Ecosystem, root_name: &str, target: &str, declared: &HashSet<String>) -> bool {
    if stdlib::is_stdlib(eco, root_name) {
        return true;
    }
    let normalized = normalize(root_name);
    if declared.contains(&normalized) {
        return true;
    }
    if eco == Ecosystem::Pip {
        if let Some((_, package)) = PYTHON_IMPORT_ALIASES.iter().find(|(import, _)| *import == normalized) {
            return declared.contains(*package);
        }
    }
    if eco == Ecosystem::Go {
        return declared.iter().any(|d| target == *d || target.starts_with(&format!("{d}/")));
    }
    false
}

fn resolves_internally(root_name: &str, project_root: &Path, file_path: &Path, eco: Ecosystem) -> bool {
    let mut dirs = vec![project_root.to_path_buf(), project_root.join("src"), project_root.join("lib")];
    if let Some(parent) = file_path.parent() {
        dirs.push(parent.to_path_buf());
    }
    dirs.iter().any(|dir| dir_resolves(dir, root_name, internal_exts(eco)))
}

const INTERNAL_EXTS: &[(Ecosystem, &[&str])] = &[
    (Ecosystem::Pip, &["py"]),
    (Ecosystem::Npm, &["ts", "tsx", "js", "jsx", "mjs", "cjs"]),
    (Ecosystem::RubyGems, &["rb"]),
    (Ecosystem::Cargo, &["rs"]),
];

fn internal_exts(eco: Ecosystem) -> &'static [&'static str] {
    INTERNAL_EXTS.iter().find(|(e, _)| *e == eco).map_or(&[], |(_, exts)| exts)
}

fn dir_resolves(dir: &Path, root_name: &str, exts: &[&str]) -> bool {
    if dir.join(root_name).is_dir() {
        return true;
    }
    exts.iter().any(|ext| dir.join(format!("{root_name}.{ext}")).is_file())
}

fn normalize_set(names: &HashSet<String>) -> HashSet<String> {
    names.iter().map(|n| normalize(n)).collect()
}

fn finding_for(root_name: &str, import: &RawImport) -> Finding {
    Finding {
        smell: Smell::HallucinatedImport,
        location: Location::Function { name: root_name.to_string(), start_line: import.line, end_line: import.line },
        detail: format!(
            "`{}` matches no declared dependency, lockfile entry, or standard-library module — verify the package name or declare it",
            import.target
        ),
    }
}
