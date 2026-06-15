use pulse::audit::finding::{AuditFinding, AuditKind, ImportConfidence};
use pulse::audit::{self, AuditOpts, PassChoice};
use pulse::config::AuditSuppression;
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

fn workspace(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (name, body) in files {
        let path = dir.path().join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    }
    dir
}

fn undeclared_pairs(findings: &[AuditFinding]) -> Vec<(String, String)> {
    findings
        .iter()
        .filter_map(|f| match &f.kind {
            AuditKind::UndeclaredModuleDependency(e) => Some((e.from_component.clone(), e.to_component.clone())),
            _ => None,
        })
        .collect()
}

fn unused_pairs(findings: &[AuditFinding]) -> Vec<(String, String)> {
    findings
        .iter()
        .filter_map(|f| match &f.kind {
            AuditKind::UnusedDeclaredDependency(e) => Some((e.from_component.clone(), e.to_component.clone())),
            _ => None,
        })
        .collect()
}

fn bloated_names(findings: &[AuditFinding]) -> Vec<String> {
    findings
        .iter()
        .filter_map(|f| match &f.kind {
            AuditKind::BloatedDependency(e) => Some(e.name.clone()),
            _ => None,
        })
        .collect()
}

const ROOT_MANIFEST: &str = "[workspace]\nmembers = [\"member_a\", \"member_b\", \"member_c\"]\n";
const MANIFEST_A: &str = "[package]\nname = \"member_a\"\nversion = \"0.1.0\"\n\n[dependencies]\nmember_b = { path = \"../member_b\" }\nmember_c = { path = \"../member_c\" }\n";
const MANIFEST_B: &str = "[package]\nname = \"member_b\"\nversion = \"0.1.0\"\n";
const MANIFEST_C: &str = "[package]\nname = \"member_c\"\nversion = \"0.1.0\"\n";

fn canonical_workspace() -> tempfile::TempDir {
    workspace(&[
        ("Cargo.toml", ROOT_MANIFEST),
        ("member_a/Cargo.toml", MANIFEST_A),
        ("member_b/Cargo.toml", MANIFEST_B),
        ("member_c/Cargo.toml", MANIFEST_C),
        ("member_a/src/lib.rs", "use member_b::helper;\npub fn run() -> u32 {\n    helper()\n}\n"),
        ("member_b/src/lib.rs", "use member_a::run;\npub fn helper() -> u32 {\n    run()\n}\n"),
        ("member_c/src/lib.rs", "pub fn three() -> u32 {\n    3\n}\n"),
    ])
}

#[test]
fn undeclared_cross_member_import_is_flagged() {
    let dir = canonical_workspace();
    let pairs = undeclared_pairs(&run_deps(dir.path()));
    assert_eq!(pairs, vec![("member_b".to_string(), "member_a".to_string())], "{pairs:?}");
}

#[test]
fn unused_declared_member_dep_is_flagged() {
    let dir = canonical_workspace();
    let pairs = unused_pairs(&run_deps(dir.path()));
    assert_eq!(pairs, vec![("member_a".to_string(), "member_c".to_string())], "{pairs:?}");
}

#[test]
fn workspace_internal_deps_are_not_reported_as_bloated() {
    let dir = canonical_workspace();
    let names = bloated_names(&run_deps(dir.path()));
    assert!(names.is_empty(), "{names:?}");
}

#[test]
fn declared_and_imported_member_edges_are_silent() {
    let dir = canonical_workspace();
    let findings = run_deps(dir.path());
    let a_to_b = ("member_a".to_string(), "member_b".to_string());
    assert!(!undeclared_pairs(&findings).contains(&a_to_b));
    assert!(!unused_pairs(&findings).contains(&a_to_b));
}

