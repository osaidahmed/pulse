use pulse::buildmeta::{discover, DepScope, Ecosystem};
use std::path::PathBuf;

fn meta_root(eco: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures").join("buildmeta").join(eco)
}

fn dep<'a>(meta: &'a pulse::buildmeta::BuildMeta, name: &str) -> &'a pulse::buildmeta::DeclaredDep {
    meta.manifests
        .iter()
        .flat_map(|m| &m.deps)
        .find(|d| d.name == name)
        .unwrap_or_else(|| panic!("dep {name} not extracted: {:?}", meta.manifests))
}

#[test]
fn cargo_manifest_extracts_scoped_dependencies() {
    let meta = discover(&meta_root("cargo"));
    assert_eq!(dep(&meta, "serde").scope, DepScope::Deployed);
    assert_eq!(dep(&meta, "serde").constraint, "1.0");
    assert_eq!(dep(&meta, "criterion").scope, DepScope::Dev);
    assert_eq!(dep(&meta, "cc").scope, DepScope::Build);
    assert_eq!(dep(&meta, "libc").scope, DepScope::Deployed, "target-gated deps are deployed");
    assert_eq!(dep(&meta, "fancy-regex").name, "fancy-regex", "renamed deps resolve to the registry name");
    assert!(dep(&meta, "serde").line > 0, "manifest line resolved");
}

#[test]
fn cargo_lockfile_lists_resolved_versions() {
    let meta = discover(&meta_root("cargo"));
    let lock = meta.lockfiles.iter().find(|l| l.ecosystem == Ecosystem::Cargo).expect("cargo lock");
    assert!(lock.resolved.contains(&("serde".to_string(), "1.0.200".to_string())), "{:?}", lock.resolved);
}

#[test]
fn cargo_workspace_members_are_discovered() {
    let meta = discover(&meta_root("cargo_workspace"));
    let names: Vec<&str> = meta.manifests.iter().flat_map(|m| m.deps.iter().map(|d| d.name.as_str())).collect();
    assert!(names.contains(&"anyhow"), "workspace-level deps: {names:?}");
    assert!(names.contains(&"itoa"), "member manifest parsed via members list: {names:?}");
}

#[test]
fn npm_manifest_extracts_scoped_dependencies() {
    let meta = discover(&meta_root("npm"));
    assert_eq!(dep(&meta, "express").scope, DepScope::Deployed);
    assert_eq!(dep(&meta, "express").constraint, "^4.18.0");
    assert_eq!(dep(&meta, "vitest").scope, DepScope::Dev);
    assert_eq!(dep(&meta, "react").scope, DepScope::Deployed, "peer deps count as deployed");
}

#[test]
fn npm_lockfile_strips_node_modules_prefixes() {
    let meta = discover(&meta_root("npm"));
    let lock = meta.lockfiles.iter().find(|l| l.ecosystem == Ecosystem::Npm).expect("npm lock");
    assert!(lock.resolved.contains(&("express".to_string(), "4.18.2".to_string())), "{:?}", lock.resolved);
    assert!(lock.resolved.contains(&("qs".to_string(), "6.11.0".to_string())), "nested entries: {:?}", lock.resolved);
}

#[test]
fn pyproject_covers_pep621_and_poetry() {
    let meta = discover(&meta_root("python"));
    assert_eq!(dep(&meta, "requests").scope, DepScope::Deployed);
    assert_eq!(dep(&meta, "pydantic").scope, DepScope::Deployed, "extras and markers are stripped");
    assert_eq!(dep(&meta, "pytest").scope, DepScope::Dev, "optional groups are dev");
    assert_eq!(dep(&meta, "httpx").scope, DepScope::Deployed);
    assert_eq!(dep(&meta, "ruff").scope, DepScope::Dev, "poetry groups are dev");
    let names: Vec<&str> = meta.manifests.iter().flat_map(|m| m.deps.iter().map(|d| d.name.as_str())).collect();
    assert!(!names.contains(&"python"), "the interpreter constraint is not a dependency");
}

#[test]
fn requirements_lines_parse_names_and_skip_directives() {
    let meta = discover(&meta_root("python"));
    assert_eq!(dep(&meta, "flask").constraint, "==3.0.0");
    assert_eq!(dep(&meta, "scikit-learn").constraint, "");
    let names: Vec<&str> = meta.manifests.iter().flat_map(|m| m.deps.iter().map(|d| d.name.as_str())).collect();
    assert!(!names.contains(&"r"), "flag lines are skipped: {names:?}");
}

#[test]
fn go_mod_handles_single_and_block_requires() {
    let meta = discover(&meta_root("golang"));
    assert_eq!(dep(&meta, "github.com/pkg/errors").constraint, "v0.9.1");
    assert_eq!(dep(&meta, "github.com/spf13/cobra").constraint, "v1.8.0");
    assert_eq!(dep(&meta, "golang.org/x/sync").constraint, "v0.6.0", "indirect comment stripped");
}

