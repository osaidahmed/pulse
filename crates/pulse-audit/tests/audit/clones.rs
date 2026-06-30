use std::collections::HashSet;

use pulse_audit::finding::{AuditFinding, AuditKind, CloneClusterEvidence, ImportConfidence};
use pulse_audit::suppression::AuditSuppression;
use pulse_audit::{self as audit, AuditOpts, IgnoreFilter, PassChoice};
use pulse_config::IgnoreMatcher;

use crate::audit_common::*;

fn run_clones(files: &[(&str, &str)], pass: Option<PassChoice>) -> Vec<AuditFinding> {
    let dir = tempfile::tempdir().unwrap();
    for (name, body) in files {
        std::fs::write(dir.path().join(name), body).unwrap();
    }
    let matcher = IgnoreMatcher::from_patterns(&[]);
    let opts = AuditOpts {
        root: dir.path().to_path_buf(),
        pass,
        json: false,
        include_tests: true,
        show_noise: false,
        suppression: AuditSuppression::new(),
    };
    let filter = IgnoreFilter::new(&matcher, dir.path());
    audit::run_with_filter(&opts, &t().audit, &filter)
}

fn clusters(findings: &[AuditFinding]) -> Vec<&CloneClusterEvidence> {
    findings
        .iter()
        .filter_map(|f| match &f.kind {
            AuditKind::NearDuplicate(e) => Some(e),
            _ => None,
        })
        .collect()
}

fn has_cross_file_cluster(cl: &[&CloneClusterEvidence]) -> bool {
    cl.iter().any(|e| {
        let files: HashSet<_> = e.members.iter().map(|m| &m.file).collect();
        files.len() >= 2
    })
}

const PY_A: &str =
    "def process_items(items):\n    total = 0\n    for item in items:\n        if item > 0:\n            total = total + item\n    return total\n";
const PY_B: &str =
    "def accumulate(values):\n    acc = 0\n    for value in values:\n        if value > 1:\n            acc = acc + value\n    return acc\n";

#[test]
fn near_duplicate_functions_clustered_across_files() {
    let found = run_clones(&[("mod_a.py", PY_A), ("mod_b.py", PY_B)], Some(PassChoice::Clones));
    let cl = clusters(&found);
    assert!(!cl.is_empty(), "expected a near-duplicate cluster");
    assert!(has_cross_file_cluster(&cl), "cluster should span both files");
}

#[test]
fn cross_file_clone_is_medium_confidence() {
    let found = run_clones(&[("mod_a.py", PY_A), ("mod_b.py", PY_B)], Some(PassChoice::Clones));
    let cross =
        clusters(&found).into_iter().find(|e| e.members.iter().map(|m| &m.file).collect::<HashSet<_>>().len() >= 2);
    let e = cross.expect("expected a cross-file cluster");
    assert_eq!(e.confidence, ImportConfidence::Medium);
    assert!(e.member_count >= 2);
}

#[test]
fn dissimilar_functions_not_clustered() {
    let other = "def render(name):\n    parts = name.split(\",\")\n    cleaned = [p.strip() for p in parts]\n    return \", \".join(cleaned)\n";
    let found = run_clones(&[("mod_a.py", PY_A), ("other.py", other)], Some(PassChoice::Clones));
    assert!(!has_cross_file_cluster(&clusters(&found)), "dissimilar code must not cross-cluster");
}

#[test]
fn tiny_fragments_below_min_loc_ignored() {
    let tiny_a = "def f(x):\n    return x + 1\n";
    let tiny_b = "def g(y):\n    return y + 1\n";
    let found = run_clones(&[("a.py", tiny_a), ("b.py", tiny_b)], Some(PassChoice::Clones));
    assert!(clusters(&found).is_empty(), "fragments below min_loc must not cluster");
}

#[test]
fn clones_opt_in_not_in_default_passes() {
    let found = run_clones(&[("mod_a.py", PY_A), ("mod_b.py", PY_B)], None);
    let any_clone = found.iter().any(|f| matches!(f.kind, AuditKind::NearDuplicate(_)));
    assert!(!any_clone, "clones must not run in the default (All) pass");
}

fn cluster_signature(findings: &[AuditFinding]) -> Vec<(u32, Vec<String>)> {
    let mut sig: Vec<(u32, Vec<String>)> = clusters(findings)
        .iter()
        .map(|e| {
            let mut locs: Vec<String> = e
                .members
                .iter()
                .map(|m| {
                    let name = m.file.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned());
                    format!("{name}:{}", m.line)
                })
                .collect();
            locs.sort();
            (e.member_count, locs)
        })
        .collect();
    sig.sort();
    sig
}

#[test]
fn clone_clustering_is_deterministic() {
    let files = &[("mod_a.py", PY_A), ("mod_b.py", PY_B)];
    let first = cluster_signature(&run_clones(files, Some(PassChoice::Clones)));
    let second = cluster_signature(&run_clones(files, Some(PassChoice::Clones)));
    assert!(!first.is_empty());
    assert_eq!(first, second, "clone clustering must be reproducible");
}

#[test]
fn rust_near_duplicate_functions_clustered() {
    let a = "fn process(items: &[i32]) -> i32 {\n    let mut total = 0;\n    for item in items {\n        if *item > 0 {\n            total += item;\n        }\n    }\n    total\n}\n";
    let b = "fn accumulate(values: &[i32]) -> i32 {\n    let mut acc = 0;\n    for value in values {\n        if *value > 1 {\n            acc += value;\n        }\n    }\n    acc\n}\n";
    let found = run_clones(&[("mod_a.rs", a), ("mod_b.rs", b)], Some(PassChoice::Clones));
    assert!(has_cross_file_cluster(&clusters(&found)), "expected a cross-file Rust clone cluster");
}
