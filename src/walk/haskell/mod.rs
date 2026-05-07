mod analysis;
mod complexity;
mod walk_tree;

use tree_sitter::{Node, Tree};

use super::shared::{self, GlobalMetricsConfig};
use super::{count_code_lines, find_child_by_kind, node_text, FileMetrics, FunctionMetrics, ModuleMetrics};

use analysis::{analyze_bind, analyze_function_group};

const COMMENT_PREFIXES: &[&str] = &["--", "{-", "*"];
const NESTING_BRANCH_KINDS: &[&str] = &["conditional", "case"];
const GLOBAL_CFG: GlobalMetricsConfig = GlobalMetricsConfig {
    cond: &["conditional"],
    loops: &[],
    branches: NESTING_BRANCH_KINDS,
    recurse: &[],
};

pub fn walk(tree: &Tree, source: &str) -> FileMetrics {
    let root = tree.root_node();
    let total_loc = count_code_lines(source, COMMENT_PREFIXES);
    let mut functions = Vec::new();
    let mut gcond: u32 = 0;
    let mut gnest: u32 = 0;

    let decls = find_child_by_kind(root, "declarations").unwrap_or(root);
    collect_functions(decls, source, &mut functions, "");
    shared::collect_global_metrics(root, &mut gcond, &mut gnest, &GLOBAL_CFG);

    let module = ModuleMetrics {
        total_loc,
        total_functions: functions.len() as u32,
        sum_cc: functions.iter().map(|f| f.cc).sum(),
        global_conditional_count: gcond,
        global_max_nesting: gnest,
        declaration_count: count_declarations(decls),
        struct_fields: Vec::new(),
    };
    FileMetrics { functions, module }
}

fn count_declarations(decls: Node) -> u32 {
    let mut cursor = decls.walk();
    decls
        .children(&mut cursor)
        .filter(|c| matches!(c.kind(), "data_type" | "newtype" | "class" | "type_synomym"))
        .count() as u32
}

pub(crate) fn extract_name(node: Node, source: &str) -> String {
    let mut cursor = node.walk();
    let result = node.children(&mut cursor)
        .find(|c| c.kind() == "variable")
        .map_or_else(|| "<anonymous>".into(), |n| node_text(n, source).to_string());
    result
}

fn prefixed(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_string()
    } else {
        format!("{prefix}.{name}")
    }
}

fn collect_functions(node: Node, source: &str, fns: &mut Vec<FunctionMetrics>, prefix: &str) {
    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();
    let mut i = 0;
    let cap = children.len().saturating_add(1);
    let mut steps = 0usize;
    while i < children.len() && steps < cap {
        match children[i].kind() {
            "function" => {
                let group = gather_equation_group(&children, &mut i, source);
                let name = prefixed(prefix, &extract_name(group[0], source));
                let m = analyze_function_group(&group, source, &name);
                emit_analyzed(&group, m, source, fns, &name);
            }
            "bind" => {
                let node = children[i];
                let name = prefixed(prefix, &extract_name(node, source));
                let m = analyze_bind(node, source, &name);
                emit_analyzed(&[node], m, source, fns, &name);
            }
            "class" => emit_typed_methods(children[i], source, fns, "class_declarations"),
            "instance" => emit_typed_methods(children[i], source, fns, "instance_declarations"),
            _ => {}
        }
        i += 1;
        steps += 1;
    }
}

fn gather_equation_group<'a>(children: &[Node<'a>], i: &mut usize, source: &str) -> Vec<Node<'a>> {
    let name = extract_name(children[*i], source);
    let mut group = vec![children[*i]];
    let cap = children.len().saturating_add(1);
    let mut steps = 0usize;
    while steps < cap
        && *i + 1 < children.len()
        && children[*i + 1].kind() == "function"
        && extract_name(children[*i + 1], source) == name
    {
        *i += 1;
        group.push(children[*i]);
        steps += 1;
    }
    group
}

fn emit_analyzed(
    nodes: &[Node],
    metrics: Option<FunctionMetrics>,
    source: &str,
    fns: &mut Vec<FunctionMetrics>,
    name: &str,
) {
    if let Some(m) = metrics {
        fns.push(m);
    }
    collect_where_binds(nodes, source, fns, name);
}

fn collect_where_binds(nodes: &[Node], source: &str, fns: &mut Vec<FunctionMetrics>, parent: &str) {
    for &node in nodes {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "local_binds" {
                collect_functions(child, source, fns, parent);
            }
        }
    }
}

fn emit_typed_methods(node: Node, source: &str, fns: &mut Vec<FunctionMetrics>, decl_kind: &str) {
    let cls = find_child_by_kind(node, "name")
        .map(|n| node_text(n, source).to_string())
        .unwrap_or_default();
    if let Some(decls) = find_child_by_kind(node, decl_kind) {
        collect_functions(decls, source, fns, &cls);
    }
}
