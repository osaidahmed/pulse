use std::collections::HashMap;

use crate::thresholds::AuditThresholds;

use super::call_graph::{CallGraph, MethodIdentity, MethodIndex};
use super::definitions::DefinitionRecord;
use super::finding::{AuditFinding, AuditKind, FeatureEnvyEvidence, ImportConfidence};

pub fn detect(
    defs: &[DefinitionRecord],
    graph: &CallGraph,
    t: &AuditThresholds,
) -> Vec<AuditFinding> {
    let mut findings = Vec::new();
    for (i, def) in defs.iter().enumerate() {
        if def.identity.class.is_none() {
            continue;
        }
        if let Some(idx) = method_index_for(graph, &def.identity) {
            if let Some(f) = evaluate_method(def, graph, idx, t) {
                findings.push(f);
            }
        }
        let _ = i;
    }
    findings.sort_by_key(|f| std::cmp::Reverse(envy_atfd(f)));
    findings
}

fn method_index_for(graph: &CallGraph, identity: &MethodIdentity) -> Option<MethodIndex> {
    graph
        .registry
        .methods
        .iter()
        .position(|m| m == identity)
        .map(|i| MethodIndex(i as u32))
}

fn evaluate_method(
    def: &DefinitionRecord,
    graph: &CallGraph,
    idx: MethodIndex,
    t: &AuditThresholds,
) -> Option<AuditFinding> {
    let atfd = distinct_foreign_targets(&def.foreign_field_accesses);
    let (intra, foreign) = count_intra_and_foreign(graph, idx, def.identity.class.as_deref());
    let total = intra.saturating_add(foreign);
    let ft = &t.named_smells.feature_envy;
    if total == 0 || atfd <= ft.atfd {
        return None;
    }
    let ratio = f64::from(foreign) / f64::from(total);
    if ratio <= ft.foreign_ratio {
        return None;
    }
    let envied = dominant_envied_class(&def.foreign_field_accesses);
    let evidence = FeatureEnvyEvidence {
        method_file: def.identity.file.clone(),
        method_class: def.identity.class.clone(),
        method_name: def.identity.name.clone(),
        method_line: def.identity.line,
        atfd,
        foreign_call_count: foreign,
        intra_call_count: intra,
        envied_class: envied,
        confidence: ImportConfidence::Medium,
    };
    Some(AuditFinding {
        kind: AuditKind::FeatureEnvy(evidence),
        representative_snippet: String::new(),
        support: foreign,
        file_count: atfd,
        idf_score: None,
        action_label: None,
        locations: Vec::new(),
    })
}

fn distinct_foreign_targets(foreign: &[(String, String)]) -> u32 {
    foreign.len() as u32
}

fn count_intra_and_foreign(
    graph: &CallGraph,
    idx: MethodIndex,
    self_class: Option<&str>,
) -> (u32, u32) {
    let outgoing = graph.adjacency.outgoing(idx);
    let mut intra = 0u32;
    let mut foreign = 0u32;
    for edge in outgoing {
        let Some(target) = graph.registry.get(edge.target) else { continue };
        if target.class.as_deref() == self_class {
            intra += 1;
        } else {
            foreign += 1;
        }
    }
    (intra, foreign)
}

fn dominant_envied_class(foreign: &[(String, String)]) -> Option<String> {
    let mut counts: HashMap<&str, u32> = HashMap::new();
    for (recv, _) in foreign {
        *counts.entry(recv.as_str()).or_insert(0) += 1;
    }
    counts
        .into_iter()
        .max_by_key(|(_, n)| *n)
        .map(|(k, _)| k.to_string())
}

fn envy_atfd(f: &AuditFinding) -> u32 {
    if let AuditKind::FeatureEnvy(e) = &f.kind {
        e.atfd
    } else {
        0
    }
}
