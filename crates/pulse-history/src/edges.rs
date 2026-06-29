use std::collections::HashSet;
use std::path::{Path, PathBuf};

use pulse_audit::graph::{ImportGraph, InputEdge};
use pulse_audit::imports;
use pulse_syntax::parse::{self, Language};

pub fn build_graph(typed_files: &[(PathBuf, Language)], root: &Path) -> ImportGraph {
    let edges = collect_edges(typed_files, root);
    ImportGraph::build(&edges)
}

pub fn directly_linked(graph: &ImportGraph, a: &Path, b: &Path) -> bool {
    let Some(a_idx) = graph.registry.lookup(a) else { return false };
    let Some(b_idx) = graph.registry.lookup(b) else { return false };
    graph.adjacency.outgoing(a_idx).contains(&b_idx) || graph.adjacency.outgoing(b_idx).contains(&a_idx)
}

fn collect_edges(typed_files: &[(PathBuf, Language)], root: &Path) -> Vec<InputEdge> {
    let typed_set: HashSet<PathBuf> = typed_files.iter().map(|(p, _)| p.clone()).collect();
    let mut edges: Vec<InputEdge> = Vec::new();
    for (path, lang) in typed_files {
        edges.extend(edges_for_file(path, *lang, root, &typed_set, typed_files));
    }
    edges
}

fn edges_for_file(
    path: &Path,
    lang: Language,
    root: &Path,
    typed_set: &HashSet<PathBuf>,
    typed_files: &[(PathBuf, Language)],
) -> Vec<InputEdge> {
    let Ok(source) = std::fs::read_to_string(path) else { return Vec::new() };
    let Some(tree) = parse::parse_guarded(&source, lang) else { return Vec::new() };
    let raws = imports::extract_imports(&tree, &source, lang);
    let mut out: Vec<InputEdge> = Vec::new();
    for raw in raws {
        let Some(resolved) = resolve_one(&raw.target, path, root, lang, typed_set) else {
            continue;
        };
        let target_lang = typed_files.iter().find(|(p, _)| p == &resolved).map_or(lang, |(_, l)| *l);
        out.push(InputEdge { source: path.to_path_buf(), target: resolved, source_lang: lang, target_lang });
    }
    out
}

fn resolve_one(
    raw: &str,
    source_file: &Path,
    root: &Path,
    lang: Language,
    typed_set: &HashSet<PathBuf>,
) -> Option<PathBuf> {
    if let Some(p) = imports::resolve_target(raw, source_file, root, lang) {
        if typed_set.contains(&p) {
            return Some(p);
        }
    }
    imports::resolve_by_suffix(raw, lang, typed_set)
}
