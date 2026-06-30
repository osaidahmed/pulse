use std::fs;

use pulse_audit::finding::{AuditFinding, AuditKind};
use pulse_audit::freshness::run_from;
use pulse_buildmeta::registry::{write_atomic, FreshnessThresholds};

fn thresholds(min_missed: u32) -> FreshnessThresholds {
    FreshnessThresholds { min_missed, abandon_years: 2, max_findings: 30 }
}

fn outdated_names(findings: &[AuditFinding]) -> Vec<String> {
    findings
        .iter()
        .filter_map(|f| match &f.kind {
            AuditKind::OutdatedDependency(e) => Some(e.name.clone()),
            _ => None,
        })
        .collect()
}

fn cargo_project_with_cached_deps(n: usize) -> (tempfile::TempDir, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let mut cargo_toml = String::from("[package]\nname = \"demo\"\nversion = \"0.1.0\"\n\n[dependencies]\n");
    let mut cargo_lock = String::from("version = 3\n");
    let entry = r#"{"versions":[{"versionKey":{"version":"1.0.0"},"publishedAt":"2018-01-01T00:00:00Z","isDefault":false},{"versionKey":{"version":"2.0.0"},"publishedAt":"2024-01-01T00:00:00Z","isDefault":true}]}"#;
    for i in 0..n {
        let name = format!("dep{i:02}");
        cargo_toml.push_str(&format!("{name} = \"1\"\n"));
        cargo_lock.push_str(&format!("\n[[package]]\nname = \"{name}\"\nversion = \"1.0.0\"\n"));
        fs::write(cache.path().join(format!("cargo__{name}.json")), entry).unwrap();
    }
    fs::write(dir.path().join("Cargo.toml"), cargo_toml).unwrap();
    fs::write(dir.path().join("Cargo.lock"), cargo_lock).unwrap();
    (dir, cache)
}

#[test]
fn run_from_drives_the_progress_path_when_forced_over_the_min_deps() {
    let n = 25;
    let (dir, cache) = cargo_project_with_cached_deps(n);

    std::env::set_var("PULSE_FORCE_PROGRESS", "1");
    let findings = run_from(dir.path(), cache.path(), false, &thresholds(1));
    std::env::remove_var("PULSE_FORCE_PROGRESS");

    let names = outdated_names(&findings);
    assert_eq!(names.len(), n, "all {n} cached deps are flagged outdated while progress is active");
    let mut sorted = names.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), n, "each dep is reported exactly once through the locked progress loop");
}

#[test]
fn forced_progress_run_is_deterministic_across_repeats() {
    let n = 24;
    let (dir, cache) = cargo_project_with_cached_deps(n);

    std::env::set_var("PULSE_FORCE_PROGRESS", "1");
    let r1 = run_from(dir.path(), cache.path(), false, &thresholds(1));
    let r2 = run_from(dir.path(), cache.path(), false, &thresholds(1));
    std::env::remove_var("PULSE_FORCE_PROGRESS");

    let mut a = outdated_names(&r1);
    let mut b = outdated_names(&r2);
    a.sort();
    b.sort();
    assert_eq!(a, b, "the progress-active parallel run produces identical output across repeats");
    assert_eq!(a.len(), n, "every cached dep is accounted for under forced progress");
}

#[test]
fn write_atomic_creates_parents_and_renames_into_place() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("deep/nested/cargo__serde.json");
    write_atomic(&path, "{\"versions\":[]}");
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "{\"versions\":[]}",
        "write_atomic builds the missing parent chain and renames the tmp file into place"
    );
}

#[test]
fn write_atomic_overwrites_an_existing_entry_via_rename() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cargo__serde.json");
    write_atomic(&path, "old");
    write_atomic(&path, "new");
    assert_eq!(fs::read_to_string(&path).unwrap(), "new", "a second write atomically replaces the prior contents");
}

#[test]
fn write_atomic_leaves_no_tmp_artifacts_in_the_cache_dir() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cargo__rayon.json");
    write_atomic(&path, "body");
    let leftovers: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().starts_with(".pulse-tmp"))
        .collect();
    assert!(leftovers.is_empty(), "the rename consumes the tmp file, leaving only the final cache entry");
}
