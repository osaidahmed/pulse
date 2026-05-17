use std::path::{Path, PathBuf};

use pulse::history::edges::{build_graph, directly_linked};
use pulse::parse::Language;
use tempfile::TempDir;

fn write_file(root: &Path, rel: &str, content: &str) -> PathBuf {
    let full = root.join(rel);
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&full, content).unwrap();
    full
}

fn fixture_python_pair_with_import() -> (TempDir, Vec<(PathBuf, Language)>) {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "a.py", "from b import bar\nx = 1\n");
    let b = write_file(dir.path(), "b.py", "def bar():\n    return 1\n");
    let typed = vec![(a, Language::Python), (b, Language::Python)];
    (dir, typed)
}

fn fixture_python_pair_no_import() -> (TempDir, Vec<(PathBuf, Language)>) {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "a.py", "x = 1\n");
    let b = write_file(dir.path(), "b.py", "y = 2\n");
    let typed = vec![(a, Language::Python), (b, Language::Python)];
    (dir, typed)
}

fn fixture_rust_pair_with_use() -> (TempDir, Vec<(PathBuf, Language)>) {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "src/foo.rs", "use crate::bar;\npub fn x() { bar::baz(); }\n");
    let b = write_file(dir.path(), "src/bar.rs", "pub fn baz() {}\n");
    let typed = vec![(a, Language::Rust), (b, Language::Rust)];
    (dir, typed)
}

#[test]
fn build_graph_empty_typed_files_returns_empty_graph() {
    let dir = tempfile::tempdir().unwrap();
    let graph = build_graph(&[], dir.path());
    assert_eq!(graph.adjacency.edges().len(), 0);
}

#[test]
fn build_graph_python_pair_with_import_creates_edge() {
    let (dir, typed) = fixture_python_pair_with_import();
    let graph = build_graph(&typed, dir.path());
    assert!(!graph.adjacency.edges().is_empty(), "expected at least one edge");
}

#[test]
fn build_graph_python_pair_without_import_no_edges() {
    let (dir, typed) = fixture_python_pair_no_import();
    let graph = build_graph(&typed, dir.path());
    assert_eq!(graph.adjacency.edges().len(), 0);
}

#[test]
fn build_graph_rust_pair_with_use_creates_edge() {
    let (dir, typed) = fixture_rust_pair_with_use();
    let graph = build_graph(&typed, dir.path());
    assert!(!graph.adjacency.edges().is_empty());
}

#[test]
fn build_graph_handles_unparseable_file_no_panic() {
    let dir = tempfile::tempdir().unwrap();
    let p = write_file(dir.path(), "broken.py", "this is :::: not valid python (((");
    let typed = vec![(p, Language::Python)];
    let _ = build_graph(&typed, dir.path());
}

#[test]
fn build_graph_skips_external_imports() {
    let dir = tempfile::tempdir().unwrap();
    let p = write_file(dir.path(), "a.py", "import os\nimport sys\nx = 1\n");
    let typed = vec![(p, Language::Python)];
    let graph = build_graph(&typed, dir.path());
    assert_eq!(graph.adjacency.edges().len(), 0);
}

#[test]
fn directly_linked_returns_true_when_a_imports_b() {
    let (dir, typed) = fixture_python_pair_with_import();
    let graph = build_graph(&typed, dir.path());
    let a = &typed[0].0;
    let b = &typed[1].0;
    assert!(directly_linked(&graph, a, b));
}

#[test]
fn directly_linked_returns_true_when_b_imports_a_query_reversed() {
    let (dir, typed) = fixture_python_pair_with_import();
    let graph = build_graph(&typed, dir.path());
    let a = &typed[0].0;
    let b = &typed[1].0;
    assert!(directly_linked(&graph, b, a), "should detect link in reverse query");
}

#[test]
fn directly_linked_returns_false_for_unrelated_pair() {
    let (dir, typed) = fixture_python_pair_no_import();
    let graph = build_graph(&typed, dir.path());
    let a = &typed[0].0;
    let b = &typed[1].0;
    assert!(!directly_linked(&graph, a, b));
}

#[test]
fn directly_linked_returns_false_for_path_not_in_graph() {
    let (dir, typed) = fixture_python_pair_with_import();
    let graph = build_graph(&typed, dir.path());
    let a = &typed[0].0;
    let unknown = PathBuf::from("/nonexistent/path.py");
    assert!(!directly_linked(&graph, a, &unknown));
    assert!(!directly_linked(&graph, &unknown, a));
}

#[test]
fn directly_linked_handles_two_unknown_paths() {
    let dir = tempfile::tempdir().unwrap();
    let graph = build_graph(&[], dir.path());
    let p1 = PathBuf::from("/foo");
    let p2 = PathBuf::from("/bar");
    assert!(!directly_linked(&graph, &p1, &p2));
}

#[test]
fn build_graph_multi_language_project_processes_each() {
    let dir = tempfile::tempdir().unwrap();
    let py_a = write_file(dir.path(), "a.py", "from b import bar\n");
    let py_b = write_file(dir.path(), "b.py", "def bar(): pass\n");
    let rs_a = write_file(dir.path(), "src/foo.rs", "use crate::bar;\npub fn x() {}\n");
    let rs_b = write_file(dir.path(), "src/bar.rs", "pub fn baz() {}\n");
    let typed = vec![
        (py_a.clone(), Language::Python),
        (py_b.clone(), Language::Python),
        (rs_a, Language::Rust),
        (rs_b, Language::Rust),
    ];
    let graph = build_graph(&typed, dir.path());
    assert!(directly_linked(&graph, &py_a, &py_b));
}

#[test]
fn build_graph_does_not_create_edges_for_files_outside_typed_set() {
    let dir = tempfile::tempdir().unwrap();
    let a = write_file(dir.path(), "a.py", "from b import bar\n");
    let _b_outside = write_file(dir.path(), "b.py", "def bar(): pass\n");
    let typed = vec![(a, Language::Python)];
    let graph = build_graph(&typed, dir.path());
    assert_eq!(graph.adjacency.edges().len(), 0, "edge to non-typed file should not be recorded");
}
