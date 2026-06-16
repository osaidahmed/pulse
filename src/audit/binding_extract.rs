use std::collections::BTreeSet;

use tree_sitter::Node;

use crate::walk::{node_text, DepthGuard};

use super::binding::TypeEnv;

pub type TypeHead = fn(Node, &str) -> Option<String>;

#[derive(Default)]
pub struct EnvBuilder {
    env: TypeEnv,
    conflicts: BTreeSet<String>,
}

impl EnvBuilder {
    pub fn bind(&mut self, name: String, ty: String) {
        if self.conflicts.contains(&name) {
            return;
        }
        match self.env.get(&name) {
            Some(existing) if *existing != ty => {
                self.env.remove(&name);
                self.conflicts.insert(name);
            }
            Some(_) => {}
            None => {
                self.env.insert(name, ty);
            }
        }
    }

    pub fn into_env(mut self, type_vars: &BTreeSet<String>) -> TypeEnv {
        self.env.retain(|_, ty| !type_vars.contains(ty.as_str()));
        self.env
    }
}

pub fn head_of(text: &str) -> Option<String> {
    let no_generics = strip_generics(text);
    let unqualified = no_generics.rsplit("::").next()?.rsplit('.').next()?.trim();
    let simple = unqualified.trim_end_matches('?').trim();
    if simple.is_empty() {
        None
    } else {
        Some(simple.to_string())
    }
}

fn strip_generics(text: &str) -> String {
    let mut out = String::new();
    let mut depth: u32 = 0;
    for c in text.chars() {
        match c {
            '<' => depth += 1,
            '>' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out
}

pub fn c_declarator_name(node: Node, source: &str) -> Option<String> {
    match node.kind() {
        "identifier" | "field_identifier" => Some(node_text(node, source).to_string()),
        "pointer_declarator" | "reference_declarator" | "init_declarator" => {
            node.child_by_field_name("declarator").and_then(|d| c_declarator_name(d, source))
        }
        _ => None,
    }
}

pub fn bind_c_decl(decl: Node, source: &str, builder: &mut EnvBuilder, type_head: TypeHead) {
    let Some(ty) = decl.child_by_field_name("type").and_then(|t| type_head(t, source)) else {
        return;
    };
    let mut cursor = decl.walk();
    for child in decl.children(&mut cursor) {
        if let Some(name) = c_declarator_name(child, source) {
            builder.bind(name, ty.clone());
        }
    }
}

pub fn collect_c_locals(node: Node, source: &str, builder: &mut EnvBuilder, type_head: TypeHead, boundaries: &[&str]) {
    let Some(_guard) = DepthGuard::enter() else {
        return;
    };
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let kind = child.kind();
        if boundaries.contains(&kind) {
            continue;
        }
        if kind == "declaration" {
            bind_c_decl(child, source, builder, type_head);
        }
        collect_c_locals(child, source, builder, type_head, boundaries);
    }
}
