use pulse::audit::finding::{AuditFinding, AuditKind};
use pulse::audit::suppression::AuditSuppression;
use pulse::audit::{self, AuditOpts, PassChoice};
use std::path::Path;

use crate::audit_common::t;

fn run_deps(root: &Path) -> Vec<AuditFinding> {
    let opts = AuditOpts {
        root: root.to_path_buf(),
        pass: Some(PassChoice::Deps),
        json: false,
        include_tests: false,
        show_noise: false,
        suppression: AuditSuppression::new(),
    };
    audit::run(&opts, &t().audit)
}

fn constraint_problems(findings: &[AuditFinding]) -> Vec<(String, String)> {
    findings
        .iter()
        .filter_map(|f| match &f.kind {
            AuditKind::ConstraintSmell(e) => Some((e.name.clone(), e.problem.clone())),
            _ => None,
        })
        .collect()
}

fn project(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (name, body) in files {
        std::fs::write(dir.path().join(name), body).unwrap();
    }
    dir
}

#[test]
fn wildcard_constraints_are_flagged() {
    let dir = project(&[(
        "package.json",
        r#"{"name": "demo", "dependencies": {"anything": "*", "pinnedish": "^1.2.0", "newest": "latest"}}"#,
    )]);
    let problems = constraint_problems(&run_deps(dir.path()));
    let names: Vec<&str> = problems.iter().map(|(n, _)| n.as_str()).collect();
    assert!(names.contains(&"anything"), "{problems:?}");
    assert!(names.contains(&"newest"), "{problems:?}");
    assert!(!names.contains(&"pinnedish"), "caret ranges are deliberate: {problems:?}");
}

#[test]
fn exact_pins_without_a_lockfile_are_flagged_once() {
    let dir =
        project(&[("package.json", r#"{"name": "demo", "dependencies": {"a": "1.0.0", "b": "2.0.0", "c": "3.0.0"}}"#)]);
    let problems = constraint_problems(&run_deps(dir.path()));
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].1.contains("no lockfile"), "{problems:?}");
}

#[test]
fn exact_pins_with_a_lockfile_are_fine() {
    let dir = project(&[
        ("package.json", r#"{"name": "demo", "dependencies": {"a": "1.0.0", "b": "2.0.0", "c": "3.0.0"}}"#),
        (
            "package-lock.json",
            r#"{"lockfileVersion": 3, "packages": {"node_modules/a": {"version": "1.0.0"}, "node_modules/b": {"version": "2.0.0"}, "node_modules/c": {"version": "3.0.0"}}}"#,
        ),
    ]);
    let problems = constraint_problems(&run_deps(dir.path()));
    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
fn mixed_constraints_do_not_trip_the_pinning_smell() {
    let dir = project(&[(
        "package.json",
        r#"{"name": "demo", "dependencies": {"a": "1.0.0", "b": "2.0.0", "c": "^3.0.0"}}"#,
    )]);
    let problems = constraint_problems(&run_deps(dir.path()));
    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
fn npm_dot_x_range_is_not_a_divergence() {
    let dir = project(&[
        ("package.json", r#"{"name": "demo", "dependencies": {"typescript": "5.8.x"}}"#),
        (
            "package-lock.json",
            r#"{"lockfileVersion": 3, "packages": {"node_modules/typescript": {"version": "5.8.3"}}}"#,
        ),
    ]);
    let problems = constraint_problems(&run_deps(dir.path()));
    assert!(problems.is_empty(), "`5.8.x` is a range, not an exact pin — 5.8.3 satisfies it: {problems:?}");
}

#[test]
fn build_metadata_suffix_is_not_a_divergence() {
    let dir = project(&[
        (
            "Cargo.toml",
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n\n[dependencies]\nre_log = \"=0.34.0-alpha.1\"\n",
        ),
        ("Cargo.lock", "version = 3\n\n[[package]]\nname = \"re_log\"\nversion = \"0.34.0-alpha.1+dev\"\n"),
        ("src-lib.rs", ""),
    ]);
    let problems = constraint_problems(&run_deps(dir.path()));
    assert!(problems.is_empty(), "semver build metadata (+dev) is not a version divergence: {problems:?}");
}

#[test]
fn pin_and_lockfile_divergence_is_flagged() {
    let dir = project(&[
        ("Cargo.toml", "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n\n[dependencies]\nserde = \"=1.0.100\"\n"),
        ("Cargo.lock", "version = 3\n\n[[package]]\nname = \"serde\"\nversion = \"1.0.200\"\n"),
        ("src-lib.rs", ""),
    ]);
    let problems = constraint_problems(&run_deps(dir.path()));
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].1.contains("1.0.100") && problems[0].1.contains("1.0.200"), "{problems:?}");
}

#[test]
fn matching_pin_and_lock_are_silent() {
    let dir = project(&[
        ("Cargo.toml", "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n\n[dependencies]\nserde = \"=1.0.200\"\n"),
        ("Cargo.lock", "version = 3\n\n[[package]]\nname = \"serde\"\nversion = \"1.0.200\"\n"),
    ]);
    let problems = constraint_problems(&run_deps(dir.path()));
    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
fn composer_same_vendor_wildcard_is_an_exempt_monorepo_sibling() {
    let dir =
        project(&[("composer.json", r#"{"name": "acme/app", "require": {"acme/core": "*", "vendorx/lib": "*"}}"#)]);
    let names: Vec<String> = constraint_problems(&run_deps(dir.path())).into_iter().map(|(n, _)| n).collect();
    assert!(!names.contains(&"acme/core".to_string()), "a same-vendor wildcard is an intentional sibling: {names:?}");
    assert!(names.contains(&"vendorx/lib".to_string()), "a foreign-vendor wildcard is still flagged: {names:?}");
}

#[test]
fn composer_v_prefixed_and_partial_lock_versions_are_not_divergences() {
    let dir = project(&[
        ("composer.json", r#"{"name": "acme/app", "require": {"v/a": "1.2.3", "p/b": "2.2"}}"#),
        (
            "composer.lock",
            r#"{"packages": [{"name": "v/a", "version": "v1.2.3"}, {"name": "p/b", "version": "2.2.0"}]}"#,
        ),
    ]);
    let problems = constraint_problems(&run_deps(dir.path()));
    assert!(problems.is_empty(), "a `v` prefix and a partial pin both match the resolved version: {problems:?}");
}

#[test]
fn composer_genuine_version_divergence_is_still_flagged() {
    let dir = project(&[
        ("composer.json", r#"{"name": "acme/app", "require": {"v/a": "5.2.4"}}"#),
        ("composer.lock", r#"{"packages": [{"name": "v/a", "version": "2.8.2"}]}"#),
    ]);
    let problems = constraint_problems(&run_deps(dir.path()));
    assert_eq!(problems.len(), 1, "a real major-version mismatch is a divergence: {problems:?}");
}

#[test]
fn go_modules_are_exempt_from_pinning_smells() {
    let dir = project(&[(
        "go.mod",
        "module example.com/demo\n\ngo 1.22\n\nrequire (\n\tgithub.com/a/a v1.0.0\n\tgithub.com/b/b v2.0.0\n\tgithub.com/c/c v3.0.0\n)\n",
    )]);
    let problems = constraint_problems(&run_deps(dir.path()));
    assert!(problems.is_empty(), "go pins by design: {problems:?}");
}
