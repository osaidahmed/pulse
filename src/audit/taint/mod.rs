use std::path::{Path, PathBuf};

use tree_sitter::Node;

use crate::parse::Language;
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
    pub prop_sources: &'static [&'static str],
    pub source_kinds: &'static [&'static str],
    pub ident_kinds: &'static [&'static str],
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
    prop_sources: &[],
    source_kinds: &[],
    ident_kinds: &["identifier"],
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
    prop_sources: &[],
    source_kinds: &[],
    ident_kinds: &["identifier"],
    sinks: &["execute", "query", "query_unchecked", "batch_execute"],
    sanitizers: &["escape", "sanitize", "quote"],
    opacity_kinds: &["closure_expression"],
    opacity_calls: &["transmute"],
};

const JAVA: TaintLang = TaintLang {
    fn_kinds: &["method_declaration", "constructor_declaration"],
    body_kind: "block",
    assign_kinds: &["assignment_expression", "variable_declarator"],
    aug_kinds: &[],
    call_kinds: &["method_invocation"],
    sources: &[
        "getParameter",
        "getParameterValues",
        "getHeader",
        "getQueryString",
        "getCookies",
        "nextLine",
        "readLine",
    ],
    prop_sources: &[],
    source_kinds: &[],
    ident_kinds: &["identifier"],
    sinks: &["executeQuery", "executeUpdate", "execute", "exec", "eval"],
    sanitizers: &["escapeHtml", "escapeSql", "encode", "quote", "escape"],
    opacity_kinds: &["lambda_expression"],
    opacity_calls: &["invoke", "getMethod", "newInstance"],
};

const GO: TaintLang = TaintLang {
    fn_kinds: &["function_declaration", "method_declaration"],
    body_kind: "block",
    assign_kinds: &["short_var_declaration", "assignment_statement", "var_spec"],
    aug_kinds: &[],
    call_kinds: &["call_expression"],
    sources: &["FormValue", "PostFormValue", "Param"],
    prop_sources: &[],
    source_kinds: &[],
    ident_kinds: &["identifier"],
    sinks: &["Query", "QueryRow", "Exec", "QueryContext", "ExecContext", "Command"],
    sanitizers: &["EscapeString", "QuoteIdentifier", "HTMLEscapeString", "Escape"],
    opacity_kinds: &["func_literal"],
    opacity_calls: &["ValueOf"],
};

const TYPESCRIPT: TaintLang = TaintLang {
    fn_kinds: &["function_declaration", "method_definition", "arrow_function", "function_expression"],
    body_kind: "statement_block",
    assign_kinds: &["assignment_expression", "augmented_assignment_expression", "variable_declarator"],
    aug_kinds: &["augmented_assignment_expression"],
    call_kinds: &["call_expression"],
    sources: &[],
    prop_sources: &["req.query", "req.body", "req.params", "request.query", "request.body", "request.params"],
    source_kinds: &["member_expression"],
    ident_kinds: &["identifier"],
    sinks: &["query", "execute", "exec", "eval"],
    sanitizers: &["escape", "encode", "sanitize", "escapeHtml", "encodeURIComponent"],
    opacity_kinds: &["arrow_function", "function_expression"],
    opacity_calls: &[],
};

const JAVASCRIPT: TaintLang = TYPESCRIPT;

const CSHARP: TaintLang = TaintLang {
    fn_kinds: &["method_declaration", "constructor_declaration", "local_function_statement"],
    body_kind: "block",
    assign_kinds: &["assignment_expression", "variable_declarator"],
    aug_kinds: &[],
    call_kinds: &["invocation_expression"],
    sources: &[],
    prop_sources: &["Request.Form", "Request.QueryString", "Request.Params", "Request.Query", "Request.Headers"],
    source_kinds: &["member_access_expression"],
    ident_kinds: &["identifier"],
    sinks: &["ExecuteReader", "ExecuteNonQuery", "ExecuteScalar", "ExecuteQuery", "Execute"],
    sanitizers: &["Encode", "HtmlEncode", "Escape"],
    opacity_kinds: &["lambda_expression"],
    opacity_calls: &["Invoke", "GetMethod"],
};

const PHP: TaintLang = TaintLang {
    fn_kinds: &["function_definition", "method_declaration"],
    body_kind: "compound_statement",
    assign_kinds: &["assignment_expression", "augmented_assignment_expression"],
    aug_kinds: &["augmented_assignment_expression"],
    call_kinds: &["function_call_expression", "member_call_expression", "scoped_call_expression"],
    sources: &[],
    prop_sources: &["$_GET", "$_POST", "$_REQUEST", "$_COOKIE"],
    source_kinds: &["variable_name"],
    ident_kinds: &["identifier", "variable_name"],
    sinks: &["query", "mysqli_query", "mysql_query", "exec", "system", "shell_exec", "eval", "passthru"],
    sanitizers: &[
        "mysqli_real_escape_string",
        "real_escape_string",
        "htmlspecialchars",
        "htmlentities",
        "addslashes",
        "escapeshellarg",
        "intval",
    ],
    opacity_kinds: &["dynamic_variable_name"],
    opacity_calls: &["extract", "call_user_func", "compact"],
};

