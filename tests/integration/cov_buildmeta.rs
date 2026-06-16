use pulse::buildmeta::declared::{declared_names_for, manifest_names, nearest_manifest_root};
use pulse::buildmeta::{discover, DepScope, Ecosystem};
use std::path::PathBuf;

fn write_pkg(dir: &std::path::Path, body: &str) {
    std::fs::write(dir.join("package.json"), body).unwrap();
}

fn dep_named<'a>(meta: &'a pulse::buildmeta::BuildMeta, name: &str) -> &'a pulse::buildmeta::DeclaredDep {
    meta.manifests
        .iter()
        .flat_map(|m| &m.deps)
        .find(|d| d.name == name)
        .unwrap_or_else(|| panic!("dep {name} not extracted: {:?}", meta.manifests))
}

fn dep_names(meta: &pulse::buildmeta::BuildMeta) -> Vec<String> {
    meta.manifests.iter().flat_map(|m| m.deps.iter().map(|d| d.name.clone())).collect()
}

#[test]
fn npm_manifest_maps_every_dependency_table_to_a_scope() {
    let dir = tempfile::tempdir().unwrap();
    write_pkg(
        dir.path(),
        r#"{
          "name": "self-pkg",
          "dependencies": { "left-pad": "^1.0.0" },
          "peerDependencies": { "react": "^18.0.0" },
          "optionalDependencies": { "fsevents": "^2.0.0" },
          "devDependencies": { "jest": "^29.0.0" }
        }"#,
    );
    let meta = discover(dir.path());
    assert_eq!(dep_named(&meta, "left-pad").scope, DepScope::Deployed, "dependencies are deployed");
    assert_eq!(dep_named(&meta, "react").scope, DepScope::Deployed, "peerDependencies are deployed");
    assert_eq!(dep_named(&meta, "fsevents").scope, DepScope::Deployed, "optionalDependencies are deployed");
    assert_eq!(dep_named(&meta, "jest").scope, DepScope::Dev, "devDependencies are dev-scoped");
    assert_eq!(dep_named(&meta, "left-pad").constraint, "^1.0.0", "string constraint is captured");
}