#[test]
fn qualified_reference_suppresses_the_unused_finding() {
    let dir = workspace(&[
        ("Cargo.toml", "[workspace]\nmembers = [\"member_a\", \"member_c\"]\n"),
        (
            "member_a/Cargo.toml",
            "[package]\nname = \"member_a\"\nversion = \"0.1.0\"\n\n[dependencies]\nmember_c = { path = \"../member_c\" }\n",
        ),
        ("member_c/Cargo.toml", MANIFEST_C),
        ("member_a/src/lib.rs", "pub fn run() -> u32 {\n    member_c::three()\n}\n"),
        ("member_c/src/lib.rs", "pub fn three() -> u32 {\n    3\n}\n"),
    ]);
    let pairs = unused_pairs(&run_deps(dir.path()));
    assert!(pairs.is_empty(), "{pairs:?}");
}

#[test]
fn dev_scoped_member_dep_is_not_reported_unused() {
    let dir = workspace(&[
        ("Cargo.toml", "[workspace]\nmembers = [\"member_a\", \"member_c\"]\n"),
        (
            "member_a/Cargo.toml",
            "[package]\nname = \"member_a\"\nversion = \"0.1.0\"\n\n[dev-dependencies]\nmember_c = { path = \"../member_c\" }\n",
        ),
        ("member_c/Cargo.toml", MANIFEST_C),
        ("member_a/src/lib.rs", "pub fn run() -> u32 {\n    1\n}\n"),
        ("member_c/src/lib.rs", "pub fn three() -> u32 {\n    3\n}\n"),
    ]);
    let pairs = unused_pairs(&run_deps(dir.path()));
    assert!(pairs.is_empty(), "{pairs:?}");
}

#[test]
fn dev_scoped_declaration_suppresses_divergence() {
    let dir = workspace(&[
        ("Cargo.toml", "[workspace]\nmembers = [\"member_a\", \"member_b\"]\n"),
        (
            "member_a/Cargo.toml",
            "[package]\nname = \"member_a\"\nversion = \"0.1.0\"\n\n[dev-dependencies]\nmember_b = { path = \"../member_b\" }\n",
        ),
        ("member_b/Cargo.toml", MANIFEST_B),
        ("member_a/src/lib.rs", "use member_b::helper;\npub fn run() -> u32 {\n    helper()\n}\n"),
        ("member_b/src/lib.rs", "pub fn helper() -> u32 {\n    1\n}\n"),
    ]);
    let pairs = undeclared_pairs(&run_deps(dir.path()));
    assert!(pairs.is_empty(), "{pairs:?}");
}

#[test]
fn renamed_member_dep_counts_as_declared() {
    let dir = workspace(&[
        ("Cargo.toml", "[workspace]\nmembers = [\"member_a\", \"member_b\"]\n"),
        (
            "member_a/Cargo.toml",
            "[package]\nname = \"member_a\"\nversion = \"0.1.0\"\n\n[dependencies]\nb_alias = { package = \"member_b\", path = \"../member_b\" }\n",
        ),
        ("member_b/Cargo.toml", MANIFEST_B),
        ("member_a/src/lib.rs", "use member_b::helper;\npub fn run() -> u32 {\n    helper()\n}\n"),
        ("member_b/src/lib.rs", "pub fn helper() -> u32 {\n    1\n}\n"),
    ]);
    let pairs = undeclared_pairs(&run_deps(dir.path()));
    assert!(pairs.is_empty(), "{pairs:?}");
}

#[test]
fn non_workspace_repos_produce_no_reflexion_findings() {
    let dir = workspace(&[
        ("Cargo.toml", "[package]\nname = \"solo\"\nversion = \"0.1.0\"\n\n[dependencies]\nserde = \"1.0\"\n"),
        ("src/lib.rs", "pub fn run() -> u32 {\n    1\n}\n"),
    ]);
    let findings = run_deps(dir.path());
    assert!(undeclared_pairs(&findings).is_empty());
    assert!(unused_pairs(&findings).is_empty());
    assert_eq!(bloated_names(&findings), vec!["serde".to_string()]);
}

