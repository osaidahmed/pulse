use std::path::{Path, PathBuf};

use tree_sitter::Node;

use crate::parse::{self, Language};

use super::binding::{self, ClassBinding, TypeEnv};
use super::call_graph::MethodIdentity;
use super::calls::{self, RawCall};

#[derive(Debug, Clone)]
pub struct LocatedCall {
    pub call: RawCall,
    pub caller: Option<MethodIdentity>,
    pub file: PathBuf,
}

#[derive(Debug, Default)]
pub struct CallWalkOut {
    pub calls: Vec<LocatedCall>,
    pub method_envs: Vec<(MethodIdentity, TypeEnv)>,
    pub classes: Vec<ClassBinding>,
}

const FUNCTION_KINDS: &[&str] = &[
    "function_definition",
    "function_declaration",
    "function_item",
    "method_declaration",
    "method_definition",
    "constructor_declaration",
    "function",
    "method",
    "fn_decl",
    "func_decl",
    "func_definition",
    "lambda_expression",
    "func_literal",
];

const CLASS_KINDS: &[&str] = &[
    "class_definition",
    "class_declaration",
    "class_specifier",
    "struct_specifier",
    "impl_item",
    "interface_declaration",
    "trait_item",
    "object_declaration",
    "struct_item",
    "enum_item",
    "type_definition",
    "type_spec",
];

pub fn calls_for_file(path: &Path, lang: Language) -> Vec<LocatedCall> {
    let Ok(source) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let Some(tree) = parse::parse_guarded(&source, lang) else {
        return Vec::new();
    };
    walk_tree(&tree, &source, lang, path).calls
}

pub fn calls_from(file: &super::corpus::CorpusFile) -> Vec<LocatedCall> {
    calls_and_bindings_from(file).calls
}

pub fn calls_and_bindings_from(file: &super::corpus::CorpusFile) -> CallWalkOut {
    let Some((source, tree)) = file.parsed() else {
        return CallWalkOut::default();
    };
    walk_tree(tree, source, file.lang, &file.path)
}

fn walk_tree(tree: &tree_sitter::Tree, source: &str, lang: Language, path: &Path) -> CallWalkOut {
    let ctx = CallCtx { source, lang, path };
    let mut out = CallWalkOut::default();
    let mut stack: Vec<EnclosingFrame> = Vec::new();
    visit(tree.root_node(), &ctx, &mut stack, &mut out);
    out
}

struct CallCtx<'a> {
    source: &'a str,
    lang: Language,
    path: &'a Path,
}

#[derive(Debug, Clone)]
struct EnclosingFrame {
    class: Option<String>,
    method: Option<MethodIdentity>,
}

fn visit(node: Node, ctx: &CallCtx, stack: &mut Vec<EnclosingFrame>, out: &mut CallWalkOut) {
    let pushed_class = maybe_push_class(node, ctx, stack, out);
    let pushed_method = maybe_push_method(node, ctx, stack, out);
    collect_calls_at(node, ctx, stack, &mut out.calls);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit(child, ctx, stack, out);
    }
    if pushed_method {
        stack.pop();
    }
    if pushed_class {
        stack.pop();
    }
}

fn collect_calls_at(node: Node, ctx: &CallCtx, stack: &[EnclosingFrame], out: &mut Vec<LocatedCall>) {
    let raw_calls = single_node_calls(node, ctx.source, ctx.lang);
    let caller = stack.iter().rev().find_map(|f| f.method.clone());
    for raw in raw_calls {
        out.push(LocatedCall { call: raw, caller: caller.clone(), file: ctx.path.to_path_buf() });
    }
}

fn single_node_calls(node: Node, source: &str, lang: Language) -> Vec<RawCall> {
    let dispatchers = calls::dispatchers_for_lang(lang);
    if dispatchers.is_empty() {
        return Vec::new();
    }
    calls::match_single(node, source, dispatchers).into_iter().collect()
}

fn maybe_push_class(node: Node, ctx: &CallCtx, stack: &mut Vec<EnclosingFrame>, out: &mut CallWalkOut) -> bool {
    if !CLASS_KINDS.contains(&node.kind()) {
        return false;
    }
    let class_name = identifier_in(node, ctx.source);
    if binding::supports(ctx.lang) {
        if let Some(name) = class_name.clone() {
            out.classes.push(ClassBinding {
                file: ctx.path.to_path_buf(),
                name,
                parents: binding::class_parents(node, ctx.source, ctx.lang),
                fields: binding::class_field_types(node, ctx.source, ctx.lang),
            });
        }
    }
    stack.push(EnclosingFrame { class: class_name, method: None });
    true
}

fn maybe_push_method(node: Node, ctx: &CallCtx, stack: &mut Vec<EnclosingFrame>, out: &mut CallWalkOut) -> bool {
    if !FUNCTION_KINDS.contains(&node.kind()) {
        return false;
    }
    let name = identifier_in(node, ctx.source).unwrap_or_else(|| "<anonymous>".to_string());
    let class =
        binding::caller_class(node, ctx.source, ctx.lang).or_else(|| stack.iter().rev().find_map(|f| f.class.clone()));
    let identity =
        MethodIdentity { file: ctx.path.to_path_buf(), class, name, line: node.start_position().row as u32 + 1 };
    if binding::supports(ctx.lang) {
        out.method_envs.push((identity.clone(), binding::method_var_types(node, ctx.source, ctx.lang)));
    }
    stack.push(EnclosingFrame { class: None, method: Some(identity) });
    true
}

fn identifier_in(node: Node, source: &str) -> Option<String> {
    if let Some(name) = node.child_by_field_name("name") {
        return Some(source[name.byte_range()].to_string());
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if matches!(
            child.kind(),
            "identifier" | "type_identifier" | "name" | "field_identifier" | "property_identifier"
        ) {
            return Some(source[child.byte_range()].to_string());
        }
    }
    None
}
