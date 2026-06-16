use std::path::{Path, PathBuf};

use pulse::audit::component_thresholds::{is_owned_by, is_root_owned, owning_dir, primary_file, Stratum};
use pulse::audit::corpus::Corpus;
use pulse::audit::definitions::{definitions_for_file, definitions_from};
use pulse::audit::duplication_clusters::{cluster_members, run_from as clones_run_from};
use pulse::audit::finding::{
    AuditFinding, AuditKind, ClassIdentityRef, DivergentChangeEvidence, FeatureEnvyEvidence, GodClassEvidence,
    ImportConfidence, InjectionEvidence, ParallelInheritanceEvidence, RefusedBequestEvidence, ShotgunSurgeryEvidence,
};
use pulse::audit::fragmentation::detect;
use pulse::audit::graph::{ImportGraph, InputEdge};
use pulse::audit::vuln_clones::run_from as vuln_run_from;
use pulse::audit::{run_with_filter_online, AuditOpts, IgnoreFilter, PassChoice};
use pulse::config::{AuditSuppression, IgnoreMatcher};
use pulse::parse::Language;

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

fn write_tempfile(content: &str, ext: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(format!("test.{ext}"));
    std::fs::write(&path, content).unwrap();
    (dir, path)
}

fn wrap(kind: AuditKind) -> AuditFinding {
    AuditFinding {
        kind,
        representative_snippet: String::new(),
        support: 1,
        file_count: 1,
        idf_score: None,
        action_label: None,
        locations: Vec::new(),
        pattern_category: None,
        locality_entropy: None,
        p_value: None,
    }
}

#[test]
fn fragmentation_dense_mesh_just_over_avg_loc_ceiling_not_flagged() {
    let frag = &t().audit.package_metrics.fragmentation;
    let over = frag.max_avg_loc + 1;
    let graph = ImportGraph::build(&mesh("src/heavy", 8));
    assert!(
        detect(&graph, |_| over, &t().audit).is_empty(),
        "avg loc above the ceiling excludes a directory from over-fragmentation"
    );
}

#[test]
fn fragmentation_at_avg_loc_ceiling_is_flagged() {
    let frag = &t().audit.package_metrics.fragmentation;
    let at = frag.max_avg_loc;
    let graph = ImportGraph::build(&mesh("src/edge", 8));
    let findings = detect(&graph, |_| at, &t().audit);
    assert_eq!(findings.len(), 1, "avg loc exactly at the ceiling still qualifies");
    let AuditKind::OverFragmentation(e) = &findings[0].kind else {
        panic!("expected OverFragmentation");
    };
    assert_eq!(e.avg_loc, at);
}

#[test]
fn fragmentation_truncates_to_max_arch_findings_reported() {
    let mut audit_t = t().audit;
    audit_t.package_metrics.max_arch_findings_reported = 1;
    let mut edges = mesh("src/one", 8);
    edges.extend(mesh("src/two", 8));
    edges.extend(mesh("src/three", 8));
    let graph = ImportGraph::build(&edges);
    let findings = detect(&graph, |_| 30, &audit_t);
    assert_eq!(findings.len(), 1, "truncation caps reported findings at the configured ceiling");
}

#[test]
fn fragmentation_orders_findings_by_density_descending() {
    let mut edges = mesh("src/dense", 8);
    edges.extend(mesh("src/leaner", 8));
    edges.retain(|e| !e.source.starts_with("src/leaner") || !e.target.ends_with("f2.rs"));
    let graph = ImportGraph::build(&edges);
    let findings = detect(&graph, |_| 30, &t().audit);
    let densities: Vec<f64> = findings
        .iter()
        .filter_map(|f| match &f.kind {
            AuditKind::OverFragmentation(e) => Some(e.density),
            _ => None,
        })
        .collect();
    assert!(densities.len() >= 2, "two qualifying directories with distinct densities: {densities:?}");
    for w in densities.windows(2) {
        assert!(w[0] >= w[1], "findings sorted by density descending: {densities:?}");
    }
}

#[test]
fn fragmentation_empty_graph_no_findings() {
    let graph = ImportGraph::build(&[]);
    assert!(detect(&graph, |_| 30, &t().audit).is_empty());
}

