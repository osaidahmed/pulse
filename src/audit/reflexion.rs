use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::buildmeta::{self, BuildMeta, DeclaredDep, DepScope, Ecosystem, Manifest};
use crate::import_check::{ecosystem_for, external_root, normalize};
use crate::thresholds::AuditThresholds;

use super::corpus::Corpus;
use super::deps_reconcile::wrap;
use super::finding::{
    AuditFinding, AuditKind, ImportConfidence, UndeclaredModuleDepEvidence, UnusedDeclaredDepEvidence,
};
use super::imports::extract_imports;

struct Component {
    name: String,
    dir: PathBuf,
    manifest: PathBuf,
    deps: Vec<DeclaredDep>,
    ecosystem: Ecosystem,
}

struct DeclaredEdge {
    manifest: PathBuf,
    line: u32,
    deployed: bool,
}

struct Witness {
    file: PathBuf,
    line: u32,
    target: String,
}

pub fn run_from(corpus: &Corpus, root: &Path, _thresholds: &AuditThresholds) -> Vec<AuditFinding> {
    let meta = buildmeta::discover(root);
    let comps = components(&meta);
    if comps.len() < 2 {
        return Vec::new();
    }
    let declared = declared_edges(&comps);
    let actual = actual_edges(&comps, corpus);
    let mut findings = divergence_findings(&comps, &declared, &actual);
    findings.extend(absence_findings(&comps, &declared, &actual, corpus));
    findings.sort_by(|a, b| a.representative_snippet.cmp(&b.representative_snippet));
    findings
}

pub fn workspace_internal_names(meta: &BuildMeta) -> HashSet<String> {
    let comps = components(meta);
    if comps.len() < 2 {
        return HashSet::new();
    }
    comps.iter().map(|c| normalize(&c.name)).collect()
}

fn workspace_manifest_name(eco: Ecosystem) -> Option<&'static str> {
    match eco {
        Ecosystem::Cargo => Some("Cargo.toml"),
        Ecosystem::Npm => Some("package.json"),
        _ => None,
    }
}

fn components(meta: &BuildMeta) -> Vec<Component> {
    let mut comps = Vec::new();
    let workspaces = meta
        .manifests
        .iter()
        .filter(|m| workspace_manifest_name(m.ecosystem).is_some() && !m.workspace_members.is_empty());
    for ws in workspaces {
        let (Some(ws_dir), Some(member_file)) = (ws.path.parent(), workspace_manifest_name(ws.ecosystem)) else {
            continue;
        };
        push_component(&mut comps, ws, ws_dir);
        for member in &ws.workspace_members {
            let dir = ws_dir.join(member);
            if let Some(m) = meta.manifests.iter().find(|m| m.path == dir.join(member_file)) {
                push_component(&mut comps, m, &dir);
            }
        }
    }
    comps
}

fn push_component(comps: &mut Vec<Component>, manifest: &Manifest, dir: &Path) {
    let Some(own) = manifest.deps.iter().find(|d| d.own) else { return };
    if comps.iter().any(|c| c.dir == dir) {
        return;
    }
    comps.push(Component {
        name: own.name.clone(),
        dir: dir.to_path_buf(),
        manifest: manifest.path.clone(),
        deps: manifest.deps.clone(),
        ecosystem: manifest.ecosystem,
    });
}

fn name_index(comps: &[Component]) -> HashMap<String, usize> {
    comps.iter().enumerate().map(|(i, c)| (normalize(&c.name), i)).collect()
}

fn edge_target(index: &HashMap<String, usize>, name: &str, from: usize) -> Option<usize> {
    let &j = index.get(&normalize(name))?;
    (j != from).then_some(j)
}

fn merge_edges<V>(pairs: Vec<((usize, usize), V)>, merge: fn(&mut V, &V)) -> HashMap<(usize, usize), V> {
    let mut edges: HashMap<(usize, usize), V> = HashMap::new();
    for (pair, value) in pairs {
        if let Some(existing) = edges.get_mut(&pair) {
            merge(existing, &value);
        } else {
            edges.insert(pair, value);
        }
    }
    edges
}

