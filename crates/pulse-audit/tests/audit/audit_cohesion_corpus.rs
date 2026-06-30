use std::fs;
use std::path::{Path, PathBuf};

use pulse_audit::call_graph::CallGraph;
use pulse_audit::class_registry::ClassRegistry;
use pulse_audit::corpus::Corpus;
use pulse_audit::definitions::definitions_from;
use pulse_audit::detector_conceptual_cohesion;
use pulse_audit::finding::AuditKind;
use pulse_audit::method_vocab::{method_vocab_from, VocabByMethod};
use pulse_audit::walk_typed_source_files;
use pulse_thresholds::Thresholds;

const LANGS: &[&str] = &["java", "kotlin", "csharp", "swift", "php", "typescript", "python", "ruby", "cpp"];

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key).ok().and_then(|s| s.parse().ok()).unwrap_or(default)
}

fn repos(corpus: &str, cap: usize) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for lang in LANGS {
        let Ok(entries) = fs::read_dir(Path::new(corpus).join(lang)) else { continue };
        for entry in entries.flatten().filter(|e| e.path().is_dir()) {
            out.push(entry.path());
        }
    }
    out.sort();
    out.truncate(cap);
    out
}

fn scan_repo(repo: &Path, file_cap: usize, audit: &pulse_thresholds::AuditThresholds) -> Vec<(String, f64, u32)> {
    let mut files = walk_typed_source_files(repo, false);
    files.truncate(file_cap);
    let corpus = Corpus::load(&files);
    let mut defs = Vec::new();
    let mut vocab = VocabByMethod::new();
    for file in &corpus.files {
        defs.extend(definitions_from(file));
        vocab.extend(method_vocab_from(file));
    }
    let graph = CallGraph::build(defs.clone(), Vec::new());
    let registry = ClassRegistry::from_definitions(&defs, &graph.registry);
    detector_conceptual_cohesion::detect(&registry, &graph, &vocab, audit)
        .into_iter()
        .filter_map(|f| match f.kind {
            AuditKind::LowConceptualCohesion(e) => Some((e.class_name, e.cohesion, e.method_count)),
            _ => None,
        })
        .collect()
}

#[test]
fn scan_cohesion_corpus() {
    let Ok(corpus) = std::env::var("SCAN_COHESION") else { return };
    let file_cap = env_usize("SCAN_COHESION_FILECAP", 250);
    let mut audit = Thresholds::default().audit;
    audit.named_smells.conceptual_cohesion.enabled = true;
    audit.named_smells.max_findings_reported = 1_000_000;
    if std::env::var("SCAN_COHESION_DIST").is_ok() {
        audit.named_smells.conceptual_cohesion.min_cohesion = 1.0;
    }
    let mut cohesions: Vec<f64> = Vec::new();
    eprintln!("=== COHESION CORPUS SCAN ===");
    for repo in repos(&corpus, env_usize("SCAN_COHESION_REPOS", 40)) {
        let hits = scan_repo(&repo, file_cap, &audit);
        let name = repo.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if !hits.is_empty() {
            eprintln!("REPO {name}: {} findings", hits.len());
            for (class, c3, m) in hits.iter().take(6) {
                eprintln!("  SAMPLE {class} c3={c3:.3} m={m}");
            }
        }
        cohesions.extend(hits.iter().map(|(_, c3, _)| *c3));
    }
    cohesions.sort_by(f64::total_cmp);
    eprintln!("TOTAL qualifying classes: {}", cohesions.len());
    let pct = |p: f64| cohesions.get(((cohesions.len() as f64 * p) as usize).min(cohesions.len().saturating_sub(1)));
    if !cohesions.is_empty() {
        eprintln!(
            "c3 p1/p2/p5/p10/p25/p50: {:.3}/{:.3}/{:.3}/{:.3}/{:.3}/{:.3}",
            pct(0.01).unwrap_or(&0.0),
            pct(0.02).unwrap_or(&0.0),
            pct(0.05).unwrap_or(&0.0),
            pct(0.10).unwrap_or(&0.0),
            pct(0.25).unwrap_or(&0.0),
            pct(0.50).unwrap_or(&0.0),
        );
        for thr in [0.06, 0.08, 0.10, 0.12, 0.15] {
            let n = cohesions.iter().filter(|&&c| c < thr).count();
            eprintln!("  threshold {thr:.2} -> {n} flagged ({:.1}%)", 100.0 * n as f64 / cohesions.len() as f64);
        }
    }
}
