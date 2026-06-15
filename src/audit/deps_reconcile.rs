use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::buildmeta::{self, stdlib, BuildMeta, DepScope, Ecosystem};
use crate::import_check::{ecosystem_for, external_root, normalize, PYTHON_IMPORT_ALIASES};
use crate::parse::Language;
use crate::thresholds::AuditThresholds;

use super::corpus::Corpus;
use super::finding::{AuditFinding, AuditKind, BloatedDepEvidence, ImportConfidence, PhantomDepEvidence};
use super::imports::extract_imports;

struct ImportedRoots {
    by_eco: HashMap<Ecosystem, HashSet<String>>,
    first_seen: HashMap<(Ecosystem, String), (PathBuf, u32)>,
    go_targets: Vec<String>,
}

pub fn run_from(corpus: &Corpus, root: &Path, _thresholds: &AuditThresholds) -> Vec<AuditFinding> {
    let mut meta = buildmeta::discover(root);
    if corpus.files.iter().any(|f| f.lang == Language::CSharp) {
        meta.manifests.extend(buildmeta::csproj_manifests(root));
    }
    if meta.manifests.is_empty() {
        return Vec::new();
    }
    let imported = collect_imports(corpus);
    let mut findings = bloated_findings(&meta, &imported, corpus);
    findings.extend(phantom_findings(&meta, &imported));
    findings
}

fn collect_imports(corpus: &Corpus) -> ImportedRoots {
    let mut imported = ImportedRoots { by_eco: HashMap::new(), first_seen: HashMap::new(), go_targets: Vec::new() };
    for file in &corpus.files {
        let Some(eco) = ecosystem_for(file.lang) else { continue };
        let Some((source, tree)) = file.parsed() else { continue };
        for import in extract_imports(tree, source, file.lang) {
            let Some(root_name) = external_root(eco, &import.target) else { continue };
            if eco == Ecosystem::Go {
                imported.go_targets.push(root_name.clone());
            }
            let normalized = normalize(&root_name);
            let key = (eco, normalized.clone());
            imported.first_seen.entry(key).or_insert_with(|| (file.path.clone(), import.line));
            imported.by_eco.entry(eco).or_default().insert(normalized);
        }
    }
    imported
}

fn bloated_findings(meta: &BuildMeta, imported: &ImportedRoots, corpus: &Corpus) -> Vec<AuditFinding> {
    let workspace_internal = super::reflexion::workspace_internal_names(meta);
    let mut findings = Vec::new();
    for manifest in &meta.manifests {
        for dep in &manifest.deps {
            if is_bloated(manifest, dep, &workspace_internal, imported, corpus) {
                findings.push(bloated(manifest.path.clone(), dep.line, dep, manifest.ecosystem));
            }
        }
    }
    findings.sort_by(|a, b| a.representative_snippet.cmp(&b.representative_snippet));
    findings
}

fn is_bloated(
    manifest: &buildmeta::Manifest,
    dep: &buildmeta::DeclaredDep,
    workspace_internal: &HashSet<String>,
    imported: &ImportedRoots,
    corpus: &Corpus,
) -> bool {
    if dep.own || dep.scope != DepScope::Deployed || bloat_unverifiable(manifest.ecosystem) {
        return false;
    }
    let name = normalize(&dep.name);
    if matches!(manifest.ecosystem, Ecosystem::Cargo | Ecosystem::Npm | Ecosystem::Go)
        && workspace_internal.contains(&name)
    {
        return false;
    }
    !is_imported(manifest.ecosystem, &name, imported) && !is_referenced_in_source(&dep.name, corpus)
}

fn bloat_unverifiable(eco: Ecosystem) -> bool {
    eco == Ecosystem::RubyGems
}

fn is_imported(eco: Ecosystem, name: &str, imported: &ImportedRoots) -> bool {
    let empty = HashSet::new();
    let roots = imported.by_eco.get(&eco).unwrap_or(&empty);
    if roots.contains(name) {
        return true;
    }
    if eco == Ecosystem::Pip {
        return PYTHON_IMPORT_ALIASES.iter().any(|(import, package)| *package == name && roots.contains(*import));
    }
    if eco == Ecosystem::Go {
        return imported.go_targets.iter().any(|t| t == name || t.starts_with(&format!("{name}/")));
    }
    if eco == Ecosystem::NuGet {
        return nuget_imported(name, roots);
    }
    false
}

