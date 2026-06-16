use std::collections::HashSet;

use crate::thresholds::AuditThresholds;

use super::binding::BindingTable;
use super::call_graph::CallGraph;
use super::class_registry::{ClassIdentity, ClassIndex, ClassRegistry};
use super::definitions::DefinitionRecord;
use super::finding::{AuditFinding, AuditKind, ImportConfidence, RefusedBequestEvidence};

struct RbCtx<'a> {
    registry: &'a ClassRegistry,
    defs: &'a [DefinitionRecord],
    graph: &'a CallGraph,
    abstractness_of: &'a dyn Fn(&std::path::Path) -> Option<f64>,
    t: &'a AuditThresholds,
}

pub fn detect(
    registry: &ClassRegistry,
    defs: &[DefinitionRecord],
    graph: &CallGraph,
    abstractness_of: &dyn Fn(&std::path::Path) -> Option<f64>,
    t: &AuditThresholds,
) -> Vec<AuditFinding> {
    let ctx = RbCtx { registry, defs, graph, abstractness_of, t };
    let mut findings: Vec<AuditFinding> =
        (0..registry.count()).filter_map(|i| evaluate_class(&ctx, ClassIndex(i as u32))).collect();
    findings.sort_by_key(|f| std::cmp::Reverse(refused_count(f)));
    findings
}

fn evaluate_class(ctx: &RbCtx, sub_idx: ClassIndex) -> Option<AuditFinding> {
    let subclass = ctx.registry.get(sub_idx)?;
    subclass.parent_class.as_ref()?;
    let parents = ctx.registry.lookup_parent(sub_idx);
    let parent_idx = parents.first().copied()?;
    let parent = ctx.registry.get(parent_idx)?;
    if subclass.name == parent.name {
        return None;
    }
    if parent_is_abstract(parent, ctx.abstractness_of) {
        return None;
    }
    let parent_method_count = count_non_ctor_methods(ctx.registry, parent_idx, ctx.defs);
    let rt = &ctx.t.named_smells.refused_bequest;
    if parent_method_count < rt.min_parent_methods {
        return None;
    }
    let override_count = count_overrides(ctx.registry, ctx.defs, sub_idx, parent_idx);
    let override_ratio = f64::from(override_count) / f64::from(parent_method_count);
    let used = inherited_usage(ctx, sub_idx, parent_idx);
    if f64::from(used) / f64::from(parent_method_count) >= rt.max_override_ratio {
        return None;
    }
    Some(AuditFinding {
        kind: AuditKind::RefusedBequest(RefusedBequestEvidence {
            subclass_file: subclass.file.clone(),
            subclass_name: subclass.name.clone(),
            parent_file: parent.file.clone(),
            parent_name: parent.name.clone(),
            override_count,
            parent_method_count,
            override_ratio,
            confidence: ImportConfidence::Medium,
        }),
        representative_snippet: String::new(),
        support: parent_method_count.saturating_sub(override_count),
        file_count: parent_method_count,
        idf_score: None,
        action_label: None,
        pattern_category: None,
        locality_entropy: None,
        p_value: None,
        locations: Vec::new(),
    })
}

pub fn is_decorator_wrapper(e: &RefusedBequestEvidence, bindings: &BindingTable) -> bool {
    let mut supertypes = bindings.ancestors(&e.subclass_name);
    supertypes.push(e.parent_name.clone());
    let classes: Vec<&str> =
        std::iter::once(e.subclass_name.as_str()).chain(supertypes.iter().map(String::as_str)).collect();
    classes
        .iter()
        .any(|&cls| bindings.field_types_for_name(cls).iter().any(|ty| supertypes.iter().any(|s| s.as_str() == *ty)))
}

fn inherited_usage(ctx: &RbCtx, sub_idx: ClassIndex, parent_idx: ClassIndex) -> u32 {
    let parent_names = method_names(ctx.registry, ctx.defs, parent_idx);
    let sub_names = method_names(ctx.registry, ctx.defs, sub_idx);
    let mut used: HashSet<String> = parent_names.intersection(&sub_names).cloned().collect();
    let Some(parent) = ctx.registry.get(parent_idx) else {
        return used.len() as u32;
    };
    for m in ctx.registry.methods_in(sub_idx) {
        for edge in ctx.graph.adjacency.outgoing(*m) {
            let Some(target) = ctx.graph.registry.get(edge.target) else { continue };
            if target.class.as_deref() == Some(parent.name.as_str()) && parent_names.contains(&target.name) {
                used.insert(target.name.clone());
            }
        }
    }
    used.len() as u32
}

fn parent_is_abstract(parent: &ClassIdentity, abstractness_of: &dyn Fn(&std::path::Path) -> Option<f64>) -> bool {
    abstractness_of(&parent.file).is_some_and(|a| a >= 0.5)
}

fn count_non_ctor_methods(registry: &ClassRegistry, idx: ClassIndex, defs: &[DefinitionRecord]) -> u32 {
    let Some(class) = registry.get(idx) else { return 0 };
    let mut count: u32 = 0;
    for def in defs {
        if def.identity.class.as_deref() == Some(&class.name) && def.identity.file == class.file && !def.is_constructor
        {
            count = count.saturating_add(1);
        }
    }
    count
}

fn count_overrides(
    registry: &ClassRegistry,
    defs: &[DefinitionRecord],
    sub_idx: ClassIndex,
    parent_idx: ClassIndex,
) -> u32 {
    let sub_names = method_names(registry, defs, sub_idx);
    let parent_names = method_names(registry, defs, parent_idx);
    sub_names.intersection(&parent_names).count() as u32
}

fn method_names(registry: &ClassRegistry, defs: &[DefinitionRecord], idx: ClassIndex) -> HashSet<String> {
    let Some(class) = registry.get(idx) else { return HashSet::new() };
    let mut names = HashSet::new();
    for def in defs {
        if def.identity.class.as_deref() == Some(&class.name) && def.identity.file == class.file && !def.is_constructor
        {
            names.insert(def.identity.name.clone());
        }
    }
    names
}

fn refused_count(f: &AuditFinding) -> u32 {
    if let AuditKind::RefusedBequest(e) = &f.kind {
        e.parent_method_count.saturating_sub(e.override_count)
    } else {
        0
    }
}
