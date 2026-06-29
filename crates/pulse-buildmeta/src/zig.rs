use std::path::Path;

use tree_sitter::Node;

use pulse_syntax::parse::{parse_guarded, Language};
use pulse_syntax::walk::{find_child_by_kind, node_text, DepthGuard};

use super::{DeclaredDep, DepScope, Ecosystem, Manifest};

pub(super) fn parse_manifest(path: &Path, source: &str) -> Option<Manifest> {
    let tree = parse_guarded(source, Language::Zig)?;
    let top = find_struct(tree.root_node())?;
    let mut deps = Vec::new();
    for (key, value) in struct_fields(top, source) {
        match key.as_str() {
            "name" => push_own(value, source, &mut deps),
            "dependencies" => collect_deps(value, source, &mut deps),
            _ => {}
        }
    }
    (!deps.is_empty()).then(|| Manifest {
        path: path.to_path_buf(),
        ecosystem: Ecosystem::Zig,
        deps,
        workspace_members: Vec::new(),
    })
}

fn find_struct(node: Node) -> Option<Node> {
    let _guard = DepthGuard::enter()?;
    if node.kind() == "anonymous_struct_initializer" {
        return Some(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(found) = find_struct(child) {
            return Some(found);
        }
    }
    None
}

fn struct_fields<'a>(struct_node: Node<'a>, source: &'a str) -> Vec<(String, Node<'a>)> {
    let Some(list) = find_child_by_kind(struct_node, "initializer_list") else { return Vec::new() };
    let mut out = Vec::new();
    let mut cursor = list.walk();
    for child in list.children(&mut cursor) {
        if child.kind() != "assignment_expression" {
            continue;
        }
        let (Some(left), Some(right)) = (child.child_by_field_name("left"), child.child_by_field_name("right")) else {
            continue;
        };
        if let Some(key) = member(left, source) {
            out.push((key, right));
        }
    }
    out
}

fn collect_deps(deps_struct: Node, source: &str, out: &mut Vec<DeclaredDep>) {
    for (name, value) in struct_fields(deps_struct, source) {
        if struct_fields(value, source).iter().any(|(k, _)| k == "path") {
            continue;
        }
        out.push(DeclaredDep {
            name,
            constraint: String::new(),
            scope: DepScope::Deployed,
            line: value.start_position().row as u32 + 1,
            own: false,
        });
    }
}

fn push_own(value: Node, source: &str, out: &mut Vec<DeclaredDep>) {
    let name = match value.kind() {
        "field_expression" => member(value, source),
        "string" => string_text(value, source),
        _ => None,
    };
    if let Some(name) = name {
        out.push(DeclaredDep { name, constraint: String::new(), scope: DepScope::Deployed, line: 0, own: true });
    }
}

fn member(node: Node, source: &str) -> Option<String> {
    node.child_by_field_name("member").map(|m| node_text(m, source).to_string())
}

fn string_text(node: Node, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    let content: String =
        node.children(&mut cursor).filter(|c| c.kind() == "string_content").map(|c| node_text(c, source)).collect();
    (!content.is_empty()).then_some(content)
}
