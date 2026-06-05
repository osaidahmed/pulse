use std::path::{Path, PathBuf};

use tree_sitter::Node;

use crate::parse::{self, Language};
use crate::thresholds::AuditThresholds;
use crate::walk::{find_child_by_kind, node_text, DepthGuard};

use super::finding::AuditFinding;

mod flow;
mod nodes;
mod state;

pub struct TaintLang {
    pub fn_kinds: &'static [&'static str],
    pub body_kind: &'static str,
    pub assign_kinds: &'static [&'static str],
    pub aug_kinds: &'static [&'static str],
    pub call_kinds: &'static [&'static str],
    pub sources: &'static [&'static str],
    pub sinks: &'static [&'static str],
    pub sanitizers: &'static [&'static str],
    pub opacity_kinds: &'static [&'static str],
    pub opacity_calls: &'static [&'static str],
}

#[derive(Clone, Copy)]
pub struct Caps {
    pub visit_cap: u32,
    pub max_depth: u32,
}

pub struct FileCtx<'a> {
    pub lang: &'a TaintLang,
    pub source: &'a str,
    pub path: &'a Path,
    pub caps: Caps,
}

const PYTHON: TaintLang = TaintLang {
    fn_kinds: &["function_definition"],
    body_kind: "block",
    assign_kinds: &["assignment", "augmented_assignment"],
    aug_kinds: &["augmented_assignment"],
    call_kinds: &["call"],
    sources: &["input", "raw_input", "getenv"],
    sinks: &["execute", "executemany", "executescript", "system", "popen", "eval", "exec"],
    sanitizers: &["escape", "quote", "sanitize", "clean", "shlex_quote"],
    opacity_kinds: &["dictionary_splat", "list_splat", "dictionary_splat_pattern", "list_splat_pattern"],
    opacity_calls: &["getattr", "setattr", "vars", "locals", "globals"],
};

const RUST: TaintLang = TaintLang {
    fn_kinds: &["function_item"],
    body_kind: "block",
    assign_kinds: &["let_declaration", "assignment_expression", "compound_assignment_expr"],
    aug_kinds: &["compound_assignment_expr"],
    call_kinds: &["call_expression"],
    sources: &["var", "args", "read_line", "read_to_string"],
    sinks: &["execute", "query", "query_unchecked", "batch_execute"],
    sanitizers: &["escape", "sanitize", "quote"],
    opacity_kinds: &["closure_expression"],
    opacity_calls: &["transmute"],
};

fn lang_for(lang: Language) -> Option<&'static TaintLang> {
    match lang {
        Language::Python => Some(&PYTHON),
        Language::Rust => Some(&RUST),
        _ => None,
    }
}

pub fn run(typed_files: &[(PathBuf, Language)], thresholds: &AuditThresholds) -> Vec<AuditFinding> {
    let mut out = Vec::new();
    for (path, lang) in typed_files {
        let Some(tl) = lang_for(*lang) else { continue };
        analyze_file(path, *lang, tl, thresholds, &mut out);
    }
    out.truncate(thresholds.taint.max_findings);
    out
}

fn analyze_file(
    path: &Path,
    lang: Language,
    tl: &TaintLang,
    thresholds: &AuditThresholds,
    out: &mut Vec<AuditFinding>,
) {
    let Ok(source) = std::fs::read_to_string(path) else { return };
    let Some(tree) = parse::parse_only(&source, lang) else { return };
    let ctx = FileCtx {
        lang: tl,
        source: &source,
        path,
        caps: Caps { visit_cap: thresholds.taint.visit_cap, max_depth: thresholds.taint.max_depth },
    };
    let mut functions = Vec::new();
    collect_functions(tree.root_node(), tl.fn_kinds, &mut functions);
    for f in functions {
        analyze_function(&ctx, f, out);
    }
}

fn collect_functions<'a>(node: Node<'a>, fn_kinds: &[&str], out: &mut Vec<Node<'a>>) {
    let Some(_g) = DepthGuard::enter() else { return };
    if fn_kinds.contains(&node.kind()) {
        out.push(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_functions(child, fn_kinds, out);
    }
}

fn analyze_function(ctx: &FileCtx, f: Node, out: &mut Vec<AuditFinding>) {
    let Some(body) = find_child_by_kind(f, ctx.lang.body_kind) else { return };
    let name = find_child_by_kind(f, "identifier")
        .map_or_else(|| "<anonymous>".to_string(), |n| node_text(n, ctx.source).to_string());
    let mut analyzer = flow::Analyzer::new(ctx, name);
    analyzer.visit(body);
    out.extend(analyzer.into_findings());
}