const NUGET_ALIASES: &[(&str, &str)] = &[("awssdk", "amazon"), ("castle.core", "castle")];

fn nuget_imported(name: &str, roots: &HashSet<String>) -> bool {
    let direct = roots
        .iter()
        .any(|t| t.starts_with(&format!("{name}.")) || (t.contains('.') && name.starts_with(&format!("{t}."))));
    if direct {
        return true;
    }
    NUGET_ALIASES
        .iter()
        .filter(|(pkg, _)| name == *pkg || name.starts_with(&format!("{pkg}.")))
        .any(|(_, ns)| roots.iter().any(|t| t == ns || t.starts_with(&format!("{ns}."))))
}

fn is_referenced_in_source(name: &str, corpus: &Corpus) -> bool {
    let underscore = name.replace('-', "_");
    corpus
        .files
        .iter()
        .filter(|f| f.path.file_name().and_then(|n| n.to_str()) != Some("Package.swift"))
        .filter_map(|f| f.source.as_deref())
        .any(|source| buildmeta::line_of(source, name) != 0 || buildmeta::line_of(source, &underscore) != 0)
}

fn bloated(manifest: PathBuf, line: u32, dep: &buildmeta::DeclaredDep, eco: Ecosystem) -> AuditFinding {
    let confidence = if matches!(eco, Ecosystem::NuGet | Ecosystem::Swift) {
        ImportConfidence::Low
    } else {
        ImportConfidence::Medium
    };
    wrap(
        dep.name.clone(),
        AuditKind::BloatedDependency(BloatedDepEvidence {
            manifest,
            line,
            name: dep.name.clone(),
            constraint: dep.constraint.clone(),
            confidence,
        }),
    )
}

fn phantom_findings(meta: &BuildMeta, imported: &ImportedRoots) -> Vec<AuditFinding> {
    let manifest_names = names_by_eco(meta);
    let lockfile_names = locked_by_eco(meta);
    let mut findings = Vec::new();
    for ((eco, name), (file, line)) in &imported.first_seen {
        if stdlib::is_stdlib(*eco, name) {
            continue;
        }
        let declared = manifest_names.get(eco).is_some_and(|names| prefix_member(*eco, names, name));
        let locked = lockfile_names.get(eco).is_some_and(|names| prefix_member(*eco, names, name));
        if !declared && locked {
            findings.push(wrap(
                name.clone(),
                AuditKind::PhantomDependency(PhantomDepEvidence {
                    file: file.clone(),
                    line: *line,
                    name: name.clone(),
                    confidence: ImportConfidence::Medium,
                }),
            ));
        }
    }
    findings.sort_by(|a, b| a.representative_snippet.cmp(&b.representative_snippet));
    findings
}

fn prefix_member(eco: Ecosystem, names: &HashSet<String>, target: &str) -> bool {
    if names.contains(target) {
        return true;
    }
    let sep = match eco {
        Ecosystem::Go => "/",
        Ecosystem::NuGet => ".",
        _ => return false,
    };
    names.iter().any(|n| target.starts_with(&format!("{n}{sep}")))
}

fn names_by_eco(meta: &BuildMeta) -> HashMap<Ecosystem, HashSet<String>> {
    grouped_names(meta.manifests.iter().flat_map(|m| m.deps.iter().map(|d| (m.ecosystem, normalize(&d.name)))))
}

fn locked_by_eco(meta: &BuildMeta) -> HashMap<Ecosystem, HashSet<String>> {
    grouped_names(meta.lockfiles.iter().flat_map(|l| l.resolved.iter().map(|(n, _)| (l.ecosystem, normalize(n)))))
}

fn grouped_names(pairs: impl Iterator<Item = (Ecosystem, String)>) -> HashMap<Ecosystem, HashSet<String>> {
    let mut out: HashMap<Ecosystem, HashSet<String>> = HashMap::new();
    for (eco, name) in pairs {
        out.entry(eco).or_default().insert(name);
    }
    out
}

pub(super) fn wrap(snippet: String, kind: AuditKind) -> AuditFinding {
    AuditFinding {
        kind,
        representative_snippet: snippet,
        support: 1,
        file_count: 1,
        idf_score: None,
        action_label: None,
        pattern_category: None,
        locality_entropy: None,
        p_value: None,
        locations: Vec::new(),
    }
}
