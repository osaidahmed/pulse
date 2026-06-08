use std::path::{Path, PathBuf};

use tree_sitter::Node;

use crate::audit::finding::{AuditFinding, AuditKind, ImportConfidence, NaturalnessEvidence};
use crate::parse::{self, Language};
use crate::thresholds::{AuditThresholds, NaturalnessThresholds};
use crate::walk::{find_child_by_kind, node_text, DepthGuard};

mod model;
mod score;

use model::{Interner, NgramModel};

const MIN_CORPUS_FUNCTIONS: usize = 10;
const MAD_TO_Z: f64 = 0.6745;
const MEANAD_TO_Z: f64 = 0.7979;

struct FnDoc {
    file: PathBuf,
    line: u32,
    name: String,
    tokens: Vec<u32>,
}

pub fn run(typed_files: &[(PathBuf, Language)], thresholds: &AuditThresholds) -> Vec<AuditFinding> {
    let mut out = Vec::new();
    for lang in [Language::Python, Language::Rust] {
        run_language(typed_files, lang, &thresholds.naturalness, &mut out);
    }
    out
}

fn run_language(
    typed_files: &[(PathBuf, Language)],
    lang: Language,
    nthr: &NaturalnessThresholds,
    out: &mut Vec<AuditFinding>,
) {
    let Some(fn_kinds) = fn_kinds_for(lang) else { return };
    let mut interner = Interner::new();
    let docs = collect_docs(typed_files, lang, fn_kinds, &mut interner);
    if docs.len() < MIN_CORPUS_FUNCTIONS {
        return;
    }
    let token_seqs: Vec<Vec<u32>> = docs.iter().map(|d| d.tokens.clone()).collect();
    let ngram = model::train(&token_seqs, nthr.ngram_order as usize, interner.vocab());
    flag_outliers(&docs, &ngram, nthr, out);
}

fn fn_kinds_for(lang: Language) -> Option<&'static [&'static str]> {
    let kinds = crate::langkinds::function_kinds(lang);
    (!kinds.is_empty()).then_some(kinds)
}

fn collect_docs(
    typed_files: &[(PathBuf, Language)],
    lang: Language,
    fn_kinds: &[&str],
    interner: &mut Interner,
) -> Vec<FnDoc> {
    let mut docs = Vec::new();
    for (path, file_lang) in typed_files.iter().filter(|(_, l)| *l == lang) {
        let Ok(source) = std::fs::read_to_string(path) else { continue };
        let Some(tree) = parse::parse_guarded(&source, *file_lang) else { continue };
        let mut functions = Vec::new();
        find_functions(tree.root_node(), fn_kinds, &mut functions);
        for f in functions {
            push_doc(f, path, &source, interner, &mut docs);
        }
    }
    docs
}

fn find_functions<'a>(node: Node<'a>, fn_kinds: &[&str], out: &mut Vec<Node<'a>>) {
    let Some(_g) = DepthGuard::enter() else { return };
    if fn_kinds.contains(&node.kind()) {
        out.push(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        find_functions(child, fn_kinds, out);
    }
}

fn push_doc(f: Node, path: &Path, source: &str, interner: &mut Interner, docs: &mut Vec<FnDoc>) {
    let mut tokens = Vec::new();
    tokenize(f, source, interner, &mut tokens);
    if tokens.is_empty() {
        return;
    }
    let name = find_child_by_kind(f, "identifier")
        .map_or_else(|| "<anonymous>".to_string(), |n| node_text(n, source).to_string());
    docs.push(FnDoc { file: path.to_path_buf(), line: f.start_position().row as u32 + 1, name, tokens });
}

fn tokenize(node: Node, source: &str, interner: &mut Interner, out: &mut Vec<u32>) {
    let Some(_g) = DepthGuard::enter() else { return };
    if node.child_count() == 0 {
        let text = node_text(node, source);
        if !is_comment(node.kind()) && !text.is_empty() {
            out.push(interner.intern(text));
        }
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        tokenize(child, source, interner, out);
    }
}

fn is_comment(kind: &str) -> bool {
    kind.contains("comment")
}

fn flag_outliers(docs: &[FnDoc], model: &NgramModel, nthr: &NaturalnessThresholds, out: &mut Vec<AuditFinding>) {
    let min_tokens = nthr.min_fn_tokens as usize;
    let scored: Vec<(usize, f64)> = docs
        .iter()
        .enumerate()
        .filter(|(_, d)| d.tokens.len() >= min_tokens)
        .map(|(i, d)| (i, score::function_surprisal(&d.tokens, model, nthr)))
        .collect();
    if scored.len() < MIN_CORPUS_FUNCTIONS {
        return;
    }
    let values: Vec<f64> = scored.iter().map(|&(_, s)| s).collect();
    let med = median(&values);
    let (scale, spread) = robust_spread(&values, med);
    if spread <= 0.0 {
        return;
    }
    for &(i, surprisal) in &scored {
        let z = scale * (surprisal - med) / spread;
        if z > nthr.zscore_cutoff {
            out.push(build_finding(&docs[i], surprisal, z));
        }
    }
}

fn robust_spread(values: &[f64], med: f64) -> (f64, f64) {
    let mad = median_abs_dev(values, med);
    if mad > 0.0 {
        return (MAD_TO_Z, mad);
    }
    (MEANAD_TO_Z, mean_abs_dev(values, med))
}

fn mean_abs_dev(values: &[f64], med: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().map(|&x| (x - med).abs()).sum::<f64>() / values.len() as f64
}

fn median(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut v = values.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        f64::midpoint(v[n / 2 - 1], v[n / 2])
    }
}

fn median_abs_dev(values: &[f64], med: f64) -> f64 {
    let devs: Vec<f64> = values.iter().map(|&x| (x - med).abs()).collect();
    median(&devs)
}

fn build_finding(doc: &FnDoc, surprisal: f64, zscore: f64) -> AuditFinding {
    AuditFinding {
        kind: AuditKind::UnnaturalCode(NaturalnessEvidence {
            file: doc.file.clone(),
            function: doc.name.clone(),
            line: doc.line,
            surprisal,
            zscore,
            confidence: ImportConfidence::Low,
        }),
        representative_snippet: String::new(),
        support: 1,
        file_count: 1,
        idf_score: None,
        action_label: None,
        pattern_category: None,
        locality_entropy: None,
        p_value: None,
        locations: Vec::new(),
    }
}
