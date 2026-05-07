use std::path::PathBuf;

use pulse::audit::cycles::{
    default_max_iters, find_cycles, find_cycles_with_max_iters, TarjanDiagnostics,
};
use pulse::audit::graph::{ImportGraph, InputEdge};
use pulse::audit::{audit_traversal_cap_hit, MAX_FILES};
use pulse::parse::Language;

fn graph_chain(n: usize) -> ImportGraph {
    let edges: Vec<InputEdge> = (0..n)
        .map(|i| InputEdge {
            source: PathBuf::from(format!("m{i}.py")),
            target: PathBuf::from(format!("m{}.py", (i + 1) % n)),
            source_lang: Language::Python,
            target_lang: Language::Python,
        })
        .collect();
    ImportGraph::build(&edges)
}

#[test]
fn tarjan_default_cap_does_not_fire_on_modest_graph() {
    let graph = graph_chain(50);
    let (_, diag): (_, TarjanDiagnostics) =
        find_cycles_with_max_iters(&graph, 2, default_max_iters(&graph));
    assert!(
        !diag.iteration_cap_hit,
        "default cap must not fire on a 50-node cycle: {diag:?}"
    );
}

#[test]
fn tarjan_default_cap_does_not_fire_on_large_graph() {
    let graph = graph_chain(2_000);
    let (_, diag) = find_cycles_with_max_iters(&graph, 2, default_max_iters(&graph));
    assert!(
        !diag.iteration_cap_hit,
        "default cap must not fire on a 2000-node cycle: {diag:?}"
    );
}

#[test]
fn tarjan_cap_fires_when_max_iters_is_zero() {
    let graph = graph_chain(50);
    let (_, diag) = find_cycles_with_max_iters(&graph, 2, 0);
    assert!(
        diag.iteration_cap_hit,
        "cap must fire when max_iters=0: {diag:?}"
    );
    assert_eq!(diag.max_iters, 0);
}

#[test]
fn tarjan_cap_fires_when_max_iters_is_one() {
    let graph = graph_chain(50);
    let (_, diag) = find_cycles_with_max_iters(&graph, 2, 1);
    assert!(diag.iteration_cap_hit, "cap must fire when max_iters=1");
}

#[test]
fn tarjan_default_cap_formula_is_at_least_n_plus_edges() {
    let graph = graph_chain(100);
    let cap = default_max_iters(&graph);
    let n = 100usize;
    let total_edges = 100usize;
    assert!(
        cap >= n + total_edges,
        "default cap {cap} must be >= n+edges = {} for a 100-node ring",
        n + total_edges
    );
}

#[test]
fn tarjan_cap_truncates_output_silently_when_fired() {
    let graph = graph_chain(50);
    let (cycles_normal, _) = find_cycles_with_max_iters(&graph, 2, default_max_iters(&graph));
    let (cycles_capped, diag) = find_cycles_with_max_iters(&graph, 2, 0);
    assert!(diag.iteration_cap_hit);
    assert!(
        cycles_capped.len() <= cycles_normal.len(),
        "capped output must not exceed normal output (truncation only)"
    );
}

#[test]
fn tarjan_normal_call_returns_same_components_as_diagnostic_call() {
    let graph = graph_chain(50);
    let normal = find_cycles(&graph, 2);
    let (diag_components, diag) =
        find_cycles_with_max_iters(&graph, 2, default_max_iters(&graph));
    assert_eq!(normal.len(), diag_components.len());
    assert!(!diag.iteration_cap_hit);
}

#[test]
fn audit_max_files_cap_predicate_is_consistent() {
    assert!(!audit_traversal_cap_hit(0));
    assert!(!audit_traversal_cap_hit(MAX_FILES - 1));
    assert!(audit_traversal_cap_hit(MAX_FILES));
    assert!(audit_traversal_cap_hit(MAX_FILES + 1));
    assert!(audit_traversal_cap_hit(usize::MAX));
}

#[test]
fn audit_max_files_cap_is_generous_relative_to_realistic_repos() {
    assert!(
        MAX_FILES >= 50_000,
        "MAX_FILES must accommodate large monorepos, got {MAX_FILES}"
    );
}

#[test]
fn haskell_split_type_arrows_terminates_on_long_signature() {
    use pulse::parse::{detect_language, parse_and_walk};
    let mut src = String::from("f :: ");
    for _ in 0..200 {
        src.push_str("(Int -> [String]) -> ");
    }
    src.push_str("IO ()\n");
    src.push_str("f = undefined\n");

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("long.hs");
    std::fs::write(&path, &src).unwrap();
    let lang = detect_language(&path).expect("haskell");
    let metrics = parse_and_walk(&src, lang).expect("parse");
    assert!(!metrics.functions.is_empty());
}

#[test]
fn fingerprint_terminates_on_wide_tree() {
    use pulse::parse::{detect_language, parse_and_walk};
    let mut src = String::from("def f():\n    return [");
    for i in 0..2_000 {
        if i > 0 {
            src.push_str(", ");
        }
        src.push_str(&format!("{i}"));
    }
    src.push_str("]\n");

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("wide.py");
    std::fs::write(&path, &src).unwrap();
    let lang = detect_language(&path).expect("python");
    let metrics = parse_and_walk(&src, lang).expect("parse");
    let f = metrics.functions.iter().find(|f| f.name == "f").expect("f");
    assert_ne!(
        f.structural_hash, 0,
        "wide tree must produce non-zero fingerprint (cap must not be hit prematurely)"
    );
}

#[test]
fn cobol_walker_terminates_on_many_statements() {
    use pulse::parse::{detect_language, parse_and_walk};
    let mut src = String::from(concat!(
        "       IDENTIFICATION DIVISION.\n",
        "       PROGRAM-ID. TEST.\n",
        "       PROCEDURE DIVISION.\n",
        "       MAIN-PARA.\n",
    ));
    for i in 0..500 {
        src.push_str(&format!("           DISPLAY \"line {i}\".\n"));
    }
    src.push_str("           STOP RUN.\n");

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("big.cob");
    std::fs::write(&path, &src).unwrap();
    let lang = detect_language(&path).expect("cobol");
    let metrics = parse_and_walk(&src, lang).expect("parse");
    let main = metrics
        .functions
        .iter()
        .find(|f| f.name.contains("MAIN"))
        .expect("MAIN-PARA");
    assert!(main.loc > 100, "main paragraph loc must reflect 500 statements, got {}", main.loc);
}
