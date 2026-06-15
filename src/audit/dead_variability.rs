use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use tree_sitter::Node;

use crate::buildmeta;
use crate::parse::Language;
use crate::thresholds::AuditThresholds;
use crate::walk::{node_text, DepthGuard};

use super::corpus::Corpus;
use super::deps_reconcile::wrap;
use super::finding::{AuditFinding, AuditKind, DeadBranchEvidence, ImportConfidence};

pub fn run_from(corpus: &Corpus, root: &Path, thresholds: &AuditThresholds) -> Vec<AuditFinding> {
    let Some(db) = buildmeta::compile_db(root) else { return Vec::new() };
    let defines: HashMap<PathBuf, HashSet<String>> =
        db.entries.into_iter().map(|e| (e.file, e.defines.into_iter().collect())).collect();
    let mut findings = Vec::new();
    for file in &corpus.files {
        if !matches!(file.lang, Language::C | Language::Cpp | Language::ObjectiveC) {
            continue;
        }
        let Some(macros) = defines.get(&file.path).filter(|m| !m.is_empty()) else { continue };
        let Some((source, tree)) = file.parsed() else { continue };
        let undefs = undef_macros(source);
        let eligible: HashSet<&str> = macros.iter().map(String::as_str).filter(|m| !undefs.contains(*m)).collect();
        if eligible.is_empty() {
            continue;
        }
        collect_dead_branches(tree.root_node(), source, &eligible, &file.path, &mut findings);
    }
    findings.sort_by(|a, b| a.representative_snippet.cmp(&b.representative_snippet));
    findings.truncate(thresholds.dead_variability_max_findings as usize);
    findings
}

fn undef_macros(source: &str) -> HashSet<String> {
    source
        .lines()
        .filter_map(|line| line.trim_start().strip_prefix("#undef"))
        .filter_map(|rest| rest.split_whitespace().next())
        .map(String::from)
        .collect()
}

fn collect_dead_branches(node: Node, source: &str, eligible: &HashSet<&str>, path: &Path, out: &mut Vec<AuditFinding>) {
    let Some(_g) = DepthGuard::enter() else { return };
    if node.kind() == "preproc_ifdef" {
        if let Some(finding) = dead_branch_finding(node, source, eligible, path) {
            out.push(finding);
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.is_named() {
            collect_dead_branches(child, source, eligible, path, out);
        }
    }
}

fn dead_branch_finding(node: Node, source: &str, eligible: &HashSet<&str>, path: &Path) -> Option<AuditFinding> {
    let macro_name = node_text(node.child_by_field_name("name")?, source);
    if !eligible.contains(macro_name) {
        return None;
    }
    let directive = node.child(0).map_or("", |c| node_text(c, source));
    let dead_line = match directive {
        "#ifndef" => node.start_position().row as u32 + 1,
        "#ifdef" => node.child_by_field_name("alternative")?.start_position().row as u32 + 1,
        _ => return None,
    };
    Some(wrap(
        path.to_string_lossy().into_owned(),
        AuditKind::DeadConditionalBranch(DeadBranchEvidence {
            file: path.to_path_buf(),
            line: dead_line,
            macro_name: macro_name.to_string(),
            confidence: ImportConfidence::Medium,
        }),
    ))
}