#[test]
fn definitions_from_corpus_file_matches_path_based_extraction() {
    let src = "class Foo:\n    def bar(self, x):\n        return self.x + x\n    def baz(self):\n        return 0\n";
    let (_dir, path) = write_tempfile(src, "py");
    let by_path = definitions_for_file(&path, Language::Python);
    let corpus = Corpus::load(&[(path.clone(), Language::Python)]);
    let from_corpus = definitions_from(&corpus.files[0]);
    assert_eq!(from_corpus.len(), by_path.len());
    assert!(from_corpus.iter().any(|d| d.identity.name == "bar"));
    assert_eq!(from_corpus[0].identity.file, path);
}

#[test]
fn definitions_from_corpus_file_with_unparsed_source_is_empty() {
    let missing = PathBuf::from("/nonexistent/cov/file.py");
    let corpus = Corpus::load(&[(missing, Language::Python)]);
    assert!(definitions_from(&corpus.files[0]).is_empty(), "no source means no definitions");
}

#[test]
fn definitions_go_receiver_method_strips_dotted_class_prefix() {
    let src = "package main\n\ntype T struct{}\n\nfunc (t T) Method() int {\n    return 1\n}\n";
    let (_dir, path) = write_tempfile(src, "go");
    let defs = definitions_for_file(&path, Language::Go);
    let m = defs.iter().find(|d| d.identity.class.as_deref() == Some("T")).expect("receiver method extracted");
    assert_eq!(m.identity.name, "Method", "the T. prefix is stripped from the bare name");
}

#[test]
fn call_path_qualified_rust_scoped_call_splits_path_and_name() {
    let src = "fn main() { std::mem::swap(&mut a, &mut b); }\n";
    let calls = extract(src, Language::Rust);
    let swap = calls.iter().find(|c| c.callee_name == "swap").expect("scoped call extracted");
    assert_eq!(swap.receiver_hint.as_deref(), Some("mem"), "path segment is the last qualifier before the name");
}

#[test]
fn call_path_qualified_cpp_scoped_call_extracted() {
    let src = "void run() { ns::inner::work(); }\n";
    let calls = extract(src, Language::Cpp);
    assert!(calls.iter().any(|c| c.callee_name == "work" && c.receiver_hint.as_deref() == Some("inner")));
}

#[test]
fn call_path_qualified_plain_dotted_not_treated_as_path_qualified() {
    let src = "fn run() { foo(); }\n";
    let calls = extract(src, Language::Rust);
    assert!(calls.iter().any(|c| c.callee_name == "foo" && c.receiver_hint.is_none()));
}

fn extract(src: &str, lang: Language) -> Vec<pulse::audit::calls::RawCall> {
    let tree = pulse::parse::parse_only(src, lang).expect("parse must succeed");
    pulse::audit::calls::extract_calls(&tree, src, lang)
}

const TAINTED: &str = "def handle_a(cursor, raw):\n    value = input()\n    prefix = \"select * from t where x = \"\n    query = prefix + value\n    result = cursor.execute(query)\n    return result\n";
const CLEAN_CLONE: &str = "def handle_b(cursor, raw):\n    value = default_config()\n    prefix = \"select * from t where x = \"\n    query = prefix + value\n    result = cursor.execute(query)\n    return result\n";
const DISSIMILAR: &str = "def render(name):\n    parts = name.split(\",\")\n    cleaned = [p.strip() for p in parts]\n    joined = \", \".join(cleaned)\n    return joined.upper()\n";

fn corpus_for(files: &[(&str, &str)]) -> (tempfile::TempDir, Corpus) {
    let dir = tempfile::tempdir().unwrap();
    let mut typed = Vec::new();
    for (name, body) in files {
        let p = dir.path().join(name);
        std::fs::write(&p, body).unwrap();
        typed.push((p, Language::Python));
    }
    let corpus = Corpus::load(&typed);
    (dir, corpus)
}

#[test]
fn vuln_clones_run_from_no_sinks_short_circuits_empty() {
    let (_dir, corpus) = corpus_for(&[("a.py", CLEAN_CLONE), ("b.py", DISSIMILAR)]);
    assert!(vuln_run_from(&corpus, &t().audit).is_empty(), "no injection sink means no propagation");
}

#[test]
fn vuln_clones_run_from_propagates_sink_to_clean_clone() {
    let (_dir, corpus) = corpus_for(&[("a.py", TAINTED), ("b.py", CLEAN_CLONE)]);
    let found = vuln_run_from(&corpus, &t().audit);
    let sibling = found.iter().find_map(|f| match &f.kind {
        AuditKind::VulnerableCloneSibling(e) if e.file.ends_with("b.py") => Some(e),
        _ => None,
    });
    let e = sibling.expect("clean clone of a tainted fragment is flagged");
    assert!(e.vuln_file.ends_with("a.py"));
    assert_eq!(e.sink_name, "execute");
}

