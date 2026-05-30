use crate::thresholds::AuditThresholds;

use super::call_graph::CallGraph;
use super::class_registry::{class_cc, class_fanout, class_method_count, ClassIndex, ClassRegistry};
use super::finding::{AuditFinding, AuditKind, DivergentChangeEvidence, ImportConfidence};

pub fn detect(
    registry: &ClassRegistry,
    graph: &CallGraph,
    t: &AuditThresholds,
) -> Vec<AuditFinding> {
    let mut findings = Vec::new();
    for i in 0..registry.count() {
        let idx = ClassIndex(i as u32);
        if let Some(f) = evaluate_class(registry, graph, idx, t) {
            findings.push(f);
        }
    }
    findings.sort_by_key(|f| std::cmp::Reverse(ordering_key(f)));
    findings
}

fn evaluate_class(
    registry: &ClassRegistry,
    graph: &CallGraph,
    idx: ClassIndex,
    t: &AuditThresholds,
) -> Option<AuditFinding> {
    let class = registry.get(idx)?;
    let cc = class_cc(registry, graph, idx);
    let fanout = class_fanout(registry, graph, idx);
    let method_count = class_method_count(registry, idx);
    let dt = &t.named_smells.divergent_change;
    if cc <= dt.changing_classes || fanout <= dt.fanout || method_count <= dt.method_count {
        return None;
    }
    let evidence = DivergentChangeEvidence {
        class_file: class.file.clone(),
        class_name: class.name.clone(),
        changing_classes: cc,
        fanout,
        method_count,
        confidence: ImportConfidence::Medium,
    };
    Some(AuditFinding {
        kind: AuditKind::DivergentChange(evidence),
        representative_snippet: String::new(),
        support: cc,
        file_count: method_count,
        idf_score: None,
        action_label: None,
        pattern_category: None,
        locality_entropy: None,
        p_value: None,
        locations: Vec::new(),
    })
}

fn ordering_key(f: &AuditFinding) -> (u32, u32, String) {
    let AuditKind::DivergentChange(e) = &f.kind else {
        return (0, 0, String::new());
    };
    (e.changing_classes, e.fanout, format!("{}", e.class_file.display()))
}
