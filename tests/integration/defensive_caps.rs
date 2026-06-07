use std::path::PathBuf;

use pulse::audit::cycles::{default_max_iters, find_cycles, find_cycles_with_max_iters, TarjanDiagnostics};
use pulse::audit::graph::{ImportGraph, InputEdge};
use pulse::audit::{audit_traversal_cap_hit, MAX_FILES};
use pulse::parse::{
    parse_and_walk, parse_and_walk_guarded, parse_and_walk_scoped, Language, MAX_INPUT_LINES, MAX_LINE_BYTES,
};

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
    let (_, diag): (_, TarjanDiagnostics) = find_cycles_with_max_iters(&graph, 2, default_max_iters(&graph));
    assert!(!diag.iteration_cap_hit, "default cap must not fire on a 50-node cycle: {diag:?}");
}

#[test]
fn tarjan_default_cap_does_not_fire_on_large_graph() {
    let graph = graph_chain(2_000);
    let (_, diag) = find_cycles_with_max_iters(&graph, 2, default_max_iters(&graph));
    assert!(!diag.iteration_cap_hit, "default cap must not fire on a 2000-node cycle: {diag:?}");
}

#[test]
fn tarjan_cap_fires_when_max_iters_is_zero() {
    let graph = graph_chain(50);
    let (_, diag) = find_cycles_with_max_iters(&graph, 2, 0);
    assert!(diag.iteration_cap_hit, "cap must fire when max_iters=0: {diag:?}");
    assert_eq!(diag.max_iters, 0);
}

#[test]
fn tarjan_cap_fires_when_max_iters_is_one() {
    let graph = graph_chain(50);
    let (_, diag) = find_cycles_with_max_iters(&graph, 2, 1);
    assert!(diag.iteration_cap_hit, "cap must fire when max_iters=1");
}

#[test]
fn guarded_matches_unguarded_on_normal_input() {
    let src = "def a():\n    if x:\n        return 1\n    return 0\n";
    let guarded = parse_and_walk_guarded(src, Language::Python).expect("guarded analyzes normal input");
    let plain = parse_and_walk(src, Language::Python).expect("plain analyzes normal input");
    assert_eq!(guarded.functions.len(), plain.functions.len());
    assert_eq!(guarded.module.sum_cc, plain.module.sum_cc);
}

#[test]
fn guarded_rejects_excessive_line_count() {
    let src = "x = 1\n".repeat(MAX_INPUT_LINES + 1_000);
    assert!(parse_and_walk_guarded(&src, Language::Python).is_none());
}

#[test]
fn guarded_rejects_excessive_line_length() {
    let src = format!("x = \"{}\"\n", "a".repeat(MAX_LINE_BYTES + 1_000));
    assert!(parse_and_walk_guarded(&src, Language::Python).is_none());
}

#[test]
fn guarded_survives_deeply_nested_expression() {
    let depth = 5_000;
    let src = format!("fn f() {{ let _x = {}1{}; }}", "(".repeat(depth), ")".repeat(depth));
    let _ = parse_and_walk_guarded(&src, Language::Rust);
}

#[test]
fn scoped_walk_computes_extras_only_for_edited_function() {
    let src = "fn untouched() {\n    let x = 1;\n}\n\nfn edited() {\n    let y = 2;\n}\n";
    let pos = src.find("let y").unwrap();
    let m = parse_and_walk_scoped(src, Language::Rust, Some((pos, pos + 5)), false).unwrap();
    let untouched = m.functions.iter().find(|f| f.name == "untouched").unwrap();
    let edited = m.functions.iter().find(|f| f.name == "edited").unwrap();
    assert_ne!(edited.skeleton_hash, 0, "edited function must compute extras");
    assert_eq!(edited.short_var_count, 1, "edited short-var counted");
    assert_eq!(untouched.skeleton_hash, 0, "untouched function skips extras");
    assert_eq!(untouched.short_var_count, 0, "untouched extras skipped");
    assert!(untouched.cc >= 1, "walk_body still runs for sum_cc");
}

#[test]
fn full_walk_computes_extras_for_all_functions() {
    let src = "fn a() {\n    let x = 1;\n}\n\nfn b() {\n    let y = 2;\n}\n";
    let m = parse_and_walk_scoped(src, Language::Rust, None, false).unwrap();
    for f in &m.functions {
        assert_ne!(f.skeleton_hash, 0, "{} must have extras in a full walk", f.name);
        assert_eq!(f.short_var_count, 1, "{} short-var counted in full walk", f.name);
    }
}

#[test]
fn tarjan_default_cap_formula_is_at_least_n_plus_edges() {
    let graph = graph_chain(100);
    let cap = default_max_iters(&graph);
    let n = 100usize;
    let total_edges = 100usize;
    assert!(cap >= n + total_edges, "default cap {cap} must be >= n+edges = {} for a 100-node ring", n + total_edges);
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
    let (diag_components, diag) = find_cycles_with_max_iters(&graph, 2, default_max_iters(&graph));
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
    let max_files = MAX_FILES;
    assert!(max_files >= 50_000, "MAX_FILES must accommodate large monorepos, got {max_files}");
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
    assert_ne!(f.structural_hash, 0, "wide tree must produce non-zero fingerprint (cap must not be hit prematurely)");
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
    let main = metrics.functions.iter().find(|f| f.name.contains("MAIN")).expect("MAIN-PARA");
    assert!(main.loc > 100, "main paragraph loc must reflect 500 statements, got {}", main.loc);
}
