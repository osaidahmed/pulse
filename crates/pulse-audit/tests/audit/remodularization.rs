use std::path::{Path, PathBuf};

use pulse_audit::finding::{
    AuditKind, ImportConfidence, MergeComponentsEvidence, MoveFileEvidence, SplitComponentEvidence,
};
use pulse_audit::graph::InputEdge;
use pulse_audit::martin::AbstractnessRecord;
use pulse_audit::package_metrics::{run_from_edges, ModuleProfile};
use pulse_syntax::parse::Language;

use crate::audit_common::*;

fn edge(src: &str, dst: &str) -> InputEdge {
    InputEdge {
        source: PathBuf::from(src),
        target: PathBuf::from(dst),
        source_lang: Language::Rust,
        target_lang: Language::Rust,
    }
}

fn profile(_path: &Path) -> ModuleProfile {
    ModuleProfile {
        abstractness: AbstractnessRecord { abstractness: Some(0.0), confidence: ImportConfidence::High },
        import_confidence: ImportConfidence::High,
        loc: 0,
    }
}

fn split_components(edges: &[InputEdge]) -> Vec<SplitComponentEvidence> {
    run_from_edges(edges, profile, &t().audit)
        .into_iter()
        .filter_map(|f| match f.kind {
            AuditKind::SplitComponent(e) => Some(e),
            _ => None,
        })
        .collect()
}

fn move_files(edges: &[InputEdge]) -> Vec<MoveFileEvidence> {
    run_from_edges(edges, profile, &t().audit)
        .into_iter()
        .filter_map(|f| match f.kind {
            AuditKind::MoveFile(e) => Some(e),
            _ => None,
        })
        .collect()
}

fn merge_components(edges: &[InputEdge]) -> Vec<MergeComponentsEvidence> {
    run_from_edges(edges, profile, &t().audit)
        .into_iter()
        .filter_map(|f| match f.kind {
            AuditKind::MergeComponents(e) => Some(e),
            _ => None,
        })
        .collect()
}

fn clique(dir_files: &[&str], bridge: Option<(&str, &str)>) -> Vec<InputEdge> {
    let mut edges = Vec::new();
    for i in 0..dir_files.len() {
        for j in (i + 1)..dir_files.len() {
            edges.push(edge(dir_files[i], dir_files[j]));
        }
    }
    if let Some((a, b)) = bridge {
        edges.push(edge(a, b));
    }
    edges
}

#[test]
fn directory_split_across_communities_is_flagged() {
    let mut edges = clique(&["core/x.rs", "core/y.rs", "mixed/a.rs", "mixed/b.rs"], None);
    edges.extend(clique(&["util/p.rs", "util/q.rs", "mixed/c.rs", "mixed/d.rs"], None));
    edges.push(edge("core/x.rs", "util/p.rs"));

    let splits = split_components(&edges);
    let mixed = splits
        .iter()
        .find(|e| e.component.as_path() == Path::new("mixed"))
        .expect("the mixed directory straddles two dependency communities");
    assert_eq!(mixed.file_count, 4);
    assert_eq!(mixed.community_count, 2);
    assert!((mixed.cohesion - 0.5).abs() < 1e-9, "its four files split evenly across two communities");
    assert_eq!(mixed.confidence, ImportConfidence::Medium);
}

#[test]
fn stray_file_in_foreign_community_is_flagged_for_move() {
    let edges = clique(&["core/a.rs", "core/b.rs", "core/c.rs", "core/d.rs", "stray/x.rs"], None);

    let moves = move_files(&edges);
    let stray = moves
        .iter()
        .find(|e| e.file.as_path() == Path::new("stray/x.rs"))
        .expect("the lone stray file clusters with the core community");
    assert_eq!(stray.current_dir.as_path(), Path::new("stray"));
    assert_eq!(stray.target_dir.as_path(), Path::new("core"));
    assert_eq!(stray.community_size, 5);
    assert!(stray.home_share >= t().audit.package_metrics.community.split_cohesion);
    assert_eq!(stray.confidence, ImportConfidence::Medium);
}

#[test]
fn cohesive_minority_is_not_relocated_piecemeal() {
    let edges = clique(&["core/a.rs", "core/b.rs", "core/c.rs", "core/d.rs", "helpers/x.rs", "helpers/y.rs"], None);
    assert!(
        move_files(&edges).is_empty(),
        "a cohesive multi-file minority is a secondary cluster, not piecemeal strays"
    );
}

#[test]
fn community_inside_one_directory_has_no_moves() {
    let edges = clique(&["core/a.rs", "core/b.rs", "core/c.rs", "core/d.rs"], None);
    assert!(move_files(&edges).is_empty(), "a community wholly inside one directory needs no relocation");
}

#[test]
fn two_directories_in_one_community_are_flagged_for_merge() {
    let edges = clique(&["core/a.rs", "core/b.rs", "shared/c.rs", "shared/d.rs"], None);

    let merges = merge_components(&edges);
    let merge = merges
        .iter()
        .find(|e| e.components.len() == 2)
        .expect("the two directories form a single dependency community");
    let comps: Vec<&Path> = merge.components.iter().map(std::path::PathBuf::as_path).collect();
    assert!(comps.contains(&Path::new("core")), "got: {comps:?}");
    assert!(comps.contains(&Path::new("shared")), "got: {comps:?}");
    assert_eq!(merge.community_files, 4);
    assert_eq!(merge.confidence, ImportConfidence::Medium);
}

#[test]
fn directories_in_distinct_communities_are_not_merged() {
    let mut edges = clique(&["core/a.rs", "core/b.rs"], None);
    edges.extend(clique(&["util/p.rs", "util/q.rs"], None));

    assert!(merge_components(&edges).is_empty(), "directories that form separate communities must not be merged");
}

#[test]
fn cohesive_directories_are_not_flagged() {
    let mut edges = clique(&["core/a.rs", "core/b.rs", "core/c.rs", "core/d.rs"], None);
    edges.extend(clique(&["util/p.rs", "util/q.rs", "util/r.rs", "util/s.rs"], None));
    edges.push(edge("core/a.rs", "util/p.rs"));

    assert!(split_components(&edges).is_empty(), "directories whose files share one community need no split");
}
