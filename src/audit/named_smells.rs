use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::parse::Language;
use crate::thresholds::AuditThresholds;

use super::call_graph::{CallGraph, MethodIdentity, MethodIndex};
use super::call_walker::LocatedCall;
use super::class_registry::ClassRegistry;
use super::definitions::DefinitionRecord;
use super::finding::{AuditFinding, AuditKind, AuditLocation, ImportConfidence, ShotgunSurgeryEvidence};

pub fn run(typed_files: &[(PathBuf, Language)], root: &Path, thresholds: &AuditThresholds) -> Vec<AuditFinding> {
    run_from(&super::corpus::Corpus::load(typed_files), root, thresholds)
}

pub fn run_from(corpus: &super::corpus::Corpus, _root: &Path, thresholds: &AuditThresholds) -> Vec<AuditFinding> {
    let mut definitions = Vec::new();
    let mut calls = Vec::new();
    for file in &corpus.files {
        definitions.extend(super::definitions::definitions_from(file));
        calls.extend(super::call_walker::calls_from(file));
    }
    let graph = CallGraph::build(definitions.clone(), calls);
    let registry = ClassRegistry::from_definitions(&definitions, &graph.registry);
    let method_idx_lookup = super::detector_god_class::build_method_idx_lookup(&graph, &definitions);
    let inh = super::inheritance::build_inheritance_graph(&registry);
    let mut all = Vec::new();
    all.extend(detect_shotgun_surgery(&graph, thresholds));
    all.extend(super::detector_divergent_change::detect(&registry, &graph, thresholds));
    all.extend(super::detector_feature_envy::detect(&definitions, &graph, thresholds));
    all.extend(super::detector_god_class::detect(&registry, &definitions, &method_idx_lookup, thresholds));
    all.extend(super::detector_parallel_inheritance::detect(&registry, &inh, thresholds));
    let file_lang = |path: &std::path::Path| -> Option<Language> { corpus.get(path).map(|f| f.lang) };
    all.extend(super::detector_refused_bequest::detect(&registry, &definitions, &file_lang, thresholds));
    apply_named_confidence(&mut all, &file_lang);
    all
}

fn apply_named_confidence(findings: &mut [AuditFinding], file_lang: &impl Fn(&Path) -> Option<Language>) {
    for finding in findings {
        set_named_confidence(&mut finding.kind, file_lang);
    }
}

fn set_named_confidence(kind: &mut AuditKind, file_lang: &impl Fn(&Path) -> Option<Language>) {
    use super::confidence::{confidence_for_file, EvidenceQuality};
    match kind {
        AuditKind::DivergentChange(e) => {
            e.confidence = confidence_for_file(file_lang, &e.class_file, EvidenceQuality::NameKeyedUnique);
        }
        AuditKind::FeatureEnvy(e) => {
            e.confidence = confidence_for_file(file_lang, &e.method_file, EvidenceQuality::Heuristic);
        }
        AuditKind::GodClass(e) => {
            e.confidence = confidence_for_file(file_lang, &e.class_file, EvidenceQuality::NameKeyedAmbiguous);
        }
        AuditKind::ParallelInheritance(e) => {
            e.confidence = confidence_for_file(file_lang, &e.root_a.file, EvidenceQuality::Heuristic);
        }
        AuditKind::RefusedBequest(e) => {
            e.confidence = confidence_for_file(file_lang, &e.subclass_file, EvidenceQuality::NameKeyedUnique);
        }
        _ => {}
    }
}

fn detect_shotgun_surgery(graph: &CallGraph, t: &AuditThresholds) -> Vec<AuditFinding> {
    let mut findings: Vec<AuditFinding> = Vec::new();
    for idx in 0..graph.registry.count() {
        let method_idx = MethodIndex(idx as u32);
        if let Some(finding) = evaluate_method(graph, method_idx, t) {
            findings.push(finding);
        }
    }
    let folded = fold_name_collisions(findings);
    let mut findings = folded;
    findings.sort_by_key(|f| std::cmp::Reverse(ordering_key(f)));
    if findings.len() > t.named_smells.max_findings_reported {
        findings.truncate(t.named_smells.max_findings_reported);
    }
    findings
}

type CollisionKey = (String, Vec<(String, u32)>);

fn fold_name_collisions(findings: Vec<AuditFinding>) -> Vec<AuditFinding> {
    let mut groups: std::collections::HashMap<CollisionKey, Vec<AuditFinding>> = std::collections::HashMap::new();
    let mut order: Vec<CollisionKey> = Vec::new();
    for f in findings {
        let key = collision_key(&f);
        if !groups.contains_key(&key) {
            order.push(key.clone());
        }
        groups.entry(key).or_default().push(f);
    }
    let mut out: Vec<AuditFinding> = Vec::with_capacity(order.len());
    for key in order {
        let mut group = groups.remove(&key).expect("group present");
        if group.len() == 1 {
            out.push(group.remove(0));
        } else {
            out.push(merge_collision_group(group));
        }
    }
    out
}