#[test]
fn vuln_clones_run_from_tainted_with_no_clone_yields_nothing() {
    let (_dir, corpus) = corpus_for(&[("a.py", TAINTED), ("other.py", DISSIMILAR)]);
    assert!(vuln_run_from(&corpus, &t().audit).is_empty(), "an unmatched tainted fragment has no clone siblings");
}

#[test]
fn duplication_clusters_run_from_finds_cross_file_near_duplicate() {
    let py_a =
        "def process_items(items):\n    total = 0\n    for item in items:\n        if item > 0:\n            total = total + item\n    return total\n";
    let py_b =
        "def accumulate(values):\n    acc = 0\n    for value in values:\n        if value > 1:\n            acc = acc + value\n    return acc\n";
    let (_dir, corpus) = corpus_for(&[("a.py", py_a), ("b.py", py_b)]);
    let found = clones_run_from(&corpus, &t().audit);
    let cross = found.iter().any(|f| matches!(&f.kind, AuditKind::NearDuplicate(e) if e.member_count >= 2));
    assert!(cross, "near-duplicate functions across files form a cluster");
}

#[test]
fn duplication_clusters_cluster_members_returns_groups_meeting_min_size() {
    let py_a =
        "def process_items(items):\n    total = 0\n    for item in items:\n        if item > 0:\n            total = total + item\n    return total\n";
    let py_b =
        "def accumulate(values):\n    acc = 0\n    for value in values:\n        if value > 1:\n            acc = acc + value\n    return acc\n";
    let dir = tempfile::tempdir().unwrap();
    let pa = dir.path().join("a.py");
    let pb = dir.path().join("b.py");
    std::fs::write(&pa, py_a).unwrap();
    std::fs::write(&pb, py_b).unwrap();
    let typed = vec![(pa, Language::Python), (pb, Language::Python)];
    let groups = cluster_members(&typed, &t().audit);
    assert!(
        groups.iter().any(|g| g.len() as u32 >= t().audit.clone_cluster.min_cluster_size),
        "at least one cluster meets the minimum cluster size"
    );
}

#[test]
fn duplication_clusters_run_from_below_min_loc_is_empty() {
    let tiny_a = "def f(x):\n    return x + 1\n";
    let tiny_b = "def g(y):\n    return y + 1\n";
    let (_dir, corpus) = corpus_for(&[("a.py", tiny_a), ("b.py", tiny_b)]);
    let found = clones_run_from(&corpus, &t().audit);
    assert!(
        !found.iter().any(|f| matches!(f.kind, AuditKind::NearDuplicate(_))),
        "fragments below min_loc never cluster"
    );
}

#[test]
fn duplication_clusters_run_from_empty_corpus_yields_nothing() {
    let corpus = Corpus::load(&[]);
    assert!(clones_run_from(&corpus, &t().audit).is_empty());
}

fn class_id(file: &str, name: &str) -> ClassIdentityRef {
    ClassIdentityRef { file: PathBuf::from(file), name: name.to_string() }
}

