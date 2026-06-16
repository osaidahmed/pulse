use std::fs;

use pulse::audit::corpus::Corpus;
use pulse::audit::finding::{AuditKind, ImportConfidence};
use pulse::audit::freshness::{assess, run_from};
use pulse::audit::vuln_deps;
use pulse::parse::Language;
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
fn progress_only_for_many_deps_on_a_terminal() {
    use pulse::audit::freshness::should_show_progress;
    assert!(!should_show_progress(20, true), "at the threshold (not over) -> no progress");
    assert!(should_show_progress(21, true), "over the threshold on a tty -> progress");
    assert!(!should_show_progress(1000, false), "never on a non-terminal (piped/CI)");
    assert!(!should_show_progress(5, true), "few deps -> no progress");
}

#[test]
fn progress_helpers_write_to_stderr_without_panic_and_honor_force_env() {
    use pulse::audit::progress;
    progress::show("  checking dependencies 1/2");
    progress::clear();
    assert!(!progress::is_active(), "no force env and a non-terminal test harness -> inactive");
    std::env::set_var("PULSE_FORCE_PROGRESS", "1");
    assert!(progress::is_active(), "the force env overrides the terminal check");
    std::env::remove_var("PULSE_FORCE_PROGRESS");
}

#[test]
fn parallel_lookups_are_deterministic_over_many_cached_deps() {
    let dir = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let n = 30;
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

    let names = |fs: &[pulse::audit::finding::AuditFinding]| -> Vec<String> {
        fs.iter()
            .filter_map(|f| match &f.kind {
                AuditKind::OutdatedDependency(e) => Some(e.name.clone()),
                _ => None,
            })
            .collect()
    };
    let r1 = run_from(dir.path(), cache.path(), false, &thresholds(1));
    let r2 = run_from(dir.path(), cache.path(), false, &thresholds(1));
    assert_eq!(r1.len(), n, "all {n} cached deps are outdated");
    assert_eq!(names(&r1), names(&r2), "parallel lookups produce identical, deterministic output");
}

#[test]
fn cache_write_round_trips_and_overwrites() {
    use pulse::registry::write_atomic;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested/entry.json");
    write_atomic(&path, "first");
    assert_eq!(fs::read_to_string(&path).unwrap(), "first", "creates parent dirs and writes");
    write_atomic(&path, "second");
    assert_eq!(fs::read_to_string(&path).unwrap(), "second", "atomic overwrite");
}

#[test]
fn cache_write_is_safe_under_concurrent_writers() {
    use pulse::registry::write_atomic;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("entry.json");
    std::thread::scope(|s| {
        for _ in 0..16 {
            s.spawn(|| write_atomic(&path, "{\"complete\":true}"));
        }
    });
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "{\"complete\":true}",
        "concurrent writers never leave a partial/corrupt cache file"
    );
}

#[test]
fn parse_version_detail_reads_advisory_keys() {
    let d = parse_version_detail(r#"{"advisoryKeys":[{"id":"GHSA-aaaa"},{"id":"GHSA-bbbb"}]}"#)
        .expect("valid version detail");
    assert_eq!(d.advisory_keys.len(), 2);
    assert_eq!(d.advisory_keys[0].id, "GHSA-aaaa");
}

fn lodash_project() -> (tempfile::TempDir, tempfile::TempDir) {
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
    (dir, cache)
}

fn vuln_evidence(findings: &[pulse::audit::finding::AuditFinding]) -> Vec<&pulse::audit::finding::VulnDepEvidence> {
    findings
        .iter()
        .filter_map(|f| match &f.kind {
            AuditKind::VulnerableDependency(e) => Some(e),
            _ => None,
        })
        .collect()
}

#[test]
fn vuln_deps_flags_a_reachable_cached_vulnerable_version_without_network() {
    let (dir, cache) = lodash_project();
    fs::write(dir.path().join("index.js"), "const _ = require('lodash');\n_.merge({}, {});\n").unwrap();
    let corpus = Corpus::load(&[(dir.path().join("index.js"), Language::JavaScript)]);

    let findings = vuln_deps::run_from(dir.path(), &corpus, cache.path(), false, 30);
    let vulns = vuln_evidence(&findings);
    assert_eq!(vulns.len(), 1);
    assert_eq!(vulns[0].name, "lodash");
    assert_eq!(vulns[0].version, "4.17.11");
    assert_eq!(vulns[0].advisory_ids.len(), 2);
    assert!(vulns[0].reachable, "lodash is imported, so the advisory is reachable");
    assert_eq!(vulns[0].confidence, ImportConfidence::High);
}

#[test]
fn vuln_on_an_unimported_dependency_is_downgraded_as_unreachable() {
    let (dir, cache) = lodash_project();
    let corpus = Corpus::load(&[]);

    let findings = vuln_deps::run_from(dir.path(), &corpus, cache.path(), false, 30);
    let vulns = vuln_evidence(&findings);
    assert_eq!(vulns.len(), 1);
    assert!(!vulns[0].reachable, "lodash is never imported, so the advisory is unreachable");
    assert_eq!(vulns[0].confidence, ImportConfidence::Low);
}
