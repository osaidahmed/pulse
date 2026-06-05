use pulse::audit::finding::{AuditFinding, AuditKind, VulnCloneEvidence};
use pulse::audit::{self, AuditOpts, IgnoreFilter, PassChoice};
use pulse::config::{AuditSuppression, IgnoreMatcher};

use crate::audit_common::*;

const TAINTED: &str = "def handle_a(cursor, raw):\n    value = input()\n    prefix = \"select * from t where x = \"\n    query = prefix + value\n    result = cursor.execute(query)\n    return result\n";
const CLEAN_CLONE: &str = "def handle_b(cursor, raw):\n    value = default_config()\n    prefix = \"select * from t where x = \"\n    query = prefix + value\n    result = cursor.execute(query)\n    return result\n";
const CLEAN_TWIN: &str = "def handle_c(cursor, raw):\n    value = other_config()\n    prefix = \"select * from t where x = \"\n    query = prefix + value\n    result = cursor.execute(query)\n    return result\n";
const DISSIMILAR: &str = "def render(name):\n    parts = name.split(\",\")\n    cleaned = [p.strip() for p in parts]\n    joined = \", \".join(cleaned)\n    return joined.upper()\n";

fn run_pass(files: &[(&str, &str)], pass: Option<PassChoice>) -> Vec<AuditFinding> {
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

fn vuln_clones(findings: &[AuditFinding]) -> Vec<&VulnCloneEvidence> {
    findings
        .iter()
        .filter_map(|f| match &f.kind {
            AuditKind::VulnerableCloneSibling(e) => Some(e),
            _ => None,
        })
        .collect()
}

fn named(path: &std::path::Path, name: &str) -> bool {
    path.file_name().is_some_and(|n| n == name)
}

#[test]
fn clean_clone_of_tainted_fragment_is_flagged() {
    let found = run_pass(&[("a.py", TAINTED), ("b.py", CLEAN_CLONE)], Some(PassChoice::VulnClones));
    let flagged = vuln_clones(&found);
    let sibling = flagged.iter().find(|e| named(&e.file, "b.py"));
    let e = sibling.expect("the clean clone of a tainted fragment should be flagged");
    assert!(named(&e.vuln_file, "a.py"), "should point at the tainted sibling");
    assert_eq!(e.sink_name, "execute");
}

#[test]
fn no_taint_means_no_propagation() {
    let found = run_pass(&[("a.py", CLEAN_CLONE), ("b.py", CLEAN_TWIN)], Some(PassChoice::VulnClones));
    assert!(vuln_clones(&found).is_empty(), "no injection flow means nothing to propagate");
}

#[test]
fn no_clone_means_no_propagation() {
    let found = run_pass(&[("a.py", TAINTED), ("other.py", DISSIMILAR)], Some(PassChoice::VulnClones));
    assert!(vuln_clones(&found).is_empty(), "a tainted fragment with no clone has no siblings to flag");
}

#[test]
fn vuln_clones_opt_in_not_in_default_passes() {
    let found = run_pass(&[("a.py", TAINTED), ("b.py", CLEAN_CLONE)], None);
    let any = found.iter().any(|f| matches!(f.kind, AuditKind::VulnerableCloneSibling(_)));
    assert!(!any, "vuln-clone propagation must not run in the default (All) pass");
}