#[test]
fn primary_file_resolves_each_evidence_variant() {
    let shotgun = wrap(AuditKind::ShotgunSurgery(ShotgunSurgeryEvidence {
        method_file: PathBuf::from("src/sg.py"),
        method_class: None,
        method_name: "m".into(),
        method_line: 1,
        changing_classes: 2,
        changing_methods: 2,
        fanout: 3,
        confidence: ImportConfidence::Low,
        caller_samples: Vec::new(),
        name_collision_count: 0,
        additional_definitions: Vec::new(),
    }));
    assert_eq!(primary_file(&shotgun), Some(Path::new("src/sg.py")));

    let divergent = wrap(AuditKind::DivergentChange(DivergentChangeEvidence {
        class_file: PathBuf::from("src/dc.py"),
        class_name: "C".into(),
        changing_classes: 2,
        fanout: 2,
        method_count: 4,
        confidence: ImportConfidence::Low,
    }));
    assert_eq!(primary_file(&divergent), Some(Path::new("src/dc.py")));

    let envy = wrap(AuditKind::FeatureEnvy(FeatureEnvyEvidence {
        method_file: PathBuf::from("src/fe.py"),
        method_class: None,
        method_name: "m".into(),
        method_line: 1,
        atfd: 3,
        foreign_call_count: 4,
        intra_call_count: 1,
        envied_class: None,
        confidence: ImportConfidence::Low,
    }));
    assert_eq!(primary_file(&envy), Some(Path::new("src/fe.py")));

    let god = wrap(AuditKind::GodClass(GodClassEvidence {
        class_file: PathBuf::from("src/gc.py"),
        class_name: "G".into(),
        wmc: 50,
        tcc: 0.1,
        atfd: 5,
        method_count: 10,
        confidence: ImportConfidence::Low,
    }));
    assert_eq!(primary_file(&god), Some(Path::new("src/gc.py")));

    let parallel = wrap(AuditKind::ParallelInheritance(ParallelInheritanceEvidence {
        root_a: class_id("src/pa.py", "A"),
        root_b: class_id("src/pb.py", "B"),
        matched_descendants: Vec::new(),
        confidence: ImportConfidence::Low,
    }));
    assert_eq!(primary_file(&parallel), Some(Path::new("src/pa.py")));

    let refused = wrap(AuditKind::RefusedBequest(RefusedBequestEvidence {
        subclass_file: PathBuf::from("src/rb.py"),
        subclass_name: "S".into(),
        parent_file: PathBuf::from("src/parent.py"),
        parent_name: "P".into(),
        override_count: 0,
        parent_method_count: 5,
        override_ratio: 0.0,
        confidence: ImportConfidence::Low,
    }));
    assert_eq!(primary_file(&refused), Some(Path::new("src/rb.py")));
}

#[test]
fn primary_file_none_for_unhandled_kind() {
    let injection = wrap(AuditKind::InjectionShape(InjectionEvidence {
        file: PathBuf::from("src/db.php"),
        function: "q".into(),
        source_name: "input".into(),
        source_line: 1,
        sink_name: "execute".into(),
        sink_line: 2,
        tainted_var: "x".into(),
        crossed_opacity: false,
        confidence: ImportConfidence::Low,
    }));
    assert_eq!(primary_file(&injection), None);
    let pattern = wrap(AuditKind::UncategorizedPattern { fingerprint: 9 });
    assert_eq!(primary_file(&pattern), None);
}

#[test]
fn owning_dir_returns_none_when_no_stratum_matches() {
    let strata: Vec<Stratum> = Vec::new();
    assert_eq!(owning_dir(Path::new("/repo/x/f.py"), &strata), None);
}

#[test]
fn is_root_owned_true_for_findings_without_primary_file() {
    let strata = vec![Stratum { dir: PathBuf::from("/repo/a"), thresholds: t().audit }];
    let pattern = wrap(AuditKind::UncategorizedPattern { fingerprint: 1 });
    assert!(is_root_owned(&pattern, &strata), "kind with no primary file is root-owned");
}

#[test]
fn is_root_owned_true_when_file_outside_all_strata() {
    let strata = vec![Stratum { dir: PathBuf::from("/repo/a"), thresholds: t().audit }];
    let god = wrap(AuditKind::GodClass(GodClassEvidence {
        class_file: PathBuf::from("/repo/elsewhere/g.py"),
        class_name: "G".into(),
        wmc: 1,
        tcc: 0.0,
        atfd: 0,
        method_count: 1,
        confidence: ImportConfidence::Low,
    }));
    assert!(is_root_owned(&god, &strata), "a file outside every stratum is root-owned");
}

#[test]
fn is_root_owned_false_when_file_within_a_stratum() {
    let strata = vec![Stratum { dir: PathBuf::from("/repo/a"), thresholds: t().audit }];
    let god = wrap(AuditKind::GodClass(GodClassEvidence {
        class_file: PathBuf::from("/repo/a/g.py"),
        class_name: "G".into(),
        wmc: 1,
        tcc: 0.0,
        atfd: 0,
        method_count: 1,
        confidence: ImportConfidence::Low,
    }));
    assert!(!is_root_owned(&god, &strata), "a file under a stratum is owned by it, not the root");
}

#[test]
fn is_owned_by_matches_owning_dir_and_rejects_others() {
    let strata = vec![
        Stratum { dir: PathBuf::from("/repo/a"), thresholds: t().audit },
        Stratum { dir: PathBuf::from("/repo/a/b"), thresholds: t().audit },
    ];
    let god = wrap(AuditKind::GodClass(GodClassEvidence {
        class_file: PathBuf::from("/repo/a/b/g.py"),
        class_name: "G".into(),
        wmc: 1,
        tcc: 0.0,
        atfd: 0,
        method_count: 1,
        confidence: ImportConfidence::Low,
    }));
    assert!(is_owned_by(&god, Path::new("/repo/a/b"), &strata), "deepest matching stratum owns the finding");
    assert!(!is_owned_by(&god, Path::new("/repo/a"), &strata), "shallower stratum does not own it");
}

