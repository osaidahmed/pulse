use tree_sitter::Node;
use xxhash_rust::xxh3::xxh3_64;

use crate::parse::{self, Language};
use crate::walk::DepthGuard;

pub fn compute_simhash(node: Node) -> u64 {
    let mut acc = [0i32; 64];
    accumulate(node, "", &mut acc);
    let mut hash = 0u64;
    for (i, &v) in acc.iter().enumerate() {
        if v > 0 {
            hash |= 1u64 << i;
        }
    }
    hash
}

fn accumulate(node: Node, parent_kind: &str, acc: &mut [i32; 64]) {
    let Some(_g) = DepthGuard::enter() else { return };
    let kind = node.kind();
    let h = token_hash(parent_kind, kind);
    for (i, slot) in acc.iter_mut().enumerate() {
        if (h >> i) & 1 == 1 {
            *slot += 1;
        } else {
            *slot -= 1;
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.is_named() {
            accumulate(child, kind, acc);
        }
    }
}

fn token_hash(parent: &str, kind: &str) -> u64 {
    let mut buf = String::with_capacity(parent.len() + kind.len() + 1);
    buf.push_str(parent);
    buf.push('>');
    buf.push_str(kind);
    xxh3_64(buf.as_bytes())
}

pub fn simhash_of(source: &str, lang: Language) -> Option<u64> {
    let tree = parse::parse_guarded(source, lang)?;
    Some(compute_simhash(tree.root_node()))
}
