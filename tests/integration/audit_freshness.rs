use std::fs;

use pulse::audit::finding::AuditKind;
use pulse::audit::freshness::{assess, run_from};
use pulse::audit::vuln_deps;
use pulse::registry::{parse_package, parse_version_detail, FreshnessThresholds, PackageInfo, VersionInfo, VersionKey};

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
fn assess_flags_a_version_behind_latest() {
    let p = pkg(vec![ver("1.0", "2020-01-01T00:00:00Z", false), ver("2.0", "2024-01-01T00:00:00Z", true)]);
    let v = assess(&p, "1.0", 2024, &thresholds(1)).expect("a version behind the latest is flagged");
    assert_eq!(v.latest_version, "2.0");
    assert_eq!(v.current_version, "1.0");
    assert_eq!(v.missed_releases, 1);
}

#[test]
fn assess_is_silent_when_current_is_the_recent_latest() {
    let p = pkg(vec![ver("1.0", "2020-01-01T00:00:00Z", false), ver("2.0", "2024-01-01T00:00:00Z", true)]);
    assert!(assess(&p, "2.0", 2024, &thresholds(1)).is_none(), "current == latest and recent → not outdated");
}

#[test]
fn assess_flags_abandoned_even_at_latest() {
    let p = pkg(vec![ver("1.0", "2018-01-01T00:00:00Z", true)]);
    let v = assess(&p, "1.0", 2026, &thresholds(99)).expect("an old latest release is abandoned");
    assert!(v.abandoned);
    assert_eq!(v.missed_releases, 0);
}

#[test]
fn parse_handles_depsdev_response_shape() {
    let p = parse_package(
        r#"{"versions":[{"versionKey":{"system":"CARGO","name":"serde","version":"1.0.0"},"publishedAt":"2020-01-01T00:00:00Z","isDefault":true}]}"#,
    )
    .expect("valid deps.dev json");
    assert_eq!(p.versions.len(), 1);
    assert_eq!(p.versions[0].version_key.version, "1.0.0");
    assert!(p.versions[0].is_default);
}

#[test]
fn run_from_emits_outdated_from_a_cached_registry_without_network() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n\n[dependencies]\nserde = \"1\"\n",
    )
    .unwrap();
    fs::write(dir.path().join("Cargo.lock"), "version = 3\n\n[[package]]\nname = \"serde\"\nversion = \"1.0.100\"\n")
        .unwrap();

    let cache = tempfile::tempdir().unwrap();
    fs::write(
        cache.path().join("cargo__serde.json"),
        r#"{"versions":[{"versionKey":{"version":"1.0.100"},"publishedAt":"2019-08-01T00:00:00Z","isDefault":false},{"versionKey":{"version":"1.0.200"},"publishedAt":"2024-01-01T00:00:00Z","isDefault":true}]}"#,
    )
    .unwrap();

    let findings = run_from(dir.path(), cache.path(), false, &thresholds(1));
    let outdated: Vec<_> = findings
        .iter()
        .filter_map(|f| match &f.kind {
            AuditKind::OutdatedDependency(e) => Some(e),
            _ => None,
        })
        .collect();
    assert_eq!(outdated.len(), 1, "{findings:#?}");
    assert_eq!(outdated[0].name, "serde");
    assert_eq!(outdated[0].current_version, "1.0.100");
    assert_eq!(outdated[0].latest_version, "1.0.200");
    assert_eq!(outdated[0].missed_releases, 1);
}

#[test]
fn parse_version_detail_reads_advisory_keys() {
    let d = parse_version_detail(r#"{"advisoryKeys":[{"id":"GHSA-aaaa"},{"id":"GHSA-bbbb"}]}"#)
        .expect("valid version detail");
    assert_eq!(d.advisory_keys.len(), 2);
    assert_eq!(d.advisory_keys[0].id, "GHSA-aaaa");
}

#[test]
fn vuln_deps_flags_a_cached_vulnerable_version_without_network() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("package.json"), r#"{"name":"app","dependencies":{"lodash":"^4.17.0"}}"#).unwrap();
    fs::write(
        dir.path().join("package-lock.json"),
        r#"{"lockfileVersion":3,"packages":{"node_modules/lodash":{"version":"4.17.11"}}}"#,
    )
    .unwrap();

    let cache = tempfile::tempdir().unwrap();
    fs::write(
        cache.path().join("npm__lodash__4.17.11.json"),
        r#"{"advisoryKeys":[{"id":"GHSA-jf85-cpcp-j695"},{"id":"GHSA-p6mc-m468-83gw"}]}"#,
    )
    .unwrap();

    let findings = vuln_deps::run_from(dir.path(), cache.path(), false, 30);
    let vulns: Vec<_> = findings
        .iter()
        .filter_map(|f| match &f.kind {
            AuditKind::VulnerableDependency(e) => Some(e),
            _ => None,
        })
        .collect();
    assert_eq!(vulns.len(), 1, "{findings:#?}");
    assert_eq!(vulns[0].name, "lodash");
    assert_eq!(vulns[0].version, "4.17.11");
    assert_eq!(vulns[0].advisory_ids.len(), 2);
}
