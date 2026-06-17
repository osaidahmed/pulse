use std::collections::HashSet;

use crate::thresholds::{AuditThresholds, ConceptualCohesionThresholds};

use super::call_graph::CallGraph;
use super::class_registry::{ClassIndex, ClassRegistry};
use super::conceptual_cohesion::{ConceptualCohesionEvidence, VocabModel};
use super::finding::{AuditFinding, AuditKind, ImportConfidence};
use super::method_vocab::VocabByMethod;

struct Analysis<'a> {
    registry: &'a ClassRegistry,
    graph: &'a CallGraph,
    vocab: &'a VocabByMethod,
    model: &'a VocabModel,
}

pub fn detect(
    registry: &ClassRegistry,
    graph: &CallGraph,
    vocab: &VocabByMethod,
    thresholds: &AuditThresholds,
) -> Vec<AuditFinding> {
    let t = thresholds.named_smells.conceptual_cohesion;
    if !t.enabled {
        return Vec::new();
    }
    let model = VocabModel::build(&vocab.values().cloned().collect::<Vec<_>>());
    let analysis = Analysis { registry, graph, vocab, model: &model };
    let mut findings: Vec<AuditFinding> =
        (0..registry.count()).filter_map(|i| evaluate_class(&analysis, ClassIndex(i as u32), &t)).collect();
    findings.sort_by(|a, b| cohesion_of(a).total_cmp(&cohesion_of(b)));
    findings.truncate(thresholds.named_smells.max_findings_reported);
    findings
}

fn evaluate_class(a: &Analysis, idx: ClassIndex, t: &ConceptualCohesionThresholds) -> Option<AuditFinding> {
    let class = a.registry.get(idx)?;
    let mut method_vocabs: Vec<&Vec<String>> = Vec::new();
    let mut class_line = u32::MAX;
    for m in a.registry.methods_in(idx) {
        let Some(id) = a.graph.registry.get(*m) else { continue };
        if let Some(words) = a.vocab.get(&(id.file.clone(), id.line)) {
            method_vocabs.push(words);
            class_line = class_line.min(id.line);
        }
    }
    if (method_vocabs.len() as u32) < t.min_methods {
        return None;
    }
    let distinct: HashSet<&str> = method_vocabs.iter().flat_map(|v| v.iter().map(String::as_str)).collect();
    if (distinct.len() as u32) < t.min_vocab {
        return None;
    }
    let cohesion = a.model.cohesion(&method_vocabs)?;
    if cohesion >= t.min_cohesion {
        return None;
    }
    Some(finding(ConceptualCohesionEvidence {
        class_file: class.file.clone(),
        class_line,
        class_name: class.name.clone(),
        cohesion,
        method_count: method_vocabs.len() as u32,
        confidence: ImportConfidence::Low,
    }))
}

fn finding(evidence: ConceptualCohesionEvidence) -> AuditFinding {
    AuditFinding {
        support: evidence.method_count,
        kind: AuditKind::LowConceptualCohesion(evidence),
        representative_snippet: String::new(),
        file_count: 1,
        idf_score: None,
        action_label: None,
        pattern_category: None,
        locality_entropy: None,
        p_value: None,
        locations: Vec::new(),
    }
}

fn cohesion_of(f: &AuditFinding) -> f64 {
    match &f.kind {
        AuditKind::LowConceptualCohesion(e) => e.cohesion,
        _ => 1.0,
    }
}