fn collision_key(f: &AuditFinding) -> CollisionKey {
    let AuditKind::ShotgunSurgery(e) = &f.kind else {
        return (String::new(), Vec::new());
    };
    let mut callers: Vec<(String, u32)> =
        e.caller_samples.iter().map(|c| (c.file.display().to_string(), c.line)).collect();
    callers.sort();
    callers.dedup();
    (e.method_name.clone(), callers)
}

fn merge_collision_group(mut group: Vec<AuditFinding>) -> AuditFinding {
    let mut head = group.remove(0);
    let AuditKind::ShotgunSurgery(ref mut head_e) = head.kind else {
        return head;
    };
    head_e.name_collision_count = (group.len() as u32) + 1;
    for sibling in group {
        let AuditKind::ShotgunSurgery(e) = sibling.kind else { continue };
        head_e.additional_definitions.push(AuditLocation { file: e.method_file, line: e.method_line });
    }
    head
}

fn ordering_key(f: &AuditFinding) -> (u32, u32, String, u32) {
    let AuditKind::ShotgunSurgery(e) = &f.kind else {
        return (0, 0, String::new(), 0);
    };
    (e.changing_classes, e.changing_methods, format!("{}", e.method_file.display()), e.method_line)
}

#[derive(Debug, Clone, Copy)]
struct MethodMetrics {
    cc: u32,
    cm: u32,
    fanout: u32,
}

impl MethodMetrics {
    fn exceeds(self, t: &AuditThresholds) -> bool {
        self.cc > t.named_smells.shotgun_surgery.changing_classes
            && self.cm > t.named_smells.shotgun_surgery.changing_methods
            && self.fanout > t.named_smells.shotgun_surgery.fanout
    }
}

fn evaluate_method(graph: &CallGraph, idx: MethodIndex, t: &AuditThresholds) -> Option<AuditFinding> {
    let method = graph.registry.get(idx)?;
    let incoming = graph.adjacency.incoming(idx);
    let outgoing = graph.adjacency.outgoing(idx);
    let metrics = MethodMetrics {
        cc: count_changing_classes(graph, incoming),
        cm: incoming.len() as u32,
        fanout: count_fanout(outgoing),
    };
    if !metrics.exceeds(t) {
        return None;
    }
    let confidence = aggregate_confidence(incoming);
    let caller_samples = collect_caller_samples(graph, incoming, t);
    let evidence = ShotgunSurgeryEvidence {
        method_file: method.file.clone(),
        method_class: method.class.clone(),
        method_name: method.name.clone(),
        method_line: method.line,
        changing_classes: metrics.cc,
        changing_methods: metrics.cm,
        fanout: metrics.fanout,
        confidence,
        caller_samples,
        name_collision_count: 0,
        additional_definitions: Vec::new(),
    };
    Some(AuditFinding {
        kind: AuditKind::ShotgunSurgery(evidence),
        representative_snippet: String::new(),
        support: metrics.cm,
        file_count: metrics.cc,
        idf_score: None,
        action_label: None,
        pattern_category: None,
        locality_entropy: None,
        p_value: None,
        locations: Vec::new(),
    })
}

fn count_changing_classes(graph: &CallGraph, incoming: &[super::call_graph::CallEdge]) -> u32 {
    distinct_count(incoming, |edge| graph.registry.get(edge.source).map(caller_bucket))
}

fn count_fanout(outgoing: &[super::call_graph::CallEdge]) -> u32 {
    distinct_count(outgoing, |edge| Some(edge.target.0))
}

fn distinct_count<T, F>(edges: &[super::call_graph::CallEdge], key: F) -> u32
where
    T: std::hash::Hash + Eq,
    F: Fn(&super::call_graph::CallEdge) -> Option<T>,
{
    let mut seen: HashSet<T> = HashSet::new();
    for edge in edges {
        if let Some(k) = key(edge) {
            seen.insert(k);
        }
    }
    seen.len() as u32
}

fn caller_bucket(caller: &MethodIdentity) -> String {
    match &caller.class {
        Some(cls) => format!("{}::{}", caller.file.display(), cls),
        None => format!("__free::{}", caller.file.display()),
    }
}

fn aggregate_confidence(incoming: &[super::call_graph::CallEdge]) -> ImportConfidence {
    let mut acc: Option<ImportConfidence> = None;
    for edge in incoming {
        acc = Some(acc.map_or(edge.confidence, |c| c.min(edge.confidence)));
    }
    acc.unwrap_or(ImportConfidence::Medium)
}

fn collect_caller_samples(
    graph: &CallGraph,
    incoming: &[super::call_graph::CallEdge],
    t: &AuditThresholds,
) -> Vec<AuditLocation> {
    let mut samples: Vec<AuditLocation> = Vec::new();
    for edge in incoming.iter().take(t.named_smells.max_caller_samples_per_finding) {
        if let Some(caller) = graph.registry.get(edge.source) {
            samples.push(AuditLocation { file: caller.file.clone(), line: caller.line });
        }
    }
    samples
}

pub fn run_from_inputs(
    definitions: Vec<DefinitionRecord>,
    calls: Vec<LocatedCall>,
    thresholds: &AuditThresholds,
) -> Vec<AuditFinding> {
    let graph = CallGraph::build(definitions, calls);
    detect_shotgun_surgery(&graph, thresholds)
}