#[test]
fn nested_members_resolve_by_longest_prefix() {
    let dir = workspace(&[
        ("Cargo.toml", "[workspace]\nmembers = [\"pkgs/core\", \"pkgs/core/sub\"]\n"),
        ("pkgs/core/Cargo.toml", "[package]\nname = \"core_crate\"\nversion = \"0.1.0\"\n"),
        ("pkgs/core/sub/Cargo.toml", "[package]\nname = \"sub_crate\"\nversion = \"0.1.0\"\n"),
        ("pkgs/core/src/lib.rs", "pub fn base() -> u32 {\n    1\n}\n"),
        ("pkgs/core/sub/src/lib.rs", "use core_crate::base;\npub fn run() -> u32 {\n    base()\n}\n"),
    ]);
    let pairs = undeclared_pairs(&run_deps(dir.path()));
    assert_eq!(pairs, vec![("sub_crate".to_string(), "core_crate".to_string())], "{pairs:?}");
}

#[test]
fn root_package_participates_as_a_component() {
    let dir = workspace(&[
        (
            "Cargo.toml",
            "[workspace]\nmembers = [\"member_b\"]\n\n[package]\nname = \"root_crate\"\nversion = \"0.1.0\"\n",
        ),
        ("member_b/Cargo.toml", MANIFEST_B),
        ("src/lib.rs", "use member_b::helper;\npub fn run() -> u32 {\n    helper()\n}\n"),
        ("member_b/src/lib.rs", "pub fn helper() -> u32 {\n    1\n}\n"),
    ]);
    let pairs = undeclared_pairs(&run_deps(dir.path()));
    assert_eq!(pairs, vec![("root_crate".to_string(), "member_b".to_string())], "{pairs:?}");
}

const NPM_ROOT: &str = "{\"private\":true,\"workspaces\":[\"packages/a\",\"packages/b\",\"packages/c\"]}";
const NPM_A: &str = "{\"name\":\"@org/a\",\"dependencies\":{\"@org/b\":\"1.0.0\",\"@org/c\":\"1.0.0\"}}";
const NPM_B: &str = "{\"name\":\"@org/b\"}";
const NPM_C: &str = "{\"name\":\"@org/c\"}";

fn npm_workspace() -> tempfile::TempDir {
    workspace(&[
        ("package.json", NPM_ROOT),
        ("packages/a/package.json", NPM_A),
        ("packages/b/package.json", NPM_B),
        ("packages/c/package.json", NPM_C),
        ("packages/a/index.ts", "import { helper } from \"@org/b\";\nexport function run() { return helper(); }\n"),
        ("packages/b/index.ts", "import { run } from \"@org/a\";\nexport function helper() { return run(); }\n"),
        ("packages/c/index.ts", "export function three() { return 3; }\n"),
    ])
}

#[test]
fn npm_undeclared_cross_package_import_is_flagged() {
    let dir = npm_workspace();
    let pairs = undeclared_pairs(&run_deps(dir.path()));
    assert_eq!(pairs, vec![("@org/b".to_string(), "@org/a".to_string())], "{pairs:?}");
}

#[test]
fn npm_unused_declared_package_dep_is_flagged() {
    let dir = npm_workspace();
    let pairs = unused_pairs(&run_deps(dir.path()));
    assert_eq!(pairs, vec![("@org/a".to_string(), "@org/c".to_string())], "{pairs:?}");
}

#[test]
fn npm_workspace_internal_deps_are_not_reported_as_bloated() {
    let dir = npm_workspace();
    let names = bloated_names(&run_deps(dir.path()));
    assert!(names.is_empty(), "{names:?}");
}

fn undeclared_confidences(findings: &[AuditFinding]) -> Vec<ImportConfidence> {
    findings
        .iter()
        .filter_map(|f| match &f.kind {
            AuditKind::UndeclaredModuleDependency(e) => Some(e.confidence),
            _ => None,
        })
        .collect()
}

