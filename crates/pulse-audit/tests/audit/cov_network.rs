use std::fs;
use std::sync::Mutex;

use pulse_audit::finding::AuditKind;
use pulse_audit::freshness::{assess, run_from};
use pulse_buildmeta::registry::{
    cache_dir, depsdev_system, lookup, lookup_version, parse_package, parse_version_detail, write_atomic,
    FreshnessThresholds, PackageInfo, VersionInfo, VersionKey,
};
use pulse_buildmeta::Ecosystem;

static ENV_GUARD: Mutex<()> = Mutex::new(());

fn ver(version: &str, published_at: &str, is_default: bool) -> VersionInfo {
    VersionInfo { version_key: VersionKey { version: version.into() }, published_at: published_at.into(), is_default }
}

fn pkg(versions: Vec<VersionInfo>) -> PackageInfo {
    PackageInfo { versions }
}

fn thresholds(min_missed: u32) -> FreshnessThresholds {
    FreshnessThresholds { min_missed, abandon_years: 2, max_findings: 30 }
}

#[test]
fn defaults_constant_matches_documented_values() {
    let d = FreshnessThresholds::DEFAULTS;
    assert_eq!(d.min_missed, 5);
    assert_eq!(d.abandon_years, 2);
    assert_eq!(d.max_findings, 30);
}

