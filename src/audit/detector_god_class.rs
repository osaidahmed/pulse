use std::collections::HashMap;

use crate::thresholds::AuditThresholds;

use super::call_graph::MethodIndex;
use super::class_registry::{
    class_atfd, class_method_count, class_tcc, class_wmc, ClassIndex, ClassRegistry,
};
use super::definitions::DefinitionRecord;
use super::finding::{AuditFinding, AuditKind, GodClassEvidence, ImportConfidence};

pub fn detect(
    registry: &ClassRegistry,
    defs: &[DefinitionRecord],
    method_idx_lookup: &HashMap<MethodIndex, usize>,
    t: &AuditThresholds,
) -> Vec<AuditFinding> {
    let mut findings = Vec::new();
    for i in 0..registry.count() {
        let idx = ClassIndex(i as u32);
        if let Some(f) = evaluate_class(registry, defs, method_idx_lookup, idx, t) {
            findings.push(f);
        }
    }
    findings.sort_by_key(|f| std::cmp::Reverse(god_wmc(f)));
    findings
}

fn evaluate_class(
    registry: &ClassRegistry,
    defs: &[DefinitionRecord],
    method_idx_lookup: &HashMap<MethodIndex, usize>,
    idx: ClassIndex,
    t: &AuditThresholds,
) -> Option<AuditFinding> {
    let class = registry.get(idx)?;
    let cc_lookup = |m: MethodIndex| -> u32 {
        method_idx_lookup
            .get(&m)
            .and_then(|i| defs.get(*i))
            .map_or(0, |d| d.cc)
    };
    let wmc = class_wmc(registry, idx, &cc_lookup);
    let foreign_lookup = |m: MethodIndex| -> Vec<(String, String)> {
        method_idx_lookup
            .get(&m)
            .and_then(|i| defs.get(*i))
            .map(|d| d.foreign_field_accesses.clone())
            .unwrap_or_default()
    };
    let atfd = class_atfd(registry, idx, &foreign_lookup);
    let fields_lookup = |m: MethodIndex| -> (Vec<String>, bool) {
        method_idx_lookup
            .get(&m)
            .and_then(|i| defs.get(*i))
            .map(|d| (d.field_accesses.clone(), d.is_constructor))
            .unwrap_or_default()
    };
    let tcc = class_tcc(registry, idx, &fields_lookup);
    let method_count = class_method_count(registry, idx);
    let gt = &t.named_smells.god_class;
    if method_count < 3 {
        return None;
    }
    if wmc <= gt.wmc || tcc >= gt.tcc || atfd <= gt.atfd {
        return None;
    }
    let evidence = GodClassEvidence {
        class_file: class.file.clone(),
        class_name: class.name.clone(),
        wmc,
        tcc,
        atfd,
        method_count,
        confidence: ImportConfidence::Medium,
    };
    Some(AuditFinding {
        kind: AuditKind::GodClass(evidence),
        representative_snippet: String::new(),
        support: wmc,
        file_count: method_count,
        idf_score: None,
        action_label: None,
        pattern_category: None,
        locations: Vec::new(),
    })
}

fn god_wmc(f: &AuditFinding) -> u32 {
    if let AuditKind::GodClass(e) = &f.kind {
        e.wmc
    } else {
        0
    }
}

pub fn build_method_idx_lookup(
    graph: &super::call_graph::CallGraph,
    defs: &[DefinitionRecord],
) -> HashMap<MethodIndex, usize> {
    let mut map = HashMap::new();
    for (i, def) in defs.iter().enumerate() {
        if let Some(j) = graph
            .registry
            .methods
            .iter()
            .position(|m| m == &def.identity)
        {
            map.insert(MethodIndex(j as u32), i);
        }
    }
    map
}
