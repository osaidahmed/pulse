use std::collections::{BTreeMap, HashSet};

use crate::thresholds::AuditThresholds;

use super::class_registry::{ClassIdentity, ClassIndex, ClassRegistry};
use super::finding::{
    AuditFinding, AuditKind, ClassIdentityRef, ImportConfidence, ParallelInheritanceEvidence,
};
use super::inheritance::{descendants_of, InheritanceGraph};

pub fn detect(
    registry: &ClassRegistry,
    inh: &InheritanceGraph,
    t: &AuditThresholds,
) -> Vec<AuditFinding> {
    let signatures = build_signature_map(registry, inh);
    let roots: Vec<ClassIndex> = signatures.keys().copied().collect();
    let mut findings = Vec::new();
    for i in 0..roots.len() {
        for j in (i + 1)..roots.len() {
            let a = roots[i];
            let b = roots[j];
            if let Some(f) = pair_finding(registry, &signatures, a, b, t) {
                findings.push(f);
            }
        }
    }
    findings.sort_by_key(|f| std::cmp::Reverse(match_count(f)));
    findings
}

type SignatureMap = BTreeMap<ClassIndex, Vec<(ClassIndex, String)>>;

fn build_signature_map(registry: &ClassRegistry, inh: &InheritanceGraph) -> SignatureMap {
    let mut map: SignatureMap = BTreeMap::new();
    for i in 0..registry.count() {
        let idx = ClassIndex(i as u32);
        if let Some(sigs) = signatures_for_root(registry, inh, idx) {
            map.insert(idx, sigs);
        }
    }
    map
}

fn signatures_for_root(
    registry: &ClassRegistry,
    inh: &InheritanceGraph,
    idx: ClassIndex,
) -> Option<Vec<(ClassIndex, String)>> {
    let descendants = descendants_of(inh, idx);
    if descendants.is_empty() {
        return None;
    }
    let root = registry.get(idx)?;
    let sigs: Vec<(ClassIndex, String)> = descendants
        .into_iter()
        .filter_map(|d| signature_for_descendant(registry, root, d))
        .collect();
    if sigs.is_empty() {
        return None;
    }
    Some(sigs)
}

fn signature_for_descendant(
    registry: &ClassRegistry,
    root: &ClassIdentity,
    d: ClassIndex,
) -> Option<(ClassIndex, String)> {
    let child = registry.get(d)?;
    let token = derive_token(&root.name, &child.name)?;
    Some((d, token))
}

fn pair_finding(
    registry: &ClassRegistry,
    signatures: &SignatureMap,
    a: ClassIndex,
    b: ClassIndex,
    t: &AuditThresholds,
) -> Option<AuditFinding> {
    let sigs_a = signatures.get(&a)?;
    let sigs_b = signatures.get(&b)?;
    let matched = matched_descendant_pairs(sigs_a, sigs_b, registry);
    if matched.len() < t.named_smells.parallel_inheritance.min_overlap as usize {
        return None;
    }
    let root_a = registry.get(a)?;
    let root_b = registry.get(b)?;
    let evidence = ParallelInheritanceEvidence {
        root_a: ClassIdentityRef { file: root_a.file.clone(), name: root_a.name.clone() },
        root_b: ClassIdentityRef { file: root_b.file.clone(), name: root_b.name.clone() },
        matched_descendants: matched,
        confidence: ImportConfidence::Medium,
    };
    Some(AuditFinding {
        kind: AuditKind::ParallelInheritance(evidence),
        representative_snippet: String::new(),
        support: 0,
        file_count: 0,
        idf_score: None,
        action_label: None,
        pattern_category: None,
        locations: Vec::new(),
    })
}

fn matched_descendant_pairs(
    sigs_a: &[(ClassIndex, String)],
    sigs_b: &[(ClassIndex, String)],
    registry: &ClassRegistry,
) -> Vec<(String, String)> {
    let tokens_b: HashSet<&str> = sigs_b.iter().map(|(_, t)| t.as_str()).collect();
    let mut out = Vec::new();
    for (a_idx, a_token) in sigs_a {
        if !tokens_b.contains(a_token.as_str()) {
            continue;
        }
        if let Some(pair) = find_matching_pair(*a_idx, a_token, sigs_b, registry) {
            out.push(pair);
        }
    }
    out
}

fn find_matching_pair(
    a_idx: ClassIndex,
    a_token: &str,
    sigs_b: &[(ClassIndex, String)],
    registry: &ClassRegistry,
) -> Option<(String, String)> {
    let a_class = registry.get(a_idx)?;
    for (b_idx, b_token) in sigs_b {
        if b_token != a_token {
            continue;
        }
        let b_class = registry.get(*b_idx)?;
        return Some((a_class.name.clone(), b_class.name.clone()));
    }
    None
}

fn derive_token(root_name: &str, child_name: &str) -> Option<String> {
    if let Some(stripped) = child_name.strip_suffix(root_name) {
        if !stripped.is_empty() {
            return Some(stripped.to_string());
        }
    }
    if let Some(stripped) = child_name.strip_prefix(root_name) {
        if !stripped.is_empty() {
            return Some(stripped.to_string());
        }
    }
    None
}

fn match_count(f: &AuditFinding) -> usize {
    if let AuditKind::ParallelInheritance(e) = &f.kind {
        e.matched_descendants.len()
    } else {
        0
    }
}

#[allow(dead_code)]
pub fn unused_class_workaround(_c: &ClassIdentity) {}
