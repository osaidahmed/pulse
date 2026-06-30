use std::collections::BTreeMap;

use pulse_thresholds::AuditThresholds;

use crate::call_graph::{CallGraph, MethodIdentity, MethodIndex};
use crate::definitions::DefinitionRecord;
use crate::finding::{AuditFinding, AuditKind, FeatureEnvyEvidence, ImportConfidence};

pub fn detect(defs: &[DefinitionRecord], graph: &CallGraph, t: &AuditThresholds) -> Vec<AuditFinding> {
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
    findings.sort_by_key(|f| (envied_unresolved(f), std::cmp::Reverse(envy_atfd(f))));
    findings
}

fn envied_unresolved(f: &AuditFinding) -> u8 {
    if let AuditKind::FeatureEnvy(e) = &f.kind {
        if e.envied_class.is_none() {
            return 1;
        }
    }
    0
}

fn method_index_for(graph: &CallGraph, identity: &MethodIdentity) -> Option<MethodIndex> {
    graph.registry.methods.iter().position(|m| m == identity).map(|i| MethodIndex(i as u32))
}

fn evaluate_method(
    def: &DefinitionRecord,
    graph: &CallGraph,
    idx: MethodIndex,
    t: &AuditThresholds,
) -> Option<AuditFinding> {
    let ft = &t.named_smells.feature_envy;
    let (atfd, envied, dominant) = named_provider_envy(&def.foreign_field_accesses);
    if atfd <= ft.atfd {
        return None;
    }
    if f64::from(dominant) / f64::from(atfd) < ft.locality {
        return None;
    }
    if envies_own_class(envied.as_deref(), def.identity.class.as_deref()) {
        return None;
    }
    let (intra, foreign) = count_intra_and_foreign(graph, idx, def.identity.class.as_deref());
    let total = intra.saturating_add(foreign);
    if total == 0 {
        return None;
    }
    let ratio = f64::from(foreign) / f64::from(total);
    if ratio <= ft.foreign_ratio {
        return None;
    }
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
        pattern_category: None,
        locality_entropy: None,
        p_value: None,
        locations: Vec::new(),
    })
}

fn envies_own_class(envied: Option<&str>, class: Option<&str>) -> bool {
    matches!(envied, Some("Self")) || (envied.is_some() && envied == class)
}

fn named_provider_envy(foreign: &[(String, String)]) -> (u32, Option<String>, u32) {
    let mut counts: BTreeMap<&str, u32> = BTreeMap::new();
    let mut atfd = 0u32;
    for (recv, _) in foreign {
        if recv == "?" {
            continue;
        }
        atfd += 1;
        *counts.entry(recv.as_str()).or_insert(0) += 1;
    }
    match counts.iter().max_by_key(|(k, n)| (**n, std::cmp::Reverse(**k))) {
        Some((name, count)) => (atfd, Some((*name).to_string()), *count),
        None => (atfd, None, 0),
    }
}

fn count_intra_and_foreign(graph: &CallGraph, idx: MethodIndex, self_class: Option<&str>) -> (u32, u32) {
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

fn envy_atfd(f: &AuditFinding) -> u32 {
    if let AuditKind::FeatureEnvy(e) = &f.kind {
        e.atfd
    } else {
        0
    }
}
