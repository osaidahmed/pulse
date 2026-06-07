use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;

use super::call_graph::{CallGraph, MethodIdentity, MethodIndex};
use super::definitions::DefinitionRecord;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ClassIdentity {
    pub file: PathBuf,
    pub name: String,
    pub parent_class: Option<String>,
    pub method_indices: Vec<MethodIndex>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ClassIndex(pub u32);

#[derive(Debug, Default)]
pub struct ClassRegistry {
    pub classes: Vec<ClassIdentity>,
    pub by_file_and_name: BTreeMap<(PathBuf, String), ClassIndex>,
    pub by_name: BTreeMap<String, Vec<ClassIndex>>,
}

impl ClassRegistry {
    pub fn from_definitions(defs: &[DefinitionRecord], call_registry: &super::call_graph::MethodRegistry) -> Self {
        let groups = group_methods_by_class(defs, call_registry);
        let mut reg = Self::default();
        for ((file, name), (parent, methods)) in groups {
            let class_idx = ClassIndex(reg.classes.len() as u32);
            reg.by_file_and_name.insert((file.clone(), name.clone()), class_idx);
            reg.by_name.entry(name.clone()).or_default().push(class_idx);
            reg.classes.push(ClassIdentity { file, name, parent_class: parent, method_indices: methods });
        }
        reg
    }

    pub fn get(&self, idx: ClassIndex) -> Option<&ClassIdentity> {
        self.classes.get(idx.0 as usize)
    }

    pub fn count(&self) -> usize {
        self.classes.len()
    }

    pub fn methods_in(&self, idx: ClassIndex) -> &[MethodIndex] {
        self.get(idx).map_or(&[][..], |c| c.method_indices.as_slice())
    }

    pub fn lookup_parent(&self, idx: ClassIndex) -> Vec<ClassIndex> {
        let Some(c) = self.get(idx) else { return Vec::new() };
        let Some(parent_name) = c.parent_class.as_deref() else { return Vec::new() };
        self.by_name.get(parent_name).cloned().unwrap_or_default()
    }
}

type ClassGroupMap = BTreeMap<(PathBuf, String), (Option<String>, Vec<MethodIndex>)>;

fn group_methods_by_class(
    defs: &[DefinitionRecord],
    call_registry: &super::call_graph::MethodRegistry,
) -> ClassGroupMap {
    let mut groups: ClassGroupMap = BTreeMap::new();
    for def in defs {
        let Some(class_name) = def.identity.class.clone() else { continue };
        let key = (def.identity.file.clone(), class_name);
        let Some(idx) = lookup_method_idx(call_registry, &def.identity) else { continue };
        let entry = groups.entry(key).or_insert((def.parent_class.clone(), Vec::new()));
        if entry.0.is_none() {
            entry.0 = def.parent_class.clone();
        }
        entry.1.push(idx);
    }
    groups
}

fn lookup_method_idx(reg: &super::call_graph::MethodRegistry, identity: &MethodIdentity) -> Option<MethodIndex> {
    reg.methods.iter().position(|m| m == identity).map(|i| MethodIndex(i as u32))
}

pub fn class_method_count(reg: &ClassRegistry, idx: ClassIndex) -> u32 {
    reg.methods_in(idx).len() as u32
}

pub fn class_wmc(reg: &ClassRegistry, idx: ClassIndex, cc_for_method: &impl Fn(MethodIndex) -> u32) -> u32 {
    let mut total: u32 = 0;
    for m in reg.methods_in(idx) {
        total = total.saturating_add(cc_for_method(*m));
    }
    total
}

pub fn class_atfd<F>(reg: &ClassRegistry, idx: ClassIndex, foreign_for: &F) -> u32
where
    F: Fn(MethodIndex) -> Vec<(String, String)>,
{
    let mut foreign: HashSet<(String, String)> = HashSet::new();
    for m in reg.methods_in(idx) {
        for pair in foreign_for(*m) {
            foreign.insert(pair);
        }
    }
    foreign.len() as u32
}

pub fn class_tcc<F>(reg: &ClassRegistry, idx: ClassIndex, fields_for: &F) -> f64
where
    F: Fn(MethodIndex) -> (Vec<String>, bool),
{
    let field_sets = collect_non_ctor_field_sets(reg, idx, fields_for);
    let n = field_sets.len();
    if n < 2 {
        return 0.0;
    }
    let connected = count_connected_pairs(&field_sets);
    let total = (n * (n - 1) / 2) as u32;
    if total == 0 {
        0.0
    } else {
        f64::from(connected) / f64::from(total)
    }
}

fn collect_non_ctor_field_sets<F>(reg: &ClassRegistry, idx: ClassIndex, fields_for: &F) -> Vec<HashSet<String>>
where
    F: Fn(MethodIndex) -> (Vec<String>, bool),
{
    let mut out: Vec<HashSet<String>> = Vec::new();
    for m in reg.methods_in(idx) {
        let (fields, is_ctor) = fields_for(*m);
        if is_ctor {
            continue;
        }
        out.push(fields.into_iter().collect());
    }
    out
}

fn count_connected_pairs(field_sets: &[HashSet<String>]) -> u32 {
    let mut connected: u32 = 0;
    let n = field_sets.len();
    for i in 0..n {
        for j in (i + 1)..n {
            if !field_sets[i].is_disjoint(&field_sets[j]) {
                connected += 1;
            }
        }
    }
    connected
}

#[derive(Debug, Clone, Copy)]
pub enum EdgeDirection {
    Incoming,
    Outgoing,
}

pub fn class_cc(reg: &ClassRegistry, graph: &CallGraph, idx: ClassIndex) -> u32 {
    distinct_neighbor_classes(reg, graph, idx, EdgeDirection::Incoming)
}

pub fn class_fanout(reg: &ClassRegistry, graph: &CallGraph, idx: ClassIndex) -> u32 {
    distinct_neighbor_classes(reg, graph, idx, EdgeDirection::Outgoing)
}

fn distinct_neighbor_classes(reg: &ClassRegistry, graph: &CallGraph, idx: ClassIndex, dir: EdgeDirection) -> u32 {
    let self_bucket = bucket_for_class(reg.get(idx));
    let mut buckets: HashSet<String> = HashSet::new();
    for m in reg.methods_in(idx) {
        let edges = match dir {
            EdgeDirection::Incoming => graph.adjacency.incoming(*m),
            EdgeDirection::Outgoing => graph.adjacency.outgoing(*m),
        };
        for edge in edges {
            let other_idx = match dir {
                EdgeDirection::Incoming => edge.source,
                EdgeDirection::Outgoing => edge.target,
            };
            try_insert_neighbor_bucket(other_idx, graph, &self_bucket, &mut buckets);
        }
    }
    buckets.len() as u32
}

fn try_insert_neighbor_bucket(idx: MethodIndex, graph: &CallGraph, self_bucket: &str, buckets: &mut HashSet<String>) {
    let Some(other) = graph.registry.get(idx) else { return };
    let bucket = bucket_for_method(other);
    if bucket != self_bucket {
        buckets.insert(bucket);
    }
}

pub fn bucket_for_class(class: Option<&ClassIdentity>) -> String {
    let Some(c) = class else { return String::new() };
    format_bucket(&c.file.display().to_string(), Some(c.name.as_str()))
}

pub fn bucket_for_method(m: &MethodIdentity) -> String {
    format_bucket(&m.file.display().to_string(), m.class.as_deref())
}

fn format_bucket(file: &str, class: Option<&str>) -> String {
    class.map_or_else(|| format!("__free::{file}"), |cls| format!("{file}::{cls}"))
}
