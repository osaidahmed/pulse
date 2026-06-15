use std::collections::BTreeMap;
use std::path::PathBuf;

use super::call_walker::LocatedCall;
use super::definitions::DefinitionRecord;
use super::finding::ImportConfidence;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MethodIdentity {
    pub file: PathBuf,
    pub class: Option<String>,
    pub name: String,
    pub line: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MethodIndex(pub u32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallEdge {
    pub source: MethodIndex,
    pub target: MethodIndex,
    pub confidence: ImportConfidence,
}

#[derive(Debug, Default)]
pub struct MethodRegistry {
    pub methods: Vec<MethodIdentity>,
    pub by_name: BTreeMap<String, Vec<MethodIndex>>,
    pub by_class_name: BTreeMap<(String, String), Vec<MethodIndex>>,
}

impl MethodRegistry {
    pub fn from_definitions(mut defs: Vec<MethodIdentity>) -> Self {
        defs.sort();
        let mut reg = Self::default();
        for d in defs {
            reg.intern(d);
        }
        reg
    }

    pub fn intern(&mut self, identity: MethodIdentity) -> MethodIndex {
        let idx = MethodIndex(self.methods.len() as u32);
        self.by_name.entry(identity.name.clone()).or_default().push(idx);
        if let Some(class) = identity.class.clone() {
            self.by_class_name.entry((class, identity.name.clone())).or_default().push(idx);
        }
        self.methods.push(identity);
        idx
    }

    pub fn get(&self, idx: MethodIndex) -> Option<&MethodIdentity> {
        self.methods.get(idx.0 as usize)
    }

    pub fn count(&self) -> usize {
        self.methods.len()
    }

    pub fn lookup_by_name(&self, name: &str) -> &[MethodIndex] {
        self.by_name.get(name).map_or(&[][..], Vec::as_slice)
    }

    pub fn lookup_by_class_and_name(&self, class: &str, name: &str) -> &[MethodIndex] {
        self.by_class_name.get(&(class.to_string(), name.to_string())).map_or(&[][..], Vec::as_slice)
    }
}

#[derive(Debug, Default)]
pub struct CallAdjacency {
    pub outgoing: Vec<Vec<CallEdge>>,
    pub incoming: Vec<Vec<CallEdge>>,
}

impl CallAdjacency {
    pub fn with_capacity(n: usize) -> Self {
        Self { outgoing: vec![Vec::new(); n], incoming: vec![Vec::new(); n] }
    }

    pub fn insert(&mut self, edge: CallEdge) {
        let s = edge.source.0 as usize;
        let t = edge.target.0 as usize;
        self.outgoing[s].push(edge.clone());
        self.incoming[t].push(edge);
    }

    pub fn outgoing(&self, idx: MethodIndex) -> &[CallEdge] {
        self.outgoing.get(idx.0 as usize).map_or(&[][..], Vec::as_slice)
    }

    pub fn incoming(&self, idx: MethodIndex) -> &[CallEdge] {
        self.incoming.get(idx.0 as usize).map_or(&[][..], Vec::as_slice)
    }
}

#[derive(Debug, Default)]
pub struct CallGraph {
    pub registry: MethodRegistry,
    pub adjacency: CallAdjacency,
}

impl CallGraph {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn build(definitions: Vec<DefinitionRecord>, calls: Vec<LocatedCall>) -> Self {
        let identities: Vec<MethodIdentity> = definitions.into_iter().map(|d| d.identity).collect();
        let registry = MethodRegistry::from_definitions(identities);
        let n = registry.count();
        let mut adjacency = CallAdjacency::with_capacity(n);
        for call in calls {
            insert_resolved(&registry, &call, &mut adjacency);
        }
        Self { registry, adjacency }
    }
}

fn insert_resolved(reg: &MethodRegistry, call: &LocatedCall, adj: &mut CallAdjacency) {
    let Some(caller) = call.caller.as_ref() else { return };
    let Some(caller_idx) = find_caller_index(reg, caller) else { return };
    let resolved = resolve_targets(reg, call, caller);
    for (target_idx, confidence) in resolved {
        adj.insert(CallEdge { source: caller_idx, target: target_idx, confidence });
    }
}

fn find_caller_index(reg: &MethodRegistry, caller: &MethodIdentity) -> Option<MethodIndex> {
    reg.methods.iter().enumerate().find(|(_, m)| *m == caller).map(|(i, _)| MethodIndex(i as u32))
}

fn resolve_targets(
    reg: &MethodRegistry,
    call: &LocatedCall,
    caller: &MethodIdentity,
) -> Vec<(MethodIndex, ImportConfidence)> {
    let receiver_class = effective_class(call, caller);
    if let Some(class) = receiver_class {
        let exact = reg.lookup_by_class_and_name(&class, &call.call.callee_name);
        if !exact.is_empty() {
            let conf = if exact.len() == 1 { ImportConfidence::High } else { ImportConfidence::Medium };
            return exact.iter().map(|i| (*i, conf)).collect();
        }
    }
    let by_name = reg.lookup_by_name(&call.call.callee_name);
    if by_name.is_empty() || distinct_target_classes(reg, by_name) > MAX_AMBIGUOUS_TARGET_CLASSES {
        return Vec::new();
    }
    let confidence = if by_name.len() == 1 { ImportConfidence::Medium } else { ImportConfidence::Low };
    by_name.iter().map(|i| (*i, confidence)).collect()
}

const MAX_AMBIGUOUS_TARGET_CLASSES: usize = 4;

fn distinct_target_classes(reg: &MethodRegistry, idxs: &[MethodIndex]) -> usize {
    let mut classes: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for idx in idxs {
        if let Some(class) = reg.get(*idx).and_then(|m| m.class.as_deref()) {
            classes.insert(class);
        }
    }
    classes.len()
}

fn effective_class(call: &LocatedCall, caller: &MethodIdentity) -> Option<String> {
    let hint = call.call.receiver_hint.as_deref()?;
    if matches!(hint, "self" | "this" | "cls") {
        return caller.class.clone();
    }
    Some(hint.to_string())
}