#[test]
fn go_sum_resolves_unique_modules() {
    let meta = discover(&meta_root("golang"));
    let lock = meta.lockfiles.iter().find(|l| l.ecosystem == Ecosystem::Go).expect("go.sum");
    let errors_entries = lock.resolved.iter().filter(|(n, _)| n == "github.com/pkg/errors").count();
    assert_eq!(errors_entries, 1, "go.sum duplicates collapse: {:?}", lock.resolved);
}

#[test]
fn gemfile_walks_gem_calls_and_group_scopes() {
    let meta = discover(&meta_root("gem"));
    assert_eq!(dep(&meta, "rails").constraint, "~> 7.1");
    assert_eq!(dep(&meta, "pg").scope, DepScope::Deployed);
    assert_eq!(dep(&meta, "rspec-rails").scope, DepScope::Dev, "development group");
    assert_eq!(dep(&meta, "rspec-rails").constraint, ">= 6.0, < 7");
    assert_eq!(dep(&meta, "puma").scope, DepScope::Deployed, "production group stays deployed");
    assert!(dep(&meta, "rails").line > 0);
}

#[test]
fn gemfile_lock_reads_top_level_specs_only() {
    let meta = discover(&meta_root("gem"));
    let lock = meta.lockfiles.iter().find(|l| l.ecosystem == Ecosystem::RubyGems).expect("gem lock");
    assert!(lock.resolved.contains(&("rails".to_string(), "7.1.3".to_string())), "{:?}", lock.resolved);
    assert!(
        !lock.resolved.iter().any(|(n, _)| n == "actionpack"),
        "transitive indented entries are skipped: {:?}",
        lock.resolved
    );
}

#[test]
fn declared_names_unions_manifests_and_lockfiles() {
    let meta = discover(&meta_root("npm"));
    let names = meta.declared_names();
    assert!(names.contains("vitest"), "manifest-only dep present");
    assert!(names.contains("qs"), "lockfile-only transitive present");
}

#[test]
fn swift_package_manifest_extracts_dependencies() {
    let meta = discover(&meta_root("swift"));
    assert_eq!(dep(&meta, "Alamofire").scope, DepScope::Deployed);
    assert_eq!(dep(&meta, "Alamofire").constraint, "5.8.0");
    assert_eq!(dep(&meta, "swift-log").name, "swift-log", "url-derived name strips .git");
    assert_eq!(dep(&meta, "Renamed").name, "Renamed", "explicit name: wins over url");
    assert!(meta.manifests.iter().flat_map(|m| &m.deps).all(|d| d.name != "LocalLib"), "path: deps are local, skipped");
    assert!(dep(&meta, "Alamofire").line > 0, "manifest line resolved");
}

#[test]
fn swift_package_resolved_lists_pinned_versions() {
    let meta = discover(&meta_root("swift"));
    let lock = meta.lockfiles.iter().find(|l| l.ecosystem == Ecosystem::Swift).expect("Package.resolved");
    assert!(lock.resolved.contains(&("alamofire".to_string(), "5.8.1".to_string())), "{:?}", lock.resolved);
}

#[test]
fn empty_root_yields_empty_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let meta = discover(dir.path());
    assert!(meta.manifests.is_empty() && meta.lockfiles.is_empty());
}

#[test]
fn compile_db_extracts_defines_and_includes_from_both_forms() {
    let dir = tempfile::tempdir().unwrap();
    let ccj = "[\
      {\"directory\":\".\",\"command\":\"cc -DDEBUG -DLEVEL=2 -I/usr/inc -c a.c\",\"file\":\"/p/a.c\"},\
      {\"directory\":\".\",\"arguments\":[\"cc\",\"-D\",\"SEPARATED\",\"-I\",\"/sep/inc\",\"-c\",\"b.c\"],\"file\":\"/p/b.c\"}]";
    std::fs::write(dir.path().join("compile_commands.json"), ccj).unwrap();
    let db = pulse::buildmeta::compile_db(dir.path()).expect("compile_commands.json parsed");
    assert_eq!(db.entries.len(), 2);
    assert!(db.entries[0].defines.contains(&"DEBUG".to_string()), "{:?}", db.entries[0].defines);
    assert!(db.entries[0].defines.contains(&"LEVEL".to_string()), "=value is stripped to the macro name");
    assert!(db.entries[0].includes.contains(&PathBuf::from("/usr/inc")));
    assert!(db.entries[1].defines.contains(&"SEPARATED".to_string()), "separated `-D NAME` form");
    assert!(db.entries[1].includes.contains(&PathBuf::from("/sep/inc")), "separated `-I path` form");
}