#[test]
fn npm_manifest_records_own_name_as_a_self_dep() {
    let dir = tempfile::tempdir().unwrap();
    write_pkg(dir.path(), r#"{ "name": "my-package", "dependencies": { "lodash": "^4.0.0" } }"#);
    let meta = discover(dir.path());
    let own = dep_named(&meta, "my-package");
    assert!(own.own, "the manifest's own name is flagged as own");
    assert_eq!(own.constraint, "", "own dep carries no constraint");
    assert_eq!(own.line, 0, "own dep has no resolved line");
    assert_eq!(own.scope, DepScope::Deployed, "own dep is deployed-scoped");
}

#[test]
fn npm_manifest_without_name_omits_self_dep() {
    let dir = tempfile::tempdir().unwrap();
    write_pkg(dir.path(), r#"{ "dependencies": { "axios": "^1.0.0" } }"#);
    let meta = discover(dir.path());
    assert!(meta.manifests.iter().flat_map(|m| &m.deps).all(|d| !d.own), "no own dep when name is absent");
    assert_eq!(dep_named(&meta, "axios").constraint, "^1.0.0");
}

#[test]
fn npm_manifest_non_string_constraint_falls_back_to_empty() {
    let dir = tempfile::tempdir().unwrap();
    write_pkg(dir.path(), r#"{ "dependencies": { "weird": 42 } }"#);
    let meta = discover(dir.path());
    assert_eq!(dep_named(&meta, "weird").constraint, "", "non-string version defaults to empty constraint");
}

#[test]
fn npm_manifest_with_no_dependency_tables_yields_zero_deps() {
    let dir = tempfile::tempdir().unwrap();
    write_pkg(dir.path(), r#"{ "version": "1.0.0", "scripts": { "build": "tsc" } }"#);
    let meta = discover(dir.path());
    let npm = meta.manifests.iter().find(|m| m.ecosystem == Ecosystem::Npm).expect("npm manifest parsed");
    assert!(npm.deps.is_empty(), "a manifest with no dependency tables and no name has no deps");
    assert!(npm.workspace_members.is_empty(), "no workspaces key means no members");
}

#[test]
fn npm_workspaces_array_form_expands_explicit_members() {
    let dir = tempfile::tempdir().unwrap();
    write_pkg(dir.path(), r#"{ "name": "root", "workspaces": ["packages/app"] }"#);
    let member = dir.path().join("packages").join("app");
    std::fs::create_dir_all(&member).unwrap();
    write_pkg(&member, r#"{ "name": "app", "dependencies": { "vue": "^3.0.0" } }"#);
    let meta = discover(dir.path());
    assert!(
        dep_names(&meta).contains(&"vue".to_string()),
        "array workspace member manifest is parsed: {:?}",
        dep_names(&meta)
    );
}

#[test]
fn npm_workspaces_object_packages_form_expands_members() {
    let dir = tempfile::tempdir().unwrap();
    write_pkg(dir.path(), r#"{ "name": "root", "workspaces": { "packages": ["libs/core"], "nohoist": ["x"] } }"#);
    let member = dir.path().join("libs").join("core");
    std::fs::create_dir_all(&member).unwrap();
    write_pkg(&member, r#"{ "name": "core", "dependencies": { "rxjs": "^7.0.0" } }"#);
    let meta = discover(dir.path());
    assert!(
        dep_names(&meta).contains(&"rxjs".to_string()),
        "object.packages workspace member is parsed: {:?}",
        dep_names(&meta)
    );
}

#[test]
fn npm_workspaces_glob_form_discovers_member_directories() {
    let dir = tempfile::tempdir().unwrap();
    write_pkg(dir.path(), r#"{ "name": "root", "workspaces": ["packages/*"] }"#);
    for (m, d) in [("alpha", "preact"), ("beta", "svelte")] {
        let md = dir.path().join("packages").join(m);
        std::fs::create_dir_all(&md).unwrap();
        write_pkg(&md, &format!(r#"{{ "name": "{m}", "dependencies": {{ "{d}": "^1.0.0" }} }}"#));
    }
    let meta = discover(dir.path());
    let names = dep_names(&meta);
    assert!(names.contains(&"preact".to_string()), "glob expands first member: {names:?}");
    assert!(names.contains(&"svelte".to_string()), "glob expands second member: {names:?}");
}

#[test]
fn npm_malformed_manifest_is_skipped_silently() {
    let dir = tempfile::tempdir().unwrap();
    write_pkg(dir.path(), "{ this is not json ");
    let meta = discover(dir.path());
    assert!(
        meta.manifests.iter().all(|m| m.ecosystem != Ecosystem::Npm),
        "invalid JSON yields no npm manifest: {:?}",
        meta.manifests
    );
}

#[test]
fn npm_lockfile_v3_packages_strips_node_modules_prefix() {
    let dir = tempfile::tempdir().unwrap();
    write_pkg(dir.path(), r#"{ "name": "root", "dependencies": { "express": "^4.0.0" } }"#);
    std::fs::write(
        dir.path().join("package-lock.json"),
        r#"{
          "lockfileVersion": 3,
          "packages": {
            "": { "name": "root", "version": "1.0.0" },
            "node_modules/express": { "version": "4.19.2" },
            "node_modules/express/node_modules/qs": { "version": "6.12.0" },
            "node_modules/noversion": {}
          }
        }"#,
    )
    .unwrap();
    let meta = discover(dir.path());
    let lock = meta.lockfiles.iter().find(|l| l.ecosystem == Ecosystem::Npm).expect("npm lock parsed");
    assert!(lock.resolved.contains(&("express".to_string(), "4.19.2".to_string())), "{:?}", lock.resolved);
    assert!(
        lock.resolved.contains(&("qs".to_string(), "6.12.0".to_string())),
        "nested node_modules collapses to leaf name: {:?}",
        lock.resolved
    );
    assert!(
        lock.resolved.contains(&("noversion".to_string(), String::new())),
        "missing version defaults to empty string: {:?}",
        lock.resolved
    );
    assert!(
        !lock.resolved.iter().any(|(n, _)| n == "root"),
        "the empty root key (no node_modules) is filtered out: {:?}",
        lock.resolved
    );
}

#[test]
fn npm_lockfile_v1_dependencies_records_top_level_versions() {
    let dir = tempfile::tempdir().unwrap();
    write_pkg(dir.path(), r#"{ "name": "root", "dependencies": { "lodash": "^4.0.0" } }"#);
    std::fs::write(
        dir.path().join("package-lock.json"),
        r#"{
          "lockfileVersion": 1,
          "dependencies": {
            "lodash": { "version": "4.17.21" },
            "blank": {}
          }
        }"#,
    )
    .unwrap();
    let meta = discover(dir.path());
    let lock = meta.lockfiles.iter().find(|l| l.ecosystem == Ecosystem::Npm).expect("npm lock parsed");
    assert!(lock.resolved.contains(&("lodash".to_string(), "4.17.21".to_string())), "{:?}", lock.resolved);
    assert!(
        lock.resolved.contains(&("blank".to_string(), String::new())),
        "v1 entry without version defaults to empty: {:?}",
        lock.resolved
    );
}

#[test]
fn npm_lockfile_without_known_tables_yields_no_resolved_entries() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("package-lock.json"), r#"{ "lockfileVersion": 2, "name": "x" }"#).unwrap();
    let meta = discover(dir.path());
    let lock = meta.lockfiles.iter().find(|l| l.ecosystem == Ecosystem::Npm).expect("npm lock parsed");
    assert!(lock.resolved.is_empty(), "neither packages nor dependencies present: {:?}", lock.resolved);
}

#[test]
fn npm_malformed_lockfile_is_skipped() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("package-lock.json"), "}{ broken").unwrap();
    let meta = discover(dir.path());
    assert!(
        meta.lockfiles.iter().all(|l| l.ecosystem != Ecosystem::Npm),
        "invalid lockfile JSON produces no npm lockfile: {:?}",
        meta.lockfiles
    );
}

#[test]
fn manifest_names_returns_expected_files_per_known_ecosystem() {
    assert_eq!(manifest_names(Ecosystem::Cargo), &["Cargo.toml"]);
    assert_eq!(manifest_names(Ecosystem::Npm), &["package.json"]);
    assert_eq!(manifest_names(Ecosystem::Pip), &["pyproject.toml", "requirements.txt"]);
    assert_eq!(manifest_names(Ecosystem::Go), &["go.mod"]);
    assert_eq!(manifest_names(Ecosystem::RubyGems), &["Gemfile"]);
}

#[test]
fn manifest_names_is_empty_for_ecosystems_without_a_table_entry() {
    assert!(manifest_names(Ecosystem::NuGet).is_empty(), "NuGet has no entry in the manifest table");
    assert!(manifest_names(Ecosystem::Swift).is_empty(), "Swift has no entry in the manifest table");
}

#[test]
fn nearest_manifest_root_finds_manifest_in_same_directory() {
    let dir = tempfile::tempdir().unwrap();
    write_pkg(dir.path(), r#"{ "name": "root" }"#);
    let file = dir.path().join("index.js");
    std::fs::write(&file, "module.exports = {};\n").unwrap();
    let root = nearest_manifest_root(&file, Ecosystem::Npm).expect("manifest discovered beside the file");
    assert_eq!(root, dir.path(), "the file's own directory holds the manifest");
}

#[test]
fn nearest_manifest_root_walks_up_to_an_ancestor() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
    let deep = dir.path().join("src").join("a").join("b");
    std::fs::create_dir_all(&deep).unwrap();
    let file = deep.join("mod.rs");
    std::fs::write(&file, "fn x() {}\n").unwrap();
    let root = nearest_manifest_root(&file, Ecosystem::Cargo).expect("ancestor manifest found");
    assert_eq!(root, dir.path(), "the walk climbs to the directory that owns Cargo.toml");
}

#[test]
fn nearest_manifest_root_is_none_when_no_manifest_exists() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("orphan.go");
    std::fs::write(&file, "package main\n").unwrap();
    assert!(nearest_manifest_root(&file, Ecosystem::Go).is_none(), "no go.mod anywhere above the file");
}

#[test]
fn nearest_manifest_root_does_not_match_a_different_ecosystem_manifest() {
    let dir = tempfile::tempdir().unwrap();
    write_pkg(dir.path(), r#"{ "name": "root" }"#);
    let file = dir.path().join("main.rs");
    std::fs::write(&file, "fn main() {}\n").unwrap();
    assert!(
        nearest_manifest_root(&file, Ecosystem::Cargo).is_none(),
        "a package.json does not satisfy a Cargo manifest search"
    );
}

#[test]
fn nearest_manifest_root_handles_a_file_without_a_parent() {
    let bare = PathBuf::from("nonexistent-root.js");
    assert!(
        nearest_manifest_root(&bare, Ecosystem::Npm).is_none(),
        "a relative leaf whose parent has no manifest resolves to None"
    );
}

#[test]
fn declared_names_for_computes_then_serves_from_cache() {
    let baseline = tempfile::tempdir().unwrap();
    std::env::set_var("PULSE_BASELINE_DIR", baseline.path());

    let project = tempfile::tempdir().unwrap();
    write_pkg(project.path(), r#"{ "name": "proj", "dependencies": { "chalk": "^5.0.0" } }"#);

    let first = declared_names_for(project.path());
    assert!(first.contains("chalk"), "fresh discovery captures the declared dep: {first:?}");

    let cache_files: Vec<_> = std::fs::read_dir(baseline.path())
        .unwrap()
        .filter_map(std::result::Result::ok)
        .filter(|e| e.path().extension().is_some_and(|x| x == "buildmeta"))
        .collect();
    assert_eq!(cache_files.len(), 1, "a single .buildmeta cache record is written");

    let second = declared_names_for(project.path());
    assert_eq!(first, second, "the cached result matches the fresh discovery");

    std::env::remove_var("PULSE_BASELINE_DIR");
}

#[test]
fn declared_names_for_recomputes_when_the_root_stamp_changes() {
    let baseline = tempfile::tempdir().unwrap();
    std::env::set_var("PULSE_BASELINE_DIR", baseline.path());

    let project = tempfile::tempdir().unwrap();
    write_pkg(project.path(), r#"{ "name": "proj", "dependencies": { "alpha": "^1.0.0" } }"#);
    let before = declared_names_for(project.path());
    assert!(before.contains("alpha"), "initial discovery: {before:?}");

    std::thread::sleep(std::time::Duration::from_millis(1100));
    write_pkg(project.path(), r#"{ "name": "proj", "dependencies": { "alpha": "^1.0.0", "beta": "^2.0.0" } }"#);

    let after = declared_names_for(project.path());
    assert!(after.contains("beta"), "a stale stamp forces a recompute that sees the new dep: {after:?}");

    std::env::remove_var("PULSE_BASELINE_DIR");
}

#[test]
fn declared_names_for_empty_root_returns_empty_set() {
    let baseline = tempfile::tempdir().unwrap();
    std::env::set_var("PULSE_BASELINE_DIR", baseline.path());

    let project = tempfile::tempdir().unwrap();
    let names = declared_names_for(project.path());
    assert!(names.is_empty(), "a root with no manifests declares nothing: {names:?}");

    std::env::remove_var("PULSE_BASELINE_DIR");
}
