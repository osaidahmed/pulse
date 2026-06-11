use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};

use crate::audit::finding::{AuditKind, ImportConfidence};
use crate::audit::graph::InputEdge;
use crate::audit::imports;
use crate::audit::martin::AbstractnessRecord;
use crate::audit::package_metrics::{run_from_edges, ModuleProfile};
use crate::parse::{self, Language};
use crate::thresholds::{AuditThresholds, Thresholds};

use super::finding::{CatalystEvidence, DecayEvidence, HistoryFinding, HistoryKind};
use super::git;

pub fn catalyst_findings(root: &Path, commits: &[git::Commit]) -> Vec<HistoryFinding> {
    let Some(baseline) = commits.last() else {
        return Vec::new();
    };
    let audit = Thresholds::default().audit;
    let before = cycle_members(root, &baseline.hash, &audit);
    cycle_members(root, "HEAD", &audit).into_iter().filter_map(|cycle| classify(&cycle, &before)).collect()
}

fn classify(cycle: &BTreeSet<PathBuf>, before: &BTreeSet<BTreeSet<PathBuf>>) -> Option<HistoryFinding> {
    if before.iter().all(|b| b.is_disjoint(cycle)) {
        return Some(history_finding(HistoryKind::CatalystWarning(CatalystEvidence { members: members_of(cycle) })));
    }
    let previous = before.iter().filter(|b| b.len() < cycle.len() && b.is_subset(cycle)).map(BTreeSet::len).max()?;
    Some(history_finding(HistoryKind::DecayTrend(DecayEvidence {
        members: members_of(cycle),
        previous_size: previous as u32,
        current_size: cycle.len() as u32,
    })))
}

fn history_finding(kind: HistoryKind) -> HistoryFinding {
    HistoryFinding { kind, action_label: None }
}

fn members_of(cycle: &BTreeSet<PathBuf>) -> Vec<PathBuf> {
    cycle.iter().cloned().collect()
}

fn cycle_members(root: &Path, rev: &str, audit: &AuditThresholds) -> BTreeSet<BTreeSet<PathBuf>> {
    run_from_edges(&edges_at_commit(root, rev), stub_profile, audit)
        .into_iter()
        .filter_map(|f| match f.kind {
            AuditKind::ImportCycle(c) => Some(c.members.into_iter().collect()),
            _ => None,
        })
        .collect()
}

fn stub_profile(_: &Path) -> ModuleProfile {
    ModuleProfile {
        abstractness: AbstractnessRecord { abstractness: None, confidence: ImportConfidence::BestEffort },
        import_confidence: ImportConfidence::BestEffort,
        loc: 0,
    }
}

pub fn edges_at_commit(root: &Path, rev: &str) -> Vec<InputEdge> {
    let typed = typed_files_at(root, rev);
    let typed_set: HashSet<PathBuf> = typed.iter().map(|(rel, _)| root.join(rel)).collect();
    let mut edges = Vec::new();
    for (rel, lang) in &typed {
        edges.extend(edges_for(root, rev, rel, *lang, &typed_set));
    }
    edges
}

fn typed_files_at(root: &Path, rev: &str) -> Vec<(PathBuf, Language)> {
    git::files_at_commit(root, rev)
        .into_iter()
        .filter_map(|p| parse::detect_language(&p).map(|lang| (p, lang)))
        .collect()
}

fn edges_for(root: &Path, rev: &str, rel: &Path, lang: Language, typed_set: &HashSet<PathBuf>) -> Vec<InputEdge> {
    let Some(source) = git::file_at_commit(root, rev, rel) else {
        return Vec::new();
    };
    let Some(tree) = parse::parse_guarded(&source, lang) else {
        return Vec::new();
    };
    let abs = root.join(rel);
    imports::extract_imports(&tree, &source, lang)
        .into_iter()
        .filter_map(|raw| imports::resolve_by_suffix(&raw.target, lang, typed_set))
        .map(|target| {
            let target_lang = parse::detect_language(&target).unwrap_or(lang);
            InputEdge { source: abs.clone(), target, source_lang: lang, target_lang }
        })
        .collect()
}
