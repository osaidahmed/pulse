use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

use pulse::audit::IgnoreFilter;
use pulse::calibrate::priors::PriorsBuilder;
use pulse::config::IgnoreMatcher;

use crate::common::t;

const REPO_BUDGET: Duration = Duration::from_mins(10);

#[test]
fn bake_corpus_priors() {
    let Ok(corpus_root) = std::env::var("CALIBRATION_CORPUS") else { return };
    if let Ok(repo) = std::env::var("BAKE_ONE_REPO") {
        bake_single_repo(&PathBuf::from(repo), &PathBuf::from(std::env::var("BAKE_OUT").unwrap()));
        return;
    }
    drive_sweep(&PathBuf::from(&corpus_root));
}

fn bake_single_repo(repo: &Path, out: &Path) {
    let thresholds = t();
    let matcher = IgnoreMatcher::from_patterns(&[]);
    let filter = IgnoreFilter::new(&matcher, repo);
    let mut builder = PriorsBuilder::default();
    let summary = pulse::calibrate::accumulate(repo, &thresholds, &filter, &mut builder);
    std::fs::write(out, builder.to_json()).unwrap();
    eprintln!(
        "censused {} ({} main, {} test, {} vendored)",
        repo.display(),
        summary.main_files,
        summary.test_files,
        summary.vendored
    );
}

fn drive_sweep(corpus_root: &Path) {
    let exe = std::env::current_exe().unwrap();
    let snapshot_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/priors-snapshots");
    std::fs::create_dir_all(&snapshot_dir).unwrap();
    let mut crashed = Vec::new();
    for repo in repo_dirs(corpus_root) {
        let out = snapshot_dir.join(format!("{}.json", slug(&repo)));
        if out.exists() {
            eprintln!("skip (cached) {}", repo.display());
            continue;
        }
        let child = Command::new(&exe)
            .args(["priors_sweep::bake_corpus_priors", "--exact", "--nocapture"])
            .env("CALIBRATION_CORPUS", corpus_root)
            .env("BAKE_ONE_REPO", &repo)
            .env("BAKE_OUT", &out)
            .spawn()
            .unwrap();
        if let Some(reason) = wait_bounded(child) {
            eprintln!("{reason} on {}", repo.display());
            crashed.push(repo.clone());
        }
    }
    let mut builder = PriorsBuilder::default();
    let mut merged = 0usize;
    for entry in std::fs::read_dir(&snapshot_dir).unwrap().flatten() {
        if entry.path().extension().is_some_and(|e| e == "json")
            && builder.merge_json(&std::fs::read_to_string(entry.path()).unwrap())
        {
            merged += 1;
        }
    }
    let calibrate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/calibrate");
    std::fs::write(calibrate_dir.join("priors_histogram.json"), builder.to_json()).unwrap();
    write_priors_json(&builder, &calibrate_dir);
    eprintln!("priors written from {merged} repos; {} crashed: {crashed:?}", crashed.len());
}

fn write_priors_json(builder: &PriorsBuilder, calibrate_dir: &Path) {
    let table = builder.build(t().cpg.enabled);
    let out = calibrate_dir.join("priors.json");
    std::fs::write(out, serde_json::to_string(&table).unwrap()).unwrap();
}

#[test]
fn rebake_priors_json_from_histogram() {
    if std::env::var("REBAKE_PRIORS_JSON").is_err() {
        return;
    }
    let calibrate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/calibrate");
    let mut builder = PriorsBuilder::default();
    let histogram = std::fs::read_to_string(calibrate_dir.join("priors_histogram.json")).unwrap();
    assert!(builder.merge_json(&histogram), "committed histogram must parse");
    write_priors_json(&builder, &calibrate_dir);
}

fn wait_bounded(mut child: Child) -> Option<String> {
    let start = Instant::now();
    loop {
        match child.try_wait().unwrap() {
            Some(status) if status.success() => return None,
            Some(status) => return Some(format!("CRASH ({status})")),
            None if start.elapsed() > REPO_BUDGET => {
                let _ = child.kill();
                let _ = child.wait();
                return Some("TIMEOUT".to_string());
            }
            None => std::thread::sleep(Duration::from_secs(2)),
        }
    }
}

fn slug(repo: &Path) -> String {
    repo.components()
        .rev()
        .take(2)
        .filter_map(|c| c.as_os_str().to_str())
        .collect::<Vec<_>>()
        .join("_")
        .replace(['/', '.'], "_")
}

fn repo_dirs(root: &Path) -> Vec<PathBuf> {
    let mut repos = Vec::new();
    for lang_dir in subdirs(root) {
        repos.extend(subdirs(&lang_dir));
    }
    repos.sort();
    repos
}

fn subdirs(dir: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect()
}
