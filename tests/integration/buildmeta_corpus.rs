use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use pulse::audit::constraint_smells;
use pulse::audit::finding::AuditKind;
use pulse::buildmeta::{self, Ecosystem};
use pulse::thresholds::Thresholds;

const NEW_MANIFESTS: &[&str] = &["composer.json", "pom.xml", "build.gradle", "build.gradle.kts", "build.zig.zon"];

#[test]
fn scan_buildmeta_corpus() {
    let Ok(list) = std::env::var("SCAN_BUILDMETA_LIST") else { return };
    let Ok(contents) = fs::read_to_string(&list) else { return };
    let dirs: BTreeSet<PathBuf> = contents
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .filter_map(|l| Path::new(l).parent().map(Path::to_path_buf))
        .collect();
    let audit = Thresholds::default().audit;
    let new_ecos = [Ecosystem::Composer, Ecosystem::Maven, Ecosystem::Gradle, Ecosystem::Zig];
    let mut manifests: BTreeMap<String, usize> = BTreeMap::new();
    let mut deps: BTreeMap<String, usize> = BTreeMap::new();
    let mut zero_dep = 0usize;
    let mut findings = 0usize;
    let mut samples: Vec<String> = Vec::new();
    for dir in &dirs {
        let meta = buildmeta::discover_one(dir);
        let mut composer_here = false;
        for manifest in meta.manifests.iter().filter(|m| new_ecos.contains(&m.ecosystem)) {
            if manifest.ecosystem == Ecosystem::Composer {
                composer_here = true;
            }
            let key = format!("{:?}", manifest.ecosystem);
            *manifests.entry(key.clone()).or_default() += 1;
            let count = manifest.deps.iter().filter(|d| !d.own).count();
            *deps.entry(key).or_default() += count;
            if count == 0 {
                zero_dep += 1;
            }
        }
        if !composer_here {
            continue;
        }
        for finding in constraint_smells::run_from(dir, &audit) {
            let AuditKind::ConstraintSmell(ev) = &finding.kind else { continue };
            let name = ev.manifest.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            if NEW_MANIFESTS.contains(&name) && ev.manifest.parent() == Some(dir.as_path()) {
                findings += 1;
                if samples.len() < 40 {
                    samples.push(format!("{name}: {}", ev.problem));
                }
            }
        }
    }
    eprintln!("=== BUILDMETA CORPUS SCAN ===");
    eprintln!("manifest dirs: {}", dirs.len());
    eprintln!("manifests by eco: {manifests:?}");
    eprintln!("deps by eco: {deps:?}");
    eprintln!("zero-dep manifest instances: {zero_dep}");
    eprintln!("constraint findings (new ecosystems): {findings}");
    for sample in &samples {
        eprintln!("  SAMPLE {sample}");
    }
}