fn declared_edges(comps: &[Component]) -> HashMap<(usize, usize), DeclaredEdge> {
    let index = name_index(comps);
    let pairs = comps
        .iter()
        .enumerate()
        .flat_map(|(i, comp)| comp.deps.iter().filter(|d| !d.own).map(move |d| (i, comp, d)))
        .filter_map(|(i, comp, dep)| {
            let j = edge_target(&index, &dep.name, i)?;
            let edge = DeclaredEdge {
                manifest: comp.manifest.clone(),
                line: dep.line,
                deployed: dep.scope == DepScope::Deployed,
            };
            Some(((i, j), edge))
        })
        .collect();
    merge_edges(pairs, |a, b| a.deployed = a.deployed || b.deployed)
}

fn actual_edges(comps: &[Component], corpus: &Corpus) -> HashMap<(usize, usize), Witness> {
    let index = name_index(comps);
    let mut pairs = Vec::new();
    for file in &corpus.files {
        let Some(i) = maps_to(comps, &file.path) else { continue };
        let eco = comps[i].ecosystem;
        if ecosystem_for(file.lang) != Some(eco) {
            continue;
        }
        let Some((source, tree)) = file.parsed() else { continue };
        for import in extract_imports(tree, source, file.lang) {
            let Some(root_name) = external_root(eco, &import.target) else { continue };
            let Some(j) = edge_target(&index, &root_name, i) else { continue };
            pairs.push(((i, j), Witness { file: file.path.clone(), line: import.line, target: import.target }));
        }
    }
    merge_edges(pairs, |_, _| {})
}

fn maps_to(comps: &[Component], path: &Path) -> Option<usize> {
    comps
        .iter()
        .enumerate()
        .filter(|(_, c)| path.starts_with(&c.dir))
        .max_by_key(|(_, c)| c.dir.components().count())
        .map(|(i, _)| i)
}

fn divergence_findings(
    comps: &[Component],
    declared: &HashMap<(usize, usize), DeclaredEdge>,
    actual: &HashMap<(usize, usize), Witness>,
) -> Vec<AuditFinding> {
    actual
        .iter()
        .filter(|(pair, _)| !declared.contains_key(pair))
        .map(|(&(i, j), w)| {
            wrap(
                format!("{} -> {}", comps[i].name, comps[j].name),
                AuditKind::UndeclaredModuleDependency(UndeclaredModuleDepEvidence {
                    from_component: comps[i].name.clone(),
                    to_component: comps[j].name.clone(),
                    file: w.file.clone(),
                    line: w.line,
                    import_target: w.target.clone(),
                    confidence: ImportConfidence::Medium,
                }),
            )
        })
        .collect()
}

fn absence_findings(
    comps: &[Component],
    declared: &HashMap<(usize, usize), DeclaredEdge>,
    actual: &HashMap<(usize, usize), Witness>,
    corpus: &Corpus,
) -> Vec<AuditFinding> {
    declared
        .iter()
        .filter(|(pair, edge)| edge.deployed && !actual.contains_key(*pair))
        .filter(|(&(i, j), _)| !referenced_in_component(comps, i, &comps[j].name, corpus))
        .map(|(&(i, j), edge)| {
            wrap(
                format!("{} -> {}", comps[i].name, comps[j].name),
                AuditKind::UnusedDeclaredDependency(UnusedDeclaredDepEvidence {
                    manifest: edge.manifest.clone(),
                    line: edge.line,
                    from_component: comps[i].name.clone(),
                    to_component: comps[j].name.clone(),
                    confidence: ImportConfidence::Medium,
                }),
            )
        })
        .collect()
}

fn referenced_in_component(comps: &[Component], from: usize, name: &str, corpus: &Corpus) -> bool {
    let underscore = name.replace('-', "_");
    corpus
        .files
        .iter()
        .filter(|f| maps_to(comps, &f.path) == Some(from))
        .filter_map(|f| f.source.as_deref())
        .any(|source| buildmeta::line_of(source, name) != 0 || buildmeta::line_of(source, &underscore) != 0)
}