const C: TaintLang = TaintLang {
    fn_kinds: &["function_definition"],
    body_kind: "compound_statement",
    assign_kinds: &["assignment_expression", "init_declarator"],
    aug_kinds: &[],
    call_kinds: &["call_expression"],
    sources: &["getenv", "fgets", "gets"],
    prop_sources: &[],
    source_kinds: &[],
    ident_kinds: &["identifier"],
    sinks: &["system", "popen", "execl", "execlp", "execle", "execv", "execvp", "execve"],
    sanitizers: &["escape", "sanitize", "quote", "escapeshell"],
    opacity_kinds: &[],
    opacity_calls: &[],
};

const CPP: TaintLang = C;

const RUBY: TaintLang = TaintLang {
    fn_kinds: &["method", "singleton_method"],
    body_kind: "body_statement",
    assign_kinds: &["assignment", "operator_assignment"],
    aug_kinds: &["operator_assignment"],
    call_kinds: &["call"],
    sources: &["params", "gets", "cookies"],
    prop_sources: &["params", "gets", "cookies"],
    source_kinds: &["identifier"],
    ident_kinds: &["identifier"],
    sinks: &["execute", "exec", "system", "eval", "popen", "spawn"],
    sanitizers: &["sanitize", "escape", "quote", "sanitize_sql", "escape_html"],
    opacity_kinds: &["block", "do_block"],
    opacity_calls: &["send", "public_send", "instance_eval", "instance_exec"],
};

const KOTLIN: TaintLang = TaintLang {
    fn_kinds: &["function_declaration"],
    body_kind: "function_body",
    assign_kinds: &["property_declaration", "assignment"],
    aug_kinds: &[],
    call_kinds: &["call_expression"],
    sources: &["getParameter", "getHeader", "getQueryParameter", "readLine", "getenv"],
    prop_sources: &[],
    source_kinds: &[],
    ident_kinds: &["identifier"],
    sinks: &["executeQuery", "executeUpdate", "execute", "exec", "rawQuery", "compileStatement", "eval"],
    sanitizers: &["escape", "encode", "sanitize", "escapeHtml", "encodeForSql"],
    opacity_kinds: &["lambda_literal", "anonymous_function"],
    opacity_calls: &[],
};

fn lang_for(lang: Language) -> Option<&'static TaintLang> {
    const TABLE: &[(Language, &TaintLang)] = &[
        (Language::Python, &PYTHON),
        (Language::Rust, &RUST),
        (Language::Java, &JAVA),
        (Language::Go, &GO),
        (Language::TypeScript, &TYPESCRIPT),
        (Language::JavaScript, &JAVASCRIPT),
        (Language::CSharp, &CSHARP),
        (Language::Php, &PHP),
        (Language::C, &C),
        (Language::Cpp, &CPP),
        (Language::Ruby, &RUBY),
        (Language::Kotlin, &KOTLIN),
    ];
    TABLE.iter().find(|(l, _)| *l == lang).map(|(_, tl)| *tl)
}

pub fn covers(lang: Language) -> bool {
    lang_for(lang).is_some()
}

pub fn run(typed_files: &[(PathBuf, Language)], thresholds: &AuditThresholds) -> Vec<AuditFinding> {
    run_from(&super::corpus::Corpus::load(typed_files), thresholds)
}

pub fn run_from(corpus: &super::corpus::Corpus, thresholds: &AuditThresholds) -> Vec<AuditFinding> {
    use rayon::prelude::*;
    let per_file: Vec<Vec<AuditFinding>> = corpus
        .files
        .par_iter()
        .map(|file| {
            let mut out = Vec::new();
            if let Some(tl) = lang_for(file.lang) {
                analyze_file(file, tl, thresholds, &mut out);
            }
            out
        })
        .collect();
    let mut out: Vec<AuditFinding> = per_file.into_iter().flatten().collect();
    out.truncate(thresholds.taint.max_findings);
    out
}

fn analyze_file(
    file: &super::corpus::CorpusFile,
    tl: &TaintLang,
    thresholds: &AuditThresholds,
    out: &mut Vec<AuditFinding>,
) {
    let path = file.path.as_path();
    let Some((source, tree)) = file.parsed() else { return };
    let ctx = FileCtx {
        lang: tl,
        source,
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
        .or_else(|| find_child_by_kind(f, "name"))
        .map_or_else(|| "<anonymous>".to_string(), |n| node_text(n, ctx.source).to_string());
    let mut analyzer = flow::Analyzer::new(ctx, name);
    analyzer.visit(body);
    out.extend(analyzer.into_findings());
}
