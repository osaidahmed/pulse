use std::collections::{HashSet, VecDeque};

use super::class_registry::{ClassIndex, ClassRegistry};

#[derive(Debug, Default)]
pub struct InheritanceGraph {
    pub parent_of: Vec<Vec<ClassIndex>>,
    pub children_of: Vec<Vec<ClassIndex>>,
}

impl InheritanceGraph {
    pub fn parents(&self, idx: ClassIndex) -> &[ClassIndex] {
        self.parent_of.get(idx.0 as usize).map_or(&[][..], Vec::as_slice)
    }

    pub fn children(&self, idx: ClassIndex) -> &[ClassIndex] {
        self.children_of.get(idx.0 as usize).map_or(&[][..], Vec::as_slice)
    }
}

pub fn build_inheritance_graph(registry: &ClassRegistry) -> InheritanceGraph {
    let n = registry.count();
    let mut parent_of: Vec<Vec<ClassIndex>> = vec![Vec::new(); n];
    let mut children_of: Vec<Vec<ClassIndex>> = vec![Vec::new(); n];
    for (i, slot) in parent_of.iter_mut().enumerate().take(n) {
        let idx = ClassIndex(i as u32);
        let parents = registry.lookup_parent(idx);
        for parent_idx in parents {
            slot.push(parent_idx);
            children_of[parent_idx.0 as usize].push(idx);
        }
    }
    InheritanceGraph { parent_of, children_of }
}

pub fn descendants_of(graph: &InheritanceGraph, idx: ClassIndex) -> Vec<ClassIndex> {
    bfs_traverse(idx, |i| graph.children(i))
}

pub fn ancestors_of(graph: &InheritanceGraph, idx: ClassIndex) -> Vec<ClassIndex> {
    bfs_traverse(idx, |i| graph.parents(i))
}

fn bfs_traverse<'a, F>(start: ClassIndex, neighbors: F) -> Vec<ClassIndex>
where
    F: Fn(ClassIndex) -> &'a [ClassIndex],
{
    let mut visited: HashSet<ClassIndex> = HashSet::new();
    let mut queue: VecDeque<ClassIndex> = VecDeque::new();
    let mut out: Vec<ClassIndex> = Vec::new();
    queue.push_back(start);
    visited.insert(start);
    while let Some(curr) = queue.pop_front() {
        for &next in neighbors(curr) {
            if visited.insert(next) {
                out.push(next);
                queue.push_back(next);
            }
        }
    }
    out
}
