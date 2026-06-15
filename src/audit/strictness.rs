use std::path::Path;

use tree_sitter::Node;

use crate::parse::Language;
use crate::thresholds::AuditThresholds;
use crate::walk::{node_text, DepthGuard};

use super::corpus::Corpus;
use super::deps_reconcile::wrap;
use super::finding::{AuditFinding, AuditKind, ImportConfidence, StrictnessEvidence};

pub fn run_from(corpus: &Corpus, root: &Path, thresholds: &AuditThresholds) -> Vec<AuditFinding> {
    if !non_strict_tsconfig(root) {
        return Vec::new();
    }
    let min = thresholds.strictness.any_min;
    let mut findings = Vec::new();
    for file in &corpus.files {
        if file.lang != Language::TypeScript {
            continue;
        }
        let Some((source, tree)) = file.parsed() else { continue };
        let (count, first_line) = count_any(tree.root_node(), source);
        if count >= min {
            findings.push(wrap(
                file.path.to_string_lossy().into_owned(),
                AuditKind::StrictnessDebt(StrictnessEvidence {
                    file: file.path.clone(),
                    line: first_line,
                    any_count: count,
                    confidence: ImportConfidence::Medium,
                }),
            ));
        }
    }
    findings.sort_by(|a, b| a.representative_snippet.cmp(&b.representative_snippet));
    findings
}

fn non_strict_tsconfig(root: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(root.join("tsconfig.json")) else {
        return false;
    };
    json_bool(&text, "strict") != Some(true) && json_bool(&text, "noImplicitAny") != Some(true)
}

fn json_bool(text: &str, key: &str) -> Option<bool> {
    let needle = format!("\"{key}\"");
    let after = text.find(&needle).map(|p| &text[p + needle.len()..])?;
    let rest = after.trim_start().strip_prefix(':')?.trim_start();
    if rest.starts_with("true") {
        Some(true)
    } else if rest.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

#[derive(Default)]
struct AnyTally {
    count: u32,
    first_line: u32,
}

fn count_any(node: Node, source: &str) -> (u32, u32) {
    let mut tally = AnyTally::default();
    walk_any(node, source, &mut tally);
    (tally.count, tally.first_line)
}

fn walk_any(node: Node, source: &str, tally: &mut AnyTally) {
    let Some(_g) = DepthGuard::enter() else { return };
    if node.kind() == "predefined_type" && node_text(node, source) == "any" {
        tally.count += 1;
        if tally.first_line == 0 {
            tally.first_line = node.start_position().row as u32 + 1;
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.is_named() {
            walk_any(child, source, tally);
        }
    }
}
