use std::path::PathBuf;

use pulse::audit::IgnoreFilter;
use pulse::calibrate::priors::PriorsBuilder;
use pulse::config::IgnoreMatcher;

use crate::common::t;

#[test]
fn bake_corpus_priors() {
    let Ok(corpus_root) = std::env::var("CALIBRATION_CORPUS") else { return };
    let thresholds = t();
    let mut builder = PriorsBuilder::default();
    for repo in repo_dirs(&PathBuf::from(&corpus_root)) {
        let matcher = IgnoreMatcher::from_patterns(&[]);
        let filter = IgnoreFilter::new(&matcher, &repo);
        let census = pulse::calibrate::collect(&repo, &thresholds, &filter);
        eprintln!(
            "censused {} ({} main files, {} test files, {} vendored)",
            repo.display(),
            census.main.len(),
            census.tests.len(),
            census.vendored_excluded.len()
        );
        builder.add_census(&census);
    }
    let table = builder.build(thresholds.cpg.enabled);
    let json = serde_json::to_string_pretty(&table).unwrap();
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/calibrate/priors.json");
    std::fs::write(&out, json).unwrap();
    eprintln!("priors written to {}", out.display());
}

fn repo_dirs(root: &std::path::Path) -> Vec<PathBuf> {
    let mut repos = Vec::new();
    for lang_dir in subdirs(root) {
        repos.extend(subdirs(&lang_dir));
    }
    repos.sort();
    repos
}

fn subdirs(dir: &std::path::Path) -> Vec<PathBuf> {
    std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect()
}
