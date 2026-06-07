use std::collections::HashSet;

use crate::thresholds::AuditThresholds;

use super::abstractness::abstractness_for_file;
use super::class_registry::{ClassIdentity, ClassIndex, ClassRegistry};
use super::definitions::DefinitionRecord;
use super::finding::{AuditFinding, AuditKind, ImportConfidence, RefusedBequestEvidence};

pub fn detect(
    registry: &ClassRegistry,
    defs: &[DefinitionRecord],
    file_lang: &dyn Fn(&std::path::Path) -> Option<crate::parse::Language>,
    t: &AuditThresholds,
) -> Vec<AuditFinding> {
    let mut findings = Vec::new();
    for i in 0..registry.count() {
        let idx = ClassIndex(i as u32);
        if let Some(f) = evaluate_class(registry, defs, idx, file_lang, t) {
            findings.push(f);
        }
    }
    findings.sort_by_key(|f| std::cmp::Reverse(refused_count(f)));
    findings
}

fn evaluate_class(
    registry: &ClassRegistry,
    defs: &[DefinitionRecord],
    sub_idx: ClassIndex,
    file_lang: &dyn Fn(&std::path::Path) -> Option<crate::parse::Language>,
    t: &AuditThresholds,
) -> Option<AuditFinding> {
    let subclass = registry.get(sub_idx)?;
    subclass.parent_class.as_ref()?;
    let parents = registry.lookup_parent(sub_idx);
    let parent_idx = parents.first().copied()?;
    let parent = registry.get(parent_idx)?;
    if subclass.name == parent.name {
        return None;
    }
    if parent_is_abstract(parent, file_lang) {
        return None;
    }
    let parent_method_count = count_non_ctor_methods(registry, parent_idx, defs);
    let rt = &t.named_smells.refused_bequest;
    if parent_method_count < rt.min_parent_methods {
        return None;
    }
    let override_count = count_overrides(registry, defs, sub_idx, parent_idx);
    let override_ratio = f64::from(override_count) / f64::from(parent_method_count);
    if override_ratio >= rt.max_override_ratio {
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

fn parent_is_abstract(
    parent: &ClassIdentity,
    file_lang: &dyn Fn(&std::path::Path) -> Option<crate::parse::Language>,
) -> bool {
    let Some(lang) = file_lang(&parent.file) else { return false };
    let record = abstractness_for_file(&parent.file, lang);
    record.abstractness >= 0.5
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
