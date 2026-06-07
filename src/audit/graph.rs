use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::parse::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeIndex(pub u32);

#[derive(Debug, Clone)]
pub struct InputEdge {
    pub source: PathBuf,
    pub target: PathBuf,
    pub source_lang: Language,
    pub target_lang: Language,
}

#[derive(Debug, Clone, Default)]
pub struct NodeRegistry {
    paths: Vec<PathBuf>,
    languages: Vec<Language>,
    index_by_path: BTreeMap<PathBuf, NodeIndex>,
}

impl NodeRegistry {
    pub fn intern(&mut self, path: &Path, lang: Language) -> NodeIndex {
        if let Some(&idx) = self.index_by_path.get(path) {
            return idx;
        }
        let idx = NodeIndex(self.paths.len() as u32);
        self.paths.push(path.to_path_buf());
        self.languages.push(lang);
        self.index_by_path.insert(path.to_path_buf(), idx);
        idx
    }

    pub fn count(&self) -> usize {
        self.paths.len()
    }

    pub fn path_of(&self, node: NodeIndex) -> &Path {
        &self.paths[node.0 as usize]
    }

    pub fn language_of(&self, node: NodeIndex) -> Language {
        self.languages[node.0 as usize]
    }

    pub fn lookup(&self, file: &Path) -> Option<NodeIndex> {
        self.index_by_path.get(file).copied()
    }
}

#[derive(Debug, Clone)]
pub struct Adjacency {
    outgoing: Vec<Vec<NodeIndex>>,
    incoming: Vec<Vec<NodeIndex>>,
    edges: Vec<(NodeIndex, NodeIndex)>,
}

impl Adjacency {
    pub fn with_capacity(node_count: usize) -> Self {
        Self { outgoing: vec![Vec::new(); node_count], incoming: vec![Vec::new(); node_count], edges: Vec::new() }
    }

    pub fn insert_unique(&mut self, src: NodeIndex, dst: NodeIndex) -> bool {
        if self.outgoing[src.0 as usize].contains(&dst) {
            return false;
        }
        self.outgoing[src.0 as usize].push(dst);
        self.incoming[dst.0 as usize].push(src);
        self.edges.push((src, dst));
        true
    }

    pub fn finalize(&mut self) {
        for adj in &mut self.outgoing {
            adj.sort();
        }
        for adj in &mut self.incoming {
            adj.sort();
        }
        self.edges.sort();
    }

    pub fn outgoing(&self, node: NodeIndex) -> &[NodeIndex] {
        &self.outgoing[node.0 as usize]
    }

    pub fn incoming(&self, node: NodeIndex) -> &[NodeIndex] {
        &self.incoming[node.0 as usize]
    }

    pub fn edges(&self) -> &[(NodeIndex, NodeIndex)] {
        &self.edges
    }

    pub fn afferent(&self, node: NodeIndex) -> u32 {
        self.incoming(node).len() as u32
    }

    pub fn efferent(&self, node: NodeIndex) -> u32 {
        self.outgoing(node).len() as u32
    }
}

#[derive(Debug, Clone)]
pub struct ImportGraph {
    pub registry: NodeRegistry,
    pub adjacency: Adjacency,
}

pub fn build(input: &[InputEdge]) -> ImportGraph {
    let mut registry = NodeRegistry::default();
    for e in input {
        registry.intern(&e.source, e.source_lang);
        registry.intern(&e.target, e.target_lang);
    }
    let mut adjacency = Adjacency::with_capacity(registry.count());
    for e in input {
        let src = registry.lookup(&e.source).expect("interned");
        let dst = registry.lookup(&e.target).expect("interned");
        adjacency.insert_unique(src, dst);
    }
    adjacency.finalize();
    ImportGraph { registry, adjacency }
}

impl ImportGraph {
    pub fn build(input: &[InputEdge]) -> Self {
        build(input)
    }
}
