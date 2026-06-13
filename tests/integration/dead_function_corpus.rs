use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use pulse::smells::Location;

fn subdirs(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> =
        std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
    v.sort();
    v
}

fn git(repo: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git").arg("-C").arg(repo).args(args).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn base_ref(repo: &Path, depth: u32) -> Option<String> {
    git(repo, &["rev-parse", &format!("HEAD~{depth}")])
        .or_else(|| {
            git(repo, &["rev-list", "--max-parents=0", "HEAD"]).map(|s| s.lines().next().unwrap_or("").to_string())
        })
        .filter(|s| !s.is_empty())
}

fn session_flags(repo: &Path, depth: u32) -> Vec<(String, String, u32)> {
    let Some(base) = base_ref(repo, depth) else { return Vec::new() };
    pulse::dead_function::added_unused_since(repo, &base)
        .into_iter()
        .flat_map(|(file, findings)| {
            findings.into_iter().filter_map(move |f| match f.location {
                Location::Function { name, start_line, .. } => Some((file.clone(), name, start_line)),
                Location::Module => None,
            })
        })
        .collect()
}

#[test]
fn scan_corpus_for_unused_functions() {
    let Ok(corpus) = std::env::var("SCAN_CORPUS") else { return };
    let out = std::env::var("SCAN_OUT").unwrap_or_else(|_| "/tmp/dead_scan.tsv".into());
    let only = std::env::var("SCAN_LANG").ok();
    let depth: u32 = std::env::var("SCAN_DEPTH").ok().and_then(|s| s.parse().ok()).unwrap_or(50);
    let root = PathBuf::from(&corpus);
    let _ = std::fs::write(&out, "");
    for lang_dir in subdirs(&root) {
        let lang = lang_dir.file_name().unwrap_or_default().to_string_lossy().into_owned();
        if only.as_ref().is_some_and(|l| l != &lang) {
            continue;
        }
        for repo in subdirs(&lang_dir) {
            let slug = repo.file_name().unwrap_or_default().to_string_lossy().into_owned();
            let flags = session_flags(&repo, depth);
            let mut block = String::new();
            for (file, name, line) in &flags {
                block.push_str(&format!("{lang}\t{slug}\t{file}\t{line}\t{name}\n"));
            }
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&out) {
                let _ = f.write_all(block.as_bytes());
            }
            eprintln!("{lang}/{slug}: {} flagged", flags.len());
        }
    }
}
