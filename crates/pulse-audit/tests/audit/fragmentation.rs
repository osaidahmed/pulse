use std::path::PathBuf;

use pulse_audit::finding::AuditKind;
use pulse_audit::fragmentation::detect;
use pulse_audit::graph::{ImportGraph, InputEdge};
use pulse_syntax::parse::Language;

use crate::audit_common::t;

fn edge(from: &str, to: &str) -> InputEdge {
    InputEdge {
        source: PathBuf::from(from),
        target: PathBuf::from(to),
        source_lang: Language::Rust,
        target_lang: Language::Rust,
    }
}

fn mesh(dir: &str, n: usize) -> Vec<InputEdge> {
    let files: Vec<String> = (0..n).map(|i| format!("{dir}/f{i}.rs")).collect();
    let mut edges = Vec::new();
    for i in 0..n {
        edges.push(edge(&files[i], &files[(i + 1) % n]));
        edges.push(edge(&files[i], &files[(i + 2) % n]));
    }
    edges
}

#[test]
fn dense_mesh_of_small_files_is_flagged() {
    let graph = ImportGraph::build(&mesh("src/mesh", 8));
    let findings = detect(&graph, |_| 30, &t().audit);
    assert_eq!(findings.len(), 1, "a dense mesh of 8 mutually-importing small files is over-fragmented");
    let AuditKind::OverFragmentation(e) = &findings[0].kind else {
        panic!("expected OverFragmentation");
    };
    assert_eq!(e.coupled_files, 8);
    assert_eq!(e.avg_loc, 30);
    assert!(e.density >= 1.5, "density {} should clear the gate", e.density);
    assert!(e.component.ends_with("src/mesh"));
}

#[test]
fn dense_mesh_of_large_files_is_not_flagged() {
    let graph = ImportGraph::build(&mesh("src/big", 8));
    assert!(
        detect(&graph, |_| 600, &t().audit).is_empty(),
        "large mutually-coupled files are a cohesive subsystem, not an over-split"
    );
}

#[test]
fn star_hub_directory_is_not_flagged() {
    let edges: Vec<InputEdge> = (0..8).map(|i| edge(&format!("src/star/leaf{i}.rs"), "src/star/hub.rs")).collect();
    let graph = ImportGraph::build(&edges);
    assert!(detect(&graph, |_| 30, &t().audit).is_empty(), "a hub/star is cohesive, not over-fragmented");
}

#[test]
fn small_dense_directory_below_min_files_not_flagged() {
    let graph = ImportGraph::build(&mesh("src/small", 5));
    assert!(detect(&graph, |_| 30, &t().audit).is_empty(), "below the coupled-file floor");
}

#[test]
fn cross_directory_imports_are_not_intra() {
    let edges: Vec<InputEdge> = (0..8).map(|i| edge(&format!("src/a/f{i}.rs"), &format!("src/b/g{i}.rs"))).collect();
    let graph = ImportGraph::build(&edges);
    assert!(detect(&graph, |_| 30, &t().audit).is_empty(), "cross-directory imports are not intra-directory coupling");
}
