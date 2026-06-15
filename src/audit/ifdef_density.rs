use tree_sitter::Node;

use crate::parse::Language;
use crate::thresholds::AuditThresholds;
use crate::walk::DepthGuard;

use super::corpus::Corpus;
use super::deps_reconcile::wrap;
use super::finding::{AuditFinding, AuditKind, IfdefDensityEvidence, ImportConfidence};

pub fn run_from(corpus: &Corpus, thresholds: &AuditThresholds) -> Vec<AuditFinding> {
    let min = thresholds.ifdef_min_conditionals;
    let mut findings = Vec::new();
    for file in &corpus.files {
        if !matches!(file.lang, Language::C | Language::Cpp | Language::ObjectiveC) {
            continue;
        }
        let Some((_, tree)) = file.parsed() else { continue };
        let (count, first_line) = count_conditionals(tree.root_node());
        if count >= min {
            findings.push(wrap(
                file.path.to_string_lossy().into_owned(),
                AuditKind::IfdefDensity(IfdefDensityEvidence {
                    file: file.path.clone(),
                    line: first_line,
                    conditional_count: count,
                    confidence: ImportConfidence::Medium,
                }),
            ));
        }
    }
    findings.sort_by(|a, b| a.representative_snippet.cmp(&b.representative_snippet));
    findings
}

#[derive(Default)]
struct CondTally {
    count: u32,
    first_line: u32,
}

fn count_conditionals(node: Node) -> (u32, u32) {
    let mut tally = CondTally::default();
    walk_conditionals(node, &mut tally);
    (tally.count, tally.first_line)
}

fn walk_conditionals(node: Node, tally: &mut CondTally) {
    let Some(_g) = DepthGuard::enter() else { return };
    if node.kind().starts_with("preproc_if") {
        tally.count += 1;
        if tally.first_line == 0 {
            tally.first_line = node.start_position().row as u32 + 1;
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.is_named() {
            walk_conditionals(child, tally);
        }
    }
}