#[test]
fn duplicate_package_names_do_not_self_reference() {
    let dir = workspace(&[
        ("package.json", "{\"private\":true,\"workspaces\":[\"packages/a\",\"packages/b\"]}"),
        ("packages/a/package.json", "{\"name\":\"@org/dup\"}"),
        ("packages/b/package.json", "{\"name\":\"@org/dup\"}"),
        ("packages/a/index.ts", "import { x } from \"@org/dup\";\nexport const y = x;\n"),
        ("packages/b/index.ts", "export const x = 1;\n"),
    ]);
    let findings = run_deps(dir.path());
    assert!(
        !undeclared_pairs(&findings).iter().any(|(from, to)| from == to),
        "duplicate package names must not produce a self-referential reflexion edge"
    );
}

#[test]
fn npm_reflexion_findings_are_low_confidence() {
    let confs = undeclared_confidences(&run_deps(npm_workspace().path()));
    assert!(!confs.is_empty(), "the npm workspace fixture produces an undeclared finding");
    assert!(confs.iter().all(|c| *c == ImportConfidence::Low), "npm imports resolve via hoisting → Low confidence");
}

#[test]
fn cargo_reflexion_findings_are_medium_confidence() {
    let confs = undeclared_confidences(&run_deps(canonical_workspace().path()));
    assert!(!confs.is_empty());
    assert!(confs.iter().all(|c| *c == ImportConfidence::Medium), "cargo requires declaration → Medium confidence");
}

#[test]
fn npm_declared_and_imported_package_edge_is_silent() {
    let dir = npm_workspace();
    let findings = run_deps(dir.path());
    let a_to_b = ("@org/a".to_string(), "@org/b".to_string());
    assert!(!undeclared_pairs(&findings).contains(&a_to_b));
    assert!(!unused_pairs(&findings).contains(&a_to_b));
}

const GO_WORK: &str = "go 1.21\n\nuse (\n\t./mod-a\n\t./mod-b\n\t./mod-c\n)\n";
const GO_A: &str = "module example.com/a\n\ngo 1.21\n\nrequire (\n\texample.com/b v0.0.0\n\texample.com/c v0.0.0\n)\n";
const GO_B: &str = "module example.com/b\n\ngo 1.21\n";
const GO_C: &str = "module example.com/c\n\ngo 1.21\n";

fn go_workspace() -> tempfile::TempDir {
    workspace(&[
        ("go.work", GO_WORK),
        ("mod-a/go.mod", GO_A),
        ("mod-b/go.mod", GO_B),
        ("mod-c/go.mod", GO_C),
        ("mod-a/main.go", "package a\n\nimport \"example.com/b/sub\"\n\nfunc Run() {}\n"),
        ("mod-b/main.go", "package b\n\nimport \"example.com/a\"\n\nfunc Helper() {}\n"),
        ("mod-c/main.go", "package c\n\nfunc Three() {}\n"),
    ])
}

#[test]
fn go_undeclared_cross_module_import_is_flagged() {
    let dir = go_workspace();
    let pairs = undeclared_pairs(&run_deps(dir.path()));
    assert_eq!(pairs, vec![("example.com/b".to_string(), "example.com/a".to_string())], "{pairs:?}");
}

#[test]
fn go_unused_declared_module_dep_is_flagged() {
    let dir = go_workspace();
    let pairs = unused_pairs(&run_deps(dir.path()));
    assert_eq!(pairs, vec![("example.com/a".to_string(), "example.com/c".to_string())], "{pairs:?}");
}

#[test]
fn go_subpackage_import_resolves_to_module_by_prefix() {
    let dir = go_workspace();
    let findings = run_deps(dir.path());
    let a_to_b = ("example.com/a".to_string(), "example.com/b".to_string());
    assert!(!undeclared_pairs(&findings).contains(&a_to_b), "declared+imported edge stays silent");
    assert!(!unused_pairs(&findings).contains(&a_to_b));
}

#[test]
fn go_workspace_internal_deps_are_not_reported_as_bloated() {
    let dir = go_workspace();
    let names = bloated_names(&run_deps(dir.path()));
    assert!(names.is_empty(), "{names:?}");
}