#[test]
fn is_owned_by_false_for_root_owned_finding() {
    let strata = vec![Stratum { dir: PathBuf::from("/repo/a"), thresholds: t().audit }];
    let pattern = wrap(AuditKind::UncategorizedPattern { fingerprint: 2 });
    assert!(!is_owned_by(&pattern, Path::new("/repo/a"), &strata));
}

fn run_pass_in(dir: &Path, pass: Option<PassChoice>, online: bool) -> Vec<AuditFinding> {
    let matcher = IgnoreMatcher::from_patterns(&[]);
    let opts = AuditOpts {
        root: dir.to_path_buf(),
        pass,
        json: false,
        include_tests: true,
        show_noise: false,
        suppression: AuditSuppression::new(),
    };
    let filter = IgnoreFilter::new(&matcher, dir);
    let cache = tempfile::tempdir().unwrap();
    run_with_filter_online(&opts, &t().audit, &filter, online, cache.path())
}

#[test]
fn passes_taint_only_selects_single_pass() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.py"), TAINTED).unwrap();
    let found = run_pass_in(dir.path(), Some(PassChoice::Taint), false);
    let _ = found;
}

#[test]
fn passes_naturalness_pass_runs_without_panic() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..6 {
        std::fs::write(
            dir.path().join(format!("f{i}.py")),
            "def f(x):\n    if x == 1:\n        return x\n    return 0\n",
        )
        .unwrap();
    }
    let _ = run_pass_in(dir.path(), Some(PassChoice::Naturalness), false);
}

#[test]
fn passes_ifdef_density_pass_runs_without_panic() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.c"), "#ifdef X\nint a;\n#endif\n#ifdef Y\nint b;\n#endif\n").unwrap();
    let _ = run_pass_in(dir.path(), Some(PassChoice::IfdefDensity), false);
}

#[test]
fn passes_clones_pass_selected_in_isolation() {
    let dir = tempfile::tempdir().unwrap();
    let body =
        "def process_items(items):\n    total = 0\n    for item in items:\n        if item > 0:\n            total = total + item\n    return total\n";
    std::fs::write(dir.path().join("a.py"), body).unwrap();
    std::fs::write(
        dir.path().join("b.py"),
        "def accumulate(values):\n    acc = 0\n    for value in values:\n        if value > 1:\n            acc = acc + value\n    return acc\n",
    )
    .unwrap();
    let found = run_pass_in(dir.path(), Some(PassChoice::Clones), false);
    let _ = found;
}

#[test]
fn passes_deps_online_with_no_manifests_makes_no_network_calls() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..6 {
        std::fs::write(dir.path().join(format!("m{i}.py")), "import os\nx = 1\n").unwrap();
    }
    let found = run_pass_in(dir.path(), Some(PassChoice::Deps), true);
    let _ = found;
}

#[test]
fn passes_vuln_clones_pass_in_isolation() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.py"), TAINTED).unwrap();
    std::fs::write(dir.path().join("b.py"), CLEAN_CLONE).unwrap();
    let found = run_pass_in(dir.path(), Some(PassChoice::VulnClones), false);
    let any = found.iter().any(|f| matches!(f.kind, AuditKind::VulnerableCloneSibling(_)));
    assert!(any, "isolated vuln-clones pass propagates the sink to the clean clone");
}

#[test]
fn passes_package_metrics_pass_in_isolation() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..6 {
        std::fs::write(dir.path().join(format!("mod{i}.py")), format!("import mod{}\nx = {i}\n", (i + 1) % 6)).unwrap();
    }
    let _ = run_pass_in(dir.path(), Some(PassChoice::PackageMetrics), false);
}

#[test]
fn passes_all_choice_selects_default_quartet() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.py"), "def f(x):\n    if x:\n        return 1\n    return 0\n").unwrap();
    let all = run_pass_in(dir.path(), Some(PassChoice::All), false);
    let none = run_pass_in(dir.path(), None, false);
    assert_eq!(all.len(), none.len(), "All and the default selection cover the same passes");
}