#[test]
fn parse_package_accepts_minimal_and_defaults_missing_fields() {
    let p = parse_package(r#"{"versions":[{"versionKey":{"version":"1.0.0"}}]}"#)
        .expect("missing publishedAt/isDefault default cleanly");
    assert_eq!(p.versions.len(), 1);
    assert_eq!(p.versions[0].published_at, "");
    assert!(!p.versions[0].is_default);
}

#[test]
fn parse_package_defaults_versions_when_absent() {
    let p = parse_package("{}").expect("empty object yields a package with no versions");
    assert!(p.versions.is_empty());
}

#[test]
fn parse_package_rejects_malformed_json() {
    assert!(parse_package("{not json").is_none());
    assert!(parse_package("").is_none());
    assert!(parse_package("42").is_none(), "a bare scalar is not a PackageInfo object");
}

#[test]
fn parse_version_detail_defaults_empty_advisory_keys() {
    let d = parse_version_detail("{}").expect("missing advisoryKeys defaults to empty");
    assert!(d.advisory_keys.is_empty());
}

#[test]
fn parse_version_detail_rejects_malformed_json() {
    assert!(parse_version_detail("garbage{").is_none());
    assert!(parse_version_detail("").is_none());
}

#[test]
fn depsdev_system_maps_supported_ecosystems() {
    assert_eq!(depsdev_system(Ecosystem::Cargo), Some("cargo"));
    assert_eq!(depsdev_system(Ecosystem::Npm), Some("npm"));
    assert_eq!(depsdev_system(Ecosystem::Pip), Some("pypi"));
    assert_eq!(depsdev_system(Ecosystem::Go), Some("go"));
    assert_eq!(depsdev_system(Ecosystem::NuGet), Some("nuget"));
}

#[test]
fn depsdev_system_returns_none_for_unmapped_ecosystems() {
    assert_eq!(depsdev_system(Ecosystem::RubyGems), None, "rubygems is not on deps.dev mapping");
    assert_eq!(depsdev_system(Ecosystem::Swift), None, "swift is not on deps.dev mapping");
}

#[test]
fn cache_dir_prefers_explicit_override_env() {
    let _guard = ENV_GUARD.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let prev_cache = std::env::var("PULSE_REGISTRY_CACHE").ok();
    std::env::set_var("PULSE_REGISTRY_CACHE", "/tmp/pulse-cov-explicit-cache");
    assert_eq!(cache_dir(), std::path::PathBuf::from("/tmp/pulse-cov-explicit-cache"));
    match prev_cache {
        Some(v) => std::env::set_var("PULSE_REGISTRY_CACHE", v),
        None => std::env::remove_var("PULSE_REGISTRY_CACHE"),
    }
}

#[test]
fn cache_dir_falls_back_to_home_cache_path() {
    let _guard = ENV_GUARD.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let prev_cache = std::env::var("PULSE_REGISTRY_CACHE").ok();
    let prev_home = std::env::var("HOME").ok();
    std::env::remove_var("PULSE_REGISTRY_CACHE");
    std::env::set_var("HOME", "/home/coverage-user");
    assert_eq!(cache_dir(), std::path::PathBuf::from("/home/coverage-user/.cache/pulse/registry"));
    match prev_cache {
        Some(v) => std::env::set_var("PULSE_REGISTRY_CACHE", v),
        None => std::env::remove_var("PULSE_REGISTRY_CACHE"),
    }
    match prev_home {
        Some(v) => std::env::set_var("HOME", v),
        None => std::env::remove_var("HOME"),
    }
}

#[test]
fn cache_dir_falls_back_to_temp_when_home_missing() {
    let _guard = ENV_GUARD.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let prev_cache = std::env::var("PULSE_REGISTRY_CACHE").ok();
    let prev_home = std::env::var("HOME").ok();
    std::env::remove_var("PULSE_REGISTRY_CACHE");
    std::env::remove_var("HOME");
    assert_eq!(cache_dir(), std::env::temp_dir().join("pulse-registry"));
    match prev_cache {
        Some(v) => std::env::set_var("PULSE_REGISTRY_CACHE", v),
        None => std::env::remove_var("PULSE_REGISTRY_CACHE"),
    }
    match prev_home {
        Some(v) => std::env::set_var("HOME", v),
        None => std::env::remove_var("HOME"),
    }
}

#[test]
fn write_atomic_returns_silently_when_parent_cannot_be_created() {
    let dir = tempfile::tempdir().unwrap();
    let blocker = dir.path().join("is-a-file");
    fs::write(&blocker, "i am a file not a dir").unwrap();
    let under_a_file = blocker.join("child/entry.json");
    write_atomic(&under_a_file, "body");
    assert!(!under_a_file.exists(), "creating a dir under an existing file fails and is swallowed");
}

#[test]
fn lookup_reads_a_prepopulated_cache_offline() {
    let cache = tempfile::tempdir().unwrap();
    fs::write(
        cache.path().join("cargo__serde.json"),
        r#"{"versions":[{"versionKey":{"version":"1.0.0"},"publishedAt":"2020-01-01T00:00:00Z","isDefault":true}]}"#,
    )
    .unwrap();
    let pkg = lookup(Ecosystem::Cargo, "serde", cache.path(), false).expect("cached package read offline");
    assert_eq!(pkg.versions.len(), 1);
    assert_eq!(pkg.versions[0].version_key.version, "1.0.0");
}

#[test]
fn lookup_returns_none_offline_with_no_cache_entry() {
    let cache = tempfile::tempdir().unwrap();
    assert!(
        lookup(Ecosystem::Cargo, "never-cached", cache.path(), false).is_none(),
        "offline and uncached must not touch the network"
    );
}

#[test]
fn lookup_returns_none_for_unmapped_ecosystem_before_any_cache_access() {
    let cache = tempfile::tempdir().unwrap();
    assert!(lookup(Ecosystem::RubyGems, "rails", cache.path(), false).is_none());
    assert!(lookup(Ecosystem::Swift, "Alamofire", cache.path(), false).is_none());
}

#[test]
fn lookup_encodes_special_characters_in_the_cache_filename() {
    let cache = tempfile::tempdir().unwrap();
    fs::write(
        cache.path().join("npm__%40scope%2Fpkg.json"),
        r#"{"versions":[{"versionKey":{"version":"2.0.0"},"publishedAt":"2021-01-01T00:00:00Z","isDefault":true}]}"#,
    )
    .unwrap();
    let pkg =
        lookup(Ecosystem::Npm, "@scope/pkg", cache.path(), false).expect("encoded name resolves to the cache file");
    assert_eq!(pkg.versions[0].version_key.version, "2.0.0");
}

#[test]
fn lookup_version_reads_a_prepopulated_cache_offline() {
    let cache = tempfile::tempdir().unwrap();
    fs::write(cache.path().join("npm__lodash__4.17.11.json"), r#"{"advisoryKeys":[{"id":"GHSA-xxxx"}]}"#).unwrap();
    let detail =
        lookup_version(Ecosystem::Npm, "lodash", "4.17.11", cache.path(), false).expect("cached detail read offline");
    assert_eq!(detail.advisory_keys.len(), 1);
    assert_eq!(detail.advisory_keys[0].id, "GHSA-xxxx");
}

#[test]
fn lookup_version_returns_none_offline_with_no_cache_entry() {
    let cache = tempfile::tempdir().unwrap();
    assert!(lookup_version(Ecosystem::Npm, "lodash", "9.9.9", cache.path(), false).is_none());
}

#[test]
fn lookup_version_returns_none_for_unmapped_ecosystem() {
    let cache = tempfile::tempdir().unwrap();
    assert!(lookup_version(Ecosystem::RubyGems, "rails", "7.0.0", cache.path(), false).is_none());
}

#[test]
fn lookup_version_encodes_special_characters_in_both_name_and_version() {
    let cache = tempfile::tempdir().unwrap();
    fs::write(cache.path().join("pypi__a%2Bb__1.0.0%2Bbuild.json"), r#"{"advisoryKeys":[{"id":"GHSA-encoded"}]}"#)
        .unwrap();
    let detail = lookup_version(Ecosystem::Pip, "a+b", "1.0.0+build", cache.path(), false)
        .expect("plus signs are percent-encoded in the cache filename");
    assert_eq!(detail.advisory_keys[0].id, "GHSA-encoded");
}

#[test]
fn assess_returns_none_when_current_version_is_absent_from_registry() {
    let p = pkg(vec![ver("1.0", "2020-01-01T00:00:00Z", false), ver("2.0", "2024-01-01T00:00:00Z", true)]);
    assert!(assess(&p, "0.9", 2024, &thresholds(1)).is_none(), "an unknown current version yields no verdict");
}

#[test]
fn assess_returns_none_on_empty_registry() {
    let p = pkg(vec![]);
    assert!(assess(&p, "1.0", 2024, &thresholds(1)).is_none(), "no versions at all -> no current -> none");
}

#[test]
fn assess_returns_none_when_no_default_and_no_dated_versions_for_latest() {
    let p = pkg(vec![ver("1.0", "", false)]);
    assert!(
        assess(&p, "1.0", 2024, &thresholds(1)).is_none(),
        "current exists but there is no datable/default latest -> none"
    );
}

#[test]
fn assess_picks_latest_by_date_when_no_default_flag() {
    let p = pkg(vec![
        ver("1.0", "2020-01-01T00:00:00Z", false),
        ver("3.0", "2024-06-01T00:00:00Z", false),
        ver("2.0", "2022-01-01T00:00:00Z", false),
    ]);
    let v = assess(&p, "1.0", 2024, &thresholds(1)).expect("missed two releases behind the dated max");
    assert_eq!(v.latest_version, "3.0", "max-by-date wins absent an isDefault flag");
    assert_eq!(v.missed_releases, 2);
    assert!(!v.abandoned, "the latest is recent relative to now_year");
}

#[test]
fn assess_flags_abandoned_at_the_boundary_year() {
    let p = pkg(vec![ver("1.0", "2022-01-01T00:00:00Z", true)]);
    let v = assess(&p, "1.0", 2024, &thresholds(99)).expect("exactly abandon_years old is abandoned");
    assert!(v.abandoned, "now_year - year == abandon_years crosses the >= boundary");
}

#[test]
fn assess_is_silent_just_under_both_thresholds() {
    let p = pkg(vec![ver("1.0", "2023-06-01T00:00:00Z", false), ver("2.0", "2023-12-01T00:00:00Z", true)]);
    assert!(
        assess(&p, "1.0", 2024, &thresholds(5)).is_none(),
        "one missed release (< min_missed) and not abandoned -> none"
    );
}

#[test]
fn assess_ignores_releases_with_blank_dates_when_counting_missed() {
    let p = pkg(vec![
        ver("1.0", "2020-01-01T00:00:00Z", false),
        ver("1.5", "", false),
        ver("2.0", "2024-01-01T00:00:00Z", true),
    ]);
    let v = assess(&p, "1.0", 2024, &thresholds(1)).expect("a single dated newer release is missed");
    assert_eq!(v.missed_releases, 1, "blank-dated versions do not count toward missed releases");
}

#[test]
fn run_from_is_silent_when_the_locked_version_is_already_the_recent_latest() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n\n[dependencies]\nserde = \"1\"\n",
    )
    .unwrap();
    fs::write(dir.path().join("Cargo.lock"), "version = 3\n\n[[package]]\nname = \"serde\"\nversion = \"2.0.0\"\n")
        .unwrap();
    let cache = tempfile::tempdir().unwrap();
    fs::write(
        cache.path().join("cargo__serde.json"),
        r#"{"versions":[{"versionKey":{"version":"1.0.0"},"publishedAt":"2098-01-01T00:00:00Z","isDefault":false},{"versionKey":{"version":"2.0.0"},"publishedAt":"2099-01-01T00:00:00Z","isDefault":true}]}"#,
    )
    .unwrap();
    let findings = run_from(dir.path(), cache.path(), false, &thresholds(1));
    let outdated = findings.iter().filter(|f| matches!(f.kind, AuditKind::OutdatedDependency(_))).count();
    assert_eq!(outdated, 0, "locked == recent latest -> no outdated finding");
}

#[test]
fn run_from_yields_nothing_when_no_cache_and_offline() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n\n[dependencies]\nserde = \"1\"\n",
    )
    .unwrap();
    fs::write(dir.path().join("Cargo.lock"), "version = 3\n\n[[package]]\nname = \"serde\"\nversion = \"1.0.0\"\n")
        .unwrap();
    let cache = tempfile::tempdir().unwrap();
    let findings = run_from(dir.path(), cache.path(), false, &thresholds(1));
    assert!(
        findings.iter().all(|f| !matches!(f.kind, AuditKind::OutdatedDependency(_))),
        "offline with an empty cache never produces outdated findings"
    );
}

#[test]
fn run_from_on_an_empty_project_returns_no_findings() {
    let dir = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let findings = run_from(dir.path(), cache.path(), false, &thresholds(1));
    assert!(findings.is_empty(), "a project with no manifests has nothing to assess");
}
