use tree_sitter::Tree;

use super::FileMetrics;

pub fn walk(tree: &Tree, source: &str) -> FileMetrics {
    super::typescript::walk(tree, source, false)
}
