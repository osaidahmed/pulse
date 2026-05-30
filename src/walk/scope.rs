use std::collections::HashMap;

use tree_sitter::Node;

use super::node_text;

pub fn build_scope_table(func_node: Node, source: &str) -> HashMap<String, String> {
    let mut table = HashMap::new();
    walk_nodes(func_node, &mut |n| record_binding(n, source, &mut table));
    table
}

fn walk_nodes(node: Node, visit: &mut impl FnMut(Node)) {
    let Some(_guard) = super::DepthGuard::enter() else {
        return;
    };
    visit(node);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_nodes(child, visit);
    }
}

fn record_binding(node: Node, source: &str, table: &mut HashMap<String, String>) {
    if !is_binding_kind(node.kind()) {
        return;
    }
    if let Some((var, class)) = extract_binding(node, source) {
        table.entry(var).or_insert(class);
    }
}

fn extract_binding(node: Node, source: &str) -> Option<(String, String)> {
    let mut idents: Vec<String> = Vec::new();
    walk_nodes(node, &mut |n| {
        if is_identifier_kind(n.kind()) {
            idents.push(node_text(n, source).to_string());
        }
    });
    let var = idents.iter().find(|n| is_var_name(n))?.clone();
    let class = idents.iter().find(|n| is_class_name(n))?.clone();
    Some((var, class))
}

fn is_binding_kind(kind: &str) -> bool {
    kind.contains("assign")
        || matches!(
            kind,
            "short_var_declaration"
                | "var_declaration"
                | "const_declaration"
                | "let_declaration"
                | "variable_declaration"
                | "variable_declarator"
                | "lexical_declaration"
                | "local_variable_declaration"
                | "init_declarator"
        )
}

fn is_identifier_kind(kind: &str) -> bool {
    matches!(kind, "identifier" | "simple_identifier" | "type_identifier" | "field_identifier")
}

fn is_var_name(name: &str) -> bool {
    name.chars().next().is_some_and(char::is_lowercase)
}

fn is_class_name(name: &str) -> bool {
    name.chars().next().is_some_and(char::is_uppercase) && name.chars().any(char::is_lowercase)
}
