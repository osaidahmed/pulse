use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use pulse::audit::{corpus_df, record_extraction, walk_typed_source_files_filtered, IgnoreFilter};
use pulse::config::IgnoreMatcher;
use pulse::parse::Language;

use crate::common::t;
use crate::sweep_harness;

const DEPTH: usize = 4;
const WIDTH: usize = 1 << 17;

#[test]
fn bake_corpus_df() {
    let Ok(corpus_root) = std::env::var("CORPUS_DF_ROOT") else { return };
    if let Ok(repo) = std::env::var("CORPUS_DF_REPO") {
        bake_single_repo(&PathBuf::from(repo), &PathBuf::from(std::env::var("CORPUS_DF_OUT").unwrap()));
        return;
    }
    drive_sweep(&PathBuf::from(&corpus_root));
}

fn bake_single_repo(repo: &Path, out: &Path) {
    let thresholds = t().audit;
    let matcher = IgnoreMatcher::from_patterns(&[]);
    let filter = IgnoreFilter::new(&matcher, repo);
    let typed = walk_typed_source_files_filtered(repo, true, &filter);
    let records = record_extraction::records_only(&typed, &thresholds);

    let mut file_lang: HashMap<PathBuf, Language> = HashMap::new();
    for (path, lang) in &typed {
        file_lang.insert(path.clone(), *lang);
    }
    let mut by_file: HashMap<PathBuf, HashSet<u64>> = HashMap::new();
    for record in &records {
        by_file.entry(record.file.clone()).or_default().insert(record.fingerprint);
    }

    let mut df = corpus_df::with_dims(DEPTH, WIDTH);
    for (file, fingerprints) in &by_file {
        if let Some(&lang) = file_lang.get(file) {
            let tag = corpus_df::lang_tag(lang);
            for &fp in fingerprints {
                corpus_df::add(&mut df, tag, fp);
            }
        }
    }
    let mut lang_counts: HashMap<u32, u64> = HashMap::new();
    for (_path, lang) in &typed {
        *lang_counts.entry(corpus_df::lang_tag(*lang)).or_insert(0) += 1;
    }
    for (tag, total) in lang_counts {
        corpus_df::set_lang_total(&mut df, tag, total);
    }
    std::fs::write(out, corpus_df::to_bytes(&df)).unwrap();
    eprintln!("baked {} ({} files, {} records)", repo.display(), typed.len(), records.len());
}

fn drive_sweep(corpus_root: &Path) {
    let exe = std::env::current_exe().unwrap();
    let snapshot_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/corpus-df-snapshots");
    let crashed = sweep_harness::drive_per_repo(corpus_root, &snapshot_dir, "bin", |repo, out| {
        Command::new(&exe)
            .args(["corpus_df_sweep::bake_corpus_df", "--exact", "--nocapture"])
            .env("CORPUS_DF_ROOT", corpus_root)
            .env("CORPUS_DF_REPO", repo)
            .env("CORPUS_DF_OUT", out)
            .spawn()
            .unwrap()
    });
    let mut acc = corpus_df::with_dims(DEPTH, WIDTH);
    let mut merged = 0usize;
    for entry in std::fs::read_dir(&snapshot_dir).unwrap().flatten() {
        if entry.path().extension().is_some_and(|e| e == "bin") {
            if let Some(part) = corpus_df::from_bytes(&std::fs::read(entry.path()).unwrap()) {
                corpus_df::merge(&mut acc, &part);
                merged += 1;
            }
        }
    }
    let blob = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/audit/corpus_df.bin");
    std::fs::write(&blob, corpus_df::to_bytes(&acc)).unwrap();
    eprintln!("corpus_df baked from {merged} repos; {} crashed: {crashed:?}", crashed.len());
}

#[test]
fn inspect_corpus_df_distribution() {
    let Ok(corpus_root) = std::env::var("CORPUS_DF_CALIB") else { return };
    let df = corpus_df::corpus_df();
    let thresholds = t().audit;
    let min_support = thresholds.pattern_mining.freqt_min_support;
    let mut buckets = [0usize; 6];
    let mut total = 0usize;
    let repos = sweep_harness::repo_dirs(&PathBuf::from(&corpus_root));
    let stride = (repos.len() / 50).max(1);
    for repo in repos.iter().step_by(stride) {
        let name = repo.to_string_lossy();
        if name.contains("systemd") || name.contains("purchases-ios") {
            continue;
        }
        let matcher = IgnoreMatcher::from_patterns(&[]);
        let filter = IgnoreFilter::new(&matcher, repo);
        let typed = walk_typed_source_files_filtered(repo, true, &filter);
        let file_lang: HashMap<PathBuf, Language> = typed.iter().map(|(p, l)| (p.clone(), *l)).collect();
        let records = record_extraction::records_only(&typed, &thresholds);
        let mut support: HashMap<u64, usize> = HashMap::new();
        let mut fp_lang: HashMap<u64, Language> = HashMap::new();
        for record in &records {
            *support.entry(record.fingerprint).or_insert(0) += 1;
            if let Some(&lang) = file_lang.get(&record.file) {
                fp_lang.entry(record.fingerprint).or_insert(lang);
            }
        }
        for (fp, sup) in &support {
            if *sup >= min_support {
                if let Some(&lang) = fp_lang.get(fp) {
                    buckets[bucket(corpus_df::file_frequency(df, lang, *fp))] += 1;
                    total += 1;
                }
            }
        }
    }
    eprintln!("project-frequent patterns n={total}; corpus-freq buckets [<.01,<.05,<.1,<.2,<.5,>=.5] = {buckets:?}");
}

fn bucket(freq: f64) -> usize {
    if freq < 0.01 {
        0
    } else if freq < 0.05 {
        1
    } else if freq < 0.1 {
        2
    } else if freq < 0.2 {
        3
    } else if freq < 0.5 {
        4
    } else {
        5
    }
}
